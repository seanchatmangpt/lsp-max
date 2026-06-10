# PRD/ARD: The Calculus of Manufactured Intelligence 
## lsp-max v26.6.5 — Oxigraph/SPARQL Admitted Graph Control Plane

**Release Classification:** `Status: PARTIAL_ALIVE → ALIVE candidate | Release: v26.6.5`

This document serves as the formal architectural thesis and product requirements definition for `lsp-max` v26.6.5. It transcends traditional heuristic software engineering, framing the system through the **Calculus of Manufactured Intelligence**: where process discovery, LSIF projection, SPARQL graph evaluation, POWL alignments, and BLAKE3 receipts form a zero-trust mathematical execution calculus.

---

## 1. The Master Equation & Epistemological Shift

We reject the premise that process mining and language server intelligence are disjoint heuristic analyses. They are formal executable mathematical authorities.

Let $\mathcal{L}$ be the universe of **language-bearing artifacts**:
$$ \mathcal{L} = \mathcal{L}_{code} \sqcup \mathcal{L}_{config} \sqcup \mathcal{L}_{legal} \sqcup \mathcal{L}_{process} \sqcup \mathcal{L}_{scripture} \sqcup \mathcal{L}_{policy} \sqcup \mathcal{L}_{agent} \sqcup \mathcal{L}_{institution} $$

Let raw observation be $O \in \Omega_{\mathrm{raw}}$.

Let admission be a partial morphism:
$$ \alpha: \Omega_{\mathrm{raw}} \rightharpoonup \Omega^{\star} \sqcup \bot_{\mathrm{refused}} $$

where $O^{\star} = \alpha(O)$ is **admitted, bounded, typed, queryable, process-ready state**.

The ultimate objective of `lsp-max`, realized through the integration of the Oxigraph/SPARQL control plane, is the execution of **Manufactured Intelligence**:

$$ \boxed{ \mathfrak{MI}(O) = \rho \circ \mu \circ \kappa \circ \pi \circ \alpha (O) } $$

Where:
*   $\alpha$: Admission / refusal boundary (LiveLSP).
*   $\pi$: Projection into graph/formal model (LSIF).
*   $\kappa$: Conformance / validation / route law (SPARQL/SHACL).
*   $\mu$: Bounded actuation / execution / manufacturing (WASM/ggen).
*   $\rho$: Cryptographic receipt functor.

The compact doctrine enforced by v26.6.5:
$$ \boxed{ R \vdash A = \mu(O^{\star}) } $$
*No receipt, no claim.*

---

## 2. Language-to-State Admission (The Observation Boundary)

A language artifact is not text; it is a typed field over possible process consequences. 

Let $\lambda \in \mathcal{L}$. We define a meaning-bearing relation extractor, transforming the surface into a candidate graph:
$$ \eta: \mathcal{L} \to \mathcal{G}_{cand} $$
where $\mathcal{G}_{cand} = (V_{cand}, E_{cand}, \ell_V, \ell_E)$.

Admission into the core `lsp-max` Registry is a strict constraint intersection:
$$ O^{\star} = \alpha(O) = O \cap \mathsf{Type} \cap \mathsf{Shape} \cap \mathsf{Authority} \cap \mathsf{Process} \cap \mathsf{Receiptability} $$

If any predicate fails, $\alpha(O) = \bot_{\mathrm{refused}}^{i}$, with refusal class $i \in \{\mathrm{untyped}, \mathrm{unauthorized}, \mathrm{unshaped}, \mathrm{unroutable}, \mathrm{unreceiptable}, \mathrm{nonreplayable}\}$.

---

## 3. RDF / SPARQL / Oxigraph Control Plane (The $\kappa$ Layer)

`lsp-max` v26.6.5 materializes the validation and projection layer ($\kappa$ and $\pi$) via **Oxigraph v0.5.8**.

Let admitted graph state be an RDF dataset:
$$ \mathcal{D}^{\star} = \langle G_0, G_1, \dots, G_k \rangle $$

Each named graph $G_i$ (implemented via `oxrdf::GraphName`) corresponds to a discrete law surface bound by cryptographic snapshot IDs:
$$ G_i \in \{ G_{\mathrm{LSIF}}, G_{\mathrm{LSP}}, G_{\mathrm{POWL}}, G_{\mathrm{OCEL}}, G_{\mathrm{MCP}}, G_{\mathrm{A2A}}, G_{\mathrm{PROV}}, G_{\mathrm{RECEIPT}} \} $$

A SPARQL query executed by `SparqlEvaluator` is a graph-pattern morphism:
$$ q: \mathcal{D}^{\star} \to \mathcal{B} $$
where $\mathcal{B} \subseteq Var(q) \times Term(\mathcal{D}^{\star})$.

The query is admissible iff $\mathrm{dom}(q) \subseteq \mathrm{NamedGraphs}(\mathcal{D}^{\star})$ and $\mathrm{range}(q) \subseteq \mathsf{AdmittedTerms}$.

A SHACL-style shape system defines the subobject classifier for validation:
$$ \chi_S: \mathcal{D}^{\star} \to \{\top, \bot\}^{m} $$
producing the valid graph state: $\mathcal{D}^{\star}_{valid} = \chi_S^{-1}(\top^m)$.

**The Control-Plane Law:**
$$ \boxed{ \mathrm{SPARQL} = \mathrm{projection}, \quad \mathrm{SHACL} = \mathrm{shape}, \quad \mathrm{LSIF} = \mathrm{static\ language\ artifact}, \quad \rho = \mathrm{proof} } $$

---

## 4. LSIF as Static Intelligence Reservoir

Let LSIF be a directed typed graph:
$$ \mathcal{I} = (V_{\mathrm{lsif}}, E_{\mathrm{lsif}}, \theta) $$

In v26.6.5, we applied combinatorial maximalism to strictly define edge typing via `ItemEdgeProperty`:
$$ \theta: E_{\mathrm{lsif}} \to \{ \mathrm{definitions}, \mathrm{declarations}, \mathrm{references}, \mathrm{referenceResults}, \mathrm{implementationResults}, \mathrm{typeDefinitions}, \mathrm{referenceLinks} \} $$

A static answer to an LSP client is a materialized projection (via `DashMap` caching):
$$ m_q: \mathcal{I} \to Ans_q $$
where $q \in \{hover, definition, references, \dots\}$.

**The Reservoir Law:**
$$ \boxed{ \mathrm{LSP}_{live} = \mathrm{thin\ protocol\ surface}, \quad \mathrm{LSIF}_{static} = \mathrm{stored\ relation\ state} } $$

Crucially, in the hot path:
$$ Ans_q = m_q(\mathcal{I}) \neq q_{\mathrm{SPARQL}}(\mathcal{D}^{\star}) $$

SPARQL verifies and projects out-of-band: $q_{\mathrm{SPARQL}}: \mathcal{D}^{\star} \to \mathcal{I}^{valid}$.
LSIF serves synchronously: $\mathcal{I}^{valid} \to Ans_q$.
Oxigraph is the control plane, not the execution kernel.

---

## 5. The Receipt Functor ($\rho$)

Let $\mathbf{Exec}$ be the category whose objects are admitted execution states and morphisms are bounded transitions $f_i: S_i^{\star} \to S_{i+1}^{\star}$.
Let $\mathbf{Hash}$ be the category of digest states.

The receipt functor implemented in `lsp-max`:
$$ \rho: \mathbf{Exec} \to \mathbf{Hash} $$
such that:
$$ \rho(S_{i+1}) = H_{i+1} = \mathrm{BLAKE3}( H_i \Vert id(f_i) \Vert S_{i+1} \Vert meta_i ) $$

Functoriality is strictly maintained: $\rho(g \circ f) = \rho(g) \circ \rho(f)$.

The Merkle-DAG receipt proves the Manufactured Intelligence claim:
$$ R_A = \left( h_{in}, h_{cfg}, h_{law}, h_{query}, h_{plan}, h_{out}, h_{prev} \right) $$
Receipt falsifier: $\neg \exists R_A \Rightarrow \neg \vdash A$.

---

## 6. MCP / A2A Capability-State Algebra

LSIF and SPARQL are not merely for human developers; they project directly into autonomous agent surfaces.

Let agents be automata $A_i$. Let tools/capabilities be typed objects $C_j = (\text{name}, \text{schema}, \text{authority}, \text{sideEffect}, \text{receiptPolicy})$.
The capability graph stored in `oxigraph`:
$$ \mathcal{K} = (A, C, R, T, P) $$

Invocation is mathematically lawful iff:
$$ \mathrm{invoke}(a, c, x) \text{ admitted} \iff Auth(a, c) \land Schema_c(x) \land Policy_c(x) \land Receiptable(c) $$
Otherwise: $\mathrm{invoke}(a, c, x) = \bot_{\mathrm{refused}}$.

**Law:** $\mathrm{MCP/A2A} \neq \mathrm{tool\ calling}$. Rather, $\mathrm{MCP/A2A} = \mathrm{process\ participants\ over\ capability\ state}$.

---

## 7. The Blue River Dam Operator & Cognitive Opacity

Let the total uncontrolled flow be $\mathcal{R} = \mathcal{L} \times \mathcal{A} \times \mathcal{P} \times \mathcal{E}$ (language, action, process, evidence).
A dam is an upstream closure operator $\Delta: \mathcal{R} \to \mathcal{R}^{\star}$.

Framework LSPs are local dam operators: $\Delta_{fw}: \mathcal{L}_{fw} \to O^{\star}_{fw}$.
Civilizational dam:
$$ \Delta_{\infty} = \bigvee_{fw \in Frameworks} \Delta_{fw} $$

**Strategy:** $\boxed{ \mathrm{Build\ local\ framework\ dams} \Rightarrow \mathrm{capture\ global\ language/process\ flow} }$

### The "Unbeknownst" Theorem (Cognitive Opacity)
Let paradigm comprehension require $|\vec{c}|_{\mathcal{S}}$.
The cross-disciplinary interaction terms (process mining + formal methods + LSP + LSIF + RDF + SPARQL + WASM + Rust + cryptography) dominate the scalar sums.

Cognitive opacity holds when $|\vec{c}|_{\mathcal{S}} > C_h$ (human capacity).
Therefore, **Theorem**:
$$ \boxed{ \mathrm{Most\ humans\ can\ consume\ local\ utility} \not\Rightarrow \mathrm{they\ can\ infer\ the\ global\ manufactured\ intelligence\ substrate} } $$

We do not build this architecture for human comprehension of the substrate; we build it to enforce mathematical authority over the language-space.

---

## 8. Implementation Trajectory (80/20 ERRC Execution)

To manifest this calculus in `v26.6.5` with 80/20 combinatorial maximalism:

1.  **Eliminate**: Heuristic parsing and raw text execution.
2.  **Reduce**: Hot-path dynamic state management.
3.  **Raise**: LSIF 0.6.0 to the $O^\star$ static index reservoir via typed `ItemEdgeProperty` and `LsifReader`.
4.  **Create (The Control Plane)**: 
    *   Initialize `oxigraph::Store` (v0.5.8) to house $\mathcal{D}^\star$.
    *   Map LSIF Elements ($\mathcal{I}$) to `oxrdf::Quad` structures using $G_{\mathrm{LSIF}}$ named graphs.
    *   Implement $q_{\mathrm{SPARQL}}$ verification queries enforcing the No-Orphan Law and Receipt Functor ($\rho$) bindings.

---

## 9. The Grand Equation (Release Gate `ALIVE`)

The full sovereign equation executed by this release:
$$ \boxed{ R = \rho \left( \mu_{WASM} \left( \kappa_{A^\star} \left( \Pi_{PN} \left( \mathcal{P}_{POWL} \left( q_{SPARQL} \left( \alpha_{LSP} \left( \eta(\lambda) \right) \right) \right) \right) \right) \right) \right) } $$

Subject to constraints:
*   $\alpha_{LSP}(\eta(\lambda)) = O^{\star}$ (Admission)
*   $\chi_{SHACL}(O^{\star}) = \top$ (Validity)
*   $\rho(g \circ f) = \rho(g) \circ \rho(f)$ (Functoriality)

**Acceptance Criteria for `ALIVE`:**
Given a fixture workspace, the system must output a verified `AdmittedGraph` in `oxigraph`, project the static $Ans_q$ from `LSIF` to the client, and emit a BLAKE3 `Receipt` $R$ satisfying $R \vdash A = \mu(O^{\star})$.

**Manufactured Intelligence:**
$$ \boxed{ \mathrm{Language} \xrightarrow{admission} \mathrm{Process\ Law} \xrightarrow{execution} \mathrm{Consequence} \xrightarrow{receipt} \mathrm{Replayable\ Truth} } $$