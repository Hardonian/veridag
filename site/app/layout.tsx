import type { ReactNode } from "react";
import Link from "next/link";
import "./globals.css";

export const metadata = {
  title: "Veridag — Deterministic Byzantine-Resilient Distributed Execution",
  description:
    "An implementation-independent protocol for deterministic, Byzantine-resilient distributed execution. Pure-function BFT commit, causal DAG ordering, crash-safe persistence, edge-grade footprint.",
};

const nav = [
  { href: "/", label: "Home" },
  { href: "/protocol", label: "Protocol" },
  { href: "/architecture", label: "Architecture" },
  { href: "/roadmap", label: "Roadmap" },
  { href: "/security", label: "Security" },
  { href: "/quickstart", label: "Quickstart" },
];

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header className="site-header">
          <Link href="/" className="brand">
            Veridag
          </Link>
          <nav>
            {nav.map((n) => (
              <Link key={n.href} href={n.href}>
                {n.label}
              </Link>
            ))}
          </nav>
          <a
            className="gh"
            href="https://github.com/Hardonian/veridag"
            target="_blank"
            rel="noreferrer"
          >
            GitHub
          </a>
        </header>
        <main>{children}</main>
        <footer className="site-footer">
          <span>
            Veridag — deterministic Byzantine-resilient distributed execution.
          </span>
          <span className="muted">
            Spec &gt; Formal model &gt; Implementation. Correct only if it
            satisfies Levels 1 and 2.
          </span>
        </footer>
      </body>
    </html>
  );
}
