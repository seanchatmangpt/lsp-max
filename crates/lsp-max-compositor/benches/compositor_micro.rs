//! Micro-benchmarks for lsp-max-compositor hot paths.
//!
//! Four benchmark families — all sync, no subprocess, no async runtime:
//!   BM-1  deposit_contention   — DashMap shard lock under N concurrent writers
//!   BM-2  flush_latency        — merge cost at (N servers × K diagnostics per URI)
//!   BM-3  merge_diagnostics_cpu — HashMap growth path vs REFUSED_BY_LAW sort branch
//!   BM-4  andon_prefix_match   — is_refused_by_law_with_prefixes() at scale
//!
//! Receipt written by `just bench-compositor`.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lsp_max_compositor::{
    merge::{is_refused_by_law_with_prefixes, merge_diagnostics, DiagnosticEntry, MergeContext},
    registry::ChildTier,
    DiagnosticBuffer,
};
use std::sync::Arc;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_entry(
    uri: &str,
    line: u32,
    character: u32,
    code: &str,
    message: &str,
    severity: u8,
    tier: ChildTier,
) -> DiagnosticEntry {
    DiagnosticEntry {
        uri: uri.to_string(),
        line,
        character,
        severity,
        code: code.to_string(),
        message: message.to_string(),
        source_tier: tier,
        server_id: None,
    }
}

fn make_ctx() -> Arc<MergeContext> {
    Arc::new(MergeContext::new(vec![
        "WASM4PM-".to_string(),
        "ANTI-LLM-".to_string(),
        "GGEN-".to_string(),
    ]))
}

// ── BM-1: deposit_contention ──────────────────────────────────────────────────

fn bench_deposit_contention(c: &mut Criterion) {
    let ctx = make_ctx();
    let mut group = c.benchmark_group("deposit_contention");

    for n in [5usize, 50, 500] {
        let buffer = Arc::new(DiagnosticBuffer::new(ctx.clone(), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate")))));
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("N", n), &n, |b, &n| {
            let entries: Vec<_> = (0..n)
                .map(|i| {
                    vec![make_entry(
                        "file:///bench/test.rs",
                        i as u32,
                        0,
                        "RUST-E0001",
                        "bench diagnostic",
                        1,
                        ChildTier::Primary,
                    )]
                })
                .collect();
            b.iter(|| {
                // All N server_ids deposit to the same URI — worst-case single-shard contention.
                std::thread::scope(|s| {
                    for (i, ents) in entries.iter().enumerate() {
                        let buf = buffer.clone();
                        let ents = ents.clone();
                        s.spawn(move || {
                            buf.deposit(
                                black_box("file:///bench/test.rs"),
                                black_box(&format!("server-{i}")),
                                ChildTier::Primary,
                                ents,
                            );
                        });
                    }
                });
            });
        });
    }
    group.finish();
}

// ── BM-2: flush_latency ───────────────────────────────────────────────────────

fn bench_flush_latency(c: &mut Criterion) {
    let ctx = make_ctx();
    let mut group = c.benchmark_group("flush_latency");

    for (n, k) in [
        (5usize, 10usize),
        (5, 100),
        (50, 10),
        (50, 100),
        (500, 10),
        (500, 100),
    ] {
        let buffer = DiagnosticBuffer::new(ctx.clone(), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));
        // Pre-populate: N servers × K diagnostics each for the same URI.
        for i in 0..n {
            let entries: Vec<_> = (0..k)
                .map(|j| {
                    make_entry(
                        "file:///bench/main.rs",
                        j as u32,
                        0,
                        &format!("CODE-{j}"),
                        "msg",
                        2,
                        ChildTier::Primary,
                    )
                })
                .collect();
            buffer.deposit(
                "file:///bench/main.rs",
                &format!("server-{i}"),
                ChildTier::Primary,
                entries,
            );
        }

        group.throughput(Throughput::Elements((n * k) as u64));
        group.bench_with_input(
            BenchmarkId::new("N_x_K", format!("{n}x{k}")),
            &(n, k),
            |b, _| {
                b.iter(|| {
                    let result = buffer.flush(black_box("file:///bench/main.rs"));
                    black_box(result.diagnostics.len());
                });
            },
        );
    }
    group.finish();
}

// ── BM-3: merge_diagnostics_cpu ───────────────────────────────────────────────

fn bench_merge_diagnostics_cpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_diagnostics_cpu");
    let n = 500usize;
    let k = 100usize;

    // Variant A: distinct non-law codes — exercises HashMap growth path.
    let distinct_inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)> = (0..n)
        .map(|i| {
            let entries = (0..k)
                .map(|j| {
                    make_entry(
                        "file:///a.rs",
                        j as u32,
                        0,
                        &format!("CODE-{i}-{j}"),
                        "m",
                        2,
                        ChildTier::Primary,
                    )
                })
                .collect();
            (ChildTier::Primary, entries)
        })
        .collect();

    group.throughput(Throughput::Elements((n * k) as u64));
    group.bench_function("distinct_keys_500x100", |b| {
        b.iter(|| {
            let result = merge_diagnostics(black_box(distinct_inputs.clone()), None);
            black_box(result.len());
        });
    });

    // Variant B: REFUSED_BY_LAW codes — exercises the ANDON sort branch.
    let law_inputs: Vec<(ChildTier, Vec<DiagnosticEntry>)> = (0..n)
        .map(|i| {
            let entries = (0..k)
                .map(|j| {
                    make_entry(
                        "file:///b.rs",
                        j as u32,
                        0,
                        &format!("WASM4PM-CHEAT-C{i}{j}"),
                        "law violation",
                        1,
                        if i % 2 == 0 {
                            ChildTier::Primary
                        } else {
                            ChildTier::DiagnosticsOnly
                        },
                    )
                })
                .collect();
            (ChildTier::Primary, entries)
        })
        .collect();

    group.bench_function("law_codes_500x100", |b| {
        b.iter(|| {
            let result = merge_diagnostics(black_box(law_inputs.clone()), None);
            black_box(result.len());
        });
    });
    group.finish();
}

// ── BM-4: andon_prefix_match ──────────────────────────────────────────────────

fn bench_andon_prefix_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("andon_prefix_match");

    let prefix_sets: &[(&str, &[&str])] = &[
        ("3_prefixes", &["WASM4PM-", "ANTI-LLM-", "GGEN-"]),
        (
            "5_prefixes",
            &["WASM4PM-", "ANTI-LLM-", "GGEN-", "CROWN-", "DRIFT-"],
        ),
        (
            "10_prefixes",
            &[
                "WASM4PM-",
                "ANTI-LLM-",
                "GGEN-",
                "CROWN-",
                "DRIFT-",
                "OCEL-",
                "FM5-",
                "EARL-",
                "SPC-",
                "BLAKE3-",
            ],
        ),
    ];

    // 50_000 calls mirrors 500 servers × 100 entries × 1 sort-comparator per entry.
    for call_count in [100usize, 1_000, 50_000] {
        for (prefix_label, prefixes) in prefix_sets {
            let id = format!("{prefix_label}_{call_count}calls");

            // Matching variant — early exit on first prefix match.
            let matching_code = "WASM4PM-CHEAT-C001";
            group.bench_function(format!("{id}_match"), |b| {
                b.iter(|| {
                    for _ in 0..call_count {
                        black_box(is_refused_by_law_with_prefixes(
                            black_box(matching_code),
                            black_box(prefixes),
                        ));
                    }
                });
            });

            // Non-matching variant — exhausts all prefixes.
            let nonmatching_code = "RUST-E0001";
            group.bench_function(format!("{id}_no_match"), |b| {
                b.iter(|| {
                    for _ in 0..call_count {
                        black_box(is_refused_by_law_with_prefixes(
                            black_box(nonmatching_code),
                            black_box(prefixes),
                        ));
                    }
                });
            });
        }
    }
    group.finish();
}

// ── Harness ───────────────────────────────────────────────────────────────────

criterion_group!(
    compositor_micro_benches,
    bench_deposit_contention,
    bench_flush_latency,
    bench_merge_diagnostics_cpu,
    bench_andon_prefix_match
);
criterion_main!(compositor_micro_benches);
