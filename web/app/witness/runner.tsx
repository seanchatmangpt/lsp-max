"use client";

import { useActionState } from "react";
import { runExample, type RunResult } from "./actions";

export function WitnessRunner({ examples }: { examples: string[] }) {
  const [result, formAction, pending] = useActionState<RunResult | null, FormData>(
    runExample,
    null,
  );
  return (
    <div>
      <div className="runbar">
        {examples.map((ex) => (
          <form key={ex} action={formAction}>
            <input type="hidden" name="example" value={ex} />
            <button type="submit" disabled={pending} className="runbtn">
              ▶ {ex}
            </button>
          </form>
        ))}
      </div>

      {pending && (
        <p className="running">
          Running the real binary — <code>cargo run --example …</code> — waiting on
          actual work.
        </p>
      )}

      {result && !pending && (
        <div className={`runout ${result.ok ? "ok" : "fail"}`}>
          <div className="runout-head">
            <span className="mono">{result.example}</span>
            <span className={`badge ${result.ok ? "badge-admitted" : "badge-refused"}`}>
              exit {result.exitCode ?? "—"}
            </span>
          </div>
          <pre>{result.output}</pre>
          <p className="src">ran at {result.ranAt} · cargo run --example {result.example}</p>
        </div>
      )}
    </div>
  );
}
