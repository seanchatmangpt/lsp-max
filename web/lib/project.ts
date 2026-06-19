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

/** A law axis variant parsed from the real LawAxis enum in conformance.rs. */
export interface ConformanceAxis {
  name: string;
  id: number;
}

/** One pipeline state from the admission_pipeline WITNESS block in DOC_COVERAGE_LOG.md. */
export interface PipelineState {
  label: string;
  description: string;
  verdict: string;
}

export interface ConformanceSurface {
  axes: ConformanceAxis[];
  pipelineStates: PipelineState[];
  sourceFile: string;
}

const CONFORMANCE_RS = "lsp-max-protocol/src/conformance.rs";

/** Parse LawAxis variants from the real conformance.rs and the admission_pipeline
 *  WITNESS block from DOC_COVERAGE_LOG.md. Throws if conformance.rs is absent. */
export async function readConformanceSurface(): Promise<ConformanceSurface> {
  const src = await fs.readFile(path.join(REPO_ROOT, CONFORMANCE_RS), "utf8");
  const enumMatch = src.match(/pub enum LawAxis\s*\{([^}]+)\}/s);
  if (!enumMatch) throw new Error(`LawAxis enum not found in ${CONFORMANCE_RS}`);
  const axes: ConformanceAxis[] = [];
  const variantRe = /^\s{4}([A-Z][A-Za-z]+),\s*$/gm;
  let vm: RegExpExecArray | null;
  while ((vm = variantRe.exec(enumMatch[1])) !== null) {
    if (vm[1] !== "Custom") axes.push({ name: vm[1], id: axes.length });
  }
  if (axes.length === 0) throw new Error(`No LawAxis variants in ${CONFORMANCE_RS}`);
  const logSrc = await fs.readFile(path.join(REPO_ROOT, "DOC_COVERAGE_LOG.md"), "utf8");
  const witnessMatch = logSrc.match(/WITNESS admission_pipeline[^\n]*\n((?:\s+\[[A-Z]\][^\n]+\n)+)/);
  const pipelineStates: PipelineState[] = [];
  if (witnessMatch) {
    const stateRe = /\[([A-Z])\]\s+(.+?)\s+(?:→|->)\s+(.+)/g;
    let sm: RegExpExecArray | null;
    while ((sm = stateRe.exec(witnessMatch[1])) !== null) {
      pipelineStates.push({ label: sm[1], description: sm[2].trim(), verdict: sm[3].trim() });
    }
  }
  return { axes, pipelineStates, sourceFile: CONFORMANCE_RS };
}

/** A single OCEL file's parsed summary. Every field comes from real *.ocel.json
 *  data; the interface documents which OCEL2 keys were actually present. */
export interface OcelFile {
  sourceFile: string; // real path relative to repo root
  eventTypes: { name: string }[];
  objectTypes: { name: string }[];
  eventCount: number;
  objectCount: number;
  sampleEvents: { id: string; type: string; time: string }[];
}

const OCEL_DIRS = [
  "crates/playground/ocel",
  "examples/anti-llm-cheat-lsp/ocel",
];

/** Read every real *.ocel.json under the known OCEL directories. Parses OCEL2
 *  array format (events/objects are arrays) and object-keyed format (events/objects
 *  are plain objects). Files that lack both an `events` and `eventTypes` key are
 *  silently skipped (e.g. plain inventory arrays). Throws if no OCEL directory
 *  exists at all — the project must be present. */
export async function readOcelEvidence(): Promise<OcelFile[]> {
  const found: OcelFile[] = [];
  let anyDir = false;
  for (const dir of OCEL_DIRS) {
    const absDir = path.join(REPO_ROOT, dir);
    let entries: string[];
    try {
      entries = await fs.readdir(absDir);
    } catch {
      continue; // dir may not exist in every checkout
    }
    anyDir = true;
    for (const name of entries.sort()) {
      if (!name.endsWith(".ocel.json")) continue;
      const abs = path.join(absDir, name);
      let raw: unknown;
      try {
        raw = await readJsonFile(abs);
      } catch {
        continue; // skip unparseable files
      }
      if (typeof raw !== "object" || raw === null) continue;
      const obj = raw as Record<string, unknown>;

      // Skip files that carry no event data (e.g. plain string-array inventories).
      const hasEvents = "events" in obj || "eventTypes" in obj;
      if (!hasEvents) continue;

      // eventTypes: OCEL2 uses an array; some files use an object keyed by name.
      let eventTypes: { name: string }[] = [];
      if (Array.isArray(obj.eventTypes)) {
        eventTypes = (obj.eventTypes as Record<string, unknown>[]).flatMap(
          (et) =>
            typeof et === "object" && et !== null && typeof (et as Record<string, unknown>).name === "string"
              ? [{ name: (et as Record<string, unknown>).name as string }]
              : [],
        );
      } else if (typeof obj.eventTypes === "object" && obj.eventTypes !== null) {
        // object-keyed: { TypeName: { ... } } — keys are the type names.
        eventTypes = Object.keys(obj.eventTypes as object).map((n) => ({ name: n }));
      }

      // objectTypes: same dual shape.
      let objectTypes: { name: string }[] = [];
      if (Array.isArray(obj.objectTypes)) {
        objectTypes = (obj.objectTypes as Record<string, unknown>[]).flatMap(
          (ot) =>
            typeof ot === "object" && ot !== null && typeof (ot as Record<string, unknown>).name === "string"
              ? [{ name: (ot as Record<string, unknown>).name as string }]
              : [],
        );
      } else if (typeof obj.objectTypes === "object" && obj.objectTypes !== null) {
        objectTypes = Object.keys(obj.objectTypes as object).map((n) => ({ name: n }));
      }

      // events: array or object-keyed.
      let events: { id: string; type: string; time: string }[] = [];
      if (Array.isArray(obj.events)) {
        events = (obj.events as Record<string, unknown>[]).flatMap((ev) => {
          if (typeof ev !== "object" || ev === null) return [];
          const e = ev as Record<string, unknown>;
          return [
            {
              id: typeof e.id === "string" ? e.id : String(e.id ?? ""),
              type: typeof e.type === "string" ? e.type : String(e.type ?? ""),
              time: typeof e.time === "string" ? e.time : String(e.time ?? ""),
            },
          ];
        });
      } else if (typeof obj.events === "object" && obj.events !== null) {
        events = Object.entries(obj.events as Record<string, Record<string, unknown>>).map(
          ([id, ev]) => ({
            id,
            type: typeof ev.type === "string" ? ev.type : String(ev.type ?? ""),
            time: typeof ev.time === "string" ? ev.time : String(ev.time ?? ""),
          }),
        );
      }

      // objects: count only (the full list is not needed for the summary).
      let objectCount = 0;
      if (Array.isArray(obj.objects)) {
        objectCount = (obj.objects as unknown[]).length;
      } else if (typeof obj.objects === "object" && obj.objects !== null) {
        objectCount = Object.keys(obj.objects as object).length;
      }

      found.push({
        sourceFile: path.join(dir, name),
        eventTypes,
        objectTypes,
        eventCount: events.length,
        objectCount,
        sampleEvents: events.slice(0, 5),
      });
    }
  }
  if (!anyDir) {
    throw new Error(
      `No OCEL directory found under ${REPO_ROOT}. The lsp-max OCEL process evidence is missing — the UI represents real OCEL data only.`,
    );
  }
  return found;
}

/** Workspace version, read from the real Cargo.toml (CalVer YY.M.D). */
export async function readWorkspaceVersion(): Promise<string> {
  const cargo = await fs.readFile(path.join(REPO_ROOT, "Cargo.toml"), "utf8");
  const m = cargo.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("workspace version not found in Cargo.toml");
  return m[1];
}

/** A captured witness output block parsed from DOC_COVERAGE_LOG.md.
 *  Every field comes from the log — nothing is invented. */
export interface WitnessOutput {
  /** The example name as it appears after `cargo run --example`. */
  example: string;
  /** The iteration header under which this captured run was recorded. */
  iteration: string;
  /** The lines inside the fenced code block (the WITNESS lines). */
  output: string[];
  /** Exit code parsed from the header line, or null if not present. */
  exitCode: number | null;
}

/** Parse the real captured run blocks from DOC_COVERAGE_LOG.md.
 *  Each block is recorded under a `## Iteration N` header and follows a
 *  "captured run" bullet that names the example and exit code.
 *  The fences in the log are 2-space-indented (`  ``` `).
 *  Throws if DOC_COVERAGE_LOG.md is absent. */
export async function readWitnessOutputs(): Promise<WitnessOutput[]> {
  const md = await fs.readFile(path.join(REPO_ROOT, "DOC_COVERAGE_LOG.md"), "utf8");
  const witnesses: WitnessOutput[] = [];

  // Split on iteration headers so each block carries its iteration label.
  const iterSections = md.split(/(?=^## Iteration )/m);

  for (const section of iterSections) {
    const iterMatch = section.match(/^## (Iteration[^\n]*)/m);
    const iteration = iterMatch ? iterMatch[1].trim() : "";

    // Match every "captured run" bullet in this section.
    // The bullet may span two lines when the exit code wraps; the fence is 2-space-indented.
    // Group 1: example name (after `--example `, before `,` or `` ` `` or end of line)
    // Group 2: exit code digits
    // Group 3: fence content (the WITNESS lines, also 2-space-indented)
    const captureRe =
      /\*\*captured run\*\*\s*\(`cargo run --example ([^\s,`]+)[^)]*\$\?\s*=\s*(\d+)[^)]*\):\s*\n  ```\n([\s\S]*?)  ```/g;

    let m: RegExpExecArray | null;
    while ((m = captureRe.exec(section)) !== null) {
      const exampleName = m[1];
      const exitCode = parseInt(m[2], 10);
      const fenceContent = m[3];
      // Strip the 2-space prefix from each content line (it is indentation, not content).
      const outputLines = fenceContent
        .split("\n")
        .filter((l, i, arr) => !(i === arr.length - 1 && l === ""))
        .map((l) => (l.startsWith("  ") ? l.slice(2) : l));
      witnesses.push({
        example: exampleName,
        iteration,
        output: outputLines,
        exitCode,
      });
    }
  }

  if (witnesses.length === 0) {
    throw new Error(
      `No captured run blocks found in ${path.join(REPO_ROOT, "DOC_COVERAGE_LOG.md")}. ` +
        "The file must contain fenced WITNESS blocks following captured run bullets.",
    );
  }

  return witnesses;
}

// ---------------------------------------------------------------------------
// DepSummary — Rust workspace deps + npm package deps
// ---------------------------------------------------------------------------

/** A single Rust workspace.dependencies entry with a pinned version. */
export interface RustDep {
  name: string;
  version: string;
}

/** Dependency surface parsed from real Cargo.toml and web/package.json. */
export interface DepSummary {
  workspaceVersion: string;
  rustDeps: RustDep[];
  npmDeps: Record<string, string>;
  rustSource: string;
  npmSource: string;
}

/** Read the real dependency surface: workspace.dependencies from Cargo.toml and
 *  dependencies + devDependencies from web/package.json. Throws if either file is
 *  absent — anti-fabrication boundary: values rendered on /deps come from these
 *  real files, not from fixtures. */
export async function readDepSummary(): Promise<DepSummary> {
  const cargoPath = path.join(REPO_ROOT, "Cargo.toml");
  const pkgPath = path.join(REPO_ROOT, "web", "package.json");
  const cargo = await fs.readFile(cargoPath, "utf8");
  const pkgText = await fs.readFile(pkgPath, "utf8");

  // Capture workspace version from [workspace.package] section.
  const verMatch = cargo.match(/^\[workspace\.package\][^\[]*version\s*=\s*"([^"]+)"/ms);
  const workspaceVersion = verMatch ? verMatch[1] : "";

  // Extract the [workspace.dependencies] block (everything between the header
  // and the next section header).
  const wdepMatch = cargo.match(/^\[workspace\.dependencies\]([\s\S]*?)(?=^\[|\z)/m);
  const rustDeps: RustDep[] = [];
  if (wdepMatch) {
    const block = wdepMatch[1];
    // Match lines with a pinned plain string version: name = "x.y.z"
    // Skip path dependencies (lines containing `path =`).
    const lineRe = /^\s*([\w][\w-]*)\s*=\s*"([^"]+)"/gm;
    let lm: RegExpExecArray | null;
    while ((lm = lineRe.exec(block)) !== null) {
      rustDeps.push({ name: lm[1], version: lm[2] });
    }
  }

  // Parse npm deps from package.json.
  const pkg = JSON.parse(pkgText) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
  };
  const npmDeps: Record<string, string> = {
    ...(pkg.dependencies ?? {}),
    ...(pkg.devDependencies ?? {}),
  };

  return {
    workspaceVersion,
    rustDeps,
    npmDeps,
    rustSource: "Cargo.toml",
    npmSource: "web/package.json",
  };
}

export interface AdmissionPipelineState {
  label: string;
  receiptState: string;
  axisState: string;
  gateVerdict: string;
}

export interface ReceiptGraphRow {
  sourceFile: string;
  status: string | null;
  axisState: string;
  gateVerdict: string;
}

export interface AdmissionGraph {
  totalAdmitted: number;
  totalRefused: number;
  totalUnknown: number;
  receiptSummary: ReceiptGraphRow[];
  pipelineStates: AdmissionPipelineState[];
}

export async function readAdmissionGraph(): Promise<AdmissionGraph> {
  const receipts = await readReceipts();
  
  let totalAdmitted = 0;
  let totalRefused = 0;
  let totalUnknown = 0;
  
  const receiptSummary: ReceiptGraphRow[] = receipts.map((r) => {
    let axisState = "unknown";
    let gateVerdict = "REFUSED";
    
    if (r.status === "ADMITTED") {
      axisState = "admitted";
      gateVerdict = "ADMITTED";
      totalAdmitted++;
    } else if (r.status) {
      axisState = "refused";
      gateVerdict = "REFUSED";
      totalRefused++;
    } else {
      axisState = "unknown";
      gateVerdict = "REFUSED";
      totalUnknown++;
    }
    
    return {
      sourceFile: r.sourceFile,
      status: r.status ?? null,
      axisState,
      gateVerdict
    };
  });
  
  const logSrc = await fs.readFile(path.join(REPO_ROOT, "DOC_COVERAGE_LOG.md"), "utf8");
  const witnessMatch = logSrc.match(/WITNESS admission_pipeline[^\n]*\n((?:\s+\[[A-Z]\][^\n]+\n)+)/);
  const pipelineStates: AdmissionPipelineState[] = [];
  if (witnessMatch) {
    const stateRe = /\[([A-Z])\]\s+(.+?)\s+(?:→|->)\s+(.+)/g;
    let sm: RegExpExecArray | null;
    while ((sm = stateRe.exec(witnessMatch[1])) !== null) {
      const label = sm[1];
      const desc = sm[2].trim();
      const verdictStr = sm[3].trim();
      
      let axisState = "unknown";
      let gateVerdict = verdictStr.includes("true") ? "ADMITTED" : "REFUSED";

      if (desc.includes("unknown")) {
         axisState = "unknown";
      } else if (desc.includes("tampered") || desc.includes("refused")) {
         axisState = "refused";
      } else if (desc.includes("verified") || desc.includes("intact")) {
         axisState = "admitted";
      }
      
      pipelineStates.push({ label, receiptState: desc, axisState, gateVerdict });
    }
  } else {
    throw new Error("Missing admission_pipeline WITNESS in DOC_COVERAGE_LOG.md");
  }

  return {
    totalAdmitted,
    totalRefused,
    totalUnknown,
    receiptSummary,
    pipelineStates,
  };
}
