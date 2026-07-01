# Performance Tuning & Profiling Guide

This document outlines performance expectations, profiling workflows, and optimization strategies for **lsp-max**: the law-state LSP runtime. It covers test budgets, criterion benchmarking, flamegraph profiling, memory layout, and scaling limits.

---

## 1. Test Time Budgets

The workspace enforces strict test execution time targets to ensure CI remains fast and developer iteration loops stay responsive.

### Default Test Budgets

| Test Category | Wall-Clock Limit | Rationale |
|---|---|---|
| Unit tests (`#[test]`) | ≤ 5 seconds | Local dev iteration; invoked on every build |
| Integration tests (`tests/`) | ≤ 5 seconds | Composition layer and end-to-end verification |
| Ignored tests (`#[ignore]`) | ≤ 30 seconds | Slow paths: subprocess spawning, 50-500 servers, fuzzing |
| E2E subprocess tests | ≤ 30 seconds | Real child processes, realistic LSP I/O |

**Rationale**: A 5-second test suite enables developers to re-run full test suites in <10 milliseconds per commit (acceptable regression detection latency). Ignored tests allow deep stress testing without blocking normal iteration.

### Timeout Reduction Philosophy

Tests use `Duration::from_millis()` rather than `from_secs()` to enforce sub-second granularity and expose slow paths early.

**Pattern**: If a test sleeps, debounces, or polls, use millisecond timeouts that are 2-5x the expected latency:

```rust
// Good: Debounce expected to settle within 20ms; use 80ms timeout
tokio::time::sleep(Duration::from_millis(80)).await;
let _result = rx.changed().await;

// Bad: from_secs(1) masks slow paths and inflates test time
tokio::time::sleep(Duration::from_secs(1)).await;
```

### Checking Test Execution Time

```bash
# Run with timing summary
cargo test -- --test-threads=1 --nocapture 2>&1 | tail -20

# Test a single crate with time tracking
time cargo test -p lsp-max-compositor

# Ignored tests only (slow paths)
cargo test -- --ignored --test-threads=1
```

---

## 2. Criterion Benchmarks

**lsp-max** uses [Criterion](https://bheisler.github.io/criterion.rs/book/) (version 0.5+) for statistically rigorous benchmarking of hot paths. Criterion runs benchmarks in isolation, collects samples, detects regressions, and generates HTML reports.

### Benchmark Directory Structure

Benchmarks live in `crates/*/benches/` directories (Cargo automatically discovers `*.rs` files with harness disabled).

```
crates/lsp-max-compositor/
  benches/
    compositor_micro.rs       # Sync micro-benchmarks (fastest paths)
    compositor_fanout.rs      # Async subprocess spawn & fanout
    compositor_backpressure.rs # Diagnostics merge under load
```

### Running Benchmarks

```bash
# Run all compositor benchmarks (generates HTML report)
cargo bench -p lsp-max-compositor

# Run a specific benchmark family
cargo bench -p lsp-max-compositor -- deposit_contention

# Run with verbose output
cargo bench -p lsp-max-compositor -- --verbose

# Compare against baseline (saved after first run)
cargo bench -p lsp-max-compositor -- --baseline main
```

Criterion HTML reports appear in `target/criterion/` — open `index.html` in a browser for graphs and regression detection.

### Hot Paths Requiring Benchmarks

These are the highest-frequency and most critical paths:

#### 1. **JSON-RPC Routing** (`src/jsonrpc/router.rs`)
- **Operation**: Deserialize incoming JSON, route to handler, serialize response
- **Frequency**: Every LSP request (orders of 100/second in active editing)
- **Benchmark**: Micro-benchmark for deserialize + lookup + serialize latency with varying request payload sizes (10B–100KB)
- **Target**: < 1ms for typical (5KB) requests

#### 2. **Receipt Validation** (` lsp-max-protocol`)
- **Operation**: Verify receipt signature, check chain depth, validate digest
- **Frequency**: Every diagnostic published, every gate status change
- **Benchmark**: Validate N receipts in a chain (max depth 5); throughput test at N=1, 5, 10
- **Target**: < 100µs per receipt validation

#### 3. **Conformance Scoring** (`lsp-max-runtime`)
- **Operation**: Score a `ConformanceVector` (admitted/refused/unknown law axes)
- **Frequency**: Every conformance query; used in composition decisions
- **Benchmark**: Compute score for vectors with 5, 50, 500 law axes
- **Target**: < 10µs for typical (50-axis) vectors

#### 4. **Diagnostic Merge & Deduplication** (`crates/lsp-max-compositor`)
- **Operation**: Merge diagnostics from M servers for K URIs with ANDON prefix matching
- **Frequency**: On every `textDocument/publishDiagnostics` from any upstream
- **Benchmark**: 8 families in `compositor_micro.rs`:
  - `deposit_contention`: N threads depositing to same URI
  - `merge_diagnostics_cpu`: Sort + dedup path with REFUSED_BY_LAW branch
  - `andon_prefix_match`: Daachorse automaton at scale
  - `multi_uri_flush_fanout`: M=10–50 URIs (real editor workload)
- **Target**: < 5ms for M=50 URIs × K=100 diagnostics per URI

#### 5. **Document Store Lookups** (`src/primitives/document_store.rs`)
- **Operation**: Read/write document content and version (Arc-wrapped RwLock<FxHashMap<Url, Document>>)
- **Frequency**: Per-document change event; multiple lookups per request
- **Benchmark**: Concurrent reads (N threads) + writes to M documents
- **Target**: < 100µs for M=100 documents, N=10 readers

### Writing a New Criterion Benchmark

```rust
// benches/my_bench.rs
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_my_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_operation");
    
    for n in [10usize, 100, 1000] {
        let data = black_box(setup_data(n));
        
        group.bench_with_input(BenchmarkId::new("N", n), &n, |b, _| {
            b.iter(|| my_expensive_function(&data))
        });
    }
    
    group.finish();
}

criterion_group!(benches, bench_my_operation);
criterion_main!(benches);
```

**Important**: Wrap inputs in `black_box()` to prevent compiler dead-code elimination. This ensures Criterion measures *actual* work, not optimized-away calls.

---

## 3. Profiling Workflow

### Flamegraph Profiling (Linux/macOS)

Flamegraphs show where CPU time is spent. Use them to identify hot functions and bottlenecks.

#### Prerequisites

Install flamegraph tools:

```bash
# Linux (using perf)
# Usually pre-installed; verify:
perf record --version

# macOS (using dtrace)
# Built-in on modern macOS
```

Install the Rust integration:

```bash
cargo install flamegraph
```

#### Capturing a Flamegraph

```bash
# Profile a binary (e.g., lsp-max-compositor)
cargo flamegraph --bin lsp-max-compositor -- --config /path/to/config.toml

# Profile a specific benchmark
cargo flamegraph --bench compositor_micro -- --profile-time 30

# Profile with custom options
cargo flamegraph -o flamegraph.svg -- <args>
```

This generates `flamegraph.svg` in the current directory.

#### Interpreting Flamegraph Output

A flamegraph is a heat map of CPU usage stacked by call chain:

- **Width** = time spent (wider = more CPU time)
- **Height** = call stack depth
- **Color** = sampled function (red = hot, yellow = warm, blue = cold)
- **X-axis** = time (left to right, but not chronological)

**Reading a flamegraph**:

1. **Identify the widest bars** at the bottom (leaf functions taking most time)
2. **Trace upward** to see the call chain
3. **Look for unexpected functions** taking time (e.g., allocations, locks, system calls)

**Example**: If `merge_diagnostics` appears wide with a `VecDeque::pop` call above it, the diagnostic merge is spending time on allocations — candidate for optimization (e.g., pre-allocate, reuse buffers).

#### Regression Detection with Flamegraphs

Take flamegraphs before and after a change:

```bash
# Before optimization
git checkout main
cargo flamegraph --bin lsp-max-compositor -o flamegraph-before.svg -- --config test.toml

# After optimization
git checkout feature-branch
cargo flamegraph --bin lsp-max-compositor -o flamegraph-after.svg -- --config test.toml

# Compare side-by-side in a browser
# If a function gets narrower, the optimization worked.
# If a new wide bar appears, you've introduced a regression.
```

### Tracing with `tracing` Spans

The codebase uses the `tracing` crate (already a dependency) to emit timing events. Spans can be captured and analyzed:

```rust
use tracing::instrument;

#[instrument(skip(diagnostics))]
async fn merge_diagnostics(diagnostics: Vec<Diagnostic>) -> Result<Vec<Diagnostic>> {
    // Span automatically records entry, exit, and duration
    // ...
}
```

#### Collecting Traces

```bash
# With default subscriber (stderr output)
RUST_LOG=lsp_max=debug cargo run -- <args> 2>&1 | grep -E "(enter|exit|time)"

# More structured: capture to a file, process with external tools
export RUST_LOG=lsp_max=trace
cargo run -- <args> 2>trace.log
# Then analyze trace.log with tools like Perfetto or flamegraph-compatible tracers
```

#### Existing Spans in the Codebase

Search for instrumented functions:

```bash
grep -r "#\[instrument\]" /home/user/lsp-max --include="*.rs"
```

Add spans to performance-critical hot paths:

```rust
#[instrument(skip(buffer), fields(uri, server_id))]
pub fn deposit(&self, uri: &str, server_id: &str, entries: Vec<DiagnosticEntry>) {
    // Automatically records uri, server_id, and execution time
}
```

---

## 4. Memory Layout & Sharing

**lsp-max** uses **Arc** (atomic reference counting) for shared, read-only data and **parking_lot RwLock** for low-contention synchronized access.

### Arc-Based Sharing Pattern

Arc enables cheap cloning (O(1) increment of reference count) without copying the underlying data.

#### DocumentStore

```rust
#[derive(Clone, Debug)]
pub struct DocumentStore {
    inner: Arc<RwLock<FxHashMap<Url, VersionedDocument>>>,
}
```

- **Clone cost**: O(1) increment
- **Lookup cost**: RwLock read-lock acquisition + HashMap lookup
- **Concurrent readers**: Arbitrary (RwLock allows N read locks)
- **Write cost**: Exclusive lock (blocks readers)

**Pattern**: Clone `DocumentStore` for each request/task; the underlying HashMap is shared.

#### DiagnosticSink

```rust
pub struct DiagnosticSink {
    client: Client,
    last: Arc<RwLock<FxHashMap<Url, (u64, u64)>>>,  // (axis_bits, fnv_hash)
}
```

- **Fast path (no change)**: Read lock + HashMap lookup ≈ 100–200ns
- **Slow path (publish)**: Write lock + update ≈ 1–2µs

Deduplication by FNV-1a hash avoids cloning diagnostic vectors on the fast path.

### parking_lot RwLock (vs std::sync::RwLock)

**lsp-max** uses `parking_lot::RwLock` instead of `std::sync::RwLock` because:

1. **Faster lock acquisition**: parking_lot uses futexes (kernel-assisted); std::RwLock uses atomic spin loops
2. **Smaller memory footprint**: 16 bytes vs. 56+ bytes per lock
3. **No poisoning**: RwLock doesn't panic if a lock holder panics (simpler error handling)

### Lock Contention Guidelines

**Acceptable contention**:
- Document store: 10–100 concurrent readers, 1 occasional writer — **Low contention** ✓
- Diagnostic sink: 100+ read-heavy, 1–5 writes/sec — **Very low contention** ✓

**High contention scenarios** (avoid):
- Shared Vec where every task pushes — use `dashmap::DashMap` or `kanal` channels instead
- Arc<Mutex<T>> where operations > 1µs — consider lock-free structures or channels

### Memory Layout Example: ComposedServer State

```rust
pub struct CompositionState {
    pub capability_tracker: CapabilityTracker,           // 256 bytes (approx)
    pub doc_tracker: DocumentVersionTracker,             // Arc -> 16 bytes
    pub edit_gate: TransactionEditGate,                  // Arc -> 16 bytes
    pub upstream_timeout_ms: u64,                        // 8 bytes
    pub diagnostics: HashMap<String, HashMap<...>>,      // Arc -> 16 bytes
    pub request_counter: u64,                            // 8 bytes
    pub request_traces: Arc<Mutex<Vec<Value>>>,          // 16 bytes
}
// Total stack: ~400 bytes; heap allocation via Arc depends on internal maps
```

All mutable state is either Arc-wrapped or locked, enabling safe `Send + Sync` cloning across task boundaries.

---

## 5. Scaling Limits

These are the documented maximum capacity targets for **lsp-max**:

### Document Count

| Metric | Limit | Rationale |
|---|---|---|
| Concurrent open documents | 5,000 | Typical editor session: 10–100; CI workload: 1,000–5,000 |
| DocumentStore lookup latency | < 1µs | FxHashMap with RwLock; O(1) average case |

**Scaling test**: Benchmark DocumentStore with M=1K, 5K, 10K documents. If latency increases super-linearly, rehashing is a bottleneck.

### Receipt Chain Depth

| Metric | Limit | Rationale |
|---|---|---|
| Max receipt chain depth | 5 | Each receipt references prior receipt's digest; depth 5 = ~10 admission steps |
| Validation latency per chain | < 500µs | Sequential digest validation: 100µs × 5 receipts |

**Why ≤5**: Receipt chains form a linked list. Depth 5 allows ~10 distinct admission/refusal transitions before re-baselining. Beyond this, performance degrades linearly (O(depth) validation).

### Composition Source Count

| Metric | Limit | Rationale |
|---|---|---|
| Concurrent upstream servers | 50 | Typical composition: 3–5 sources (Rust analyzer, clippy, rustfmt); stress test: 50 |
| Fanout latency per source | < 100ms | `upstream_timeout_ms` default = 150; fanout to all 50 = max 7.5 seconds wall-clock |
| Merge latency (M sources × K diagnostics) | < 50ms | Criterion target for M=50 URIs, K=100 diags/URI = 5,000 total diagnostics |

### Diagnostic Merging at Scale

The composition layer merges diagnostics from M upstream servers into a single stream to the client.

**Scaling formula**:
- **Deposit (per server)**: O(K) where K = diagnostics for one URI = 1–50 typical
- **Flush (all servers to client)**: O(M × K) = O(5,000) at worst case; Criterion targets < 5ms

**Benchmark result targets** (from `compositor_micro.rs`):
- `deposit_contention` N=500: < 10µs per deposit
- `merge_diagnostics_cpu`: < 100µs for M=50 servers, K=100 diags
- `multi_uri_flush_fanout`: < 5ms for M=10 URIs, sequential flushes

**Exceeding limits**: If you need > 50 sources or > 5,000 concurrent documents:
1. Profile with flamegraph to identify the bottleneck
2. Consider shard-based design (e.g., DashMap with 16 shards instead of single RwLock)
3. Batch diagnostic updates (flush once per 100ms instead of per-request)

---

## 6. Performance Regression Detection

### Automated Regression Detection

Criterion automatically compares new benchmark results against saved baselines:

```bash
cargo bench -p lsp-max-compositor -- --baseline main

# Output:
# deposit_contention/N/10    time:   [8.234 ms 8.456 ms 8.712 ms]
#                   change: [+15.3% +17.2% +19.8%] (likely regressed)
#                   threaded model heuristic suggests this is a REGRESSION.
```

If a regression is detected (> 5% slower), Criterion highlights it in red. HTML report includes a regression graph.

### Manual Regression Detection

Compare flamegraphs before/after:

```bash
# Generate before/after flamegraphs
git stash
cargo flamegraph --bin lsp-max-compositor -o before.svg -- --stress 1000
git stash pop
cargo flamegraph --bin lsp-max-compositor -o after.svg -- --stress 1000

# Visual comparison: functions that get wider are regressions
```

### Continuous Regression Monitoring

Add a GitHub Actions workflow to run benchmarks on every PR:

```yaml
# .github/workflows/bench.yml
name: Benchmark
on: [push, pull_request]
jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: rust-lang/rust-toolchain@v1
      - run: cargo bench -p lsp-max-compositor --baseline main
      - uses: actions/upload-artifact@v3
        with:
          name: criterion-report
          path: target/criterion/
```

---

## 7. Example: Optimizing a Hot Path

### Scenario: Diagnostic Merge is 50ms (should be 5ms)

1. **Profile**: `cargo flamegraph --bin lsp-max-compositor -- --stress-diags 10000`
   - Flamegraph shows wide `Vec::push` inside `merge_diagnostics`

2. **Root Cause**: Diagnostics are being pushed one-by-one to a `Vec` that keeps reallocating

3. **Optimization**: Pre-allocate with capacity

   ```rust
   // Before
   let mut result = Vec::new();
   for diag in diagnostics {
       result.push(diag);  // O(n) reallocations
   }

   // After
   let mut result = Vec::with_capacity(diagnostics.len());
   for diag in diagnostics {
       result.push(diag);  // O(1) amortized
   }
   ```

4. **Validate**: Re-run flamegraph; `Vec::push` bar should narrow significantly

5. **Benchmark**: `cargo bench -p lsp-max-compositor -- merge_diagnostics --baseline main`
   - Expect 10x speedup or regression detection if no improvement

---

## 8. Profiling Checklist

Before declaring a change "performance neutral", verify:

- [ ] **Criterion benchmark run**: `cargo bench -p <crate>` shows no regressions (< 5% variance acceptable)
- [ ] **Flamegraph comparison**: Before/after flamegraphs show no new wide bars
- [ ] **Tracing output**: No unexpected log statements in hot paths (logging can be 10–100x slower than compute)
- [ ] **Memory usage**: `valgrind --tool=massif` or similar shows no unexpected allocations
- [ ] **Lock contention**: No new lock acquisitions in loops
- [ ] **Test execution time**: `cargo test` still completes in < 10 seconds

---

## 9. Key Metrics & Targets

| Metric | Target | Comment |
|---|---|---|
| **cargo test (all)** | < 5 seconds | Enables fast dev iteration |
| **cargo test --ignored** | < 30 seconds | Allows deep stress testing |
| **JSON-RPC roundtrip** | < 1ms | Request deserialize + route + serialize |
| **Receipt validation** | < 100µs per receipt | Chain depth max 5 |
| **Conformance scoring** | < 10µs | Law-axis vector evaluation |
| **Diagnostic merge (50 servers × 100 diags)** | < 5ms | Dedup + ANDON matching |
| **Document store lookup** | < 1µs | RwLock<FxHashMap> read |
| **Flamegraph regression** | < 10% width increase | Acceptable variance on noisy system |

---

## References

- **Criterion.rs documentation**: https://bheisler.github.io/criterion.rs/book/
- **flamegraph (Rust integration)**: https://www.brendangregg.com/flamegraphs.html
- **parking_lot**: https://docs.rs/parking_lot/
- **tracing crate**: https://docs.rs/tracing/
- **Cargo bench**: https://doc.rust-lang.org/cargo/commands/cargo-bench.html

---

## Contributing Performance Improvements

When submitting a performance optimization:

1. **Include a Criterion benchmark** showing the improvement (before/after)
2. **Flamegraph before & after** (optional but encouraged for large changes)
3. **Document why** the bottleneck existed (e.g., missing pre-allocation, lock contention)
4. **Verify no regressions** in other hot paths (run full benchmark suite)
5. **Update this doc** if you've discovered new scaling limits or hot paths

