import type { ReactNode } from "react";
import Link from "next/link";
import "./globals.css";

export const metadata = {
  title: "Veridag — Deterministic Distributed Trust Fabric",
  description:
    "An implementation-independent protocol and zero-unsafe Rust engine for deterministic, Byzantine-resilient, capability-secured distributed execution across AI agents, edge devices, and enterprise state.",
  keywords: [
    "distributed systems",
    "byzantine fault tolerance",
    "dag-bft",
    "deterministic computation",
    "ai agents trust fabric",
    "capability security",
    "rust",
    "quic",
  ],
  authors: [{ name: "Veridag Protocol Contributors" }],
  openGraph: {
    title: "Veridag — Deterministic Distributed Trust Fabric",
    description:
      "Pure-function DAG-BFT consensus, zero-unsafe Rust core, native capability security, and sub-10MB edge footprint.",
    url: "https://veridag.dev",
    siteName: "Veridag",
    type: "website",
  },
};

const nav = [
  { href: "/", label: "Home" },
  { href: "/protocol", label: "Protocol" },
  { href: "/architecture", label: "Architecture" },
  { href: "/explorer", label: "Explorer" },
  { href: "/faucet", label: "Faucet" },
  { href: "/enterprise", label: "Enterprise" },
  { href: "/pricing", label: "Pricing" },
  { href: "/roadmap", label: "Roadmap" },
  { href: "/security", label: "Security" },
  { href: "/quickstart", label: "Quickstart" },
];



export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header className="site-header">
          <div className="brand-wrapper">
            <Link href="/" className="brand">
              <span className="brand-icon">⚡</span>
              <span>Veridag</span>
            </Link>
            <span className="brand-badge">v0.1.0-alpha</span>
          </div>
          <nav>
            {nav.map((n) => (
              <Link key={n.href} href={n.href}>
                {n.label}
              </Link>
            ))}
          </nav>
          <a
            className="gh-btn"
            href="https://github.com/Hardonian/veridag"
            target="_blank"
            rel="noreferrer"
          >
            <span>GitHub</span>
            <span style={{ color: "var(--accent-emerald-bright)" }}>★</span>
          </a>
        </header>
        <main className="site-main">{children}</main>
        <footer className="site-footer">
          <div>
            <strong>Veridag Protocol</strong> — Deterministic, Byzantine-Resilient Distributed Trust Fabric.
          </div>
          <div className="muted">
            Three-Level Authority: Normative Spec &gt; Quint Formal Model &gt; Reference Rust Crates. Correct only if it satisfies Levels 1 and 2.
          </div>
          <div className="muted" style={{ fontSize: "12px", marginTop: "4px" }}>
            Dual-licensed under Apache-2.0 and MIT.
          </div>
        </footer>
      </body>
    </html>
  );
}
