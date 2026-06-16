import { readDepSummary } from "@/lib/project";

export const dynamic = "force-dynamic";

export default async function DepsPage() {
  const deps = await readDepSummary();
  const npmEntries = Object.entries(deps.npmDeps);

  return (
    <section>
      <h1>Dependency surface</h1>
      <p className="lede">
        Workspace version <b>{deps.workspaceVersion}</b> &mdash;{" "}
        {deps.rustDeps.length} Rust workspace deps tracked,{" "}
        {npmEntries.length} npm packages.
      </p>

      <div style={{ display: "flex", gap: "2rem", flexWrap: "wrap", alignItems: "flex-start" }}>
        <div>
          <h2>Rust workspace deps</h2>
          <table className="tbl">
            <thead>
              <tr>
                <th>Name</th>
                <th>Version</th>
              </tr>
            </thead>
            <tbody>
              {deps.rustDeps.map((d) => (
                <tr key={d.name}>
                  <td className="mono">{d.name}</td>
                  <td className="mono">{d.version}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="src">&#8627; {deps.rustSource}</p>
        </div>

        <div>
          <h2>npm packages</h2>
          <table className="tbl">
            <thead>
              <tr>
                <th>Name</th>
                <th>Version</th>
              </tr>
            </thead>
            <tbody>
              {npmEntries.map(([name, version]) => (
                <tr key={name}>
                  <td className="mono">{name}</td>
                  <td className="mono">{version}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="src">&#8627; {deps.npmSource}</p>
        </div>
      </div>
    </section>
  );
}
