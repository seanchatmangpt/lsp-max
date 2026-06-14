import { readCoverage } from "@/lib/project";

export const dynamic = "force-dynamic";

const LABEL: Record<string, string> = {
  covered: "✅ covered",
  "server-class": "⊘ server-class",
  gap: "❌ gap",
  other: "·",
};

export default async function CoveragePage() {
  // Real artifact: DOC_COVERAGE_LOG.md, produced by the doc↔example coverage loop.
  const cov = await readCoverage();
  return (
    <section>
      <h1>Doc ↔ example coverage</h1>
      <p className="lede">
        Parsed live from <code>DOC_COVERAGE_LOG.md</code> — the project&apos;s own
        coverage ledger. {cov.covered} covered, {cov.gaps} open gaps across{" "}
        {cov.iterations.length} recorded iterations. A capability is covered only
        when a running example asserts its contract.
      </p>

      <h2 className="sub">Iterations</h2>
      <ol className="iters">
        {cov.iterations.map((it) => (
          <li key={it}>{it}</li>
        ))}
      </ol>

      <h2 className="sub">Status rows</h2>
      <table className="tbl">
        <thead>
          <tr>
            <th>Item</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          {cov.rows.map((r) => (
            <tr key={r.item}>
              <td className="mono">{r.item}</td>
              <td className={`st st-${r.status}`}>{LABEL[r.status]}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="src">↳ DOC_COVERAGE_LOG.md</p>
    </section>
  );
}
