# Research Analysis: WebAssembly, Rust, and Type-Safe Execution Engines

## Overview
This report analyzes research papers related to WebAssembly (WASM), Rust, type safety, the Curry-Howard correspondence, and formal verification, with a specific focus on their relevance to building type-safe execution engines like `wasm4pm` and implementing typestate lifecycles.

## Key Findings

### 1. Curry-Howard Correspondence as a Design Principle
In the context of the `lsp-max` and `wasm4pm` ecosystems, the **Curry-Howard correspondence** is not merely a theoretical observation but a fundamental design principle. As demonstrated in *Type-Safe Process-Evidence Engineering* (Chatman, 2026), the Rust type system is used to encode process-mining laws such that:
- **Types represent propositions** (e.g., "The workflow net is sound").
- **Programs represent proofs** (e.g., a passing compilation is a constructive proof of soundness).
- **Type-checking is verification** (illegal state transitions are rejected at compile-time).

### 2. Typestate Patterns and Soundness Lifecycles
The **typestate pattern** is extensively used to manage the lifecycle of process artifacts (e.g., Workflow Nets). This pattern:
- Lifts runtime states into the type lattice using zero-cost phantom types (`PhantomData`).
- Ensures that transitions between states (Unchecked → SoundnessClaimed → SoundnessWitnessed) are strictly enforced by ownership and visibility rules.
- Eliminates the need for runtime boolean checks, providing a "sealed capability" that guarantees an artifact has passed through the required verification oracle.

### 3. Object-Centric Process-Evidence Engineering
The shift from case-centric to **Object-Centric Event Data (OCED)** (as defined in OCEL 2.0) requires execution engines to handle multiple interacting object types simultaneously. 
- The `wasm4pm-compat` crate serves as the **axiomatic type authority**, enforcing disjoint universes of events and objects.
- **Evidence Algebra** (a graded R-algebra) is used to formalize the admission, witnessing, and refusal stages of artifacts.
- The **refusal cokernel** in the algebra corresponds directly to the named refusal enums in the Rust implementation, bridging formal theory and executable code.

### 4. Formal Verification and Intermediate Languages
Research into **Dafny as a Verification-Aware Intermediate Language** suggests a path toward automated generation of verified code. By treating software frameworks as **semantic ontologies** (using RDF), the `ggen` tool can invoke **generative functors** to produce LSPs and execution modules that are correct by construction.

### 5. WebAssembly (WASM) and High-Performance Execution
WASM is identified as a primary target for high-performance, type-safe execution engines (`wasm4pm`). The integration of **Differential Syntax Calculus** (using Jacobian matrices for incremental AST updates) allows for sub-millisecond governance of AI-driven code generation within these engines.

---

## Bibliography

### Primary Sources
- **Chatman, S. (2026).** *Type-Safe Process-Evidence Engineering: A Mathematical Framework for Object-Centric Conformance Checking*. PhD Thesis, Department of Computer Science. (Focus: Evidence algebra, Rust typestate, Curry-Howard).
- **lsp-max Architecture Team. (2026).** *The Blue Ocean Singularity: Redefining Software Architecture through Ontology-Driven Generative Intelligence and Axiomatic Process Boundaries*. (Focus: wasm4pm-compat, generative functors, OCED).

### Foundational Literature
- **van der Aalst, W. M. P. (2016).** *Process Mining: Data Science in Action*. 2nd ed. Springer.
- **Berti, A., & van der Aalst, W. M. P. (2020).** *Object-Centric Petri Nets (OCPN)*.
- **Ghahfarokhi, A., et al. (2021).** *OCEL: A standard for object-centric event logs*. ADBIS 2021.
- **Küsters, A., & van der Aalst, W. M. P. (2025).** *OCPQ: Object-Centric Process Querying & Constraints*.
- **Cardelli, L. (1997).** *Type systems*. CRC Handbook of Computer Science and Engineering.
- **Curry, H. B., & Feys, R. (1958).** *Combinatory Logic*, vol. 1. North-Holland.
- **Hack, M. H. T. (1975).** *Decision Problems for Petri Nets and Vector Addition Systems*. MIT Technical Report.
- **Esparza, J., & Desel, J. (1995).** *Free Choice Petri Nets*. Cambridge University Press.
- **Amari, S. (2016).** *Information Geometry and Its Applications*. Springer.

### Related Research
- **Unknown. (2025).** *Dafny as Verification-Aware Intermediate Language for Code Generation*.
- **De Santis, F., et al. (YYYY).** *Compliance-Aware Predictive Process Monitoring: A Neuro-Symbolic Approach*.
- **Unknown. (YYYY).** *Session Types for the Transport Layer: Towards an Implementation of TCP**.
- **Hawkins, M., & Vuduc, R. (2025).** *Back to Bits: Extending Shannon's communication performance framework to computing*.
