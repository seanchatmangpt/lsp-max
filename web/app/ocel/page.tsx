import { readOcel } from "@/lib/project";

export const dynamic = "force-dynamic";

const TYPE_COLOR: Record<string, string> = {
  "checkpoint.admitted": "#5ad1a8",
  "diagnostic.published": "#d9a441",
  "file.projected": "#6aa6ff",
};

export default async function OcelPage() {
  // Real artifact: the project's object-centric event log. The graph below is
  // computed from real events + relationships; no synthetic nodes.
  const ocel = await readOcel();

  const events = [...ocel.events].sort((a, b) => a.time.localeCompare(b.time));
  const objects = ocel.objects;
  const objIndex = new Map(objects.map((o, i) => [o.id, i]));

  // Bipartite layout: events left, objects right.
  const rowH = 34;
  const top = 30;
  const evX = 230;
  const objX = 700;
  const width = 940;
  const height = top + Math.max(events.length, objects.length) * rowH + 20;

  const shortId = (s: string) => (s.length > 30 ? "…" + s.slice(-29) : s);

  return (
    <section>
      <h1>OCEL process evidence</h1>
      <p className="lede">
        Object-centric event log from <code>{ocel.sourceFile}</code>:{" "}
        {events.length} events across {ocel.eventTypes.length} event types, linked
        to {objects.length} objects ({ocel.objectTypes.join(", ")}). The graph is
        built from the real <code>relationships</code> in the log — every edge is
        an actual event→object link.
      </p>

      <div className="legend">
        {ocel.eventTypes.map((t) => (
          <span key={t} className="leg">
            <i style={{ background: TYPE_COLOR[t] ?? "#888" }} /> {t}
          </span>
        ))}
      </div>

      <div className="svgwrap">
        <svg width={width} height={height} role="img" aria-label="OCEL event-object graph">
          <text x={evX} y={18} className="svglabel" textAnchor="end">
            events ({events.length})
          </text>
          <text x={objX} y={18} className="svglabel">
            objects ({objects.length})
          </text>
          {/* edges first, from real relationships */}
          {events.map((ev, ei) => {
            const ey = top + ei * rowH;
            return ev.relationships.map((r, ri) => {
              const oi = objIndex.get(r.objectId);
              if (oi === undefined) return null;
              const oy = top + oi * rowH;
              return (
                <line
                  key={`${ev.id}-${ri}`}
                  x1={evX + 6}
                  y1={ey}
                  x2={objX - 6}
                  y2={oy}
                  stroke={TYPE_COLOR[ev.type] ?? "#555"}
                  strokeOpacity={0.5}
                />
              );
            });
          })}
          {/* event nodes */}
          {events.map((ev, ei) => {
            const ey = top + ei * rowH;
            return (
              <g key={ev.id}>
                <circle cx={evX} cy={ey} r={5} fill={TYPE_COLOR[ev.type] ?? "#888"} />
                <text x={evX - 12} y={ey + 4} className="svgtext" textAnchor="end">
                  {ev.type} · {ev.time.slice(11, 19)}
                </text>
              </g>
            );
          })}
          {/* object nodes */}
          {objects.map((o, oi) => {
            const oy = top + oi * rowH;
            return (
              <g key={o.id}>
                <rect x={objX - 4} y={oy - 5} width={10} height={10} fill="#e6e9ef" />
                <text x={objX + 14} y={oy + 4} className="svgtext">
                  {o.type}: {shortId(o.id)}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
      <p className="src">↳ {ocel.sourceFile}</p>
    </section>
  );
}
