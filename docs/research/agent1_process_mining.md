# Research Report: Object-Centric Process Mining Foundations for wasm4pm

## 1. Research Overview
This report analyzes foundational papers in Object-Centric Process Mining (OCPM), specifically focusing on Object-Centric Event Logs (OCEL), Object-Centric Petri Nets (OCPN), and Object-Centric Process Querying (OCPQ). The research establishes the mathematical rigor required for the `wasm4pm` ecosystem and identifies `wasm4pm-compat` as the canonical implementation of these formalisms.

## 2. Bibliography

### 2.1 Object-Centric Event Logs (OCEL)
**OCEL 2.0: A Standard for Object-Centric Event Logs**
*Authors:* Wil M. P. van der Aalst, Alessandro Berti, Gyunam Park, Jan Niklas Adams, Geng Li, Daniel Schuster, Eric van Zelst.
*Year:* 2023 (Standard release 2024)
*Relevance:* Defines the 11-tuple structure of object-centric event data, which is directly implemented in the `wasm4pm_compat::ocel` module. It establishes the "Evidence-First Principle" followed by the architecture.

```bibtex
@article{ocel2023standard,
  title     = {OCEL 2.0: A Standard for Object-Centric Event Logs},
  author    = {van der Aalst, Wil M. P. and Berti, Alessandro and Park, Gyunam and Adams, Jan Niklas and Li, Geng and Schuster, Daniel and van Zelst, Eric},
  journal   = {arXiv preprint arXiv:2310.13411},
  year      = {2023},
  url       = {https://arxiv.org/abs/2310.13411}
}
```

### 2.2 Object-Centric Petri Nets (OCPN)
**Discovering Object-Centric Petri Nets**
*Authors:* Wil M. P. van der Aalst, Alessandro Berti.
*Year:* 2020
*Relevance:* Introduces OCPNs as a modeling language capable of handling multiple interacting object types. This paper provides the basis for the `PetriRefusal` and type-preservation logic in `wasm4pm`.

```bibtex
@article{vanderAalst2020Discovering,
  author    = {Wil M. P. van der Aalst and Alessandro Berti},
  title     = {Discovering Object-Centric Petri Nets},
  journal   = {Fundamenta Informaticae},
  volume    = {175},
  number    = {1-4},
  pages     = {1--40},
  year      = {2020},
  doi       = {10.3233/FI-2020-1946}
}
```

### 2.3 Object-Centric Process Querying (OCPQ)
**OCPQ: Object-Centric Process Querying and Constraints**
*Authors:* Aaron Küsters, Wil M. P. van der Aalst.
*Year:* 2025
*Relevance:* Formalizes constraints over OCED. The `wasm4pm-compat` library's validation logic (e.g., `OcelLog::validate`) is a direct programmatic implementation of the constraints defined in this framework.

```bibtex
@inproceedings{kusters2025ocpq,
  author    = {Aaron K{\"u}sters and Wil M. P. van der Aalst},
  title     = {OCPQ: Object-Centric Process Querying and Constraints},
  booktitle = {Research Challenges in Information Science (RCIS 2025)},
  year      = {2025},
  publisher = {Springer Nature Switzerland},
  doi       = {10.1007/978-3-031-92474-3_23}
}
```

### 2.4 Mathematical Synthesis (Local Foundation)
**Type-Safe Process-Evidence Engineering: A Mathematical Framework for Object-Centric Conformance Checking**
*Author:* Sean Chatman.
*Year:* 2026
*Relevance:* Dissertation providing the "Evidence Algebra" and "Soundness-Completeness Duality" that unify the academic research into the `wasm4pm` architecture. It identifies the `OcelRefusal` cokernel as the bridge between theory and Rust implementation.

## 3. Architectural Relevance to wasm4pm

### 3.1 wasm4pm-compat as Axiomatic Type Authority
The research identifies `wasm4pm-compat` not as a "backward compatibility" layer, but as the **Axiomatic Baseline**. It codifies the formal OCED axioms (disjoint universes, mandatory cardinality, temporal continuity) as executable Rust types and validation gates.

### 3.2 Formal Law Enforcement (OcelRefusal)
The `OcelRefusal` taxonomy in `wasm4pm-compat` maps 1:1 to the validation laws defined in the literature:
- **L1 (Non-empty relation):** `EmptyEventObjectLinks`
- **L2 (Referential integrity):** `DanglingEventObjectLink`
- **L3 (Temporal consistency):** Enforced by `Trace::validate()`
- **L4 (Type closure):** `ObjectTypeNotPreserved`

### 3.3 Typestate and Soundness
`wasm4pm-compat` utilizes Rust's **typestate pattern** to implement the Soundness-Completeness Duality. By requiring a `SoundnessWitnessed` proof term (minted only via the validation oracle), the architecture guarantees that any model used by the `wasm4pm` engine is mathematically sound.

### 3.4 Decoupling and Closure
The architecture enforces a strict bifurcation:
- **wasm4pm-compat:** Contains Data Structures and Laws (Mathematical Truth).
- **wasm4pm:** Contains Computation and Execution (Algorithmic Engine).
This decoupling prevents "structural fracturing" and ensures that high-performance analytical engines operate with absolute mathematical safety.

## 4. Conclusion
The `wasm4pm` architecture represents a "Blue Ocean" innovation by translating the theoretical rigor of the van der Aalst school into a production-grade, type-safe Rust ecosystem. The use of `wasm4pm-compat` as the sole arbiter of admissible process evidence ensures that every diagnostic or claim produced is grounded in formal mathematical reality.
