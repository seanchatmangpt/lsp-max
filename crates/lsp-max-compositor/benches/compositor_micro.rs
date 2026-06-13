//! Micro-benchmarks for lsp-max-compositor hot paths.
//!
//! Eight benchmark families — all sync, no subprocess, no async runtime:
//!   BM-1  deposit_contention            — papaya HashMap under N concurrent writers
//!   BM-2  flush_latency                 — merge cost at (N servers × K diagnostics per URI)
//!   BM-3  merge_diagnostics_cpu         — HashMap growth path vs REFUSED_BY_LAW sort branch
//!   BM-4  andon_prefix_match            — MergeContext::is_andon_for_server (daachorse automaton) at scale
//!   BM-5  deposit_andon_hot_path        — per-call deposit() cost: clean vs ANDON variants (highest-freq path)
//!   BM-6  multi_uri_flush_fanout        — flush M distinct URIs (real editor: 10–50 open files)
//!   BM-7  merge_context_construction    — MergeContext::new() automaton build time vs prefix count
//!   BM-8  signal_flush_channel_throughput — kanal try_send throughput: sequential + concurrent N senders
//!
//! Receipt written by `just bench-compositor`.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lsp_max_compositor::{
    merge::{merge_diagnostics, DiagnosticEntry, MergeContext},
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
        let buffer = Arc::new(DiagnosticBuffer::new(
            ctx.clone(),
            Arc::new(lsp_max_compositor::GateFile::from_path(
                std::path::PathBuf::from("/tmp/test-gate"),
            )),
        ));
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
        let buffer = DiagnosticBuffer::new(
            ctx.clone(),
            Arc::new(lsp_max_compositor::GateFile::from_path(
                std::path::PathBuf::from("/tmp/test-gate"),
            )),
        );
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
// Routes through MergeContext::is_andon_for_server — the production path using
// the daachorse DoubleArrayAhoCorasick automaton. The automaton is built once at
// MergeContext construction and reused across all calls, giving O(|code|) cost
// independent of prefix count. Both match and no-match should converge to ~10ns.

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

            // Build MergeContext once — automaton construction happens here, not in the hot loop.
            // Arc because DoubleArrayAhoCorasick doesn't implement Clone.
            let ctx = Arc::new(MergeContext::new(
                prefixes.iter().map(|s| s.to_string()).collect(),
            ));

            // Matching variant — code with a known-match prefix.
            let matching_code = "WASM4PM-CHEAT-C001";
            let ctx_ref = Arc::clone(&ctx);
            group.bench_function(format!("{id}_match"), |b| {
                b.iter(|| {
                    for _ in 0..call_count {
                        black_box(ctx_ref.is_andon_for_server(black_box(matching_code), None));
                    }
                });
            });

            // Non-matching variant — code that exhausts the entire automaton before returning false.
            // This is the *common case* in a clean workspace (no violations).
            let nonmatching_code = "RUST-E0001";
            let ctx_ref2 = Arc::clone(&ctx);
            group.bench_function(format!("{id}_no_match"), |b| {
                b.iter(|| {
                    for _ in 0..call_count {
                        black_box(ctx_ref2.is_andon_for_server(black_box(nonmatching_code), None));
                    }
                });
            });
        }
    }
    group.finish();
}

// ── BM-5: deposit_andon_hot_path ──────────────────────────────────────────────
// deposit() is called N×K times per keystroke cycle — it is the highest-frequency
// path in the compositor. This bench isolates the per-call cost at diagnostic
// volumes that match real editor sessions (10–100 diagnostics per server per file).
// Both clean (no ANDON) and dirty (ANDON prefixes present) variants measured
// because the automaton check runs on every entry regardless of match outcome.

fn bench_deposit_andon_hot_path(c: &mut Criterion) {
    let ctx = make_ctx();
    let mut group = c.benchmark_group("deposit_andon_hot_path");

    for k in [10usize, 100, 500] {
        let gate = Arc::new(lsp_max_compositor::GateFile::from_path(
            std::path::PathBuf::from("/tmp/test-gate-deposit"),
        ));

        // Clean entries — no ANDON prefix. Common case in a healthy workspace.
        let clean_entries: Vec<DiagnosticEntry> = (0..k)
            .map(|j| {
                make_entry(
                    "file:///bench/a.rs",
                    j as u32,
                    0,
                    &format!("RUST-E{j:04}"),
                    "type error",
                    1,
                    ChildTier::Primary,
                )
            })
            .collect();

        // Dirty entries — every entry has an ANDON prefix. Gate-triggering worst case.
        let dirty_entries: Vec<DiagnosticEntry> = (0..k)
            .map(|j| {
                make_entry(
                    "file:///bench/b.rs",
                    j as u32,
                    0,
                    &format!("WASM4PM-CHEAT-C{j:04}"),
                    "law violation",
                    1,
                    ChildTier::Primary,
                )
            })
            .collect();

        group.throughput(Throughput::Elements(k as u64));

        let buf_clean = Arc::new(DiagnosticBuffer::new(ctx.clone(), Arc::clone(&gate)));
        let clean = clean_entries.clone();
        group.bench_function(format!("clean_{k}diags"), |b| {
            b.iter(|| {
                buf_clean.deposit(
                    black_box("file:///bench/a.rs"),
                    black_box("server-0"),
                    ChildTier::Primary,
                    black_box(clean.clone()),
                );
            });
        });

        let buf_dirty = Arc::new(DiagnosticBuffer::new(ctx.clone(), Arc::clone(&gate)));
        let dirty = dirty_entries.clone();
        group.bench_function(format!("andon_{k}diags"), |b| {
            b.iter(|| {
                buf_dirty.deposit(
                    black_box("file:///bench/b.rs"),
                    black_box("server-0"),
                    ChildTier::Primary,
                    black_box(dirty.clone()),
                );
            });
        });
    }
    group.finish();
}

// ── BM-6: multi_uri_flush_fanout ──────────────────────────────────────────────
// Real editor sessions have 10–50 open files. The coordinator flushes per URI;
// flushing M URIs in sequence means M HashMap lookups + M merge calls.
// This bench measures the flush fanout cost vs single-URI baseline.

fn bench_multi_uri_flush_fanout(c: &mut Criterion) {
    let ctx = make_ctx();
    let mut group = c.benchmark_group("multi_uri_flush_fanout");

    for m in [1usize, 10, 50] {
        let gate = Arc::new(lsp_max_compositor::GateFile::from_path(
            std::path::PathBuf::from("/tmp/test-gate-multi"),
        ));
        let buffer = Arc::new(DiagnosticBuffer::new(ctx.clone(), Arc::clone(&gate)));
        let uris: Vec<String> = (0..m)
            .map(|i| format!("file:///bench/file{i}.rs"))
            .collect();

        // Pre-populate: 5 servers × 20 diagnostics for each URI.
        for uri in &uris {
            for s in 0..5usize {
                let entries: Vec<_> = (0..20)
                    .map(|j| {
                        make_entry(
                            uri,
                            j as u32,
                            0,
                            &format!("CODE-{j}"),
                            "msg",
                            2,
                            ChildTier::Primary,
                        )
                    })
                    .collect();
                buffer.deposit(uri, &format!("server-{s}"), ChildTier::Primary, entries);
            }
        }

        group.throughput(Throughput::Elements(m as u64));
        let uris_ref = uris.clone();
        let buf_ref = Arc::clone(&buffer);
        group.bench_with_input(BenchmarkId::new("M_uris", m), &m, |b, _| {
            b.iter(|| {
                for uri in &uris_ref {
                    let result = buf_ref.flush(black_box(uri));
                    black_box(result.diagnostics.len());
                }
            });
        });
    }
    group.finish();
}

// ── BM-7: merge_context_construction ──────────────────────────────────────────
// MergeContext::new() builds the daachorse automaton from the prefix list.
// This is a one-shot cost at server startup and on lsp-max.toml reload.
// For large deployments (many servers with per-server overrides), the union prefix
// set can grow; this bench measures construction time vs prefix count so we can
// set a budget for startup latency.

fn bench_merge_context_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_context_construction");

    let prefix_sets: &[(&str, &[&str])] = &[
        ("3_prefixes", &["WASM4PM-", "ANTI-LLM-", "GGEN-"]),
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
        (
            "50_prefixes",
            // Simulate a large enterprise deployment with many diagnostic namespaces.
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
                "NS01-",
                "NS02-",
                "NS03-",
                "NS04-",
                "NS05-",
                "NS06-",
                "NS07-",
                "NS08-",
                "NS09-",
                "NS10-",
                "NS11-",
                "NS12-",
                "NS13-",
                "NS14-",
                "NS15-",
                "NS16-",
                "NS17-",
                "NS18-",
                "NS19-",
                "NS20-",
                "NS21-",
                "NS22-",
                "NS23-",
                "NS24-",
                "NS25-",
                "NS26-",
                "NS27-",
                "NS28-",
                "NS29-",
                "NS30-",
                "NS31-",
                "NS32-",
                "NS33-",
                "NS34-",
                "NS35-",
                "NS36-",
                "NS37-",
                "NS38-",
                "NS39-",
                "NS40-",
            ],
        ),
    ];

    for (label, prefixes) in prefix_sets {
        let prefix_vec: Vec<String> = prefixes.iter().map(|s| s.to_string()).collect();
        group.bench_function(format!("build_{label}"), |b| {
            b.iter(|| {
                // Construction is the entire cost — automaton compiled here.
                let ctx = MergeContext::new(black_box(prefix_vec.clone()));
                black_box(ctx);
            });
        });
    }
    group.finish();
}

// ── BM-8: signal_flush_channel_throughput ─────────────────────────────────────
// signal_flush() is called after every deposit() — N×K times per keystroke cycle.
// It uses kanal try_send (non-blocking). This bench measures raw channel send
// throughput: how fast can we enqueue FlushSignal structs before backpressure kicks in.
// Tests both sequential (single-threaded accumulation) and concurrent (N threads,
// mirrors real compositor_client fanout) variants.

fn bench_signal_flush_channel_throughput(c: &mut Criterion) {
    use lsp_max_compositor::flush_coordinator::FlushSignal;

    let mut group = c.benchmark_group("signal_flush_channel_throughput");

    // Sequential: one sender, measure raw try_send cost with headroom in the channel.
    for batch in [100usize, 500, 1_000] {
        group.throughput(Throughput::Elements(batch as u64));
        group.bench_function(format!("sequential_{batch}sends"), |b| {
            b.iter(|| {
                // Use a fresh channel per iteration so it never fills up.
                let (tx, _rx) = kanal::bounded::<FlushSignal>(batch + 1);
                for i in 0..batch {
                    let sig = FlushSignal {
                        uri: format!("file:///bench/file{i}.rs"),
                        server_id: format!("server-{i}"),
                    };
                    let _ = tx.try_send(black_box(sig));
                }
            });
        });
    }

    // Concurrent: N threads each send 1 signal — mirrors the deposit() → signal_flush()
    // call pattern where each CompositorClient is on its own tokio task.
    for n in [10usize, 100, 500] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(format!("concurrent_{n}senders"), |b| {
            b.iter(|| {
                let (tx, _rx) = kanal::bounded::<FlushSignal>(n + 1);
                std::thread::scope(|s| {
                    for i in 0..n {
                        let tx = tx.clone();
                        s.spawn(move || {
                            let sig = FlushSignal {
                                uri: "file:///bench/main.rs".to_string(),
                                server_id: format!("server-{i}"),
                            };
                            let _ = tx.try_send(black_box(sig));
                        });
                    }
                });
            });
        });
    }
    group.finish();
}

// ── Harness ───────────────────────────────────────────────────────────────────

criterion_group!(
    compositor_micro_benches,
    bench_deposit_contention,
    bench_flush_latency,
    bench_merge_diagnostics_cpu,
    bench_andon_prefix_match,
    bench_deposit_andon_hot_path,
    bench_multi_uri_flush_fanout,
    bench_merge_context_construction,
    bench_signal_flush_channel_throughput,
);
criterion_main!(compositor_micro_benches);
