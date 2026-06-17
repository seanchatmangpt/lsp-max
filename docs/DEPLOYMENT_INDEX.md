# Deployment & Remote Execution Documentation Index

Complete guide to deploying and operating lsp-max in remote environments, cloud infrastructure, and CI/CD pipelines.

---

## Quick Navigation

### For First-Time Deployments

1. **Start here**: [DEPLOYMENT_GUIDES.md](./DEPLOYMENT_GUIDES.md) § 1 (Kubernetes)
   - Namespace setup, ConfigMap, Deployment, Service, Ingress
   - Network policies, HPA, health checks
   
2. **Configure**: [CONFIGURATION_REFERENCE.md](./CONFIGURATION_REFERENCE.md) § 4-5
   - Kubernetes ConfigMap and Secrets
   - Environment variable mapping

3. **Monitor**: [OPERATIONS_AND_TROUBLESHOOTING.md](./OPERATIONS_AND_TROUBLESHOOTING.md) § 1-3
   - Health probes, metrics collection, log analysis

### For Local Development

1. [DEPLOYMENT_GUIDES.md](./DEPLOYMENT_GUIDES.md) § 2 (Docker Compose)
2. [CONFIGURATION_REFERENCE.md](./CONFIGURATION_REFERENCE.md) § 5 (Environment file)
3. [OPERATIONS_AND_TROUBLESHOOTING.md](./OPERATIONS_AND_TROUBLESHOOTING.md) § 2 (Diagnostics)

### For CI/CD Integration

1. [DEPLOYMENT_GUIDES.md](./DEPLOYMENT_GUIDES.md) § 6-7
   - GitHub Actions matrix testing
   - Release workflows
   - Cloud Build pipelines

2. [CONFIGURATION_REFERENCE.md](./CONFIGURATION_REFERENCE.md) § 1
   - Core environment variables for automation

### For Custom Agent Systems

1. [DEPLOYMENT_GUIDES.md](./DEPLOYMENT_GUIDES.md) § 5 (Agent Integration)
2. [REMOTE_EXECUTION.md](./REMOTE_EXECUTION.md) § 5 (Session Lifecycle)
3. [OPERATIONS_AND_TROUBLESHOOTING.md](./OPERATIONS_AND_TROUBLESHOOTING.md) § 2 (Diagnostics)

### For Troubleshooting

1. [OPERATIONS_AND_TROUBLESHOOTING.md](./OPERATIONS_AND_TROUBLESHOOTING.md) § 7
   - Gate stuck in ANDON
   - Memory growth
   - Timeouts
   - Network issues
   - OTel traces missing

---

## Document Descriptions

### 1. REMOTE_EXECUTION.md (11 sections)

**Overview**: Comprehensive reference for container isolation, networking, environment setup, and resource management.

**Key Topics**:
- Docker container setup and sibling repository dependencies
- Container resource limits (CPU, memory, storage)
- Filesystem mount points and ephemeral storage guarantees
- Network segmentation, mTLS, and secret management
- Environment variable configuration
- Git integration and workspace initialization
- Session lifecycle (init → active → shutdown → timeout)
- Resource limits and quotas (document size, memory, query timeout)
- GitHub Actions CI workflow
- CI/CD pipelines (GitLab CI, Jenkins, Cloud Build)
- OpenTelemetry integration and metrics
- Troubleshooting matrix

**When to Use**:
- Understanding container architecture and constraints
- Setting up networking policies
- Configuring resource limits
- Integrating with CI systems
- Implementing observability

**Key Sections**:
- § 1: Container Isolation (Docker, multi-workspace, resource limits, mounts)
- § 2: Network Policies (inbound/egress, Kubernetes NetworkPolicy, mTLS)
- § 3: Environment Configuration (environment variables)
- § 4: Git Integration (SSH, HTTPS, workspace initialization)
- § 5: Session Lifecycle (phases, duration, timeout handling)
- § 6: Resource Limits & Quotas (document size, memory, query timeout)
- § 7: GitHub Actions (matrix testing, pre-publish checks)
- § 8: CI/CD Pipelines (GitLab, Jenkins, Cloud Build)
- § 9: Observability (OTel, metrics, logging)
- § 10: Troubleshooting (gate, memory, latency, Docker builds)
- § 11: Performance Characteristics (benchmarks, limits, optimization)
- § 12: Maintenance & Updates (upgrade path, dependency updates)

---

### 2. DEPLOYMENT_GUIDES.md (7 sections)

**Overview**: Step-by-step deployment instructions for specific platforms and systems.

**Key Topics**:
- Complete Kubernetes manifests (namespace, ConfigMap, Secrets, Deployment, Service, Ingress, NetworkPolicy, HPA)
- Docker Compose single-container and multi-region setups
- AWS deployment (ECS Fargate, EC2 Auto Scaling)
- GCP deployment (Cloud Run, GKE)
- Custom agent system integration
- GitHub Actions workflow examples
- Monitoring setup (Prometheus, Grafana)

**When to Use**:
- Deploying to Kubernetes production
- Setting up local dev environment with Docker Compose
- AWS or GCP cloud deployments
- Integrating with agent orchestration
- Configuring CI/CD pipelines
- Adding monitoring/observability stack

**Key Sections**:
- § 1: Kubernetes (full manifests: namespace, ConfigMap, Secrets, Deployment, Service, Ingress, NetworkPolicy, HPA)
- § 2: Docker Compose (single-container, multi-region)
- § 3: AWS (ECS Fargate, EC2 Auto Scaling, CloudFormation)
- § 4: GCP (Cloud Run, GKE)
- § 5: Custom Agent Integration (client setup, query patterns, Docker network)
- § 6: GitHub Actions Workflow (matrix testing, release)
- § 7: Monitoring & Logging (Prometheus, Grafana)

---

### 3. CONFIGURATION_REFERENCE.md (9 sections)

**Overview**: Complete reference for all configuration options, environment variables, and settings.

**Key Topics**:
- All environment variables (server, session, OCEL, receipts, gate, OTel, external services)
- Complete YAML configuration file schema with examples
- Environment-specific configurations (dev, prod, Kubernetes)
- Kubernetes ConfigMap and Secret templates
- Docker Compose environment file format
- Command-line argument reference
- Configuration priority/precedence rules
- Validation procedures
- Performance tuning (high-throughput, low-latency)

**When to Use**:
- Setting up environment for deployment
- Fine-tuning for specific workload (high-throughput vs. low-latency)
- Understanding configuration hierarchy
- Troubleshooting configuration issues
- Looking up specific setting values

**Key Sections**:
- § 1: Environment Variables (15+ variables with ranges and defaults)
- § 2: YAML Configuration File (complete schema + dev/prod/k8s examples)
- § 3: Command-Line Arguments (CLI flags reference)
- § 4: Kubernetes (ConfigMap, Secrets, volume mounting)
- § 5: Docker Environment File (.env format)
- § 6: Configuration Priority (precedence rules)
- § 7: Validation (config validation command)
- § 8: Performance Tuning (high-throughput, low-latency)
- § 9: Troubleshooting Configuration (debug logging, path validation)

---

### 4. OPERATIONS_AND_TROUBLESHOOTING.md (10 sections)

**Overview**: Operational procedures, monitoring, diagnostics, and troubleshooting.

**Key Topics**:
- Liveness and readiness probes
- Diagnostic collection and filtering
- Performance monitoring (Prometheus metrics, latency, memory)
- Log analysis with structured JSON parsing
- OCEL trace analysis and conformance checking
- Receipt verification and artifact management
- Detailed troubleshooting (7 common issues with diagnosis and resolution)
- Maintenance procedures (graceful shutdown, log rotation, data cleanup)
- Performance optimization (profiling, benchmarking)
- Safe upgrade procedures (rolling updates, canary deployments)

**When to Use**:
- Monitoring production system health
- Diagnosing issues with gate, memory, latency, or connectivity
- Analyzing execution traces and receipts
- Performing maintenance or upgrades
- Optimizing performance

**Key Sections**:
- § 1: Health Checks (liveness/readiness probes, status checks)
- § 2: Diagnostic Collection (snapshot, filtering, history)
- § 3: Performance Monitoring (Prometheus, latency, memory, gate)
- § 4: Log Analysis (structured logs, patterns, context extraction)
- § 5: OCEL Trace Analysis (event filtering, conformance checking)
- § 6: Receipt Verification (listing, verifying, cleanup)
- § 7: Troubleshooting (6 detailed scenarios: gate ANDON, memory, timeout, Docker build, network, OTel)
- § 8: Maintenance (graceful shutdown, log rotation, cleanup)
- § 9: Performance Optimization (CPU/memory profiling, benchmarking)
- § 10: Upgrade & Migration (rolling updates, canary deployments)

---

## Deployment Decision Tree

```
START: "I need to deploy lsp-max"
  |
  ├─→ Local development?
  │   └─→ Use: DEPLOYMENT_GUIDES.md § 2 (Docker Compose)
  │
  ├─→ Production Kubernetes?
  │   └─→ Use: DEPLOYMENT_GUIDES.md § 1 (Kubernetes manifests)
  │       + CONFIGURATION_REFERENCE.md § 4 (ConfigMap/Secrets)
  │       + OPERATIONS_AND_TROUBLESHOOTING.md § 1-3 (monitoring)
  │
  ├─→ AWS cloud?
  │   └─→ Use: DEPLOYMENT_GUIDES.md § 3
  │       (ECS Fargate or EC2 Auto Scaling)
  │
  ├─→ GCP cloud?
  │   └─→ Use: DEPLOYMENT_GUIDES.md § 4
  │       (Cloud Run or GKE)
  │
  ├─→ Custom agent system?
  │   └─→ Use: DEPLOYMENT_GUIDES.md § 5 (Agent Integration)
  │       + REMOTE_EXECUTION.md § 5 (Session Lifecycle)
  │
  ├─→ CI/CD pipeline (GitHub Actions)?
  │   └─→ Use: DEPLOYMENT_GUIDES.md § 6 (GitHub Actions)
  │       + REMOTE_EXECUTION.md § 7 (GitHub Actions CI)
  │
  └─→ Something not working?
      └─→ Use: OPERATIONS_AND_TROUBLESHOOTING.md § 7
          (Troubleshooting matrix)
```

---

## Common Workflows

### Deploying to Kubernetes

```
1. Create namespace, ConfigMap, Secrets
   → DEPLOYMENT_GUIDES.md § 1.1-1.2

2. Configure environment variables
   → CONFIGURATION_REFERENCE.md § 4

3. Deploy Deployment, Service, Ingress
   → DEPLOYMENT_GUIDES.md § 1.3-1.4

4. Add Network Policy
   → DEPLOYMENT_GUIDES.md § 1.5

5. Add HPA for auto-scaling
   → DEPLOYMENT_GUIDES.md § 1.6

6. Monitor health and metrics
   → OPERATIONS_AND_TROUBLESHOOTING.md § 1-3
```

### Debugging in Production

```
1. Check health probes
   → OPERATIONS_AND_TROUBLESHOOTING.md § 1

2. Collect diagnostics
   → OPERATIONS_AND_TROUBLESHOOTING.md § 2

3. Review logs
   → OPERATIONS_AND_TROUBLESHOOTING.md § 4

4. Check metrics (if issue is latency/memory)
   → OPERATIONS_AND_TROUBLESHOOTING.md § 3

5. Analyze OCEL traces (if issue is conformance)
   → OPERATIONS_AND_TROUBLESHOOTING.md § 5

6. Use troubleshooting guide
   → OPERATIONS_AND_TROUBLESHOOTING.md § 7
```

### Setting Up CI/CD

```
1. Choose platform (GitHub Actions, GitLab, Jenkins, Cloud Build)
   → REMOTE_EXECUTION.md § 7-8
   or DEPLOYMENT_GUIDES.md § 6-7

2. Configure environment variables
   → CONFIGURATION_REFERENCE.md § 1

3. Set up sibling repo cloning
   → REMOTE_EXECUTION.md § 1

4. Add test and publish steps
   → Workflow examples in DEPLOYMENT_GUIDES.md

5. Configure release gates
   → REMOTE_EXECUTION.md § 6
```

### Optimizing for High Throughput

```
1. Increase resource limits
   → REMOTE_EXECUTION.md § 6.1

2. Configure OCEL buffering
   → CONFIGURATION_REFERENCE.md § 1.3

3. Increase max connections
   → CONFIGURATION_REFERENCE.md § 1.1

4. Tune metrics buckets
   → CONFIGURATION_REFERENCE.md § 1.6

5. Monitor and benchmark
   → OPERATIONS_AND_TROUBLESHOOTING.md § 9.3
```

---

## Related Documentation

**Project Governance**:
- [CLAUDE.md](../CLAUDE.md) — Claude Code integration and hooks
- [AGENTS.md](../AGENTS.md) — Agent composition and isolation

**Technical Architecture**:
- [README.md](../README.md) — Project overview, versioning, examples
- [docs/FEATURES.md](./FEATURES.md) — LSP 3.18 capability matrix
- [docs/TEST_INFRA.md](./TEST_INFRA.md) — Test philosophy and coverage

**Development**:
- [docs/EXAMPLES.md](./EXAMPLES.md) — Example code and explanations
- [docs/CANCELLATION_SAFETY.md](./CANCELLATION_SAFETY.md) — Async safety patterns

---

## File Reference

| File | Purpose | Length | Best For |
|------|---------|--------|----------|
| REMOTE_EXECUTION.md | Container & network setup | ~800 lines | Understanding architecture |
| DEPLOYMENT_GUIDES.md | Platform-specific deployment | ~1000 lines | Step-by-step setup |
| CONFIGURATION_REFERENCE.md | Configuration reference | ~800 lines | Environment setup |
| OPERATIONS_AND_TROUBLESHOOTING.md | Operations & debugging | ~900 lines | Production troubleshooting |
| DEPLOYMENT_INDEX.md (this file) | Navigation guide | — | Finding the right section |

---

## Quick Reference: Environment Variables

**Essential** (most deployments):

```bash
LSP_MAX_BIND_ADDRESS=0.0.0.0:8080
LSP_MAX_LOG_LEVEL=info
LSP_MAX_LOG_FORMAT=json
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
```

**For gate enforcement**:

```bash
LSP_MAX_GATE_ENABLED=true
LSP_MAX_GATE_CHECK_INTERVAL_MS=100
LSP_MAX_GATE_DIAGNOSTICS_PATTERNS="WASM4PM-.*,GGEN-.*"
```

**For observability**:

```bash
LSP_MAX_METRICS_ENABLED=true
LSP_MAX_METRICS_PORT=9091
RUST_LOG="lsp_max=info,tower_lsp=warn"
RUST_BACKTRACE=1
```

**For OCEL tracing**:

```bash
LSP_MAX_OCEL_ENABLED=true
LSP_MAX_OCEL_BUFFER_SIZE=10000
LSP_MAX_OCEL_FLUSH_INTERVAL_SECS=60
```

---

## Summary

This documentation provides **complete operational guidance** for deploying and running lsp-max in production:

- **REMOTE_EXECUTION.md**: Container architecture, networking, resource management
- **DEPLOYMENT_GUIDES.md**: Platform-specific step-by-step instructions
- **CONFIGURATION_REFERENCE.md**: All configuration options and variables
- **OPERATIONS_AND_TROUBLESHOOTING.md**: Monitoring, diagnostics, and troubleshooting

**Start with the Decision Tree above** to find the right section for your use case.

For questions not answered here, consult:
- Project README: overview, versioning, examples
- AGENTS.md: multi-server composition and agent isolation
- CLAUDE.md: Claude Code tool integration
