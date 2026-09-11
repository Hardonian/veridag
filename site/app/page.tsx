import Link from "next/link";

export default function Home() {
  return (
    <div>
      {/* Hero Section */}
      <section className="hero">
        <div className="hero-eyebrow">
          <span>⚡ Protocol v0.1.0-alpha Released</span>
          <span style={{ opacity: 0.5 }}>•</span>
          <span>🏛️ USMCA &amp; G8 Sovereign Settlement (USDV)</span>
          <span style={{ opacity: 0.5 }}>•</span>
          <span>⛓️ Ethereum Settlement Substrate</span>
        </div>
        <h1>
          <span className="hero-headline">Sovereign Dollar Liquidity &amp; </span>
          <br />
          <span className="hero-gradient">The Iron-Clad Infrastructure of Ethereum</span>
        </h1>
        <p className="hero-tagline">
          Veridag powers <strong>USDV</strong>—the US-aligned sovereign digital dollar engineered for USMCA cross-border trade corridors and G8 multilateral treasury settlement—and
          serves as the ultra-fast, zero-reorg DAG execution, sequencing, and settlement substrate for Ethereum.
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

      {/* Flagship Pillars: USDV Sovereign Dollar & Ethereum Infra */}
      <section style={{ marginBottom: "48px" }}>
        <div className="card-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(360px, 1fr))" }}>
          <div className="card" style={{ border: "1px solid rgba(52, 211, 153, 0.4)", background: "linear-gradient(180deg, rgba(16, 185, 129, 0.08) 0%, rgba(10, 16, 26, 0.8) 100%)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" }}>
              <span style={{ fontSize: "28px" }}>🏛️</span>
              <div>
                <h3 style={{ margin: 0, color: "var(--accent-emerald-bright)" }}>USMCA &amp; G8 Multilateral Dollar (USDV)</h3>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>100% US Treasury Backed • Trade Corridor Settlement</span>
              </div>
            </div>
            <p style={{ fontSize: "14px", lineHeight: "1.6", color: "var(--text-main)", marginBottom: "16px" }}>
              Engineered for USMCA cross-border trade clearing and G8 economic forum financial market standards.
              Collateralized 1:1 by short-term US Treasury bills, overnight reverse repo, and FDIC cash deposits with direct US strategic alignment.
              Features mathematical Proof of Reserves (PoR) committed directly into consensus state roots and treaty-compliant capability-enforced sanctions screening.
            </p>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "13px", color: "var(--text-muted)", display: "flex", flexDirection: "column", gap: "8px" }}>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> US Strategic Alignment: 100% backed by short-term US Treasuries &amp; Fed RRP
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> USMCA Trade Corridors: Sub-100ms wave settlement eliminating Herstatt FX risk
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> G8 Economic Governance: Real-time OFAC &amp; FATF capability-enforced compliance
              </li>
            </ul>
          </div>

          <div className="card" style={{ border: "1px solid rgba(56, 189, 248, 0.4)", background: "linear-gradient(180deg, rgba(56, 189, 248, 0.08) 0%, rgba(10, 16, 26, 0.8) 100%)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" }}>
              <span style={{ fontSize: "28px" }}>⛓️</span>
              <div>
                <h3 style={{ margin: 0, color: "var(--accent-cyan-bright)" }}>Ethereum Infrastructure Substrate</h3>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>L2 Sequencer &amp; Trustless L1 Bridge</span>
              </div>
            </div>
            <p style={{ fontSize: "14px", lineHeight: "1.6", color: "var(--text-main)", marginBottom: "16px" }}>
              High-throughput DAG-BFT sequencing and settlement for Ethereum. Delivers zero-reorg finality,
              native EVM JSON-RPC provider (<code>eth_*</code>), trustless on-chain checkpoint verification in Solidity
              (<code>VeridagLightClient.sol</code>), and two-way cross-chain portals.
            </p>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "13px", color: "var(--text-muted)", display: "flex", flexDirection: "column", gap: "8px" }}>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-cyan-bright)" }}>✓</span> L1 Light Client: 2f+1 quorum verification in Solidity
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-cyan-bright)" }}>✓</span> BMH-1 Merkle inclusion proofs for trustless bridge withdrawals
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-cyan-bright)" }}>✓</span> Native EVM JSON-RPC provider (Chain ID 0x5645)
              </li>
            </ul>
          </div>
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
            <span className="terminal-title">veridag-cli — USMCA &amp; G8 sovereign settlement &amp; ethereum bridge</span>
            <span style={{ fontSize: "11px", color: "var(--accent-emerald-bright)" }}>● 100% VERIFIED</span>
          </div>
          <div className="terminal-body">
            <div><span className="terminal-prompt">$ </span>veridag-cli usdv attest-reserves --tbills 80000000 --cash 15000000 --repo 5000000</div>
            <div className="muted">Oracle Custodian: 0xd7001a91ef8f29136f957e3b3d6fd36108c172f6c1e67f1cbe4505e2a061c01f</div>
            <div className="terminal-highlight">Total Attested: $100.00M USD (US T-Bills: $80.00M, FDIC Cash: $15.00M, RRP: $5.00M)</div>
            <div>BMH-1 State Root: <span className="terminal-success">0x988322bd84bb687273f3432e0e4b21d56cad0ba5d9c3c0e1a234161e92a165c1</span></div>
            <br />
            <div><span className="terminal-prompt">$ </span>veridag-cli usdv mint --to alice --amount 5000000</div>
            <div className="terminal-success">✓ Minted $5,000,000.000000 USDV (Supply: $5,000,000 USDV &le; Reserves $100,000,000)</div>
            <br />
            <div><span className="terminal-prompt">$ </span>veridag-cli eth bridge-proof --account alice</div>
            <div className="muted">Generating BMH-1 inclusion proof for Ethereum L1 VeridagLightClient.sol...</div>
            <div>Proof Siblings: <span className="terminal-highlight">[0x9799..., 0xd423...]</span> (Leaf Index: 2)</div>
            <div className="terminal-success">✓ Local Verification: PASSED | Verified against L1 Checkpoint StateRoot</div>
          </div>
        </div>
      </section>

      {/* Core Architectural Pillars */}
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
              Memory safety bugs cannot penetrate the core substrate.
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
            <span className="card-icon">🏛️</span>
            <h3>USMCA &amp; G8 Sovereign Settlement (USDV)</h3>
            <p>
              Institutional digital dollar collateralized by short-term US Treasuries,
              eliminating foreign exchange settlement risk across USMCA and G8 multilateral trade corridors.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">⛓️</span>
            <h3>Ethereum L2 Sequencing &amp; Settlement</h3>
            <p>
              Zero-reorg DAG-BFT sequencer eliminating MEV and front-running for Ethereum rollups,
              settling back to L1 via formal quorum checkpoint proofs.
            </p>
          </div>
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
                <td><strong>Sovereign Proof of Reserves (PoR)</strong></td>
                <td><span className="badge-yes">✓ Cryptographic Invariant (US Treasuries / RRP)</span></td>
                <td><span className="badge-no">✗ Off-chain Trust</span></td>
                <td><span className="badge-no">✗ No</span></td>
                <td><span className="badge-no">✗ No</span></td>
              </tr>
              <tr>
                <td><strong>Ethereum L1 Light Client</strong></td>
                <td><span className="badge-yes">✓ VeridagLightClient.sol</span></td>
                <td><span className="badge-partial">~ Heavy ZK / Optimistic</span></td>
                <td><span className="badge-no">✗ No</span></td>
                <td><span className="badge-no">✗ No</span></td>
              </tr>
              <tr>
                <td><strong>Conflict Scheduling</strong></td>
                <td><span className="badge-yes">✓ Parallel Prefix + Seq Suffix</span></td>
                <td><span className="badge-partial">~ Complex STM / Rollbacks</span></td>
                <td><span className="badge-no">✗ Single Thread</span></td>
                <td><span className="badge-no">✗ Partition-only</span></td>
              </tr>
              <tr>
                <td><strong>Memory Safety</strong></td>
                <td><span className="badge-yes">✓ #![forbid(unsafe_code)]</span></td>
                <td><span className="badge-partial">~ C++ / Go / Rust Unsafe</span></td>
                <td><span className="badge-partial">~ Mixed</span></td>
                <td><span className="badge-partial">~ Java / C++</span></td>
              </tr>
              <tr>
                <td><strong>Binary Footprint</strong></td>
                <td><span className="badge-yes">✓ &lt; 10MB Static</span></td>
                <td><span className="badge-no">✗ 500GB+ Node Bloat</span></td>
                <td><span className="badge-partial">~ 50–200MB</span></td>
                <td><span className="badge-no">✗ JVM / Cluster</span></td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      {/* Quickstart Call to Action */}
      <section style={{ textAlign: "center", marginTop: "60px", padding: "40px 20px", background: "var(--bg-card)", borderRadius: "var(--radius-lg)", border: "1px solid var(--border-active)" }}>
        <h2 style={{ margin: "0 0 12px", justifyContent: "center" }}>Deploy Sovereign USDV &amp; Ethereum Infrastructure</h2>
        <p style={{ maxWidth: "600px", margin: "0 auto 24px", color: "var(--text-muted)" }}>
          Run the full consensus engine, manage USMCA-grade sovereign USDV settlement, and bridge state to Ethereum in under 3 minutes.
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
