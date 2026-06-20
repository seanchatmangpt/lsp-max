# Operations and Troubleshooting Guide for lsp-max

Complete operational procedures, monitoring, diagnostics, and troubleshooting for lsp-max in production environments.

---

## 1. Health Checks & Status Monitoring

### 1.1 Liveness & Readiness Probes

lsp-max exposes two health check endpoints:

#### Liveness Probe (`/healthz`)

Indicates whether the server process is alive and responsive.

```bash
curl -v http://localhost:8080/healthz

# Success response (HTTP 200):
# OK

# Failure response (HTTP 503):
# Server unavailable
```

Kubernetes liveness configuration:

```yaml
livenessProbe:
  httpGet:
    path: /healthz
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3
```

#### Readiness Probe (`/ready`)

Indicates whether the server can accept new connections.

```bash
curl -v http://localhost:8080/ready

# Ready response (HTTP 200):
# Ready

# Not ready response (HTTP 503):
# Not ready
```

Kubernetes readiness configuration:

```yaml
readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 2
```

**Differences:**

| Probe | When Fails | Action |
|-------|-----------|--------|
| Liveness | Server unresponsive | Pod restarted |
| Readiness | Too many pending requests | Pod removed from load balancer (but not restarted) |

### 1.2 Manual Status Check

```bash
# Check if server is responding
lsp-max-cli gate check
echo "Gate status: $?" # 0 = OPEN, 1 = ANDON

# Get detailed gate status
lsp-max-cli gate status

# Output:
# Gate State: OPEN
# Last Check: 2026-06-14T22:47:00Z
# Active Diagnostics: 0
# Blocking Diagnostics: []
```

---

## 2. Diagnostic Collection & Analysis

### 2.1 Snapshot Diagnostics

Collect all active diagnostics:

```bash
# Full diagnostic snapshot
lsp-max-cli diagnostic snapshot

# Sample output (JSON):
# {
#   "timestamp": "2026-06-14T22:47:00Z",
#   "diagnostics": [
#     {
#       "code": "WASM4PM-PROCESS-MISMATCH-001",
#       "message": "Process model divergence: expected CheckpointAdmitted, got DiagnosticPublished",
#       "severity": "error",
#       "source": "wasm4pm-conformance",
#       "receipt": "gall-checkpoint-003.receipt.json"
#     }
#   ],
#   "gate_state": "ANDON",
#   "blocking_count": 1
# }
```

### 2.2 Filtering Diagnostics

Filter by code, severity, or source:

```bash
# Only ANDON-blocking diagnostics
lsp-max-cli diagnostic snapshot --filter blocking

# Only errors
lsp-max-cli diagnostic snapshot --severity error

# Only from specific source
lsp-max-cli diagnostic snapshot --source wasm4pm-conformance

# Only WASM4PM-* pattern
lsp-max-cli diagnostic snapshot --pattern "WASM4PM-.*"
```

### 2.3 Diagnostic History

View diagnostic timeline:

```bash
# Last 100 diagnostics
lsp-max-cli diagnostic history --last 100

# Diagnostics in time range
lsp-max-cli diagnostic history --from 2026-06-14T00:00:00Z --to 2026-06-14T23:59:59Z

# Output (timeline):
# 22:47:05 ERROR  WASM4PM-PROCESS-MISMATCH-001 Process model divergence
# 22:46:30 WARN   ANTI-LLM-VICTORY-LANGUAGE-001 Code contains "done" identifier
# 22:45:12 ERROR  GGEN-TYPE-AMBIGUITY-002 Conformance vector axis unknown
```

---

## 3. Performance Monitoring

### 3.1 Prometheus Metrics

Access metrics on `http://localhost:9091/metrics`:

```bash
curl http://localhost:9091/metrics | grep lsp_max

# Key metrics:
# lsp_max_request_duration_ms_bucket{method="textDocument/hover",...}
# lsp_max_session_active 5
# lsp_max_gate_state 0  # 0 = OPEN, 1 = ANDON
# lsp_max_ocel_events_buffered 2341
# lsp_max_memory_allocated_bytes 536870912
```

### 3.2 Request Latency Analysis

Analyze P50, P95, P99 latencies:

```bash
# Using Prometheus HTTP API
curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,lsp_max_request_duration_ms)'

# Using local metrics dump
lsp-max-cli telemetry histogram \
  --metric lsp_max_request_duration_ms \
  --percentiles 50,95,99

# Output:
# P50:  15ms
# P95:  85ms
# P99: 250ms
```

### 3.3 Memory Usage Tracking

Monitor heap allocation:

```bash
# Current memory usage
lsp-max-cli telemetry memory --format human

# Output:
# Allocated:  512 MB
# Reserved:   768 MB
# Threshold:  2 GB (80% = 1.6 GB)

# Memory over time (if OTel enabled)
curl 'http://prometheus:9090/api/v1/query_range?query=lsp_max_memory_allocated_bytes&start=...'
```

### 3.4 Gate Check Performance

Monitor gate check latency (should be <5ms):

```bash
# Measure gate check time
time lsp-max-cli gate check

# Output:
# real    0m0.003s
# user    0m0.001s
# sys     0m0.002s
```

High gate check latency (>10ms) indicates:
- Filesystem congestion (state file on slow storage)
- High CPU contention
- Excessive diagnostic count (requires optimization)

---

## 4. Log Analysis

### 4.1 Structured Log Parsing

With JSON logging enabled:

```bash
# View all ERROR and WARN logs
tail -f /var/log/lsp-max/server.log | jq 'select(.level | test("ERROR|WARN"))'

# Filter by specific component
tail -f /var/log/lsp-max/server.log | jq 'select(.target == "lsp_max::composition")'

# Timeline view
tail -f /var/log/lsp-max/server.log | jq '{timestamp, level, message, context:.fields}'
```

### 4.2 Common Log Patterns

**Healthy startup:**

```json
{"timestamp":"2026-06-14T22:47:00Z","level":"INFO","message":"Server listening on 0.0.0.0:8080","target":"lsp_max::server"}
{"timestamp":"2026-06-14T22:47:01Z","level":"INFO","message":"Metrics endpoint listening on 0.0.0.0:9091","target":"lsp_max::metrics"}
{"timestamp":"2026-06-14T22:47:02Z","level":"INFO","message":"Gate initialized: state=OPEN","target":"lsp_max::gate"}
```

**Session lifecycle:**

```json
{"timestamp":"2026-06-14T22:47:10Z","level":"DEBUG","message":"Client connected","fields":{"session_id":"sess-abc123"}}
{"timestamp":"2026-06-14T22:47:11Z","level":"DEBUG","message":"Initialize request received","fields":{"method":"initialize","session_id":"sess-abc123"}}
{"timestamp":"2026-06-14T22:48:00Z","level":"DEBUG","message":"Session idle timeout","fields":{"session_id":"sess-abc123","idle_seconds":48}}
```

**Gate transitions:**

```json
{"timestamp":"2026-06-14T22:47:25Z","level":"WARN","message":"Gate state transition","fields":{"from":"OPEN","to":"ANDON","diagnostic_code":"WASM4PM-PROCESS-MISMATCH-001"}}
{"timestamp":"2026-06-14T22:49:40Z","level":"INFO","message":"Gate state transition","fields":{"from":"ANDON","to":"OPEN","diagnostics_resolved":1}}
```

### 4.3 Extracting Diagnostic Context

Find related logs for a specific diagnostic:

```bash
# Find diagnostic by code
DIAG_CODE="WASM4PM-PROCESS-MISMATCH-001"
grep -i "$DIAG_CODE" /var/log/lsp-max/server.log | head -5

# Extract full context (5 lines before/after)
grep -B 5 -A 5 "$DIAG_CODE" /var/log/lsp-max/server.log | jq .

# Find what caused it (search for OCEL event that triggered)
grep "diagnostic.*published" /var/log/lsp-max/server.log | jq 'select(.diagnostic_code == "'"$DIAG_CODE"'")'
```

---

## 5. OCEL Trace Analysis

### 5.1 Viewing OCEL Logs

OCEL (Object-Centric Event Log) records execution traces:

```bash
# List OCEL files
ls -lah /var/log/lsp-max/ocel/

# View latest OCEL log
cat /var/log/lsp-max/ocel/latest.ocel.json | jq .

# Parse events
jq '.ocel.events[]' /var/log/lsp-max/ocel/latest.ocel.json | head -20

# Sample output:
# {
#   "ocel:timestamp": "2026-06-14T22:47:10.123Z",
#   "ocel:activity": "client.connected",
#   "ocel:type": "SESSION",
#   "session_id": "sess-abc123"
# }
```

### 5.2 Event Filtering

Filter OCEL events by type or activity:

```bash
# All diagnostic published events
jq '.ocel.events[] | select(.["ocel:activity"] == "diagnostic.published")' latest.ocel.json

# Events for specific session
jq '.ocel.events[] | select(.session_id == "sess-abc123")' latest.ocel.json

# Events in time range
jq '.ocel.events[] | select(.["ocel:timestamp"] > "2026-06-14T22:47:00Z")' latest.ocel.json
```

### 5.3 Conformance Checking

Verify execution against declared process model:

```bash
# Run conformance check
lsp-max-cli conformance vector --instance-id sess-abc123

# Output (JSON):
# {
#   "instance_id": "sess-abc123",
#   "conformance_vector": {
#     "ADMITTED": ["PROCESS-INITIALIZATION", "DIAGNOSTIC-PUBLICATION"],
#     "REFUSED": ["UNDECLARED-GATEWAY-TRANSITION"],
#     "UNKNOWN": ["LSP-COMPLETION-RANKING"]
#   },
#   "evidence": {
#     "ocel_path": "/var/log/lsp-max/ocel/sess-abc123.ocel.json",
#     "receipt_path": "/var/cache/lsp-max/receipts/gall-checkpoint-003.receipt.json"
#   }
# }
```

---

## 6. Receipt Verification

### 6.1 Viewing Receipt Artifacts

Receipts prove capability claims with cryptographic digests:

```bash
# List all receipts
lsp-max-cli receipt list

# Output:
# COMPOSITOR-SCALE-ADMITTED-26.6.9
#   Status: ADMITTED
#   Digest: blake3:a1b2c3d4e5f6g7h8...
#   Claims: CS1 deposit_contention, CS2 quorum_debounce
#   Path: /var/cache/lsp-max/receipts/compositor-scale-admitted.receipt.json

# View specific receipt
lsp-max-cli receipt show COMPOSITOR-SCALE-ADMITTED-26.6.9

# Output (JSON):
# {
#   "id": "COMPOSITOR-SCALE-ADMITTED-26.6.9",
#   "status": "ADMITTED",
#   "version": "26.6.9",
#   "timestamp": "2026-06-13T00:00:00Z",
#   "digest": "blake3:a1b2c3d4e5f6g7h8...",
#   "claims": [
#     {
#       "axis": "PROCESS-INITIALIZATION",
#       "status": "ADMITTED",
#       "evidence": "dogfood_gc002.rs transcript"
#     }
#   ]
# }
```

### 6.2 Receipt Verification

Cryptographically verify receipt integrity:

```bash
# Verify receipt digest
lsp-max-cli receipt verify COMPOSITOR-SCALE-ADMITTED-26.6.9

# Output on success:
# ✅ Receipt COMPOSITOR-SCALE-ADMITTED-26.6.9 verified
#    Digest matches stored value
#    All claims are internally consistent

# Output on failure:
# ❌ Receipt verification failed
#    Expected digest: blake3:expected...
#    Computed digest: blake3:actual...
#    Tampering detected
```

### 6.3 Cleanup Old Receipts

Receipts older than retention period are auto-cleaned:

```bash
# Manual cleanup of receipts older than 30 days
lsp-max-cli receipt cleanup --retention-days 30

# Output:
# Deleted 3 receipts older than 2026-05-15
# Freed 1.2 MB

# View what would be deleted (dry-run)
lsp-max-cli receipt cleanup --retention-days 30 --dry-run
```

---

## 7. Troubleshooting Common Issues

### 7.1 Gate Stuck in ANDON

**Symptom:** All Bash operations blocked; `lsp-max-cli gate check` returns exit 1.

**Root Cause:** One or more diagnostics matching ANDON pattern are active.

**Diagnosis:**

```bash
# Step 1: Identify blocking diagnostics
lsp-max-cli diagnostic snapshot --filter blocking

# Step 2: Review diagnostic in detail
lsp-max-cli diagnostic snapshot | jq '.[] | select(.code == "WASM4PM-PROCESS-MISMATCH-001")'

# Step 3: Check OCEL evidence
lsp-max-cli ocel export --diagnostic-code WASM4PM-PROCESS-MISMATCH-001

# Step 4: Verify gate state file
cat /tmp/lsp-max-gate.state | od -tx1  # Should show: 31 (hex '1' = ANDON)
```

**Resolution:**

```bash
# Fix underlying issue (process model mismatch, missing receipt, etc.)
# Example: emit repair receipt
lsp-max-cli admission repair \
  --diagnostic-id WASM4PM-PROCESS-MISMATCH-001 \
  --fix-description "Manual verification: process model matches observed events"

# Verify gate clears
lsp-max-cli gate check
echo "Exit code: $?"  # Should be 0 now

# Confirm gate state changed
cat /tmp/lsp-max-gate.state | od -tx1  # Should show: 30 (hex '0' = OPEN)
```

### 7.2 High Memory Growth

**Symptom:** Memory usage increases over time; heap doesn't shrink.

**Root Cause:** OCEL buffer not flushing, or large documents cached.

**Diagnosis:**

```bash
# Check current memory
lsp-max-cli telemetry memory --format human
# Output: Allocated: 1.5 GB (75% threshold)

# Check OCEL buffer size
lsp-max-cli telemetry ocel-buffer-size
# Output: Buffered events: 50000 (of max 10000 — NOT FLUSHING)

# Check document cache
lsp-max-cli telemetry document-cache
# Output: Cached documents: 500 (~100 MB)
```

**Resolution:**

```bash
# Option 1: Force OCEL flush
lsp-max-cli ocel flush --wait
# Output: Flushed 50000 events to disk

# Option 2: Reduce OCEL buffer size
export LSP_MAX_OCEL_BUFFER_SIZE=5000
export LSP_MAX_OCEL_FLUSH_INTERVAL_SECS=30

# Option 3: Clear document cache (if safe)
lsp-max-cli cache clear --type documents
# ⚠ This may cause re-analysis on next request

# Monitor memory recovery
watch -n 5 'lsp-max-cli telemetry memory --format human'
```

### 7.3 Timeout on Large Files

**Symptom:** Requests on 100+ MB files timeout; `textDocument/hover` returns error.

**Root Cause:** Query processing exceeds timeout; file too large for single-pass analysis.

**Diagnosis:**

```bash
# Check latency histogram
lsp-max-cli telemetry histogram \
  --metric lsp_max_request_duration_ms \
  --filter method=hover

# Output:
# P95:  8500ms (exceeds default 10s timeout!)
# P99: 15000ms

# Check request timeout setting
lsp-max-cli config show | grep request_timeout_secs
# Output: request_timeout_secs: 10
```

**Resolution:**

```bash
# Option 1: Increase request timeout
export LSP_MAX_REQUEST_TIMEOUT_SECS=30
lsp-max-server --restart

# Option 2: Enable streaming analysis (if supported)
# Analyze document in chunks instead of whole file
export LSP_MAX_STREAMING_ENABLED=true

# Option 3: Split large files at source
# For testing, use smaller reproducible case

# Verify fix
time lsp-max-cli lsp-request /large-file.rs hover --position 1:0
# Should complete within new timeout
```

### 7.4 Docker Build Fails: Sibling Dependencies Not Found

**Symptom:** Build fails with path dependency resolution error.

**Root Cause:** Sibling repos not cloned before Cargo build.

**Diagnosis:**

```bash
# Check directory structure
ls -la ../
# lsp-max/
# lsp-types-max/  ← might be missing
# wasm4pm-compat/ ← might be missing
# wasm4pm/        ← might be missing
```

**Resolution:**

```dockerfile
# Correct Dockerfile order
# 1. Clone siblings BEFORE copying lsp-max
RUN cd /tmp && \
    git clone https://github.com/seanchatmangpt/lsp-types-max.git ../lsp-types-max && \
    git clone https://github.com/seanchatmangpt/wasm4pm-compat.git ../wasm4pm-compat && \
    git clone https://github.com/seanchatmangpt/wasm4pm.git ../wasm4pm

# 2. THEN copy and build lsp-max
COPY . /build/lsp-max
WORKDIR /build/lsp-max
RUN cargo build --release --locked
```

**Or use script:**

```bash
#!/bin/bash
set -e

WORKSPACE=/build
mkdir -p "$WORKSPACE"

# Clone siblings
for repo in lsp-types-max wasm4pm-compat wasm4pm; do
  git clone "https://github.com/seanchatmangpt/$repo.git" "$WORKSPACE/$repo"
done

# Build lsp-max
cd "$WORKSPACE/lsp-max"
cargo build --release --locked
```

### 7.5 Network: LSP Server Unreachable

**Symptom:** Client cannot connect to LSP server; `connection refused` error.

**Diagnosis:**

```bash
# Check if server is running
ps aux | grep lsp-max-server

# Check if port is listening
netstat -tlnp | grep 8080
# or
ss -tlnp | grep 8080

# Try to connect
telnet localhost 8080
nc -zv localhost 8080

# Check firewall
sudo iptables -L | grep 8080
sudo ufw status | grep 8080
```

**Resolution:**

```bash
# 1. Start server if not running
lsp-max-server --bind 0.0.0.0:8080 &

# 2. Verify port is open
ss -tlnp | grep 8080
# Output: LISTEN 0 128 0.0.0.0:8080 0.0.0.0:* users:(("lsp-max-ser",pid=1234,fd=3))

# 3. Test connectivity
curl -v http://localhost:8080/healthz
# Should return HTTP 200 OK

# 4. Check firewall (if remote)
sudo ufw allow 8080
```

### 7.6 OTel Traces Not Appearing

**Symptom:** Metrics visible in Prometheus, but OTel traces not in collector.

**Diagnosis:**

```bash
# Check OTel endpoint configuration
env | grep OTEL_EXPORTER_OTLP_ENDPOINT
# Should be set and reachable

# Verify endpoint is reachable
curl http://otel-collector:4317
# Should not timeout

# Check OTel export logs
tail -f /var/log/lsp-max/server.log | jq 'select(.target == "opentelemetry")'

# Check if export is enabled
lsp-max-cli config show | grep otel_enabled
```

**Resolution:**

```bash
# 1. Verify endpoint is correct
export OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
export OTEL_EXPORTER_OTLP_INSECURE=true

# 2. Check collector is running
docker ps | grep otel-collector

# 3. Enable debug logging
export RUST_LOG="opentelemetry=debug"
lsp-max-server --restart

# 4. Verify traces are being exported
# In OTel collector logs:
docker logs otel-collector | grep "trace"
```

---

## 8. Maintenance Procedures

### 8.1 Graceful Shutdown

```bash
# Send SIGTERM (graceful shutdown)
kill -TERM <pid>

# Wait for graceful shutdown (max 10 seconds)
wait <pid>
echo "Exit code: $?"  # Should be 0

# If not shut down after timeout, force kill
kill -KILL <pid>
```

Kubernetes graceful termination:

```yaml
lifecycle:
  preStop:
    exec:
      command: ["/bin/sh", "-c", "sleep 15"]
terminationGracePeriodSeconds: 30
```

### 8.2 Log Rotation

Configure logrotate for file-based logs:

```bash
# /etc/logrotate.d/lsp-max
/var/log/lsp-max/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    create 0640 lsp-max lsp-max
    sharedscripts
    postrotate
        systemctl reload lsp-max > /dev/null 2>&1 || true
    endscript
}
```

### 8.3 Database / State Cleanup

Periodically clean old data:

```bash
# Cleanup old OCEL logs (retain 7 days)
find /var/log/lsp-max/ocel -name "*.ocel.json" -mtime +7 -delete

# Cleanup old receipts (retain 30 days)
find /var/cache/lsp-max/receipts -name "*.receipt.json" -mtime +30 -delete

# Cleanup old session state
find /tmp/lsp-max-sessions -type d -mtime +1 -exec rm -rf {} \;
```

Automated cleanup via cron:

```bash
# /etc/cron.daily/lsp-max-cleanup
#!/bin/bash
find /var/log/lsp-max/ocel -name "*.ocel.json" -mtime +7 -delete
find /var/cache/lsp-max/receipts -name "*.receipt.json" -mtime +30 -delete
```

---

## 9. Performance Optimization

### 9.1 CPU Profiling

Profile hot code paths:

```bash
# Using perf (Linux)
perf record -g -F 99 lsp-max-server
perf report

# Using flamegraph
cargo install flamegraph
cargo flamegraph --bin lsp-max-server
# Generates flamegraph.svg
```

### 9.2 Memory Profiling

Track allocation patterns:

```bash
# Using Valgrind
valgrind --tool=massif lsp-max-server
ms_print massif.out.<pid>

# Using heaptrack (recommended)
heaptrack lsp-max-server
heaptrack_gui heaptrack.lsp-max-server.<pid>.gz
```

### 9.3 Benchmarking

Run performance tests:

```bash
# Built-in benchmarks
cargo bench -p lsp-max

# Custom benchmark
lsp-max-cli benchmark \
  --request textDocument/hover \
  --document-size 1mb \
  --iterations 1000 \
  --concurrency 10

# Output:
# Throughput: 950 requests/sec
# P50: 10ms
# P95: 25ms
# P99: 50ms
```

---

## 10. Upgrade & Migration

### 10.1 Rolling Update (Kubernetes)

```bash
# Update image
kubectl set image deployment/lsp-max \
  lsp-max=gcr.io/project/lsp-max:26.7.1 \
  -n lsp-max

# Monitor rollout
kubectl rollout status deployment/lsp-max -n lsp-max

# Rollback if issues
kubectl rollout undo deployment/lsp-max -n lsp-max
```

### 10.2 Canary Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: lsp-max-canary
spec:
  replicas: 1
  selector:
    matchLabels:
      app: lsp-max
      version: canary
  template:
    metadata:
      labels:
        app: lsp-max
        version: canary
    spec:
      containers:
        - name: lsp-max
          image: gcr.io/project/lsp-max:26.7.1  # New version
```

Traffic split:

```yaml
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: lsp-max
spec:
  hosts:
    - lsp-max
  http:
    - match:
        - uri:
            prefix: /
      route:
        - destination:
            host: lsp-max
            port:
              number: 8080
            subset: stable
          weight: 90
        - destination:
            host: lsp-max
            port:
              number: 8080
            subset: canary
          weight: 10
```

---

## Summary

This guide covers:

1. **Health monitoring** via liveness/readiness probes and status checks
2. **Diagnostic collection** with filtering, history, and analysis
3. **Performance metrics** from Prometheus (latency, memory, gate state)
4. **Log analysis** with structured JSON parsing and common patterns
5. **OCEL trace verification** and conformance checking
6. **Receipt management** and cryptographic verification
7. **Common troubleshooting** with diagnosis and resolution steps
8. **Maintenance procedures** (shutdown, rotation, cleanup)
9. **Performance optimization** (profiling, benchmarking)
10. **Safe upgrades** (rolling updates, canary deployments)

For additional reference:
- `/home/user/lsp-max/docs/REMOTE_EXECUTION.md` — Environment setup
- `/home/user/lsp-max/docs/DEPLOYMENT_GUIDES.md` — Deployment patterns
- `/home/user/lsp-max/docs/CONFIGURATION_REFERENCE.md` — Environment variables
- `/home/user/lsp-max/README.md` — Project overview
