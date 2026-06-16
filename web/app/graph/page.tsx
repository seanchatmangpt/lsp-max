import { readAdmissionGraph, type AdmissionPipelineState, type ReceiptGraphRow } from "@/lib/project";

export const dynamic = "force-dynamic"; // always read real files at request time

function AxisBadge({ state }: { state: string }) {
  const cls =
    state === "admitted"
      ? "badge badge-admitted"
      : state === "refused"
        ? "badge badge-refused"
        : "badge badge-unknown";
  return <span className={cls}>{state.toUpperCase()}</span>;
}

function VerdictBadge({ verdict }: { verdict: string }) {
  const cls =
    verdict === "ADMITTED" ? "badge badge-admitted" : "badge badge-refused";
  return <span className={cls}>{verdict}</span>;
}

function PipelineFlow({ states }: { states: AdmissionPipelineState[] }) {
  return (
    <div className="card" style={{ marginBottom: "24px" }}>
      <h2 className="sub" style={{ marginTop: 0 }}>
        Pipeline states (from admission_pipeline WITNESS)
      </h2>
      <p className="lede" style={{ marginBottom: "16px" }}>
        The three states witnessed in{" "}
        <code>examples/admission_pipeline.rs</code>. Each shows how the receipt
        state maps to a ConformanceVector axis value, and what the gate verdict
        is under strict mode.
      </p>
      <table className="tbl">
        <thead>
          <tr>
            <th>State</th>
            <th>Receipt condition</th>
            <th>Receipt axis</th>
            <th>Gate verdict</th>
          </tr>
        </thead>
        <tbody>
          {states.map((s) => (
            <tr key={s.label}>
              <td className="mono">{s.label}</td>
              <td>{s.receiptState}</td>
              <td className="mono">{s.axisState}</td>
              <td>
                <VerdictBadge verdict={s.gateVerdict.replace("Gate: ", "")} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="src">
        Flow (text representation):
      </p>
      <pre
        className="mono"
        style={{
          background: "var(--panel)",
          border: "1px solid var(--line)",
          borderRadius: "8px",
          padding: "12px 16px",
          fontSize: "13px",
          overflowX: "auto",
          margin: "8px 0 0",
        }}
      >
        {states
          .map(
            (s) =>
              `${s.label} Receipt (${s.receiptState}) -> ${s.axisState} -> ${s.gateVerdict}`,
          )
          .join("\n")}
      </pre>
    </div>
  );
}

function CrossProductTable({ rows }: { rows: ReceiptGraphRow[] }) {
  return (
    <div className="card">
      <h2 className="sub" style={{ marginTop: 0 }}>
        Receipt cross-product table
      </h2>
      <p className="lede" style={{ marginBottom: "16px" }}>
        Each real <code>*.receipt.json</code> mapped to its ConformanceVector
        axis state and the gate verdict that would follow under strict mode.
        Axis state is derived from the receipt&apos;s <code>status</code> field:
        ADMITTED maps to admitted, absent status maps to unknown, anything else
        maps to refused.
      </p>
      <table className="tbl">
        <thead>
          <tr>
            <th>Receipt file</th>
            <th>Status</th>
            <th>Receipt axis</th>
            <th>Gate verdict</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.sourceFile}>
              <td className="mono small">{r.sourceFile}</td>
              <td className="mono small">{r.status ?? "(none)"}</td>
              <td>
                <AxisBadge state={r.axisState} />
              </td>
              <td>
                <VerdictBadge verdict={r.gateVerdict} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default async function GraphPage() {
  // Real data path: DOC_COVERAGE_LOG.md (pipeline states) + *.receipt.json (receipts).
  // Deleting either artifact makes this page throw — the witness against fabrication.
  const graph = await readAdmissionGraph();

  return (
    <section>
      <h1>Receipt-chain cross-product graph</h1>
      <p className="lede">
        Cross-product of real receipts and ConformanceVector law axes. The
        pipeline states are parsed from the{" "}
        <code>admission_pipeline</code> WITNESS block in{" "}
        <code>DOC_COVERAGE_LOG.md</code>. Each receipt row is derived from a
        real <code>*.receipt.json</code> artifact on disk.
      </p>

      <div className="stats">
        <div className="stat">
          <span className="stat-n">{graph.totalAdmitted}</span>
          <span className="stat-l">receipts admitted</span>
        </div>
        <div className="stat">
          <span className="stat-n" style={{ color: "var(--refused)" }}>
            {graph.totalRefused}
          </span>
          <span className="stat-l">receipts refused</span>
        </div>
        <div className="stat">
          <span className="stat-n" style={{ color: "var(--unknown)" }}>
            {graph.totalUnknown}
          </span>
          <span className="stat-l">receipts unknown</span>
        </div>
        <div className="stat">
          <span className="stat-n">{graph.receiptSummary.length}</span>
          <span className="stat-l">total receipts</span>
        </div>
      </div>

      <PipelineFlow states={graph.pipelineStates} />
      <CrossProductTable rows={graph.receiptSummary} />

      <p className="src" style={{ marginTop: "24px" }}>
        {"↳ DOC_COVERAGE_LOG.md · receipts/*.receipt.json · crates/playground/receipts/*.receipt.json"}
      </p>
    </section>
  );
}
