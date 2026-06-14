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

/** A verb (action) on a CLI noun, parsed from the real `#[verb("…")]` attrs. */
export interface CliVerb {
  verb: string;
  fn: string;
  args: string[];
  doc?: string;
}
export interface CliNoun {
  noun: string;
  sourceFile: string;
  verbs: CliVerb[];
}

const NOUNS_DIR = "crates/lsp-max-cli/src/nouns";

/** Parse the real clap-noun-verb CLI surface from the noun source files. The
 *  command surface is derived from `#[verb("…")]` attributes over `pub fn`s —
 *  change the Rust source and this changes. Throws if the noun dir is absent. */
export async function readCliSurface(): Promise<CliNoun[]> {
  const absDir = path.join(REPO_ROOT, NOUNS_DIR);
  const entries = await fs.readdir(absDir); // throws if the CLI is gone
  const nouns: CliNoun[] = [];
  for (const name of entries.sort()) {
    if (!name.endsWith(".rs") || name === "mod.rs") continue;
    const src = await fs.readFile(path.join(absDir, name), "utf8");
    const verbs: CliVerb[] = [];
    // Match: optional /// doc lines, then #[verb("x")], then pub fn name(args).
    const re =
      /#\[verb\("([^"]+)"\)\]\s*pub fn (\w+)\s*\(([^)]*)\)/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(src)) !== null) {
      const [full, verb, fn, rawArgs] = m;
      const args = rawArgs
        .split(",")
        .map((a) => a.trim())
        .filter(Boolean)
        .map((a) => a.split(":")[0].trim());
      // Grab the nearest /// doc line just above the #[verb].
      const before = src.slice(0, m.index);
      const docMatch = before.match(/\/\/\/ ([^\n]*)\n\s*$/);
      verbs.push({ verb, fn, args, doc: docMatch?.[1]?.trim() });
      void full;
    }
    if (verbs.length > 0) {
      nouns.push({ noun: name.replace(/\.rs$/, ""), sourceFile: path.join(NOUNS_DIR, name), verbs });
    }
  }
  if (nouns.length === 0) {
    throw new Error(`No CLI verbs parsed from ${absDir} — the clap-noun-verb surface is missing.`);
  }
  return nouns;
}

/** A coverage row parsed from the real DOC_COVERAGE_LOG.md status tables. */
export interface CoverageRow {
  item: string;
  status: "covered" | "gap" | "server-class" | "other";
  raw: string;
}
export interface CoverageReport {
  iterations: string[];
  rows: CoverageRow[];
  covered: number;
  gaps: number;
}

/** Parse the real doc↔example coverage log produced by the doc-coverage loop.
 *  Reads DOC_COVERAGE_LOG.md at the repo root; throws if absent. */
export async function readCoverage(): Promise<CoverageReport> {
  const md = await fs.readFile(path.join(REPO_ROOT, "DOC_COVERAGE_LOG.md"), "utf8");
  const iterations = [...md.matchAll(/^##\s+(Iteration[^\n]*)/gm)].map((m) => m[1].trim());
  const rows: CoverageRow[] = [];
  const seen = new Set<string>();
  for (const line of md.split("\n")) {
    if (!line.startsWith("|")) continue;
    if (!/✅|❌|⊘|⚠/.test(line)) continue;
    const cells = line.split("|").map((c) => c.trim()).filter(Boolean);
    if (cells.length < 2) continue;
    const item = cells[0].replace(/`/g, "");
    if (/^Example$|^Symbol$|^Capability$/.test(item) || seen.has(item)) continue;
    seen.add(item);
    const status: CoverageRow["status"] = line.includes("✅")
      ? "covered"
      : line.includes("⊘")
        ? "server-class"
        : "gap";
    rows.push({ item, status, raw: line });
  }
  return {
    iterations,
    rows,
    covered: rows.filter((r) => r.status === "covered").length,
    gaps: rows.filter((r) => r.status === "gap").length,
  };
}

/** Object-centric event log (OCEL 2.0) as actually emitted by the project. */
export interface OcelRelationship {
  objectId: string;
  qualifier: string;
}
export interface OcelEvent {
  id: string;
  type: string;
  time: string;
  relationships: OcelRelationship[];
}
export interface OcelObject {
  id: string;
  type: string;
}
export interface Ocel {
  sourceFile: string;
  eventTypes: string[];
  objectTypes: string[];
  events: OcelEvent[];
  objects: OcelObject[];
}

const OCEL_CANDIDATES = [
  "crates/playground/ocel/admitted_evidence.ocel.json",
];

/** Read the real OCEL artifact (object-centric event log). Throws if no OCEL
 *  file is present — the process graph must come from a real log. */
export async function readOcel(): Promise<Ocel> {
  let lastErr: unknown;
  for (const rel of OCEL_CANDIDATES) {
    try {
      const raw = (await readJsonFile(path.join(REPO_ROOT, rel))) as Record<string, unknown>;
      return {
        sourceFile: rel,
        eventTypes: ((raw.eventTypes as { name: string }[]) ?? []).map((t) => t.name),
        objectTypes: ((raw.objectTypes as { name: string }[]) ?? []).map((t) => t.name),
        events: (raw.events as OcelEvent[]) ?? [],
        objects: (raw.objects as OcelObject[]) ?? [],
      };
    } catch (e) {
      lastErr = e;
    }
  }
  throw new Error(`No OCEL artifact found (${OCEL_CANDIDATES.join(", ")}): ${String(lastErr)}`);
}

/** Workspace version, read from the real Cargo.toml (CalVer YY.M.D). */
export async function readWorkspaceVersion(): Promise<string> {
  const cargo = await fs.readFile(path.join(REPO_ROOT, "Cargo.toml"), "utf8");
  const m = cargo.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("workspace version not found in Cargo.toml");
  return m[1];
}
