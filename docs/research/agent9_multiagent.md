### Research Analysis: Multi-Agent Systems & A2A/MCP/Actor Models

#### 1. Primary Bibliography
*   **Chatman, S. (2026).** *Type-Safe Process-Evidence Engineering: A Mathematical Framework for Object-Centric Conformance Checking.* PhD Dissertation.
    *   *Relevance:* Establishes the "Evidence Algebra" $(\mathcal{E})$ and the Chatman Equation ($A = \mu(O)$), providing the formal "Law-State" foundations for the AGI pipeline.
*   **Unknown. (2025).** *Agent TCP:IP - An Agent-to-Agent Transaction System.* (In `agent10_master_bibliography.bib`).
    *   *Relevance:* Foundational for the A2A transaction logic implemented in the custom `max/*` RPC endpoints.
*   **Unknown. (2024).** *AI Multi-Agent Interoperability Extension for Managing Multiparty Conversations.* (In `agent10_master_bibliography.bib`).
    *   *Relevance:* Informs the multi-upstream composition strategy where multiple LSPs (agents) collaborate under the `lsp-max` router.
*   **Hechehouche, H., et al. (2024).** *LLM-Assisted Modeling of Semantic Web-Enabled Multi-Agents Systems with AJAN.*
    *   *Relevance:* Directly relates to the Oxigraph/RDF control plane for AGI reasoning.
*   **Model Context Protocol (MCP) Spec (2025-06-18).**
    *   *Relevance:* Defines the standard interface for tool discovery and resource binding used in the workspace's projection flows.
*   **Agent2Agent (A2A) Protocol Spec (2025-04).**
    *   *Relevance:* Establishes the task delegation and JSON-RPC 2.0 card-based identity system for autonomous coding agents.

#### 2. Relevance to AGI Pipeline and Actor-based LSP
*   **Multi-Agent Coordination:** The system transitions from human-centric IDE assistance to a "post-human project-state enforcement server." The A2A and MCP protocols serve as the "Agent Surface," allowing autonomous agents to discover and invoke tools (Capabilities) and materializing code intelligence as verifiable graph state.
*   **Actor Model & Tower:** The `lsp-max` architecture utilizes the Actor-like `Service` and `Layer` traits from the Tower ecosystem. This provides the modularity required to compose independent semantic intelligence "dams" (LSPs) into a single unified authority.
*   **AGI Pipeline (The $\kappa$ and $\rho$ Layers):** The pipeline uses SPARQL/SHACL (the $\kappa$ layer) to enforce process laws on agent-generated edits. Every action must emit a BLAKE3 cryptographic receipt ($\rho$ layer), satisfying $R \vdash A = \mu(O^\star)$, ensuring that no agent claim is admitted without mathematical proof.
