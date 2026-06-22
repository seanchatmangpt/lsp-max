//! Genetic search and Pareto optimization micro-benchmarks.
//!
//! Five benchmark families (all sync, no subprocess, no async runtime):
//!   PS-1  prng_throughput       — xorshift64 PRNG: next_u64, next_f64, next_usize
//!   PS-2  objectives_evaluation — Objectives::evaluate on pipelines of 1–20 nodes
//!   PS-3  pareto_dominance      — Objectives::dominates pairwise on 100 pre-scored pipelines
//!   PS-4  pipeline_search       — PipelineSearch::run at pop_size 20/50/100 × 10 gens
//!   PS-5  pareto_search         — ParetoSearch::run at pop_size 20/50/100 × 10 gens

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lsp_max::pipeline::catalog::KNOWN_BREEDS;
use lsp_max::pipeline::pareto::{Objectives, ParetoSearch};
use lsp_max::pipeline::search::{DiversityFitnessEvaluator, PipelineSearch, Prng};
use lsp_max::pipeline::types::PipelineSearchConfig;

// ── Helpers ───────────────────────────────────────────────────────────────────

const BENCH_BREEDS: &[&str] = &[
    "cbr",
    "asp",
    "bayesian_network",
    "ltl_monitor",
    "frame",
    "production_rules",
    "htn_planning",
    "soar",
    "prolog",
    "strips",
];

fn default_cfg(pop_size: usize) -> PipelineSearchConfig {
    PipelineSearchConfig {
        population_size: pop_size,
        generations: 10,
        ..Default::default()
    }
}

// ── PS-1: prng_throughput ─────────────────────────────────────────────────────
// The xorshift64 PRNG drives all stochastic decisions in PipelineSearch and
// ParetoSearch. This bench establishes the raw per-call cost floor.

fn bench_prng_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("prng_throughput");

    group.throughput(Throughput::Elements(1_000));

    group.bench_function("next_u64_1k", |b| {
        b.iter(|| {
            let mut rng = Prng::new(42);
            let mut acc = 0u64;
            for _ in 0..1_000 {
                acc ^= rng.next_u64();
            }
            black_box(acc);
        });
    });

    group.bench_function("next_f64_1k", |b| {
        b.iter(|| {
            let mut rng = Prng::new(42);
            let mut acc = 0.0f64;
            for _ in 0..1_000 {
                acc += rng.next_f64();
            }
            black_box(acc);
        });
    });

    // next_usize(56) exercises the modulo path — 56 = KNOWN_BREEDS.len().
    group.bench_function("next_usize_56_1k", |b| {
        b.iter(|| {
            let mut rng = Prng::new(42);
            let mut acc = 0usize;
            for _ in 0..1_000 {
                acc ^= rng.next_usize(56);
            }
            black_box(acc);
        });
    });

    group.finish();
}

// ── PS-2: objectives_evaluation ───────────────────────────────────────────────
// Objectives::evaluate computes four bounded scores on a breed sequence.
// The inner loop uses Vec::contains for deduplication (no Hash requirement on
// BreedCategory). Pipeline length is the main cost variable.

fn bench_objectives_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("objectives_evaluation");

    for len in [1usize, 3, 5, 10, 20] {
        let breeds: Vec<String> = (0..len)
            .map(|i| KNOWN_BREEDS[i % KNOWN_BREEDS.len()].to_string())
            .collect();

        group.throughput(Throughput::Elements(len as u64));
        group.bench_with_input(BenchmarkId::new("evaluate_len", len), &len, |b, _| {
            b.iter(|| {
                let obj = Objectives::evaluate(black_box(&breeds));
                black_box(obj.scalarized());
            });
        });
    }

    group.finish();
}

// ── PS-3: pareto_dominance ────────────────────────────────────────────────────
// Objectives::dominates is a fixed O(4) comparison but is called O(n²) times
// during merge_front. This bench isolates the raw pairwise cost at n=100 so
// we can bound the merge_front quadratic term.

fn bench_pareto_dominance(c: &mut Criterion) {
    let mut group = c.benchmark_group("pareto_dominance");

    let samples: Vec<Objectives> = (0..100)
        .map(|i| {
            let len = (i % 5) + 1;
            let breeds: Vec<String> = (0..len)
                .map(|j| KNOWN_BREEDS[(i * 3 + j) % KNOWN_BREEDS.len()].to_string())
                .collect();
            Objectives::evaluate(&breeds)
        })
        .collect();

    group.throughput(Throughput::Elements((samples.len() * samples.len()) as u64));
    group.bench_function("pairwise_100x100", |b| {
        b.iter(|| {
            let mut count = 0usize;
            for a in &samples {
                for b_obj in &samples {
                    if a.dominates(black_box(b_obj)) {
                        count += 1;
                    }
                }
            }
            black_box(count);
        });
    });

    group.finish();
}

// ── PS-4: pipeline_search ─────────────────────────────────────────────────────
// Measures the full PipelineSearch::run wall-time including population init,
// evaluation, tournament selection, crossover, and mutation.
// pop_size × generations is used as the throughput unit (evaluations budget).

fn bench_pipeline_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_search");

    for pop_size in [20usize, 50, 100] {
        let cfg = default_cfg(pop_size);
        let evals = pop_size * cfg.generations;

        group.throughput(Throughput::Elements(evals as u64));
        group.bench_with_input(
            BenchmarkId::new("run_10_gens", pop_size),
            &pop_size,
            |b, _| {
                b.iter(|| {
                    let mut search = PipelineSearch::new(
                        black_box(cfg.clone()),
                        BENCH_BREEDS,
                        Box::new(DiversityFitnessEvaluator),
                        black_box(42u64),
                    );
                    let result = search.run();
                    black_box(result.evaluations);
                });
            },
        );
    }

    group.finish();
}

// ── PS-5: pareto_search ───────────────────────────────────────────────────────
// ParetoSearch::run exercises the full search loop including merge_front (the
// O(n²) Pareto dominance check). Uses the full KNOWN_BREEDS pool (56 breeds)
// so the breed-category spread is representative of real runs.

fn bench_pareto_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("pareto_search");

    for pop_size in [20usize, 50, 100] {
        let cfg = PipelineSearchConfig {
            population_size: pop_size,
            generations: 10,
            admission_threshold: 0.6,
            ..Default::default()
        };
        let evals = pop_size * cfg.generations;

        group.throughput(Throughput::Elements(evals as u64));
        group.bench_with_input(
            BenchmarkId::new("run_10_gens", pop_size),
            &pop_size,
            |b, _| {
                b.iter(|| {
                    let mut search = ParetoSearch::new(black_box(cfg.clone()), black_box(42u64));
                    let result = search.run();
                    black_box(result.front.len());
                });
            },
        );
    }

    group.finish();
}

// ── Harness ───────────────────────────────────────────────────────────────────

criterion_group!(
    pipeline_search_benches,
    bench_prng_throughput,
    bench_objectives_evaluation,
    bench_pareto_dominance,
    bench_pipeline_search,
    bench_pareto_search,
);
criterion_main!(pipeline_search_benches);
