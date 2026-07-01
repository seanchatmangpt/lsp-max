# Configuration Reference for lsp-max

Complete reference for all environment variables, configuration files, and runtime options for lsp-max deployment and execution.

---

## 1. Environment Variables

### 1.1 Core Server Configuration

| Variable | Type | Default | Valid Range | Description |
|----------|------|---------|-------------|-------------|
| `LSP_MAX_BIND_ADDRESS` | host:port | `127.0.0.1:8080` | any TCP address | Listen address and port for incoming LSP connections |
| `LSP_MAX_LOG_LEVEL` | enum | `info` | `trace`, `debug`, `info`, `warn`, `error` | Logging verbosity level |
| `LSP_MAX_LOG_FORMAT` | enum | `text` | `text`, `json` | Log output format (JSON for structured logging) |
| `LSP_MAX_LOG_TARGETS` | string | `stdout` | comma-separated | Log targets: `stdout`, `stderr`, `file:/path/to/log.txt` |
| `LSP_MAX_CONFIG_PATH` | path | (none) | file path | Path to YAML configuration file (optional) |
| `LSP_MAX_WORKSPACE_ROOT` | path | (none) | directory | Root path of the workspace to analyze |

**Example:**

```bash
export LSP_MAX_BIND_ADDRESS=0.0.0.0:8080
export LSP_MAX_LOG_LEVEL=debug
export LSP_MAX_LOG_FORMAT=json
export LSP_MAX_WORKSPACE_ROOT=/code/myproject
```

### 1.2 Session & Timeout Configuration

| Variable | Type | Default | Unit | Description |
|----------|------|---------|------|-------------|
| `LSP_MAX_SESSION_TIMEOUT_SECS` | integer | `3600` | seconds | Idle session timeout; sessions inactive for this duration are terminated |
| `LSP_MAX_REQUEST_TIMEOUT_SECS` | integer | `10` | seconds | Maximum time to wait for LSP request completion |
| `LSP_MAX_SHUTDOWN_TIMEOUT_SECS` | integer | `10` | seconds | Maximum time to wait for graceful shutdown |
| `LSP_MAX_MAX_CONNECTIONS` | integer | `1000` | count | Maximum concurrent client connections |

**Example:**

```bash
export LSP_MAX_SESSION_TIMEOUT_SECS=7200        # 2 hours
export LSP_MAX_REQUEST_TIMEOUT_SECS=30          # 30 seconds
export LSP_MAX_MAX_CONNECTIONS=500
```

### 1.3 OCEL (Process Mining) Configuration

| Variable | Type | Default | Unit | Description |
|----------|------|---------|------|-------------|
| `LSP_MAX_OCEL_ENABLED` | boolean | `true` | - | Enable OCEL event logging |
| `LSP_MAX_OCEL_BUFFER_SIZE` | integer | `10000` | events | Max OCEL events before flush to disk |
| `LSP_MAX_OCEL_FLUSH_INTERVAL_SECS` | integer | `60` | seconds | Periodic OCEL flush interval |
| `LSP_MAX_OCEL_OUTPUT_DIR` | path | `/tmp/ocel` | directory | Directory for OCEL JSON output |
| `LSP_MAX_OCEL_MAX_FILES` | integer | `100` | count | Maximum OCEL files retained (oldest deleted) |

**Example:**

```bash
export LSP_MAX_OCEL_ENABLED=true
export LSP_MAX_OCEL_BUFFER_SIZE=5000
export LSP_MAX_OCEL_FLUSH_INTERVAL_SECS=30
export LSP_MAX_OCEL_OUTPUT_DIR=/var/log/lsp-max/ocel
```

### 1.4 Receipt & Admission Configuration

| Variable | Type | Default | Unit | Description |
|----------|------|---------|------|-------------|
| `LSP_MAX_RECEIPT_DIR` | path | `/tmp/receipts` | directory | Directory for receipt artifacts |
| `LSP_MAX_RECEIPT_ALGORITHM` | enum | `blake3` | `blake3`, `sha256` | Hashing algorithm for receipt digests |
| `LSP_MAX_RECEIPT_RETENTION_DAYS` | integer | `30` | days | Retention period for receipt artifacts |

**Example:**

```bash
export LSP_MAX_RECEIPT_DIR=/var/cache/lsp-max/receipts
export LSP_MAX_RECEIPT_ALGORITHM=blake3
export LSP_MAX_RECEIPT_RETENTION_DAYS=90
```

### 1.5 Λ_CD Gate Configuration

| Variable | Type | Default | Unit | Description |
|----------|------|---------|------|-------------|
| `LSP_MAX_GATE_ENABLED` | boolean | `true` | - | Enable Λ_CD gate enforcement |
| `LSP_MAX_GATE_CHECK_INTERVAL_MS` | integer | `100` | milliseconds | How often to check for ANDON signals |
| `LSP_MAX_GATE_DIAGNOSTICS_PATTERNS` | string | `WASM4PM-.*,GGEN-.*` | regex list | Comma-separated patterns matching blocking diagnostics |
| `LSP_MAX_GATE_STATE_FILE` | path | `/tmp/lsp-max-gate.state` | file path | Atomic gate state file (1 byte: 0=OPEN, 1=ANDON) |

**Example:**

```bash
export LSP_MAX_GATE_ENABLED=true
export LSP_MAX_GATE_CHECK_INTERVAL_MS=50
export LSP_MAX_GATE_DIAGNOSTICS_PATTERNS="WASM4PM-.*,GGEN-.*,CUSTOM-.*"
```

### 1.6 Observability & Metrics

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | URL | (none) | OpenTelemetry collector gRPC endpoint |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | enum | `grpc` | `grpc`, `http` |
| `OTEL_EXPORTER_OTLP_INSECURE` | boolean | `false` | Allow insecure (non-TLS) connections |
| `OTEL_API_KEY` | string | (none) | API key for remote OTel service |
| `LSP_MAX_METRICS_ENABLED` | boolean | `true` | Enable Prometheus metrics endpoint |
| `LSP_MAX_METRICS_PORT` | integer | `9091` | Port for Prometheus metrics |
| `LSP_MAX_METRICS_HISTOGRAM_BUCKETS` | string | `1,5,10,50,100,500,1000` | Comma-separated histogram bucket boundaries (ms) |

**Example:**

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
export OTEL_EXPORTER_OTLP_INSECURE=true
export LSP_MAX_METRICS_ENABLED=true
export LSP_MAX_METRICS_PORT=9091
export LSP_MAX_METRICS_HISTOGRAM_BUCKETS="1,5,10,25,50,100,250,500,1000,2500,5000"
```

### 1.7 Rust Runtime & Debugging

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `RUST_LOG` | string | (none) | Tracing filter (e.g., `lsp_max=debug,tower_lsp=info`) |
| `RUST_BACKTRACE` | enum | `0` | `0` (no), `1` (minimal), `full` (all frames) |
| `RUST_LIB_BACKTRACE` | enum | `0` | Same as `RUST_BACKTRACE` but for library panics |
| `CARGO_INCREMENTAL` | enum | `1` | Enable incremental compilation (dev builds) |

**Example:**

```bash
export RUST_LOG="lsp_max=debug,tower_lsp=info,lsp_max_compositor=trace"
export RUST_BACKTRACE=full
```

### 1.8 External Services & Integrations

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `GIT_TOKEN` | string | (none) | GitHub/GitLab token for authentication |
| `GIT_AUTHOR_NAME` | string | (none) | Git author name for commit operations |
| `GIT_AUTHOR_EMAIL` | string | (none) | Git author email for commit operations |
| `SSH_KEY_PATH` | path | `~/.ssh/id_rsa` | Path to SSH private key for git over SSH |
| `HTTP_PROXY` / `HTTPS_PROXY` | URL | (none) | HTTP proxy for external requests |

**Example:**

```bash
export GIT_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
export GIT_AUTHOR_NAME="lsp-max bot"
export GIT_AUTHOR_EMAIL="lsp-max@example.com"
export SSH_KEY_PATH=/etc/secrets/ssh/id_rsa
```

---

## 2. Configuration File (YAML)

lsp-max supports an optional YAML configuration file for complex setups. Point to it via:

```bash
export LSP_MAX_CONFIG_PATH=/etc/lsp-max/config.yaml
```

### 2.1 Complete Configuration File Schema

```yaml
# Main server configuration
server:
  # Listening address and port
  bind: "0.0.0.0:8080"
  
  # Maximum concurrent client connections
  max_connections: 1000
  
  # Request timeout in seconds
  request_timeout_secs: 10
  
  # Session idle timeout in seconds
  session_timeout_secs: 3600
  
  # Graceful shutdown timeout in seconds
  shutdown_timeout_secs: 10

# Logging configuration
logging:
  # Log level: trace, debug, info, warn, error
  level: info
  
  # Output format: text or json
  format: json
  
  # Log targets
  targets:
    # Log to stdout
    - type: stdout
      format: json
    
    # Log to file with rotation
    - type: file
      path: /var/log/lsp-max/server.log
      # Max file size before rotation (MB)
      rotation_size_mb: 100
      # Number of rotated files to retain
      max_backups: 10
      # Retention period in days
      retention_days: 7
      format: json

# OpenTelemetry & observability
observability:
  # Enable OpenTelemetry export
  otel_enabled: true
  
  # Collector endpoint (gRPC)
  otel_endpoint: "http://otel-collector:4317"
  
  # Allow insecure (non-TLS) connection
  otel_insecure: false
  
  # API key for remote service
  otel_api_key: ""
  
  # Metrics configuration
  metrics:
    enabled: true
    port: 9091
    path: /metrics
    
    # Histogram bucket boundaries (milliseconds)
    histogram_buckets:
      - 1
      - 5
      - 10
      - 25
      - 50
      - 100
      - 250
      - 500
      - 1000
      - 2500
      - 5000
  
  # Health check configuration
  health_check:
    enabled: true
    path: /healthz
    readiness_path: /ready

# OCEL (process mining) configuration
ocel:
  enabled: true
  
  # Event buffering
  buffer_size: 10000
  
  # Flush interval in seconds
  flush_interval_secs: 60
  
  # Output directory
  output_dir: /var/log/lsp-max/ocel
  
  # Maximum number of files retained
  max_files: 100
  
  # Log level for OCEL operations
  log_level: info

# Receipt & admission configuration
receipts:
  enabled: true
  
  # Storage directory
  output_dir: /var/cache/lsp-max/receipts
  
  # Hashing algorithm: blake3 or sha256
  algorithm: blake3
  
  # Retention period in days
  retention_days: 30
  
  # Automatic cleanup on startup
  auto_cleanup: true

# Λ_CD gate configuration
gate:
  # Enable gate enforcement
  enabled: true
  
  # Check interval in milliseconds
  check_interval_ms: 100
  
  # Patterns matching diagnostic codes that trigger ANDON
  andon_patterns:
    - "WASM4PM-.*"
    - "GGEN-.*"
  
  # State file for atomic gate storage
  state_file: /tmp/lsp-max-gate.state

# Workspace configuration
workspace:
  # Root directory of analyzed workspace
  root: /workspace
  
  # Files to ignore (gitignore patterns)
  ignore_patterns:
    - "node_modules/"
    - ".git/"
    - "target/"
    - "*.o"
    - "*.a"

# Routing & composition (for compositor)
routing:
  # Enable multi-server composition
  enabled: false
  
  # Child LSP servers
  servers:
    - id: "rust-analyzer"
      path: "/usr/bin/rust-analyzer"
      args: []
      
    - id: "pylsp"
      path: "/usr/local/bin/pylsp"
      args: []
  
  # Diagnostic merge strategy
  diagnostic_merge_strategy: "quorum"  # or "union"
  
  # Debounce window for diagnostic merging (ms)
  debounce_window_ms: 100

# Security configuration
security:
  # Enable mTLS
  mtls_enabled: false
  
  # TLS certificate paths
  tls_cert_path: /etc/tls/certs/server.crt
  tls_key_path: /etc/tls/keys/server.key
  
  # Require authentication token
  require_auth: false
  
  # Allow file system access (read-only)
  allow_fs_access: true
```

### 2.2 Environment-Specific Configurations

**Development** (`config-dev.yaml`):

```yaml
server:
  bind: "127.0.0.1:8080"
  max_connections: 10

logging:
  level: debug
  format: text
  targets:
    - type: stdout

observability:
  otel_enabled: false
  metrics:
    enabled: true
    port: 9091

ocel:
  enabled: true
  buffer_size: 100
  flush_interval_secs: 5
  output_dir: ./ocel-logs
```

**Production** (`config-prod.yaml`):

```yaml
server:
  bind: "0.0.0.0:8080"
  max_connections: 1000
  request_timeout_secs: 10

logging:
  level: info
  format: json
  targets:
    - type: file
      path: /var/log/lsp-max/server.log
      rotation_size_mb: 100
      max_backups: 20
      retention_days: 30

observability:
  otel_enabled: true
  otel_endpoint: "http://otel-collector.observability:4317"
  otel_insecure: false
  metrics:
    enabled: true
    port: 9091

ocel:
  enabled: true
  buffer_size: 50000
  flush_interval_secs: 30
  output_dir: /var/log/lsp-max/ocel
  max_files: 500

receipts:
  retention_days: 90
  auto_cleanup: true

gate:
  enabled: true
  check_interval_ms: 100

security:
  mtls_enabled: true
  tls_cert_path: /etc/tls/certs/server.crt
  tls_key_path: /etc/tls/keys/server.key
  require_auth: true
```

**Kubernetes** (`config-k8s.yaml`):

```yaml
server:
  bind: "0.0.0.0:8080"
  max_connections: 500

logging:
  level: info
  format: json
  targets:
    - type: stdout
      format: json

observability:
  otel_enabled: true
  otel_endpoint: "http://otel-collector.observability:4317"
  otel_insecure: true
  metrics:
    enabled: true
    port: 9091
    path: /metrics

ocel:
  enabled: true
  buffer_size: 10000
  flush_interval_secs: 60
  output_dir: /var/log/lsp-max/ocel

gate:
  enabled: true
  state_file: /tmp/lsp-max-gate.state
```

---

## 3. Command-Line Arguments

lsp-max CLI supports arguments for immediate overrides:

```bash
lsp-max-server \
  --bind 0.0.0.0:8080 \
  --log-level debug \
  --config /etc/lsp-max/config.yaml \
  --workspace /code/myproject \
  --otel-endpoint http://otel:4317
```

### 3.1 Full Argument Reference

| Argument | Short | Value Type | Default | Description |
|----------|-------|------------|---------|-------------|
| `--bind` | `-b` | host:port | `127.0.0.1:8080` | Server bind address |
| `--log-level` | `-l` | enum | `info` | Log level |
| `--config` | `-c` | path | (none) | Config file path |
| `--workspace` | `-w` | path | (none) | Workspace root directory |
| `--otel-endpoint` | | URL | (none) | OTel collector endpoint |
| `--metrics-port` | | integer | `9091` | Prometheus metrics port |
| `--version` | `-v` | - | - | Print version and exit |
| `--help` | `-h` | - | - | Print help message |

---

## 4. Kubernetes-Specific Configuration

### 4.1 ConfigMap Example

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: lsp-max-config
  namespace: lsp-max
data:
  config.yaml: |
    server:
      bind: "0.0.0.0:8080"
    logging:
      level: info
      format: json
    observability:
      otel_enabled: true
      otel_endpoint: http://otel-collector.observability:4317
```

Mount in Deployment:

```yaml
containers:
  - name: lsp-max
    volumeMounts:
      - name: config
        mountPath: /etc/lsp-max
        readOnly: true
    env:
      - name: LSP_MAX_CONFIG_PATH
        value: /etc/lsp-max/config.yaml

volumes:
  - name: config
    configMap:
      name: lsp-max-config
```

### 4.2 Secrets Example

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: lsp-max-secrets
  namespace: lsp-max
type: Opaque
stringData:
  git-token: "ghp_xxxxxxxxxxxxxxxxxxxx"
  otel-api-key: "your-api-key"
  tls.crt: |
    -----BEGIN CERTIFICATE-----
    ...
    -----END CERTIFICATE-----
  tls.key: |
    -----BEGIN PRIVATE KEY-----
    ...
    -----END PRIVATE KEY-----
```

Mount in Deployment:

```yaml
containers:
  - name: lsp-max
    env:
      - name: GIT_TOKEN
        valueFrom:
          secretKeyRef:
            name: lsp-max-secrets
            key: git-token
    volumeMounts:
      - name: tls
        mountPath: /etc/tls
        readOnly: true

volumes:
  - name: tls
    secret:
      secretName: lsp-max-secrets
```

---

## 5. Docker Environment File

Create `.env` for Docker Compose:

```bash
# Server
LSP_MAX_BIND_ADDRESS=0.0.0.0:8080
LSP_MAX_LOG_LEVEL=info
LSP_MAX_LOG_FORMAT=json

# Session & Timeouts
LSP_MAX_SESSION_TIMEOUT_SECS=3600
LSP_MAX_REQUEST_TIMEOUT_SECS=10

# OCEL
LSP_MAX_OCEL_ENABLED=true
LSP_MAX_OCEL_BUFFER_SIZE=10000
LSP_MAX_OCEL_FLUSH_INTERVAL_SECS=60

# Gate
LSP_MAX_GATE_ENABLED=true
LSP_MAX_GATE_CHECK_INTERVAL_MS=100

# Observability
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
LSP_MAX_METRICS_ENABLED=true
LSP_MAX_METRICS_PORT=9091

# External Services
GIT_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
```

Use in `docker-compose.yml`:

```yaml
services:
  lsp-max:
    env_file: .env
    environment:
      LSP_MAX_WORKSPACE_ROOT: /workspace
```

---

## 6. Configuration Priority (Precedence)

Environment variables override YAML configuration, which overrides command-line defaults:

```
Command-line args > Environment variables > YAML config > Hardcoded defaults
```

**Example cascade:**

```bash
# 1. Start with default: 127.0.0.1:8080
# 2. YAML config (if provided): 0.0.0.0:8080
# 3. Environment variable: 192.168.1.100:9000
export LSP_MAX_BIND_ADDRESS=192.168.1.100:9000

# 4. Command-line argument (highest priority): 10.0.0.1:7000
lsp-max-server --bind 10.0.0.1:7000
# Result: 10.0.0.1:7000 is used
```

---

## 7. Validating Configuration

Validate YAML configuration file:

```bash
# Using lsp-max-cli
lsp-max-cli config validate /etc/lsp-max/config.yaml

# Output on success:
# ✅ Configuration valid
# Server: bind=0.0.0.0:8080, max_connections=1000
# Logging: level=info, format=json
# ...

# Output on error:
# ❌ Configuration error:
# - logging.level: unknown value "trace" (valid: trace, debug, info, warn, error)
# - ocel.buffer_size: must be >= 100, got 10
```

---

## 8. Performance Tuning

### 8.1 High-Throughput Configuration

For systems with many concurrent requests:

```bash
export LSP_MAX_MAX_CONNECTIONS=5000
export LSP_MAX_OCEL_BUFFER_SIZE=100000
export LSP_MAX_OCEL_FLUSH_INTERVAL_SECS=10
export LSP_MAX_METRICS_HISTOGRAM_BUCKETS="1,10,50,100,500,1000,5000,10000"
```

YAML:

```yaml
server:
  max_connections: 5000

ocel:
  buffer_size: 100000
  flush_interval_secs: 10

observability:
  metrics:
    histogram_buckets:
      - 1
      - 10
      - 50
      - 100
      - 500
      - 1000
      - 5000
      - 10000
```

### 8.2 Low-Latency Configuration

For latency-sensitive applications:

```bash
export LSP_MAX_REQUEST_TIMEOUT_SECS=5
export LSP_MAX_GATE_CHECK_INTERVAL_MS=10
export LSP_MAX_OCEL_BUFFER_SIZE=1000
export LSP_MAX_OCEL_FLUSH_INTERVAL_SECS=5
```

YAML:

```yaml
server:
  request_timeout_secs: 5

gate:
  check_interval_ms: 10

ocel:
  buffer_size: 1000
  flush_interval_secs: 5
```

---

## 9. Troubleshooting Configuration

### 9.1 Enable Debug Logging

```bash
export LSP_MAX_LOG_LEVEL=debug
export RUST_LOG="lsp_max=debug,tower_lsp=debug,lsp_max_compositor=trace"
```

YAML:

```yaml
logging:
  level: debug

server:
  log_level: debug
```

### 9.2 Check Applied Configuration

Inspect current effective configuration:

```bash
lsp-max-cli config show

# Output:
# Current Configuration:
# ├── Server
# │   ├── bind: 0.0.0.0:8080
# │   ├── max_connections: 1000
# │   └── request_timeout_secs: 10
# ├── Logging
# │   ├── level: info
# │   └── format: json
# └── ...
```

### 9.3 Validate Paths Exist

```bash
# Check if required directories exist
test -d "$LSP_MAX_RECEIPT_DIR" || mkdir -p "$LSP_MAX_RECEIPT_DIR"
test -d "$LSP_MAX_OCEL_OUTPUT_DIR" || mkdir -p "$LSP_MAX_OCEL_OUTPUT_DIR"

# Verify permissions
ls -ld "$LSP_MAX_RECEIPT_DIR" "$LSP_MAX_OCEL_OUTPUT_DIR"
```

---

## Summary

Configuration in lsp-max follows a clear hierarchy:

1. **Environment variables** — simplest for Docker/Kubernetes
2. **YAML configuration file** — complex setups and feature toggles
3. **Command-line arguments** — immediate overrides

Reference guide sections:
- **1**: All environment variables with ranges and examples
- **2**: Complete YAML schema with examples
- **3**: CLI argument reference
- **4-5**: Kubernetes and Docker integration
- **6**: Configuration priority/precedence rules
- **7-9**: Validation and troubleshooting

For operational guidance, see:
- `/home/user/lsp-max/docs/REMOTE_EXECUTION.md` — Environment setup
- `/home/user/lsp-max/docs/DEPLOYMENT_GUIDES.md` — Production deployments
