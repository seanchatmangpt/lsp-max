import { readWitnessOutputs, type WitnessOutput } from "@/lib/project";

export const dynamic = "force-dynamic";

function WitnessCard({ w }: { w: WitnessOutput }) {
  return (
    <article className="card">
      <div className="card-head">
        <h3 className="mono">{w.example}</h3>
        <span className="dim">{w.iteration}</span>
      </div>
      {w.exitCode !== null && (
        <p className="small dim">exit {w.exitCode}</p>
      )}
      <pre className="mono small">{w.output.join("\n")}</pre>
      <p className="src">↳ DOC_COVERAGE_LOG.md</p>
    </article>
  );
}

export default async function WitnessesPage() {
  // Real data path: DOC_COVERAGE_LOG.md at the repo root, parsed for
  // captured run blocks. Deleting the file makes this page throw.
  const witnesses = await readWitnessOutputs();
  return (
    <section>
      <h1>Example witnesses</h1>
      <p className="lede">
        {witnesses.length} captured run-to-exit example outputs parsed live from{" "}
        <code>DOC_COVERAGE_LOG.md</code>. Each block was recorded during a real{" "}
        <code>cargo run --example</code> invocation — no output is invented.
      </p>
      <div className="grid">
        {witnesses.map((w, i) => (
          <WitnessCard key={`${w.example}-${i}`} w={w} />
        ))}
      </div>
      <p className="src">↳ DOC_COVERAGE_LOG.md</p>
    </section>
  );
}
