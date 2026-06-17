"use server";

import { spawn } from "node:child_process";
import { REPO_ROOT } from "@/lib/project";

export interface ConformanceVerdict {
  instanceId: string;
  ok: boolean;
  admitted: string[];
  refused: string[];
  unknown: string[];
  error?: string;
  ranAt: string;
}

// Instance ids are constrained to a safe charset — they are passed to the real
// CLI as an argument, not interpolated into a shell.
const VALID_ID = /^[A-Za-z0-9_.:-]{1,64}$/;

/** Run the real `lsp-max-cli conformance vector --instance-id <id>` and parse its
 *  actual ConformanceVector JSON. This is the project's real conformance engine
 *  executing — not a fabricated verdict. */
export async function runConformance(
  _prev: ConformanceVerdict | null,
  formData: FormData,
): Promise<ConformanceVerdict> {
  const instanceId = String(formData.get("instanceId") ?? "").trim();
  const ranAt = new Date().toISOString();
  if (!VALID_ID.test(instanceId)) {
    return { instanceId, ok: false, admitted: [], refused: [], unknown: [], error: "invalid instance id", ranAt };
  }
  return await new Promise<ConformanceVerdict>((resolve) => {
    const child = spawn(
      "cargo",
      ["run", "--quiet", "-p", "lsp-max-cli", "--bin", "lsp-max-cli", "--", "conformance", "vector", "--instance-id", instanceId],
      { cwd: REPO_ROOT, env: { ...process.env, CARGO_TERM_COLOR: "never" } },
    );
    let out = "";
    let err = "";
    child.stdout.on("data", (d) => (out += d.toString()));
    child.stderr.on("data", (d) => (err += d.toString()));
    const timer = setTimeout(() => child.kill("SIGKILL"), 120_000);
    child.on("close", (code) => {
      clearTimeout(timer);
      if (code !== 0) {
        resolve({ instanceId, ok: false, admitted: [], refused: [], unknown: [], error: err.trim().slice(-400) || `exit ${code}`, ranAt });
        return;
      }
      try {
        const json = JSON.parse(out);
        resolve({
          instanceId,
          ok: true,
          admitted: json.admitted ?? [],
          refused: json.refused ?? [],
          unknown: json.unknown ?? [],
          ranAt,
        });
      } catch (e) {
        resolve({ instanceId, ok: false, admitted: [], refused: [], unknown: [], error: `unparseable CLI output: ${String(e)}`, ranAt });
      }
    });
    child.on("error", (e) =>
      resolve({ instanceId, ok: false, admitted: [], refused: [], unknown: [], error: e.message, ranAt }),
    );
  });
}
