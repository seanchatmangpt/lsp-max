// The real-data boundary. Every value the UI renders flows through here, and
// every function reaches an actual lsp-max artifact on disk or an actual binary.
// If the project were deleted, these throw — that is the witness against
// fabrication. No fixtures, no fallbacks that invent data.

import { promises as fs } from "node:fs";
import path from "node:path";

/** Repo root = parent of web/. Server code runs with cwd = web/. */
export const REPO_ROOT = path.resolve(process.cwd(), "..");

/** A receipt as actually emitted by lsp-max (see receipts/*.receipt.json and
 *  crates/playground/receipts/GALL-CHECKPOINT-*.receipt.json). Fields are
 *  optional because the two real receipt shapes differ; the loader keeps the
 *  raw object so nothing is invented. */
export interface Receipt {
  sourceFile: string; // real path the data came from, relative to repo root
  checkpoint?: string;
  boundary?: string;
  digest?: string;
  digest_algorithm?: string;
  output_digest?: string;
  output_digest_algorithm?: string;
  output_hash?: string;
  run_id?: string;
  replay_pointer?: string;
  raw_command?: string;
  producing_workspace?: string;
  run_date?: string;
  timestamp?: string;
  status?: string;
  claims?: Record<string, string>;
  exported_artifact_digest?: string;
  export_reason?: string;
}

const RECEIPT_DIRS = ["receipts", "crates/playground/receipts"];

async function readJsonFile(abs: string): Promise<unknown> {
  const text = await fs.readFile(abs, "utf8");
  return JSON.parse(text);
}

/** Read every real *.receipt.json under the known receipt directories. Throws
 *  if no receipt directory exists — the project must be present. */
export async function readReceipts(): Promise<Receipt[]> {
  const found: Receipt[] = [];
  let anyDir = false;
  for (const dir of RECEIPT_DIRS) {
    const absDir = path.join(REPO_ROOT, dir);
    let entries: string[];
    try {
      entries = await fs.readdir(absDir);
    } catch {
      continue; // dir may not exist in every checkout
    }
    anyDir = true;
    for (const name of entries) {
      if (!name.endsWith(".receipt.json")) continue;
      const abs = path.join(absDir, name);
      const obj = (await readJsonFile(abs)) as Record<string, unknown>;
      found.push({ ...(obj as object), sourceFile: path.join(dir, name) } as Receipt);
    }
  }
  if (!anyDir) {
    throw new Error(
      `No receipt directory found under ${REPO_ROOT}. The lsp-max project artifacts are missing — the UI represents real receipts only.`,
    );
  }
  // Deterministic order: ADMITTED first, then by run_id.
  return found.sort((a, b) => (a.run_id ?? "").localeCompare(b.run_id ?? ""));
}

/** Workspace version, read from the real Cargo.toml (CalVer YY.M.D). */
export async function readWorkspaceVersion(): Promise<string> {
  const cargo = await fs.readFile(path.join(REPO_ROOT, "Cargo.toml"), "utf8");
  const m = cargo.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("workspace version not found in Cargo.toml");
  return m[1];
}
