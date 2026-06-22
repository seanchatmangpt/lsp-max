//! Process mining micro-benchmarks — Van der Aalst DFG + Declare conformance.
//!
//! Five benchmark families (all sync, no subprocess, no async runtime):
//!   PM-1  dfg_construction     — DirectlyFollowsGraph::from_traces + from_events at 100/1K/10K
//!   PM-2  declare_conformance  — DeclareModel::check + fitness at 100/1K/10K cases
//!   PM-3  trace_extraction     — extract_traces() at 1K/10K/100K OCEL events
//!   PM-4  dfg_fitness          — fitness_against_model + precision_against_model
//!   PM-5  dfg_rendering        — to_mermaid() and to_dot() on representative DFGs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lsp_max_compositor::declare::{extract_traces, DeclareModel};
use lsp_max_compositor::dfg::DirectlyFollowsGraph;
use serde_json::json;
use std::collections::HashMap;

// ── Helpers ───────────────────────────────────────────────────────────────────

const COMPOSITOR_ACTIVITIES: &[&str] = &[
    "CompositorFlush",
    "CompositorFlushAdmitted",
    "CompositorFlushBlocked",
    "AndonCodePresent",
];

fn conformant_trace() -> Vec<String> {
    vec![
        "CompositorFlush".to_owned(),
        "CompositorFlushAdmitted".to_owned(),
    ]
}

fn violating_trace() -> Vec<String> {
    vec![
        "CompositorFlush".to_owned(),
        "CompositorFlushBlocked".to_owned(),
    ]
}

/// n cases: 1/3 violating, 2/3 conformant.
fn make_traces(n: usize) -> HashMap<String, Vec<String>> {
    (0..n)
        .map(|i| {
            let trace = if i % 3 == 0 {
                violating_trace()
            } else {
                conformant_trace()
            };
            (format!("file:///bench/{i}.rs"), trace)
        })
        .collect()
}

/// n OCEL events cycling through all compositor activity types, grouped into cases of ~3.
fn make_ocel_events(n: usize) -> Vec<serde_json::Value> {
    (0..n)
        .map(|i| {
            let case_id = format!("file:///bench/{}.rs", i / 3);
            let event_type = COMPOSITOR_ACTIVITIES[i % COMPOSITOR_ACTIVITIES.len()];
            json!({
                "type": event_type,
                "attributes": { "uri": case_id }
            })
        })
        .collect()
}

fn normative_arcs() -> Vec<(String, String)> {
    vec![
        (
            "CompositorFlush".to_string(),
            "CompositorFlushAdmitted".to_string(),
        ),
        (
            "CompositorFlush".to_string(),
            "CompositorFlushBlocked".to_string(),
        ),
        (
            "CompositorFlushBlocked".to_string(),
            "AndonCodePresent".to_string(),
        ),
    ]
}

// ── PM-1: dfg_construction ────────────────────────────────────────────────────
// Measures the cost of building the DFG HashMap from pre-built trace maps,
// and from raw OCEL JSON (which includes trace extraction internally).

fn bench_dfg_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("dfg_construction");

    for n in [100usize, 1_000, 10_000] {
        let traces = make_traces(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("from_traces", n), &n, |b, _| {
            b.iter(|| {
                let dfg = DirectlyFollowsGraph::from_traces(black_box(&traces));
                black_box(dfg.edge_count());
            });
        });
    }

    for n in [1_000usize, 10_000] {
        let events = make_ocel_events(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("from_events", n), &n, |b, _| {
            b.iter(|| {
                let dfg = DirectlyFollowsGraph::from_events(black_box(&events));
                black_box(dfg.edge_count());
            });
        });
    }

    group.finish();
}

// ── PM-2: declare_conformance ─────────────────────────────────────────────────
// Measures DeclareModel::check (returns violation list) and fitness (conformance
// score). Both the compositor model (5 constraints) and anti-llm model (6
// constraints) are measured so the constraint-count scaling is visible.

fn bench_declare_conformance(c: &mut Criterion) {
    let compositor_model = DeclareModel::compositor();
    let detection_model = DeclareModel::anti_llm_detection();
    let mut group = c.benchmark_group("declare_conformance");

    // Compositor model: 5 constraints.
    for n in [100usize, 1_000, 10_000] {
        let traces = make_traces(n);
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::new("compositor_check", n), &n, |b, _| {
            b.iter(|| {
                let violations = compositor_model.check(black_box(&traces));
                black_box(violations.len());
            });
        });

        group.bench_with_input(BenchmarkId::new("compositor_fitness", n), &n, |b, _| {
            b.iter(|| {
                let score = compositor_model.fitness(black_box(&traces));
                black_box(score);
            });
        });
    }

    // Anti-llm detection model: 6 constraints, longer activity names.
    let detection_traces: HashMap<String, Vec<String>> = (0..1_000)
        .map(|i| {
            let trace: Vec<String> = if i % 5 == 0 {
                // Conformant with CheatDetected path.
                vec![
                    "ScanComplete".to_owned(),
                    "DetectionClaim".to_owned(),
                    "NegativeControlExecuted".to_owned(),
                    "CheatDetected".to_owned(),
                    "FailsetUpdated".to_owned(),
                ]
            } else {
                // Minimal conformant.
                vec!["ScanComplete".to_owned()]
            };
            (format!("scan-case-{i}"), trace)
        })
        .collect();

    group.throughput(Throughput::Elements(1_000));
    group.bench_function("anti_llm_check_1k", |b| {
        b.iter(|| {
            let v = detection_model.check(black_box(&detection_traces));
            black_box(v.len());
        });
    });
    group.bench_function("anti_llm_fitness_1k", |b| {
        b.iter(|| {
            let s = detection_model.fitness(black_box(&detection_traces));
            black_box(s);
        });
    });

    group.finish();
}

// ── PM-3: trace_extraction ────────────────────────────────────────────────────
// extract_traces() is the first stage of every DFG and Declare run: it groups
// raw OCEL JSON events into per-case activity sequences. Measured at three
// input scales to expose HashMap resizing and allocation cost.

fn bench_trace_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("trace_extraction");

    for n in [1_000usize, 10_000, 100_000] {
        let events = make_ocel_events(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("extract_traces", n), &n, |b, _| {
            b.iter(|| {
                let traces = extract_traces(black_box(&events));
                black_box(traces.len());
            });
        });
    }

    group.finish();
}

// ── PM-4: dfg_fitness_precision ───────────────────────────────────────────────
// fitness_against_model and precision_against_model are O(k) in the number of
// observed edges; they run on an already-built DFG. The DFG is constructed once
// outside the hot loop.

fn bench_dfg_fitness_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("dfg_fitness_precision");
    let arcs = normative_arcs();

    for n in [100usize, 1_000, 10_000] {
        let traces = make_traces(n);
        let dfg = DirectlyFollowsGraph::from_traces(&traces);

        group.throughput(Throughput::Elements(dfg.edge_count() as u64));

        group.bench_with_input(BenchmarkId::new("fitness", n), &n, |b, _| {
            b.iter(|| {
                let score = dfg.fitness_against_model(black_box(&arcs));
                black_box(score);
            });
        });

        group.bench_with_input(BenchmarkId::new("precision", n), &n, |b, _| {
            b.iter(|| {
                let score = dfg.precision_against_model(black_box(&arcs));
                black_box(score);
            });
        });
    }

    group.finish();
}

// ── PM-5: dfg_rendering ───────────────────────────────────────────────────────
// to_mermaid() and to_dot() both do a sort + String push per node/edge.
// Measured at 100/1K/10K-case DFGs (edge count is bounded by activity count²,
// so it plateaus quickly — the interesting variable here is String allocation).

fn bench_dfg_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("dfg_rendering");

    for n in [100usize, 1_000, 10_000] {
        let traces = make_traces(n);
        let dfg = DirectlyFollowsGraph::from_traces(&traces);
        let edges = dfg.edge_count();

        group.throughput(Throughput::Elements(edges as u64));

        group.bench_with_input(BenchmarkId::new("to_mermaid", n), &n, |b, _| {
            b.iter(|| {
                let out = dfg.to_mermaid();
                black_box(out.len());
            });
        });

        group.bench_with_input(BenchmarkId::new("to_dot", n), &n, |b, _| {
            b.iter(|| {
                let out = dfg.to_dot();
                black_box(out.len());
            });
        });
    }

    group.finish();
}

// ── Harness ───────────────────────────────────────────────────────────────────

criterion_group!(
    process_mining_benches,
    bench_dfg_construction,
    bench_declare_conformance,
    bench_trace_extraction,
    bench_dfg_fitness_precision,
    bench_dfg_rendering,
);
criterion_main!(process_mining_benches);
