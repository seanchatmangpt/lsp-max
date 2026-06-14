import { readReceipts, type Receipt } from "@/lib/project";

export const dynamic = "force-dynamic"; // always read the real files at request time

function StatusBadge({ status }: { status?: string }) {
  const s = status ?? "UNKNOWN";
  return <span className={`badge badge-${s.toLowerCase()}`}>{s}</span>;
}

function ReceiptCard({ r }: { r: Receipt }) {
  const digest = r.output_hash ?? r.digest ?? r.exported_artifact_digest;
  return (
    <article className="card">
      <div className="card-head">
        <h3>{r.checkpoint ?? r.run_id ?? r.sourceFile}</h3>
        <StatusBadge status={r.status} />
      </div>
      <dl className="kv">
        {r.boundary && (
          <>
            <dt>boundary</dt>
            <dd>{r.boundary}</dd>
          </>
        )}
        {digest && (
          <>
            <dt>digest</dt>
            <dd className="mono">
              {digest.slice(0, 16)}… <span className="dim">{r.output_digest_algorithm ?? r.digest_algorithm ?? ""}</span>
            </dd>
          </>
        )}
        {r.run_date && (
          <>
            <dt>run date</dt>
            <dd>{r.run_date}</dd>
          </>
        )}
        {r.replay_pointer && (
          <>
            <dt>replay</dt>
            <dd className="mono small">{r.replay_pointer}</dd>
          </>
        )}
      </dl>
      {r.claims && (
        <ul className="claims">
          {Object.entries(r.claims).map(([k, v]) => (
            <li key={k}>
              <b>{k}</b> {v}
            </li>
          ))}
        </ul>
      )}
      <p className="src">↳ {r.sourceFile}</p>
    </article>
  );
}

export default async function ReceiptsPage() {
  // Real data path: fs reads of the project's *.receipt.json. Deleting the
  // receipts directory makes this page throw — the witness against fabrication.
  const receipts = await readReceipts();
  const admitted = receipts.filter((r) => r.status === "ADMITTED").length;
  return (
    <section>
      <h1>Receipt ledger</h1>
      <p className="lede">
        {receipts.length} real receipts read from the repository ({admitted}{" "}
        ADMITTED). Each digest, claim, and replay pointer below is parsed from an
        actual <code>*.receipt.json</code> on disk.
      </p>
      <div className="grid">
        {receipts.map((r) => (
          <ReceiptCard key={r.sourceFile} r={r} />
        ))}
      </div>
    </section>
  );
}
