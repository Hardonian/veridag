import Link from "next/link";

export default function Home() {
  return (
    <div>
      <h1>Veridag</h1>
      <p className="tagline">
        An implementation-independent protocol for deterministic, Byzantine-resilient,
        capability-secured, verifiable distributed computation.
      </p>
      <p>
        It is not a cryptocurrency, not a blockchain clone, and not a Rust framework. It
        is a neutral distributed execution substrate for mutually distrustful humans,
        organizations, machines, services, AI agents, devices, and applications.
      </p>
      <pre><code>{`consensus + verifiable state + deterministic computation
+ capability security + data availability + cryptographic proofs
= distributed trust fabric`}</code></pre>

      <h2>What it is</h2>
      <div className="card-grid">
        <div className="card">
          <h3>Correctness first</h3>
          <p className="muted">
            Priority order: correctness &gt; determinism &gt; security &gt; implementation
            independence &gt; modularity &gt; verification &gt; operability &gt; performance.
          </p>
        </div>
        <div className="card">
          <h3>Three levels</h3>
          <p className="muted">
            Level 1 normative spec, Level 2 formal executable model (Quint), Level 3
            reference Rust implementation. An implementation is correct only if it
            satisfies Levels 1 and 2.
          </p>
        </div>
        <div className="card">
          <h3>One static binary</h3>
          <p className="muted">
            No external database, no message broker, no orchestrator. Runs on a laptop,
            a cloud VM, or a single-board computer.
          </p>
        </div>
      </div>

      <h2>Status</h2>
      <p>
        <strong className="status-done">0.1.0-alpha</strong> — the reference implementation
        is built and tested end to end: spec, formal model, and Rust implementation all
        present and satisfying the Agreement / Finality / Integrity invariants.
      </p>

      <Link href="/quickstart" className="cta">
        Run your first consensus demo
      </Link>
    </div>
  );
}
