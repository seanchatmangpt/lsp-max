# Research Analysis: LSP Tooling and State Enforcement

## Overview
This document summarizes research findings related to Language Server Protocol (LSP), static analysis, and IDE tooling, with a specific focus on their relevance to the `lsp-max` and `lsp-types-max` architecture. The primary source of this analysis is the PhD dissertation by Sean Chatman (2026) titled *"Type-Safe Process-Evidence Engineering: A Mathematical Framework for Object-Centric Conformance Checking"*, supplemented by its cited literature.

## Bibliography Entries

### Primary Work
*   **Chatman, S. (2026).** *Type-Safe Process-Evidence Engineering: A Mathematical Framework for Object-Centric Conformance Checking*. PhD Dissertation, Department of Computer Science.
    *   *Relevance:* Provides the core mathematical and type-theoretic foundations for `lsp-max`, including the Evidence Algebra and the implementation of conformance gates in Rust.

### Foundations (Petri Nets & Process Mining)
*   **van der Aalst, W. M. P. (2016).** *Process Mining: Data Science in Action*. 2nd ed. Springer.
    *   *BibTeX:*
        ```bibtex
        @book{vanderaalst2016process,
          author    = {Wil M. P. van der Aalst},
          title     = {Process Mining: Data Science in Action},
          edition   = {2nd},
          publisher = {Springer},
          year      = {2016}
        }
        ```
*   **Berti, A., & van der Aalst, W. M. P. (2020).** *Extracting multiple viewpoint models from relational databases*. SIMPDA 2018.
*   **Ghahfarokhi, A., et al. (2021).** *OCEL: A standard for object-centric event logs*. ADBIS 2021.
*   **Hack, M. H. T. (1975).** *Decision Problems for Petri Nets and Vector Addition Systems*. Technical Report TR-95, MIT.
*   **Esparza, J., & Desel, J. (1995).** *Free Choice Petri Nets*. Cambridge University Press.

### Type Systems & Implementation
*   **Cardelli, L. (1997).** *Type systems*. CRC Handbook of Computer Science and Engineering.
    *   *Relevance:* Foundations for the typestate patterns and zero-cost abstractions used in `lsp-max-runtime`.
*   **Curry, H. B., & Feys, R. (1958).** *Combinatory Logic*. North-Holland.
    *   *Relevance:* Theoretical basis for the Curry-Howard correspondence used to map process laws to Rust types.
*   **The Rust Reference (2021).**
    *   *Relevance:* Critical for the implementation of const-generics and phantom data markers in the conformance engine.

### Advanced Topics
*   **Amari, S. (2016).** *Information Geometry and Its Applications*. Springer.
    *   *Relevance:* Used to analyze the "Quality Manifold" and the trade-off between fitness and precision in project state models.
*   **Pesić, M., & van der Aalst, W. M. P. (2006).** *A declarative approach for flexible business processes management*. BPM 2006.
    *   *Relevance:* Foundations for the `Declare` constraint-based process specification implemented in `lsp-max`.

## Relevance to Architecture

### 1. lsp-max: Post-Human State Enforcement
Research indicates a shift from IDEs as human assistance tools to servers as **admission controllers** for autonomous agents. `lsp-max` adopts this "post-human" paradigm by:
*   Implementing **Evidence Algebra** (Chatman, 2026) to track the lifecycle of project artefacts (Admitted, Witnessed, Refused).
*   Treating **Diagnostics as Refused Transitions**: A diagnostic is a formal block by the kernel to prevent an illegal state transition, rather than a visual hint.

### 2. lsp-types-max: Generated Protocol Truth
To support deterministic agent workflows, `lsp-types-max` utilizes automated generation from the LSP 3.18 `metaModel.json`. This aligns with the "Protocol Doctrine" of establishing a single source of truth to prevent manual drift and ensure typestate security across the `lsp-max-runtime`.

### 3. Conformance and Repair
The research leverages **Conformance Checking** (van der Aalst, 2012) to evaluate project state against formal laws. `lsp-max` implements this via:
*   `max/conformanceVector`: Quantifying compliance.
*   `max/applyRepairTransaction`: Executing atomic, verifiable state transitions based on repair plans (Pesić, 2006).

### 4. Mathematical Integrity
The use of **Petri Nets** and **LTL (Linear Temporal Logic)** ensures that the state transitions are mathematically sound. The architecture maps these formalisms directly to the Rust type system, where a program that type-checks is effectively a proof of protocol compliance.
