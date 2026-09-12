import Link from "next/link";

export default function Home() {
  return (
    <div>
      {/* Hero Section */}
      <section className="hero">
        <div className="hero-eyebrow">
          <span>⚡ Protocol v0.1.0-alpha Released</span>
          <span style={{ opacity: 0.5 }}>•</span>
          <span>🏛️ Sovereign Multilateral USDV</span>
          <span style={{ opacity: 0.5 }}>•</span>
          <span>🤝 Settler &amp; Hardonian AI Stack</span>
          <span style={{ opacity: 0.5 }}>•</span>
          <span>🌐 Bitcoin, Ethereum &amp; Multi-Chain</span>
        </div>
        <h1>
          <span className="hero-headline">Universal Settlement Substrate for </span>
          <br />
          <span className="hero-gradient">Sovereign Finance, Cross-Chain Crypto &amp; Autonomous AI</span>
        </h1>
        <p className="hero-tagline">
          Veridag is the high-throughput DAG-BFT consensus and execution fabric engineered for <strong>USDV</strong> sovereign multilateral dollar liquidity, native monetary clearing for the <strong>Settler</strong> reconciliation engine and <strong>Hardonian Sovereign AI Stack</strong>, and cryptographic verification across <strong>Bitcoin, Ethereum, and major crypto networks</strong>.
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

      {/* Flagship Pillars: USDV, Settler/Hardonian, Bitcoin & Ethereum */}
      <section style={{ marginBottom: "48px" }}>
        <div className="card-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(340px, 1fr))" }}>
          {/* USDV Sovereign Dollar */}
          <div className="card" style={{ border: "1px solid rgba(52, 211, 153, 0.4)", background: "linear-gradient(180deg, rgba(16, 185, 129, 0.08) 0%, rgba(10, 16, 26, 0.8) 100%)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" }}>
              <span style={{ fontSize: "28px" }}>🏛️</span>
              <div>
                <h3 style={{ margin: 0, color: "var(--accent-emerald-bright)" }}>USMCA &amp; G8 Multilateral Dollar (USDV)</h3>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>100% US Treasury Backed • Trade Settlement</span>
              </div>
            </div>
            <p style={{ fontSize: "14px", lineHeight: "1.6", color: "var(--text-main)", marginBottom: "16px" }}>
              Engineered for USMCA cross-border trade clearing and G8 economic forum standards.
              Collateralized 1:1 by short-term US Treasury bills, overnight reverse repo, and FDIC cash deposits.
              Features mathematical Proof of Reserves (PoR) in state roots and capability-enforced OFAC compliance.
            </p>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "13px", color: "var(--text-muted)", display: "flex", flexDirection: "column", gap: "8px" }}>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> US Strategic Alignment: 100% backed by short-term US Treasuries &amp; Fed RRP
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> USMCA Trade Corridors: Sub-100ms wave settlement eliminating FX settlement risk
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> G8 Economic Governance: Real-time OFAC &amp; FATF capability compliance
              </li>
            </ul>
          </div>

          {/* Hardonian AI Stack & Settler Layer */}
          <div className="card" style={{ border: "1px solid rgba(168, 85, 247, 0.4)", background: "linear-gradient(180deg, rgba(168, 85, 247, 0.08) 0%, rgba(10, 16, 26, 0.8) 100%)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" }}>
              <span style={{ fontSize: "28px" }}>🤝</span>
              <div>
                <h3 style={{ margin: 0, color: "#c084fc" }}>Hardonian Sovereign AI &amp; Settler Layer</h3>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>Settler Engine • Multi-Tenant Consortium</span>
              </div>
            </div>
            <p style={{ fontSize: "14px", lineHeight: "1.6", color: "var(--text-main)", marginBottom: "16px" }}>
              Native settlement foundation for the Hardonian Sovereign AI Stack. Ingests Settler <code>EvidenceManifest</code> proofpack
              hashes, anchors reconciliation runs into the BMH-1 state tree, and executes atomic multi-tenant batch settlements
              across Settler, MissionLedger, ReadyLayer, nlsqlc, and mcpwall.
            </p>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "13px", color: "var(--text-muted)", display: "flex", flexDirection: "column", gap: "8px" }}>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "#c084fc" }}>✓</span> Native Settler Reconciliation Anchoring (Object Type 5)
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "#c084fc" }}>✓</span> Multi-Tenant Consortium Tenant Registry (Object Type 6)
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "#c084fc" }}>✓</span> Atomic USDV batch disbursement with zero variance
              </li>
            </ul>
          </div>

          {/* Bitcoin Native Substrate */}
          <div className="card" style={{ border: "1px solid rgba(245, 158, 11, 0.4)", background: "linear-gradient(180deg, rgba(245, 158, 11, 0.08) 0%, rgba(10, 16, 26, 0.8) 100%)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" }}>
              <span style={{ fontSize: "28px" }}>₿</span>
              <div>
                <h3 style={{ margin: 0, color: "#fbbf24" }}>Bitcoin Substrate &amp; SPV Bridge</h3>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>80-Byte Header SPV • UTXO Cross-Chain Codecs</span>
              </div>
            </div>
            <p style={{ fontSize: "14px", lineHeight: "1.6", color: "var(--text-main)", marginBottom: "16px" }}>
              Native Bitcoin verification substrate. Validates 80-byte block headers, compact nBits PoW targets,
              Double-SHA256 digests, and Merkle inclusion proofs. Features bidirectional UTXO deposit and withdrawal
              codecs and continuous SPV tracking.
            </p>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "13px", color: "var(--text-muted)", display: "flex", flexDirection: "column", gap: "8px" }}>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "#fbbf24" }}>✓</span> Canonical 80-Byte Header &amp; Compact nBits PoW verification
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "#fbbf24" }}>✓</span> Cryptographic Bitcoin Merkle inclusion proof engine
              </li>
              <li style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ color: "#fbbf24" }}>✓</span> UTXO cross-chain deposit/withdrawal binary codecs
              </li>
            </ul>
          </div>

          {/* Ethereum & EVM Substrate */}
          <div className="card" style={{ border: "1px solid rgba(56, 189, 248, 0.4)", background: "linear-gradient(180deg, rgba(56, 189, 248, 0.08) 0%, rgba(10, 16, 26, 0.8) 100%)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" }}>
              <span style={{ fontSize: "28px" }}>⛓️</span>
              <div>
                <h3 style={{ margin: 0, color: "var(--accent-cyan-bright)" }}>Ethereum &amp; EVM Infrastructure</h3>
                <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>L2 Sequencer &amp; Trustless L1 Bridge</span>
              </div>
            </div>
            <p style={{ fontSize: "14px", lineHeight: "1.6", color: "var(--text-main)", marginBottom: "16px" }}>
              High-throughput DAG-BFT sequencing and settlement for Ethereum and EVM ecosystems. Delivers zero-reorg finality,
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
            <span className="terminal-title">veridag-cli — Sovereign USDV, Hardonian Settler, Bitcoin SPV &amp; EVM rails</span>
            <span style={{ fontSize: "11px", color: "var(--accent-emerald-bright)" }}>● 100% VERIFIED</span>
          </div>
          <div className="terminal-body">
            <div><span className="terminal-prompt">$ </span>veridag-cli usdv attest-reserves --tbills 80000000 --cash 15000000 --repo 5000000</div>
            <div className="muted">Oracle Custodian: 0xd7001a91ef8f29136f957e3b3d6fd36108c172f6c1e67f1cbe4505e2a061c01f</div>
            <div className="terminal-highlight">Total Attested: $100.00M USD (US T-Bills: $80.00M, FDIC Cash: $15.00M, RRP: $5.00M)</div>
            <div>BMH-1 State Root: <span className="terminal-success">0x988322bd84bb687273f3432e0e4b21d56cad0ba5d9c3c0e1a234161e92a165c1</span></div>
            <br />
            <div><span className="terminal-prompt">$ </span>veridag-cli usdv settle --tenant settler-inc --run-id rec_run_01J8 --amount 2500000 --manifest-hash 0xca49b7e128...</div>
            <div className="terminal-highlight">🤝 SETTLER RECONCILIATION BATCH SETTLED ON VERIDAG</div>
            <div className="muted">Tenant: settler-inc | Payout Amount: $2,500,000.00 USDV | Anchor: 0xb534...</div>
            <div className="terminal-success">✓ Atomic Settlement: COMMITTED (State Root: 0xbfc0b0a4... | Zero Variance Guaranteed)</div>
            <br />
            <div><span className="terminal-prompt">$ </span>veridag-cli btc verify-header --header-hex 0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd...</div>
            <div className="muted">Validating Bitcoin block header &amp; target difficulty...</div>
            <div>Block Hash: <span className="terminal-highlight">000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f</span></div>
            <div className="terminal-success">✓ PoW Verification: PASSED (Satisfies target difficulty | Merkle root verified)</div>
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
              Institutional digital dollar collateralized 1:1 by short-term US Treasuries and Fed RRP,
              eliminating foreign exchange settlement risk across multilateral trade corridors.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🤝</span>
            <h3>Settler Reconciliation &amp; Audit OS</h3>
            <p>
              Native on-chain anchor and payout substrate for the Settler reconciliation engine, ensuring
              enterprise proofpack integrity with mathematical zero-variance guarantee.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🌐</span>
            <h3>Universal Multi-Chain Rails</h3>
            <p>
              Trustless cross-chain SPV header tracking, UTXO proof verification, zero-reorg L2 sequencing,
              and native EVM JSON-RPC connectivity across Bitcoin, Ethereum, and major crypto networks.
            </p>
          </div>
          <div className="card">
            <span className="card-icon">🤖</span>
            <h3>Autonomous AI Agent Swarms</h3>
            <p>
              Cryptographically verifiable multi-agent task execution, capability delegation, and tamper-proof
              audit logs for MissionLedger, ReadyLayer, nlsqlc, and mcpwall.
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
                <td><strong>Multi-Chain Light Clients &amp; SPV</strong></td>
                <td><span className="badge-yes">✓ Native Bitcoin SPV + EVM L1 Light Client</span></td>
                <td><span className="badge-partial">~ Single-Chain / Heavy ZK Rollups</span></td>
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
        <h2 style={{ margin: "0 0 12px", justifyContent: "center" }}>Deploy Sovereign Settlement &amp; Multi-Chain Infrastructure</h2>
        <p style={{ maxWidth: "600px", margin: "0 auto 24px", color: "var(--text-muted)" }}>
          Run the full consensus engine, manage USMCA-grade sovereign USDV settlement, anchor Settler reconciliation runs, and verify Bitcoin and Ethereum state in under 3 minutes.
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
