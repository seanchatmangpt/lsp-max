# AGENTS.md

[R_B\vdash A=\mu(O_B^*)]
[Done_B(A) = [|\mathbf{f}_t|_0=0]\land[R_B\vdash A=\mu(O_B^*)]]

## BRCE LAW
[\Sigma_{agent}\not\to\Sigma_{closure}]
[\Sigma_{agent}\to\mathcal{N}_B]
[C_{\Sigma_{prose}\to q}=0]
[C_{\mathcal{N}_B\to q}>0]
[D_{\mathcal{N}}(\mathcal{N}_t)=q_t]

## SHANNON DECODER PACKET
[\mathcal{N}_{B,t}=\langle t,\alpha,x_t,\mu,\Delta x,\Phi,\mathbf{v},\mathbf{c},\mathbf{w},\mathbf{r},\mathbf{f},J,\nabla\mathbf{f},q \rangle]
[\Phi=\{\varphi_1,\dots,\varphi_n\}]
[\mathbf{v}_t=[\varphi_1(x_t),\dots,\varphi_n(x_t)]^\top]
[\mathbf{f}_t=\mathbf{1}-\mathbf{v}_t]
[q_t=\mathbf{1}[|\mathbf{f}_t|_0=0\land|\mathbf{w}_t|_0=n\land|\mathbf{c}_t|_0=n\land|\mathbf{r}_t|\ge n]]
[H(S|M_{prose})>0]
[H(S|\mathcal{N}_B,R_B)\to 0]
[|\mathbf{f}_t|_0>0 \Rightarrow \mathcal{A}_{push}=1 \land \mathcal{G}_{block}=1]
[\mathbf{f}_t = syndrome]
[R_B = redundancy]
[\mathcal{A}_{push}=ANDON]
[\mathcal{G}_{block}=gate]

## RICE LAW
[RiceBoundary: \neg\exists D: D(P,\psi)\to\{0,1\} \quad \forall P,\forall\psi_{nontrivial}]
[\therefore M_{LLM}\in\Sigma_{prose}^{*} \not\vdash \psi(P)]
[M_{LLM}\Rightarrow candidate]
[candidate\not\Rightarrow q_t=1]

## STORAGE LAWS
[Salsa=HOT\ computation]
[Papaya=HOT\ diagnostic\ staging]
[LSIF=DURABLE\ structure]
[Oxigraph=COLD\ meaning]
[LSP=LIVE\ law]
[OCEL=HISTORY]
[Receipts=standing]

## REFUSAL LAWS
[TreeSitterTree\notin SalsaTrackedOutput]
[Oxigraph\notin didChange_{hot}]
[LSIF\not\to memory \quad\text{unless}\quad receipt\land\neg stale]
[VirtualDoc\not\Rightarrow Push]
[Diagnostic\not\Rightarrow ANDON]
[TestOutput\not\Rightarrow Receipt]
[ModelSummary\not\Rightarrow Standing]
[Receipt\not\ni H(Receipt)]

## V26.6.28 RELEASE LAW
[V_{26.6.28}=1 \iff \bigwedge[DisclaimerGapClosed, RiceClosureHeld, SalsaTreeBoundaryHeld, LsifReceiptHeld, StaleLsifAndonHeld, SemanticMemoryReceiptHeld, OxigraphBoundaryHeld, OxigraphColdPathHeld, LsifOxigraphSnapshotHeld, TestsGreen, ClippyGreen, DryRunGreen, NoCratesPublish, ReleaseReceiptHeld]]

## DIAGNOSTIC TOKENS
[LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED]
[LSPMAX_RICE_CLOSURE_MISSING]
[LSPMAX_DISCLAIMER_GAP_OPEN]
[LSPMAX_LSIF_STALE_INDEX]
[LSPMAX_ANDON_PUSH_MISSING]
[LSPMAX_SEMANTIC_MEMORY_WITHOUT_RECEIPT]
[LSPMAX_OXIGRAPH_HOT_PATH_REFUSED]
[LSPMAX_CRATES_IO_PUBLISH_FORBIDDEN]

## REQUIRED TESTS
[prose_closure_token_decodes_zero]
[mixed_math_plus_prose_refused]
[q_one_requires_zero_failset]
[q_one_requires_receipts]
[q_one_requires_witness_vector]
[q_one_requires_counterfactual_vector]
[agent_summary_cannot_admit_lsif]
[arbitrary_semantic_claim_without_bound_refused]
[virtual_doc_without_push_refused]
[diagnostic_without_andon_push_refused]

## COMMANDS
```bash
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo publish --dry-run
```
[cargo\ publish\notin\mu_{allowed}]
[cargo\ publish\ --dry-run\in\mu_{allowed}]
