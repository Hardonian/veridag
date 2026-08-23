import Link from "next/link";

export default function Home() {
  return (
    <div>
      {/* Hero Section */}
      <section className="hero">
        <div className="hero-eyebrow">
          <span>⚡ Protocol v0.1.0-alpha Released</span>
          <span style={{ opacity: 0.5 }}>•</span>
          <span>Formal Quint Model Verified</span>
        </div>
        <h1>
          <span className="hero-headline">The Deterministic </span>
          <br />
          <span className="hero-gradient">Distributed Trust Fabric</span>
        </h1>
        <p className="hero-tagline">
          An implementation-independent protocol and zero-unsafe Rust engine for
          deterministic, Byzantine-resilient, capability-secured distributed computation.
          Built for autonomous AI agents, enterprise multi-party state, and edge device meshes.
        </p>
        <div className="hero-actions">
          <Link href="/quickstart" className="btn-primary">
            <span>⚡ Run 3-Minute Quickstart</span>
          </Link>
          <Link href="/architecture" className="btn-secondary">
            <span>Explore Architecture →</span>
          </Link>
          <a
            href="https://github.com/Hardonian/veridag"
            target="_blank"
            rel="noreferrer"
            className="btn-secondary"
          >
            <span>GitHub Repository</span>
          </a>
        </div>
      </section>

      {/* Terminal Simulator Showcase */}
      <section>
        <div className="terminal-box">
          <div className="terminal-header">
            <div className="terminal-dots">
              <span className="terminal-dot dot-red"></span>
              <span className="terminal-dot dot-yellow"></span>
              <span className="terminal-dot dot-green"></span>
            </div>
            <span className="terminal-title">veridag-node — consensus demo (4 validators, in-process)</span>
            <span style={{ fontSize: "11px", color: "var(--accent-emerald-bright)" }}>● LIVE BFT COMMIT</span>
          </div>
          <div className="terminal-body">
            <div><span className="terminal-prompt">$ </span>cargo run -p veridag-node -- demo</div>
            <div className="muted">veridag-node demo: 4-validator committee, in-process</div>
            <div className="muted">submitted transfer alice-&gt;bob 40 to all mempools</div>
            <div className="terminal-highlight">round 1..9: max round reached 9 (wave 2 committed)</div>
            <div>validator 0: state_root=<span className="terminal-success">0xf7aa17319c5c16538466bbba21d451cb0d7d4c82b9a7c3b999fb4eb8b22a0149</span> checkpoints=1</div>
            <div>&nbsp;&nbsp;checkpoint seq=1 id=0x2c0f6f0ba82cb46a9e223dcb44f9c6d480da39b56fce2685799a779140fa7812</div>
            <div>validator 1: state_root=<span className="terminal-success">0xf7aa17319c5c16538466bbba21d451cb0d7d4c82b9a7c3b999fb4eb8b22a0149</span> checkpoints=1</div>
            <div>validator 2: state_root=<span className="terminal-success">0xf7aa17319c5c16538466bbba21d451cb0d7d4c82b9a7c3b999fb4eb8b22a0149</span> checkpoints=1</div>
            <div>validator 3: state_root=<span className="terminal-success">0xf7aa17319c5c16538466bbba21d451cb0d7d4c82b9a7c3b999fb4eb8b22a0149</span> checkpoints=1</div>
            <div className="terminal-success" style={{ marginTop: "6px" }}>✓ AGREEMENT OK: identical state root across 4 validators</div>
            <div className="terminal-highlight">bob balance: 40 (expected 40)</div>
          </div>
        </div>
      </section>

      {/* Core Capabilities */}
      <section>
        <h2>🛡️ Core Architectural Pillars</h2>
        <div className="card-grid">
          <div className="card">
            <span className="card-icon">📐</span>
            <h3>Pure-Function DAG-BFT</h3>
            <p>
              Consensus is a pure, immutable function over a causal DAG. Given the same vertices,
              every validator derives the exact same commit point and wave ordering.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🔒</span>
            <h3>Zero-Unsafe Core</h3>
            <p>
              Every consensus, execution, and state crate strictly enforces <code>#![forbid(unsafe_code)]</code>.
              Memory safety bugs cannot penetrate the BFT core.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🔑</span>
            <h3>Capability Security</h3>
            <p>
              Fine-grained object-level capabilities replace blunt permissions. Handles are scoped,
              tamper-proof, and deterministically verified at the state layer.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">💾</span>
            <h3>Instant Crash Recovery</h3>
            <p>
              Integrated with embedded <code>sled</code> storage. A validator can drop all memory mid-commit,
              restart from disk, and land on the byte-identical state root.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🚀</span>
            <h3>Sub-10MB Edge Footprint</h3>
            <p>
              Single static binary with zero external services. No Docker, no Kubernetes, no Postgres,
              and no Kafka required. Runs on Raspberry Pis and cloud VMs alike.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">⚡</span>
            <h3>Authenticated QUIC Transport</h3>
            <p>
              1-RTT handshake with self-signed Ed25519 TLS 1.3 certificates and domain-separated preimages.
              Zero head-of-line blocking across independent streams.
            </p>
          </div>
        </div>
      </section>

      {/* Target Use Cases */}
      <section>
        <h2>🎯 High-Value Production Use Cases</h2>
        <div className="card-grid">
          <div className="card">
            <span className="card-icon">🤖</span>
            <h3>Autonomous AI Agent Swarms</h3>
            <p>
              Cryptographically verifiable multi-agent task execution, capability delegation, and tamper-proof
              audit logs that prevent prompt-injection state manipulation.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🏭</span>
            <h3>Cross-Enterprise Settlement</h3>
            <p>
              Shared multi-party state machine between distinct organizations without centralized cloud coordinators
              or vendor lock-in.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">📡</span>
            <h3>Edge &amp; IoT Device Meshes</h3>
            <p>
              Lightweight Byzantine consensus running on low-power devices and connected vehicles with resilient
              offline DAG merging upon reconnection.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🌲</span>
            <h3>Verifiable State Roots</h3>
            <p>
              BMH-1 Merkle state commitments and inclusion proofs enabling microsecond state verification for
              light clients and external auditors.
            </p>
          </div>
        </div>
      </section>

      {/* Comparison Matrix */}
      <section>
        <h2>📊 Why Veridag Beats the Alternatives</h2>
        <div className="table-container">
          <table>
            <thead>
              <tr>
                <th>Feature</th>
                <th>Veridag</th>
                <th>Traditional Blockchains</th>
                <th>Raft / Paxos</th>
                <th>Message Queues (Kafka)</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td><strong>Byzantine Fault Tolerant (3f+1)</strong></td>
                <td><span className="badge-yes">✓ Yes</span></td>
                <td><span className="badge-yes">✓ Yes</span></td>
                <td><span className="badge-no">✗ Crash-Only</span></td>
                <td><span className="badge-no">✗ No</span></td>
              </tr>
              <tr>
                <td><strong>Deterministic State Roots</strong></td>
                <td><span className="badge-yes">✓ BMH-1 Merkle</span></td>
                <td><span className="badge-partial">~ Variable</span></td>
                <td><span className="badge-no">✗ No</span></td>
                <td><span className="badge-no">✗ No</span></td>
              </tr>
              <tr>
                <td><strong>No Crypto Tokens / Gas Speculation</strong></td>
                <td><span className="badge-yes">✓ 100% Free &amp; Neutral</span></td>
                <td><span className="badge-no">✗ Gas Volatility</span></td>
                <td><span className="badge-yes">✓ Free</span></td>
                <td><span className="badge-yes">✓ Free</span></td>
              </tr>
              <tr>
                <td><strong>Embedded Zero-Config Storage</strong></td>
                <td><span className="badge-yes">✓ Embedded Sled</span></td>
                <td><span className="badge-no">✗ Multi-GB DB</span></td>
                <td><span className="badge-partial">~ Varies</span></td>
                <td><span className="badge-no">✗ Heavy Cluster</span></td>
              </tr>
              <tr>
                <td><strong>Transport Protocol</strong></td>
                <td><span className="badge-yes">✓ QUIC + TLS 1.3</span></td>
                <td><span className="badge-partial">~ Custom TCP</span></td>
                <td><span className="badge-partial">~ TCP / gRPC</span></td>
                <td><span className="badge-partial">~ TCP</span></td>
              </tr>
              <tr>
                <td><strong>Formal Model Checked</strong></td>
                <td><span className="badge-yes">✓ Quint Verified</span></td>
                <td><span className="badge-partial">~ Partial</span></td>
                <td><span className="badge-partial">~ Partial</span></td>
                <td><span className="badge-no">✗ No</span></td>
              </tr>
              <tr>
                <td><strong>Binary Footprint</strong></td>
                <td><span className="badge-yes">✓ &lt; 10MB Static</span></td>
                <td><span className="badge-no">✗ Gigabytes</span></td>
                <td><span className="badge-partial">~ 50–200MB</span></td>
                <td><span className="badge-no">✗ JVM / Cluster</span></td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      {/* Quickstart Call to Action */}
      <section style={{ textAlign: "center", marginTop: "60px", padding: "40px 20px", background: "var(--bg-card)", borderRadius: "var(--radius-lg)", border: "1px solid var(--border-active)" }}>
        <h2 style={{ margin: "0 0 12px", justifyContent: "center" }}>Ready to run your first consensus run?</h2>
        <p style={{ maxWidth: "600px", margin: "0 auto 24px", color: "var(--text-muted)" }}>
          Get from zero to 4-validator consensus agreement in under 3 minutes.
        </p>
        <div style={{ display: "flex", gap: "16px", justifyContent: "center", flexWrap: "wrap" }}>
          <Link href="/quickstart" className="btn-primary">
            <span>Read the Quickstart Guide →</span>
          </Link>
          <Link href="/protocol" className="btn-secondary">
            <span>Read Protocol Specification</span>
          </Link>
        </div>
      </section>
    </div>
  );
}
