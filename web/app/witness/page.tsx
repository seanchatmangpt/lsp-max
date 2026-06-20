import { promises as fs } from "node:fs";
import path from "node:path";
import { REPO_ROOT } from "@/lib/project";
import { WitnessRunner } from "./runner";

export const dynamic = "force-dynamic";

const WITNESS_EXAMPLES = [
  "conformance_vector_explained",
  "receipt_chain_explained",
  "calver_law_explained",
  "admission_pipeline",
];

export default async function WitnessPage() {
  // Confirm the real example sources exist before offering to run them — the list
  // is gated on actual files in examples/, not a hardcoded menu dressed as real.
  const present: string[] = [];
  for (const ex of WITNESS_EXAMPLES) {
    try {
      await fs.access(path.join(REPO_ROOT, "examples", `${ex}.rs`));
      present.push(ex);
    } catch {
      // missing source ⇒ not offered
    }
  }
  if (present.length === 0) {
    throw new Error("No witness example sources found under examples/ — nothing real to run.");
  }
  return (
    <section>
      <h1>Live witnesses</h1>
      <p className="lede">
        Press a button to run the real example binary via a Next.js server action
        (<code>cargo run --example …</code> in the repo) and see its actual output
        and exit code. These are the same run-to-exit witnesses recorded in the
        coverage ledger — they assert their contract and exit non-zero if the
        capability regresses. {present.length} example sources present in{" "}
        <code>examples/</code>.
      </p>
      <WitnessRunner examples={present} />
    </section>
  );
}
