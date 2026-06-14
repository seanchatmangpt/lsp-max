import { readCliSurface } from "@/lib/project";

export const dynamic = "force-dynamic";

export default async function CliPage() {
  // Real command surface parsed from crates/lsp-max-cli/src/nouns/*.rs.
  const nouns = await readCliSurface();
  const totalVerbs = nouns.reduce((n, x) => n + x.verbs.length, 0);
  return (
    <section>
      <h1>CLI surface — clap-noun-verb</h1>
      <p className="lede">
        {nouns.length} nouns, {totalVerbs} verbs, parsed live from the real{" "}
        <code>#[verb(&quot;…&quot;)]</code> attributes in{" "}
        <code>crates/lsp-max-cli/src/nouns/*.rs</code>. Filename = noun,{" "}
        <code>#[verb]</code> = action.
      </p>
      <div className="grid">
        {nouns.map((n) => (
          <article key={n.noun} className="card">
            <div className="card-head">
              <h3>{n.noun}</h3>
              <span className="badge badge-admitted">{n.verbs.length}</span>
            </div>
            <ul className="verbs">
              {n.verbs.map((v) => (
                <li key={v.verb}>
                  <code className="mono">
                    lsp-max-cli {n.noun} {v.verb}
                    {v.args.map((a) => (
                      <span key={a} className="dim"> &lt;{a}&gt;</span>
                    ))}
                  </code>
                  {v.doc && <span className="vdoc">{v.doc}</span>}
                </li>
              ))}
            </ul>
            <p className="src">↳ {n.sourceFile}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
