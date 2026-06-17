# Deployment & Remote Execution Documentation for lsp-max

Welcome! This directory now contains comprehensive documentation for deploying and operating lsp-max in remote environments, cloud containers, CI/CD pipelines, and agent systems.

## Getting Started

**Start here:** Read [`DEPLOYMENT_INDEX.md`](./DEPLOYMENT_INDEX.md) for navigation and a decision tree.

## The 5 Core Documents

| Document | Purpose | Length | Best For |
|----------|---------|--------|----------|
| [`DEPLOYMENT_INDEX.md`](./DEPLOYMENT_INDEX.md) | Navigation & decision tree | 406 lines | Finding the right guide |
| [`REMOTE_EXECUTION.md`](./REMOTE_EXECUTION.md) | Container architecture & environment | 1,342 lines | Understanding deployment constraints |
| [`DEPLOYMENT_GUIDES.md`](./DEPLOYMENT_GUIDES.md) | Platform-specific step-by-step | 1,239 lines | Setting up on your platform |
| [`CONFIGURATION_REFERENCE.md`](./CONFIGURATION_REFERENCE.md) | All configuration options | 799 lines | Environment variable reference |
| [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md) | Operations & debugging | 911 lines | Production troubleshooting |

**Total:** 4,697 lines of production-ready documentation

## Quick Navigation

### I want to...

**Deploy to Kubernetes production**
1. Read: [`DEPLOYMENT_INDEX.md`](./DEPLOYMENT_INDEX.md) - Decision Tree
2. Follow: [`DEPLOYMENT_GUIDES.md`](./DEPLOYMENT_GUIDES.md) § 1
3. Configure: [`CONFIGURATION_REFERENCE.md`](./CONFIGURATION_REFERENCE.md) § 4

**Run locally with Docker Compose**
1. Follow: [`DEPLOYMENT_GUIDES.md`](./DEPLOYMENT_GUIDES.md) § 2
2. Configure: [`CONFIGURATION_REFERENCE.md`](./CONFIGURATION_REFERENCE.md) § 5
3. Debug: [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md) § 2

**Set up CI/CD (GitHub Actions / GitLab / Jenkins)**
1. Read: [`DEPLOYMENT_GUIDES.md`](./DEPLOYMENT_GUIDES.md) § 6-7
2. Reference: [`REMOTE_EXECUTION.md`](./REMOTE_EXECUTION.md) § 7-8
3. Configure: [`CONFIGURATION_REFERENCE.md`](./CONFIGURATION_REFERENCE.md) § 1

**Deploy on AWS or GCP**
1. Follow: [`DEPLOYMENT_GUIDES.md`](./DEPLOYMENT_GUIDES.md) § 3-4
2. Configure: [`CONFIGURATION_REFERENCE.md`](./CONFIGURATION_REFERENCE.md)
3. Monitor: [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md) § 1

**Integrate with custom agent system**
1. Setup: [`DEPLOYMENT_GUIDES.md`](./DEPLOYMENT_GUIDES.md) § 5
2. Understand: [`REMOTE_EXECUTION.md`](./REMOTE_EXECUTION.md) § 5
3. Monitor: [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md) § 2

**Debug production issues**
1. Diagnose: [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md) § 7
2. Check health: [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md) § 1
3. Analyze logs: [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md) § 4

## Document Highlights

### REMOTE_EXECUTION.md
- Container architecture (Docker, sibling repos, resources)
- Network policies & security (mTLS, secrets)
- Git integration patterns
- Session lifecycle management
- Resource limits & quotas
- CI/CD integration (GitHub Actions, GitLab, Jenkins, Cloud Build)
- OpenTelemetry & observability setup
- Common troubleshooting with solutions

### DEPLOYMENT_GUIDES.md
- **Kubernetes**: Complete manifests (Deployment, Service, Ingress, NetworkPolicy, HPA)
- **Docker Compose**: Single and multi-region setups
- **AWS**: ECS Fargate, EC2 Auto Scaling, CloudFormation
- **GCP**: Cloud Run, GKE
- **Agents**: Client setup and query patterns
- **CI/CD**: GitHub Actions and release workflows
- **Monitoring**: Prometheus & Grafana dashboards

### CONFIGURATION_REFERENCE.md
- 40+ environment variables documented
- Complete YAML schema with examples
- Environment-specific configs (dev/prod/k8s)
- Kubernetes ConfigMap/Secrets integration
- Configuration priority rules
- Validation procedures
- Performance tuning guidance

### OPERATIONS_AND_TROUBLESHOOTING.md
- Liveness/readiness health probes
- Diagnostic collection & filtering
- Prometheus metrics setup
- Structured JSON log analysis
- OCEL trace analysis
- Receipt verification
- 6 detailed troubleshooting scenarios
- Maintenance procedures
- Performance optimization
- Safe upgrade procedures

### DEPLOYMENT_INDEX.md
- Decision tree for choosing deployment path
- Quick navigation by use case
- Document crossref table
- Common workflow examples
- Essential environment variables reference

## Key Features

- **Comprehensive**: 4,697 lines covering entire deployment lifecycle
- **Practical**: Complete working manifests, not snippets
- **Platform-agnostic**: Kubernetes, Docker, AWS, GCP, custom systems
- **Production-ready**: Health checks, monitoring, troubleshooting included
- **Well-organized**: Decision tree and quick navigation
- **Aligned**: Respects lsp-max architecture (law-state, gate, OCEL, receipts)

## Platforms Covered

| Platform | Coverage | Reference |
|----------|----------|-----------|
| Kubernetes | Complete manifests & HPA | DEPLOYMENT_GUIDES § 1 |
| Docker Compose | Local & multi-region | DEPLOYMENT_GUIDES § 2 |
| AWS | ECS Fargate & EC2 ASG | DEPLOYMENT_GUIDES § 3 |
| GCP | Cloud Run & GKE | DEPLOYMENT_GUIDES § 4 |
| GitHub Actions | Matrix testing & release | DEPLOYMENT_GUIDES § 6 |
| GitLab CI | Full pipeline example | REMOTE_EXECUTION § 8 |
| Jenkins | Full pipeline example | REMOTE_EXECUTION § 8 |
| Cloud Build | GCP CI/CD integration | REMOTE_EXECUTION § 8 |
| Custom Agents | Client library integration | DEPLOYMENT_GUIDES § 5 |

## Configuration Reference

All environment variables documented:

- **Server** (bind, log level, format)
- **Session** (timeout, request timeout)
- **OCEL** (buffer size, flush interval, storage)
- **Gate** (check interval, ANDON patterns)
- **Observability** (OTel endpoint, Prometheus port, metrics)
- **External** (git token, SSH key, HTTP proxy)

Plus complete YAML schema with environment-specific examples.

## Operations & Monitoring

Complete operational guidance:

- Health checks (liveness/readiness probes)
- Diagnostic collection & analysis
- Performance monitoring (Prometheus)
- Log analysis (JSON structured logs)
- OCEL trace verification
- Receipt cryptographic verification
- Graceful shutdown
- Log rotation & cleanup
- Performance profiling
- Safe upgrades (rolling updates, canary)

## Troubleshooting Coverage

Detailed procedures for common issues:

1. **Gate stuck in ANDON** - diagnosis, resolution, prevention
2. **Memory growth** - buffer flushing, caching, cleanup
3. **Request timeout on large files** - timeout tuning, streaming
4. **Docker build failures** - sibling repo checkout order
5. **LSP server unreachable** - networking, firewall checks
6. **OTel traces not appearing** - endpoint verification, debug logging

Each with root cause analysis, step-by-step resolution, and monitoring.

## Project Alignment

Documentation respects lsp-max project requirements:

- Λ_CD gate enforcement explained
- OCEL trace collection & analysis
- Receipt-chain verification
- ConformanceVector three-valued logic
- CalVer versioning (not SemVer)
- Sibling repository dependencies documented
- Multi-client architecture (agents, CI, release gates)
- Read-only LSP surface (diagnostics/intents, no mutations)

## Cross-References

These docs complement existing lsp-max documentation:

- [`CLAUDE.md`](../CLAUDE.md) - Claude Code integration hooks
- [`AGENTS.md`](../AGENTS.md) - Agent composition & isolation
- [`README.md`](../README.md) - Project overview
- [`docs/FEATURES.md`](./FEATURES.md) - LSP 3.18 capability matrix
- [`docs/TEST_INFRA.md`](./TEST_INFRA.md) - Test architecture
- [`/.github/workflows/ci.yml`](../.github/workflows/ci.yml) - Existing CI

## Next Steps

1. **Read** [`DEPLOYMENT_INDEX.md`](./DEPLOYMENT_INDEX.md) (navigation)
2. **Choose** your deployment platform from the decision tree
3. **Follow** the recommended section from [`DEPLOYMENT_GUIDES.md`](./DEPLOYMENT_GUIDES.md)
4. **Configure** using [`CONFIGURATION_REFERENCE.md`](./CONFIGURATION_REFERENCE.md)
5. **Monitor** with [`OPERATIONS_AND_TROUBLESHOOTING.md`](./OPERATIONS_AND_TROUBLESHOOTING.md)
6. **Troubleshoot** using section 7 when issues occur

## Document Statistics

- **Total Lines**: 4,697
- **Total Size**: ~113 KB
- **5 Documents**: Each focused on specific deployment aspect
- **40+ Environment Variables**: All documented
- **9 Platforms**: Kubernetes, Docker, AWS, GCP, GitHub Actions, GitLab, Jenkins, Cloud Build, Custom Agents
- **6 Troubleshooting Scenarios**: Complete with diagnosis & resolution
- **Complete Examples**: Not snippets—production-ready manifests

---

**Start with [`DEPLOYMENT_INDEX.md`](./DEPLOYMENT_INDEX.md) for navigation and decision tree.**

Questions? Refer to the cross-reference documents or consult the troubleshooting section.
