import { ConformanceForm } from "./form";

export const dynamic = "force-dynamic";

export default function ConformancePage() {
  return (
    <section>
      <h1>Conformance verdict — live</h1>
      <p className="lede">
        Enter an instance id and compute its <code>ConformanceVector</code> by
        running the real <code>lsp-max-cli conformance vector --instance-id …</code>{" "}
        via a server action. The three-valued result (admitted / refused / unknown)
        is parsed from the CLI&apos;s actual JSON output. A fresh instance returns
        every law axis as <b>unknown</b> — no evidence yet — which is the doctrine:
        unknown never collapses into admitted or refused.
      </p>
      <ConformanceForm />
    </section>
  );
}
