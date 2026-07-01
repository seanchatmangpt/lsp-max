# Oxigraph Store Exploration Report

## Executive Summary
This report analyzes how `oxigraph::store::Store` is and should be integrated within the `lsp-max` workspace. We propose a robust configuration structure and dynamic factory instantiation pattern supporting both ephemeral in-memory execution (the default for testing and CI) and RocksDB-backed persistent storage (via configured filesystem paths), complying with W3C RDF and SPARQL W3C standards.

---

## 1. Current Workspace Usage & Footprint

As of the current codebase state, `oxigraph` is declared as a dependency in `crates/lsp-max-lsif/Cargo.toml` at version `0.5.8`:
```toml
[dependencies]
lsp-types = "0.97.0"
oxigraph = "0.5.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

No active Rust source files (`.rs`) in the workspace import or instantiate `oxigraph::store::Store` yet. The integration is currently defined only in design blueprints:
- **`docs/reports/ARD-OXIGRAPH-SPARQL.md`**: Outlines using Oxigraph v0.5.8 as the Admitted Graph Control Plane, defining vocabulary namespace mappings, Rust boundary abstractions, and SPARQL invariant queries.
- **`docs/reports/lsp-max-v26.6.5-prd-ard.md`**: Defines W3C semantic web alignment (RDF, SPARQL, SHACL) and requires separating query execution from the interactive LSP hot path.

---

## 2. Oxigraph Store v0.5.8 API Capabilities

`oxigraph::store::Store` is a Rust-native, high-performance RDF graph database offering the following capabilities:
1. **Storage Modes**:
   - **In-Memory**: Initialized via `Store::new()`. Ephemeral, fast, and does not touch the disk.
   - **Persistent**: Initialized via `Store::open(path)`. Backed by RocksDB for atomicity, durability, and high concurrency.
2. **Thread Safety**: `Store` implements `Send + Sync`, allowing it to be safely shared across threads and queried concurrently via W3C SPARQL.
3. **Transactions**: Supports ACID transactions for quad insertion and removal.
4. **SPARQL 1.1/1.2 Support**: Multi-threaded execution of SPARQL query operations.

---

## 3. Proposed Configuration Structure

To support clean instantiation, configuration should be represented in a structured Rust format that easily maps to CLI flags, environment variables, and user setting files.

### 3.1 Rust Configuration Definition
We recommend placing the configuration struct in a shared location, such as `lsp-max-protocol::core` or a new dedicated module:

```rust
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Configuration options for instantiating the Oxigraph RDF Store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OxigraphStoreConfig {
    /// Optional filesystem path to the persistent RocksDB storage directory.
    /// If `None`, the store is initialized in-memory (default, ideal for testing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_path: Option<PathBuf>,
}
```

### 3.2 Configuration Sources Hierarchy
Configuring the storage path should follow a standard priority hierarchy:
1. **CLI Arguments**: E.g., `--storage-path /var/lib/lsp-max/store`
2. **Environment Variables**: E.g., `TOWER_LSP_MAX_STORE_PATH`
3. **User Configuration File**: E.g., the `~/.lsp-max-config.json` managed by `ConfigService` (specifically the `store.storage_path` key).
4. **Default**: `None` (triggers ephemeral in-memory execution).

---

## 4. Instantiation Factory & Error Handling

To isolate initialization logic and handle filesystem side-effects gracefully, we propose the `OxigraphStoreFactory` pattern.

### 4.1 Implementation Blueprint
```rust
use std::fs;
use std::path::{Path, PathBuf};
use oxigraph::store::{Store, StorageError};
use thiserror::Error;

/// Error types occurring during Oxigraph Store initialization.
#[derive(Debug, Error)]
pub enum StoreInitializationError {
    /// Occurs when parent directories for the persistent path cannot be created.
    #[error("Failed to create parent directories for persistent path {path:?}: {source}")]
    DirectoryCreationFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Occurs when RocksDB fails to open the persistent directory (e.g., lock conflict).
    #[error("Failed to open persistent RocksDB store at {path:?}: {source}")]
    PersistentOpenFailed {
        path: PathBuf,
        #[source]
        source: StorageError,
    },

    /// Occurs when an in-memory store fails to initialize.
    #[error("Failed to initialize ephemeral in-memory store: {source}")]
    InMemoryInitFailed {
        #[source]
        source: StorageError,
    },
}

/// A factory for creating instances of `oxigraph::store::Store`.
pub struct OxigraphStoreFactory;

impl OxigraphStoreFactory {
    /// Instantiates a W3C-compliant `Store` based on the given configuration.
    pub fn create_store(config: &OxigraphStoreConfig) -> Result<Store, StoreInitializationError> {
        match &config.storage_path {
            None => {
                // Instantiates an ephemeral, thread-safe in-memory store.
                Store::new().map_err(|e| StoreInitializationError::InMemoryInitFailed { source: e })
            }
            Some(path) => {
                // Ensure parent directories exist before instantiating RocksDB.
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent).map_err(|e| {
                            StoreInitializationError::DirectoryCreationFailed {
                                path: path.clone(),
                                source: e,
                            }
                        })?;
                    }
                }
                // Opens or creates the persistent RocksDB store.
                Store::open(path).map_err(|e| StoreInitializationError::PersistentOpenFailed {
                    path: path.clone(),
                    source: e,
                })
            }
        }
    }
}
```

---

## 5. Integration with the lsp-max Control Plane

The initialized `Store` acts as the execution kernel of the Admitted Graph Control Plane. We recommend placing the boundary wrapper (`AdmittedGraph`) inside `crates/lsp-max-lsif` or `lsp-max-runtime`.

```
                    +--------------------------------+
                    |      Source Observations       |
                    +--------------------------------+
                                    |
                                    v (Parse Ingress)
                    +--------------------------------+
                    |    Parsed AST / Raw Triples    |
                    +--------------------------------+
                                    |
                                    v (Relation Admitter)
                    +--------------------------------+
                    |      W3C RDF Graph Ingestion   |
                    |    (in-memory vs persistent)   |
                    +--------------------------------+
                                    |
                                    v (Asynchronous Control Plane)
                    +--------------------------------+
                    |   SPARQL Invariant Execution   |
                    +--------------------------------+
                                    |
                                    v (Projection Queries)
                    +--------------------------------+
                    |     Materialized View Update   |
                    |     (In-Memory DashMap)        |
                    +--------------------------------+
                                    |
                                    v (Interactive LSP hot-path, sub-5ms)
                    +--------------------------------+
                    |     JSON-RPC Response Route    |
                    +--------------------------------+
```

### 5.1 RDF Quad Ingestion Schema (LSIF to RDF & Diagnostics)
During ingestion, standard LSIF NDJSON streams and LiveLSP diagnostics are parsed and converted to `oxrdf::Quad` structures before being committed to the store:

```rust
use oxrdf::{Quad, NamedNode, Subject, Term, GraphName};

/// Maps LSIF contains relation to RDF quads.
pub fn construct_lsif_quad(
    out_v: &str, 
    in_v: &str, 
    snapshot_id: &str
) -> Quad {
    let graph_name = GraphName::NamedNode(
        NamedNode::new(format!("urn:project:local:snapshot:{}", snapshot_id)).unwrap()
    );
    let subject = Subject::NamedNode(
        NamedNode::new(format!("urn:project:local:lsif:{}", out_v)).unwrap()
    );
    let predicate = NamedNode::new("https://microsoft.github.io/language-server-protocol/lsif/0.6.0/contains").unwrap();
    let object = Term::NamedNode(
        NamedNode::new(format!("urn:project:local:lsif:{}", in_v)).unwrap()
    );

    Quad::new(subject, predicate, object, graph_name)
}
```

### 5.2 Materialized Views (Bypassing Hot-Path SPARQL)
As required by product requirement **PRD-R5**, W3C SPARQL queries are too slow to run on every human keystroke. The `Store` must only write and update an in-memory `DashMap` containing pre-computed views (definition, reference, hover, diagnostics) asynchronously:
- **Write Path**: Admission updates the persistent `Store`, executes SPARQL validation, and triggers projections.
- **Read Path**: The LSP server handles interactive requests (e.g. `textDocument/definition`) strictly via `DashMap` lookups, bypassing RocksDB entirely.

---

## 6. Testing Strategy & Verification Method

To ensure reliable, deterministic, and isolated testing:
1. **In-Memory Default**: Unit tests and integration test suites (like `test_max_rpc_handlers`) must use `OxigraphStoreConfig { storage_path: None }`. This avoids disk locks, filesystem corruption, and cross-talk between parallel test runs.
2. **RocksDB Integration Tests**: A dedicated test suite should target `storage_path: Some(temp_dir)` to verify correct directory creation, write durability, and database state recovery.

### 6.1 Verification Commands
To compile and test the proposed store configuration, run:
```bash
cargo test --package lsp-max-lsif
```
