import type { Metadata } from "next";
import { readWorkspaceVersion } from "@/lib/project";
import Link from "next/link";
import "./globals.css";

export const metadata: Metadata = {
  title: "lsp-max — faithful representation",
  description: "Every rendered claim is witnessed by the real lsp-max project.",
};

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Version is read from the real Cargo.toml — banner breaks if the project moves.
  const version = await readWorkspaceVersion();
  return (
    <html lang="en">
      <body>
        <header className="topbar">
          <Link href="/" className="brand">
            lsp-max
          </Link>
          <span className="ver">v{version} · CalVer</span>
          <nav>
            <Link href="/receipts">Receipts</Link>
            <Link href="/cli">CLI</Link>
            <Link href="/coverage">Coverage</Link>
            <Link href="/witness">Live witnesses</Link>
            <Link href="/ocel">OCEL</Link>
          </nav>
        </header>
        <main className="main">{children}</main>
        <footer className="foot">
          Every value on this site is read from the lsp-max repository at request
          time. No fixtures.
        </footer>
      </body>
    </html>
  );
}
