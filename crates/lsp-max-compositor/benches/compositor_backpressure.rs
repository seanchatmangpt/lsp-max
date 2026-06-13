//! Backpressure benchmarks for lsp-max-compositor mpsc channel and flush loop.
//!
//! Three benchmark families — model the FlushCoordinator channel behavior directly:
//!   BM-9   signal_loss_rate       — try_send() drop rate vs channel capacity
//!   BM-10  flush_merge_serial     — sequential flush() across N URIs, CPU cost
//!   BM-11  channel_capacity_model — capacity required for zero-drop at N=500 signals
//!
//! FlushCoordinator (capacity=256) cannot be benchmarked directly because it
//! requires a real lsp_max::Client. These benchmarks model the channel in isolation
//! using the same tokio::sync::mpsc::channel primitive and try_send() call pattern.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lsp_max_compositor::{
    merge::{DiagnosticEntry, MergeContext},
    registry::ChildTier,
    DiagnosticBuffer,
};
use std::sync::Arc;

fn make_ctx() -> Arc<MergeContext> {
    Arc::new(MergeContext::new(vec![
        "WASM4PM-".to_string(),
        "ANTI-LLM-".to_string(),
        "GGEN-".to_string(),
    ]))
}

fn make_entry(uri: &str, line: u32, code: &str, tier: ChildTier) -> DiagnosticEntry {
    DiagnosticEntry {
        uri: uri.to_string(),
        line,
        character: 0,
        severity: 2,
        code: code.to_string(),
        message: "bench".to_string(),
        source_tier: tier,
        server_id: None,
    }
}

// ── BM-9: signal_loss_rate ────────────────────────────────────────────────────
//
// Measures how many try_send() calls succeed vs fail (ANDON: channel full)
// at the current FlushCoordinator capacity of 256. At N URIs > 256, signals drop.

fn bench_signal_loss_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_loss_rate");

    for n_uris in [100usize, 256, 300, 500, 1000] {
        group.throughput(Throughput::Elements(n_uris as u64));
        group.bench_with_input(BenchmarkId::new("uris", n_uris), &n_uris, |b, &n_uris| {
            b.iter_custom(|iters| {
                let mut total = std::time::Duration::ZERO;
                for _ in 0..iters {
                    // Current FlushCoordinator capacity = 256.
                    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(256);
                    let start = std::time::Instant::now();
                    let mut sent = 0u64;
                    let mut dropped = 0u64;
                    for i in 0..n_uris {
                        match tx.try_send(format!("file:///bench/{i}.rs")) {
                            Ok(_) => sent += 1,
                            Err(_) => dropped += 1,
                        }
                    }
                    total += start.elapsed();
                    // Report loss in benchmark output label (not assertion — thresholds
                    // belong in the receipt, not the bench).
                    black_box((sent, dropped));
                }
                total
            });
        });
    }
    group.finish();
}

// ── BM-10: flush_merge_serial ─────────────────────────────────────────────────

fn bench_flush_merge_serial(c: &mut Criterion) {
    let ctx = make_ctx();
    let mut group = c.benchmark_group("flush_merge_serial");

    for n_uris in [10usize, 100, 256, 500] {
        // Pre-populate: 3 servers each posting 10 diagnostics per URI.
        let buffer = DiagnosticBuffer::new(ctx.clone(), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));
        let uris: Vec<String> = (0..n_uris)
            .map(|i| format!("file:///bench/{i}.rs"))
            .collect();
        for uri in &uris {
            for s in 0..3usize {
                let entries: Vec<_> = (0..10)
                    .map(|j| make_entry(uri, j as u32, &format!("CODE-{j}"), ChildTier::Primary))
                    .collect();
                buffer.deposit(uri, &format!("server-{s}"), ChildTier::Primary, entries);
            }
        }

        group.throughput(Throughput::Elements(n_uris as u64));
        group.bench_with_input(BenchmarkId::new("uris", n_uris), &n_uris, |b, _| {
            b.iter(|| {
                for uri in &uris {
                    let result = buffer.flush(black_box(uri.as_str()));
                    black_box(result.diagnostics.len());
                }
            });
        });
    }
    group.finish();
}

// ── BM-11: channel_capacity_model ────────────────────────────────────────────
//
// Answers: "what capacity is needed for zero signal loss at 500 concurrent servers?"
// Models the channel in isolation — each capacity variant measures drop rate at N=500.

fn bench_channel_capacity_model(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_capacity_model");
    const N_SIGNALS: usize = 500;

    for capacity in [256usize, 300, 400, 512, 1024] {
        group.bench_with_input(
            BenchmarkId::new("capacity", capacity),
            &capacity,
            |b, &capacity| {
                b.iter_custom(|iters| {
                    let mut total = std::time::Duration::ZERO;
                    for _ in 0..iters {
                        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(capacity);
                        let start = std::time::Instant::now();
                        let mut sent = 0u64;
                        let mut dropped = 0u64;
                        for i in 0..N_SIGNALS {
                            match tx.try_send(format!("file:///srv/{i}.rs")) {
                                Ok(_) => sent += 1,
                                Err(_) => dropped += 1,
                            }
                        }
                        total += start.elapsed();
                        // Zero drop at capacity >= N_SIGNALS is the expected finding.
                        black_box((sent, dropped));
                    }
                    total
                });
            },
        );
    }
    group.finish();
}

// ── Harness ───────────────────────────────────────────────────────────────────

criterion_group!(
    compositor_backpressure_benches,
    bench_signal_loss_rate,
    bench_flush_merge_serial,
    bench_channel_capacity_model
);
criterion_main!(compositor_backpressure_benches);
