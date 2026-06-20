"use server";

import { spawn } from "node:child_process";
import { REPO_ROOT } from "@/lib/project";

// Only the real run-to-exit witness examples may be executed. These are the
// actual examples in the repo (examples/*.rs); the action runs the real binary.
const ALLOWED = new Set([
  "conformance_vector_explained",
  "receipt_chain_explained",
  "calver_law_explained",
  "admission_pipeline",
]);

export interface RunResult {
  example: string;
  exitCode: number | null;
  ok: boolean;
  output: string;
  ranAt: string;
}

/** Run `cargo run --example <name>` in the real repo and return its real output
 *  and exit code. This is the project actually executing — not a fixture. A
 *  refusing/failing example returns its real non-zero exit and stderr. */
export async function runExample(
  _prev: RunResult | null,
  formData: FormData,
): Promise<RunResult> {
  const example = String(formData.get("example") ?? "");
  if (!ALLOWED.has(example)) {
    return {
      example,
      exitCode: null,
      ok: false,
      output: `Refused: ${example} is not an allowed example. The UI only runs real witness examples.`,
      ranAt: new Date().toISOString(),
    };
  }
  return await new Promise<RunResult>((resolve) => {
    const child = spawn("cargo", ["run", "--quiet", "--example", example], {
      cwd: REPO_ROOT,
      env: { ...process.env, CARGO_TERM_COLOR: "never" },
    });
    let out = "";
    child.stdout.on("data", (d) => (out += d.toString()));
    child.stderr.on("data", (d) => (out += d.toString()));
    const timer = setTimeout(() => child.kill("SIGKILL"), 120_000);
    child.on("close", (code) => {
      clearTimeout(timer);
      // Keep only the witness section; drop sibling-crate compiler warnings noise.
      const lines = out.split("\n");
      const start = lines.findIndex((l) => l.startsWith("WITNESS"));
      const shown = start >= 0 ? lines.slice(start).join("\n") : out.trim();
      resolve({
        example,
        exitCode: code,
        ok: code === 0,
        output: shown || "(no output)",
        ranAt: new Date().toISOString(),
      });
    });
    child.on("error", (err) => {
      clearTimeout(timer);
      resolve({
        example,
        exitCode: null,
        ok: false,
        output: `spawn error: ${err.message}`,
        ranAt: new Date().toISOString(),
      });
    });
  });
}
