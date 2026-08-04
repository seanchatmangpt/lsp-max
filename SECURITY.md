# Security Policy

## Supported versions

Security fixes are applied to the default branch and the latest released CalVer line. Older snapshots are supported only when a maintainer explicitly backports a fix.

## Reporting a vulnerability

Do not open a public issue for an undisclosed vulnerability. Use GitHub's private vulnerability reporting for this repository when available. Include:

- affected commit, tag, or crate version
- impacted component and execution boundary
- minimal reproduction
- expected and observed behavior
- confidentiality, integrity, availability, or authority impact
- whether the issue permits unreceipted actuation, forged standing, replay, path escape, protocol-frame corruption, or denial of service

Do not include production secrets, personal data, or third-party credentials in the report.

## Response and disclosure

A report remains `UNKNOWN` until reproduced against the exact identified subject. Maintainers will classify it as `ALIVE`, `PARTIAL_ALIVE`, `BLOCKED`, `UNSUPPORTED`, or a typed `REFUSED` state as evidence becomes available. Public disclosure should occur after a patch and bounded verification are available, coordinated with the reporter when practical.

## Security boundaries

Changes affecting these areas require positive and negative controls:

- receipt signatures, digest binding, sequence continuity, and replay
- LSP framing and stdout isolation
- filesystem paths and repository-bound dependency resolution
- process execution, hooks, brokers, and actuation authority
- remote content, LSIF, RDF, SPARQL, and semantic-store admission
- CI credentials, pull-request checkout identity, and workflow permissions

Automated workflows must use least privilege, exact source identity, fail-closed commands, and no real `cargo publish` execution.
