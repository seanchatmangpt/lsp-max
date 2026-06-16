import Link from "next/link";
import { readReceipts, readWorkspaceVersion, readCoverage } from "@/lib/project";

export const dynamic = "force-dynamic";

export default async function Home() {
  // Real counts, computed from real artifacts at request time.
  const [receipts, version, cov] = await Promise.all([
    readReceipts(),
    readWorkspaceVersion(),
    readCoverage(),
  ]);
  const admitted = receipts.filter((r) => r.status === "ADMITTED").length;

  return (
    <section>
      <h1>lsp-max, represented faithfully</h1>
      <p className="lede">
        A law-state runtime projected through LSP, version <b>{version}</b>. Every
        number on this site is read from the project at request time — if the
        repository changed, this page would change with it.
      </p>

      <div className="stats">
        <Link href="/receipts" className="stat">
          <span className="stat-n">{receipts.length}</span>
          <span className="stat-l">real receipts</span>
        </Link>
        <div className="stat">
          <span className="stat-n">{admitted}</span>
          <span className="stat-l">ADMITTED</span>
        </div>
        <div className="stat">
          <span className="stat-n">{version}</span>
          <span className="stat-l">CalVer</span>
        </div>
        <Link href="/coverage" className="stat">
          <span className="stat-n">{cov.covered}</span>
          <span className="stat-l">covered capabilities</span>
        </Link>
        <div className="stat">
          <span className="stat-n">{cov.gaps}</span>
          <span className="stat-l">open gaps</span>
        </div>
      </div>

      <p className="note">
        Witness: these counts come from <code>readReceipts()</code> /{" "}
        <code>readWorkspaceVersion()</code> in <code>lib/project.ts</code>, which
        read <code>receipts/*.receipt.json</code> and <code>Cargo.toml</code> from
        the repository root. Delete those and the build breaks — no fixtures stand
        in.
      </p>
    </section>
  );
}
