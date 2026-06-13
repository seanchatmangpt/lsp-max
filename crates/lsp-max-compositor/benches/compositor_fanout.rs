//! Async subprocess fanout benchmarks for lsp-max-compositor.
//!
//! Four benchmark families — each spawns real lsp-echo-server subprocesses:
//!   BM-5  spawn_and_initialize_N  — OS-spawn latency + LSP handshake per N servers
//!   BM-6  fanout_did_open_serial  — current O(N×RTT) serial broadcast baseline
//!   BM-7  fanout_did_open_concurrent — O(max RTT) tokio::spawn fanout (proposed fix)
//!   BM-8  shutdown_N              — teardown cost for N initialized servers
//!
//! PID exhaustion mitigation:
//!   - BM-5 and BM-8 use iter_custom so processes are reaped within each sample.
//!   - BM-6 and BM-7 share a pre-spawned pool — zero PID growth during iter loop.
//!   - N=500 groups use sample_size(10) to cap total subprocess count.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use lsp_max::lsp_types::{DidOpenTextDocumentParams, TextDocumentItem, Url};
use lsp_max_compositor::child_process::ChildProcess;
use std::str::FromStr;

const LSP_ECHO_SERVER: &str = env!("CARGO_BIN_EXE_lsp-echo-server");

fn make_did_open_params() -> DidOpenTextDocumentParams {
    DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::from_str("file:///bench/test.rs").unwrap(),
            language_id: "rust".to_string(),
            version: 1,
            text: "fn main() {}".to_string(),
        },
    }
}

async fn spawn_n_initialized(n: usize) -> Vec<ChildProcess> {
    let mut procs = Vec::with_capacity(n);
    for i in 0..n {
        let (proc, _exit) = ChildProcess::spawn(format!("bench-{i}"), LSP_ECHO_SERVER, &[])
            .await
            .expect("echo server spawn");
        proc.initialize(None).await.expect("initialize");
        procs.push(proc);
    }
    procs
}

async fn teardown_all(procs: Vec<ChildProcess>) {
    for proc in procs {
        let _ = proc.handle.shutdown().await;
        proc.handle.exit().await;
    }
}

// ── BM-5: spawn_and_initialize_N ─────────────────────────────────────────────

fn bench_spawn_and_initialize(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("spawn_and_initialize");

    for n in [5usize, 50] {
        group.bench_with_input(BenchmarkId::new("N", n), &n, |b, &n| {
            b.iter_custom(|iters| {
                let mut total = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let start = std::time::Instant::now();
                    let procs = rt.block_on(spawn_n_initialized(n));
                    total += start.elapsed();
                    rt.block_on(teardown_all(procs));
                }
                total
            });
        });
    }

    // N=500 uses reduced sample_size to stay within PID limits.
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.bench_with_input(BenchmarkId::new("N", 500usize), &500usize, |b, &n| {
        b.iter_custom(|iters| {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..iters {
                let start = std::time::Instant::now();
                let procs = rt.block_on(spawn_n_initialized(n));
                total += start.elapsed();
                rt.block_on(teardown_all(procs));
            }
            total
        });
    });
    group.finish();
}

// ── BM-6 + BM-7: fanout_did_open — serial vs concurrent ──────────────────────

fn bench_fanout_did_open(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let params = make_did_open_params();

    for n in [5usize, 50, 500] {
        let procs = rt.block_on(spawn_n_initialized(n));
        let handles: Vec<_> = procs.iter().map(|p| p.handle.clone()).collect();

        let mut group = c.benchmark_group(format!("fanout_did_open_{n}"));
        if n == 500 {
            group.sample_size(10);
        }

        // BM-6: serial — current server.rs:264-267 loop pattern.
        let serial_handles = handles.clone();
        let serial_params = params.clone();
        group.bench_function("serial", |b| {
            b.iter(|| {
                rt.block_on(async {
                    for handle in &serial_handles {
                        handle.did_open(serial_params.clone()).await;
                    }
                });
            });
        });

        // BM-7: concurrent — tokio::spawn per handle.
        let concurrent_handles = handles.clone();
        let concurrent_params = params.clone();
        group.bench_function("concurrent", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let tasks: Vec<_> = concurrent_handles
                        .iter()
                        .map(|handle| {
                            let handle = handle.clone();
                            let p = concurrent_params.clone();
                            tokio::spawn(async move {
                                handle.did_open(p).await;
                            })
                        })
                        .collect();
                    for t in tasks {
                        let _ = t.await;
                    }
                });
            });
        });

        group.finish();

        // Teardown outside measurement window.
        rt.block_on(teardown_all(procs));
    }
}

// ── BM-8: shutdown_N ──────────────────────────────────────────────────────────

fn bench_shutdown(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("shutdown");

    for n in [5usize, 50] {
        group.bench_with_input(BenchmarkId::new("N", n), &n, |b, &n| {
            b.iter_custom(|iters| {
                let mut total = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let procs = rt.block_on(spawn_n_initialized(n));
                    let start = std::time::Instant::now();
                    rt.block_on(teardown_all(procs));
                    total += start.elapsed();
                }
                total
            });
        });
    }

    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.bench_with_input(BenchmarkId::new("N", 500usize), &500usize, |b, &n| {
        b.iter_custom(|iters| {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..iters {
                let procs = rt.block_on(spawn_n_initialized(n));
                let start = std::time::Instant::now();
                rt.block_on(teardown_all(procs));
                total += start.elapsed();
            }
            total
        });
    });
    group.finish();
}

// ── Harness ───────────────────────────────────────────────────────────────────

criterion_group!(
    compositor_fanout_benches,
    bench_spawn_and_initialize,
    bench_fanout_did_open,
    bench_shutdown
);
criterion_main!(compositor_fanout_benches);
