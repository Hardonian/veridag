import type { ReactNode } from "react";
import Link from "next/link";
import "./globals.css";

export const metadata = {
  title: "Veridag — Deterministic Byzantine-Resilient Distributed Execution",
  description:
    "Veridag is an implementation-independent protocol for deterministic, Byzantine-resilient, capability-secured, verifiable distributed computation.",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header className="site-header">
          <Link href="/" className="brand">
            Veridag
          </Link>
          <nav>
            <Link href="/protocol">Protocol</Link>
            <Link href="/roadmap">Roadmap</Link>
            <Link href="/quickstart">Quickstart</Link>
            <a href="https://github.com/Hardonian/veridag" target="_blank" rel="noreferrer">
              GitHub
            </a>
          </nav>
        </header>
        <main className="site-main">{children}</main>
        <footer className="site-footer">
          <span>Veridag — distributed trust fabric. Dual-licensed Apache-2.0 / MIT.</span>
        </footer>
      </body>
    </html>
  );
}
