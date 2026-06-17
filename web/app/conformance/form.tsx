"use client";

import { useActionState } from "react";
import { runConformance, type ConformanceVerdict } from "./actions";

function AxisChips({ axes, kind }: { axes: string[]; kind: string }) {
  if (axes.length === 0) return <span className="dim">none</span>;
  return (
    <span className="chips">
      {axes.map((a) => (
        <span key={a} className={`chip chip-${kind}`}>
          {a}
        </span>
      ))}
    </span>
  );
}

export function ConformanceForm() {
  const [v, formAction, pending] = useActionState<ConformanceVerdict | null, FormData>(
    runConformance,
    null,
  );
  return (
    <div>
      <form action={formAction} className="confform">
        <input
          name="instanceId"
          defaultValue="LSP_1"
          placeholder="instance id (e.g. LSP_1)"
          className="confinput"
        />
        <button type="submit" disabled={pending} className="runbtn">
          {pending ? "running real CLI…" : "▶ compute vector"}
        </button>
      </form>

      {v && !pending && (
        <div className={`runout ${v.ok ? "ok" : "fail"}`}>
          <div className="runout-head">
            <span className="mono">conformance vector --instance-id {v.instanceId}</span>
            <span className={`badge ${v.ok ? "badge-admitted" : "badge-refused"}`}>
              {v.ok ? `${v.admitted.length}A / ${v.refused.length}R / ${v.unknown.length}U` : "error"}
            </span>
          </div>
          {v.ok ? (
            <dl className="kv">
              <dt>admitted</dt>
              <dd><AxisChips axes={v.admitted} kind="admitted" /></dd>
              <dt>refused</dt>
              <dd><AxisChips axes={v.refused} kind="refused" /></dd>
              <dt>unknown</dt>
              <dd><AxisChips axes={v.unknown} kind="unknown" /></dd>
            </dl>
          ) : (
            <pre>{v.error}</pre>
          )}
          <p className="src">ran at {v.ranAt} · lsp-max-cli conformance vector</p>
        </div>
      )}
    </div>
  );
}
