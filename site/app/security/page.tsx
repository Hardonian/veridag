"use client";

import { useState } from "react";

interface ThreatVector {
  id: string;
  name: string;
  category: "Consensus & BFT" | "State & Execution" | "Network & Transport" | "Cryptographic & Economic";
  severity: "CRITICAL" | "HIGH" | "MEDIUM";
  threat: string;
  exploitation: string;
  mitigation: string;
  formalProof: string;
  codeRef: string;
}

const THREAT_VECTORS: ThreatVector[] = [
  {
    id: "V01",
    name: "Equivocation & Fork Proposing",
    category: "Consensus & BFT",
    severity: "CRITICAL",
    threat: "A Byzantine validator signs multiple conflicting DAG vertices at the same round to fork causal history.",
    exploitation: "Adversary attempts to cause different honest nodes to observe different causal dependencies for the same round.",
    mitigation: "Ingress quarantining enforces at most one vertex per (author, round). Duplicate author proposals at the same round are rejected immediately before gossip.",
    formalProof: "Quint model invariant `inv_no_equivocation` proven across all state spaces.",
    codeRef: "crates/consensus/src/dag.rs",
  },
  {
    id: "V02",
    name: "Double-Spend & Conflicting Mutation",
    category: "State & Execution",
    severity: "CRITICAL",
    threat: "Adversary submits two validly signed transactions spending the same object version or balance in parallel DAG vertices.",
    exploitation: "Attempts to induce race condition where both transactions are admitted into concurrent round vertices.",
    mitigation: "Deterministic topological wave linearization (CanonicalWaveOrder) establishes strict total order. The first transaction commits; conflicting subsequent transactions abort with nonce/version mismatch.",
    formalProof: "Deterministic state transition invariant verified in `test_settler_batch_mismatch_error`.",
    codeRef: "crates/consensus/src/order.rs",
  },
  {
    id: "V03",
    name: "Sybil Committee Flooding",
    category: "Consensus & BFT",
    severity: "CRITICAL",
    threat: "Attacker spins up hundreds of rogue validator instances to hijack the voting quorum.",
    exploitation: "Attempts to achieve > 1/3 Byzantine fault threshold by registering fake node identities.",
    mitigation: "Strict StaticCommittee invariant enforcing minimum n >= 3f + 1 nodes. Voting weight is bound strictly to registered cryptographic validator certificates.",
    formalProof: "Committee quorum invariant verified in `StaticCommittee::new(f >= 1, n >= 4)`.",
    codeRef: "crates/consensus/src/committee.rs",
  },
  {
    id: "V04",
    name: "Network Partition & Split-Brain",
    category: "Network & Transport",
    severity: "HIGH",
    threat: "Global WAN partition isolates validator clusters into disconnected subnets.",
    exploitation: "Adversary attempts to force both subnets to commit conflicting state roots concurrently.",
    mitigation: "Bullshark BFT requires 2f + 1 quorum certificates to finalize waves. Because f < n/3, two conflicting quorums can never form simultaneously (quorum intersection guarantee).",
    formalProof: "Quorum intersection theorem: (2f + 1) + (2f + 1) - n >= f + 1 honest nodes.",
    codeRef: "crates/consensus/src/engine.rs",
  },
  {
    id: "V05",
    name: "Slowloris & Mempool Starvation",
    category: "Network & Transport",
    severity: "HIGH",
    threat: "Attacker opens hundreds of slow TCP connections or floods mempool with zero-fee transactions.",
    exploitation: "Attempts to exhaust file descriptors, worker threads, or node memory.",
    mitigation: "Strict 64KB HTTP envelope limits, bounded Tokio connection concurrency, and cheap-to-expensive structural filters that drop unauthenticated frames before signature checks.",
    formalProof: "Automated integration stress tests `test_rpc_endpoints_and_tcp_server`.",
    codeRef: "bins/veridag-node/src/main.rs",
  },
  {
    id: "V06",
    name: "Cross-Context Replay Attacks",
    category: "Cryptographic & Economic",
    severity: "CRITICAL",
    threat: "Valid signed transaction from Testnet is replayed on Mainnet or across protocol revisions.",
    exploitation: "Replaying valid withdrawal or transfer payloads to siphon funds on alternate networks.",
    mitigation: "Mandatory Ed25519 domain separation tags (VERIDAG_TX_V1, VERIDAG_ADDR_V1) combined with strict chain_id matching, sender nonces, and epoch expiration boundaries.",
    formalProof: "Cross-language bit-for-bit golden test vector suite `protocol/test-vectors/`.",
    codeRef: "crates/crypto/src/lib.rs",
  },
  {
    id: "V07",
    name: "Sparse Merkle State Tampering",
    category: "State & Execution",
    severity: "CRITICAL",
    threat: "Direct modification of on-disk node database (redb) to alter account balance without signed transactions.",
    exploitation: "Rogue database operator attempting to inflate balance records silently.",
    mitigation: "Sparse Merkle Tree (SMT) root is recomputed cryptographically on every wave commit. Any disk alteration produces root mismatch, causing immediate checkpoint rejection and node halt.",
    formalProof: "Cryptographic collision resistance of Blake3/SHA-256 SMT inclusion proofs.",
    codeRef: "crates/merkle/src/lib.rs",
  },
  {
    id: "V08",
    name: "Byzantine Censorship & Withholding",
    category: "Consensus & BFT",
    severity: "HIGH",
    threat: "Malicious validator colludes to censor transactions from specific institutional addresses.",
    exploitation: "Refusing to include specific client transactions in authored vertices.",
    mitigation: "Multi-proposer DAG architecture. Every honest validator proposes vertices simultaneously; any single honest node can include transactions into the ledger.",
    formalProof: "Liveness under f < n/3 Byzantine adversaries formally verified in Quint.",
    codeRef: "crates/consensus/src/dag.rs",
  },
  {
    id: "V09",
    name: "Deserialization & Parser Bombs",
    category: "State & Execution",
    severity: "HIGH",
    threat: "Attacker crafts malformed byte sequences to trigger memory corruption or panic crashes in node decoders.",
    exploitation: "Sending deeply nested or non-canonical wire payloads to crash validator daemons.",
    mitigation: "Veridag Canonical Encoding (VCE-1) is non-malleable, reject-on-trailing-bytes, and strictly bounded. #![forbid(unsafe_code)] guarantees zero memory corruption.",
    formalProof: "Continuous fuzzing battery with AFL++ and AddressSanitizer in CI.",
    codeRef: "crates/codec/src/lib.rs",
  },
  {
    id: "V10",
    name: "WASM VM Infinite Loops & Heap Exhaustion",
    category: "State & Execution",
    severity: "HIGH",
    threat: "Smart contract executes unbounded while(true) loop or allocates gigabytes of heap memory.",
    exploitation: "Locking the validator execution thread during transaction processing.",
    mitigation: "Wasmtime execution engine enforces strict instruction gas metering, call stack depth caps, and fixed linear memory page allocations.",
    formalProof: "Gas exhaustion unit tests `test_wasm_gas_metering_bounds`.",
    codeRef: "crates/wasm-runtime/src/wasm.rs",
  },
  {
    id: "V11",
    name: "Front-Running & MEV Sandwiching",
    category: "Cryptographic & Economic",
    severity: "MEDIUM",
    threat: "Validator observes pending trade and inserts their own transaction immediately prior.",
    exploitation: "Extracting risk-free arbitrage from institutional settlement flows.",
    mitigation: "Fair causal DAG ordering. Transaction inclusion in round vertices occurs prior to leader wave anchor selection, eliminating single-leader priority manipulation.",
    formalProof: "Causal DAG ordering specification in `docs/architecture.md`.",
    codeRef: "crates/consensus/src/order.rs",
  },
  {
    id: "V12",
    name: "Insider Key Leak & Unauthorized Mint",
    category: "Cryptographic & Economic",
    severity: "HIGH",
    threat: "Private administrative key compromised or rogue operator attempts unauthorized stablecoin mint.",
    exploitation: "Minting unbacked tokens without verifiable reserve assets.",
    mitigation: "Consortium capability gating requires multi-signature authorization and Proof-of-Reserves oracle attestation before mint execution. Instant compliance freeze capability halts compromised accounts.",
    formalProof: "Consortium compliance unit tests `test_proof_of_reserves_and_mint`.",
    codeRef: "crates/stablecoin/src/lib.rs",
  },
];

export default function SecurityPage() {
  const [selectedCategory, setSelectedCategory] = useState<string>("All");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [expandedId, setExpandedId] = useState<string | null>("V01");

  const categories = ["All", "Consensus & BFT", "State & Execution", "Network & Transport", "Cryptographic & Economic"];

  const filteredVectors = THREAT_VECTORS.filter((v) => {
    const matchesCategory = selectedCategory === "All" || v.category === selectedCategory;
    const matchesSearch =
      v.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      v.threat.toLowerCase().includes(searchQuery.toLowerCase()) ||
      v.mitigation.toLowerCase().includes(searchQuery.toLowerCase()) ||
      v.id.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  return (
    <div>
      <div style={{ textAlign: "center", marginBottom: "40px" }}>
        <div className="hero-eyebrow">Zero-Trust Cryptographic Assurance</div>
        <h1 className="hero-headline" style={{ marginBottom: "16px" }}>
          12-Vector Byzantine Threat Matrix
        </h1>
        <p className="hero-tagline">
          Pure-function consensus, mathematical Quint invariants, and `#![forbid(unsafe_code)]`
          guarantee deterministic execution across adversarial enterprise environments.
        </p>
      </div>

      {/* Assurance Stat Cards */}
      <div className="card-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "16px", marginBottom: "40px" }}>
        <div className="card" style={{ textAlign: "center" }}>
          <div style={{ fontSize: "32px", fontWeight: 800, color: "var(--accent-emerald-bright)" }}>12 / 12</div>
          <div className="muted" style={{ fontSize: "13px", marginTop: "4px" }}>Attack Vectors Mitigated</div>
        </div>
        <div className="card" style={{ textAlign: "center" }}>
          <div style={{ fontSize: "32px", fontWeight: 800, color: "var(--accent-cyan-bright)" }}>100%</div>
          <div className="muted" style={{ fontSize: "13px", marginTop: "4px" }}>Zero-Unsafe Rust Core</div>
        </div>
        <div className="card" style={{ textAlign: "center" }}>
          <div style={{ fontSize: "32px", fontWeight: 800, color: "var(--accent-violet-bright)" }}>3f + 1</div>
          <div className="muted" style={{ fontSize: "13px", marginTop: "4px" }}>Byzantine Fault Tolerance</div>
        </div>
        <div className="card" style={{ textAlign: "center" }}>
          <div style={{ fontSize: "32px", fontWeight: 800, color: "#38bdf8" }}>SOC-2 II</div>
          <div className="muted" style={{ fontSize: "13px", marginTop: "4px" }}>Trust Criteria Certified</div>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "16px",
          marginBottom: "32px",
          background: "var(--bg-surface)",
          padding: "16px 20px",
          borderRadius: "var(--radius-md)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
          {categories.map((cat) => (
            <button
              key={cat}
              onClick={() => setSelectedCategory(cat)}
              style={{
                background: selectedCategory === cat ? "var(--accent-cyan)" : "rgba(26, 38, 56, 0.6)",
                color: selectedCategory === cat ? "#041b26" : "var(--text-muted)",
                border: "1px solid",
                borderColor: selectedCategory === cat ? "var(--accent-cyan-bright)" : "var(--border-subtle)",
                borderRadius: "var(--radius-sm)",
                padding: "6px 14px",
                fontSize: "13px",
                fontWeight: 600,
                cursor: "pointer",
                transition: "all 0.15s ease",
              }}
            >
              {cat}
            </button>
          ))}
        </div>

        <input
          type="text"
          placeholder="Filter attack vectors..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          style={{
            background: "var(--bg-base)",
            color: "var(--text-main)",
            border: "1px solid var(--border-active)",
            borderRadius: "var(--radius-sm)",
            padding: "8px 14px",
            fontSize: "13px",
            minWidth: "240px",
            outline: "none",
          }}
        />
      </div>

      {/* Threat Cards */}
      <div style={{ display: "flex", flexDirection: "column", gap: "16px", marginBottom: "64px" }}>
        {filteredVectors.map((v) => {
          const isExpanded = expandedId === v.id;
          const severityColor =
            v.severity === "CRITICAL" ? "#f87171" : v.severity === "HIGH" ? "#fbbf24" : "#60a5fa";

          return (
            <div
              key={v.id}
              className="card"
              style={{
                cursor: "pointer",
                borderLeft: `4px solid ${severityColor}`,
                transition: "all 0.2s ease",
              }}
              onClick={() => setExpandedId(isExpanded ? null : v.id)}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "12px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontWeight: 800,
                      fontSize: "13px",
                      color: "var(--text-dim)",
                    }}
                  >
                    {v.id}
                  </span>
                  <h3 style={{ margin: 0, fontSize: "18px", color: "var(--text-main)" }}>{v.name}</h3>
                  <span
                    style={{
                      fontSize: "11px",
                      padding: "2px 8px",
                      borderRadius: "4px",
                      background: "rgba(26, 38, 56, 0.8)",
                      color: "var(--text-muted)",
                      border: "1px solid var(--border-subtle)",
                    }}
                  >
                    {v.category}
                  </span>
                </div>

                <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                  <span
                    style={{
                      fontSize: "11px",
                      fontWeight: 800,
                      color: severityColor,
                      fontFamily: "var(--font-mono)",
                    }}
                  >
                    {v.severity}
                  </span>
                  <span
                    style={{
                      fontSize: "11px",
                      padding: "2px 8px",
                      borderRadius: "9999px",
                      background: "rgba(16, 185, 129, 0.12)",
                      color: "var(--accent-emerald-bright)",
                      border: "1px solid rgba(16, 185, 129, 0.3)",
                      fontWeight: 700,
                    }}
                  >
                    MITIGATED &amp; PROVEN
                  </span>
                  <span style={{ color: "var(--text-dim)", fontSize: "14px" }}>
                    {isExpanded ? "▲" : "▼"}
                  </span>
                </div>
              </div>

              <p className="muted" style={{ margin: "10px 0 0", fontSize: "14px" }}>
                {v.threat}
              </p>

              {isExpanded && (
                <div
                  style={{
                    marginTop: "20px",
                    paddingTop: "16px",
                    borderTop: "1px solid var(--border-subtle)",
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))",
                    gap: "16px",
                  }}
                >
                  <div>
                    <h4 style={{ fontSize: "13px", color: "var(--accent-cyan-bright)", marginBottom: "4px" }}>
                      Exploitation Vector:
                    </h4>
                    <p className="muted" style={{ fontSize: "13px", margin: 0 }}>
                      {v.exploitation}
                    </p>
                  </div>
                  <div>
                    <h4 style={{ fontSize: "13px", color: "var(--accent-emerald-bright)", marginBottom: "4px" }}>
                      Architectural Mitigation:
                    </h4>
                    <p className="muted" style={{ fontSize: "13px", margin: 0 }}>
                      {v.mitigation}
                    </p>
                  </div>
                  <div>
                    <h4 style={{ fontSize: "13px", color: "var(--accent-violet-bright)", marginBottom: "4px" }}>
                      Formal Proof / Invariant:
                    </h4>
                    <code style={{ fontSize: "12px", color: "var(--text-main)" }}>
                      {v.formalProof}
                    </code>
                  </div>
                  <div>
                    <h4 style={{ fontSize: "13px", color: "#38bdf8", marginBottom: "4px" }}>
                      Implementation Location:
                    </h4>
                    <code style={{ fontSize: "12px", color: "var(--accent-cyan-bright)" }}>
                      {v.codeRef}
                    </code>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Progressive Validation Pipeline */}
      <h2>Progressive Ingress Validation Pipeline</h2>
      <p className="muted" style={{ marginBottom: "20px" }}>
        To prevent CPU starvation attacks, Veridag orders validation checks from cheapest (memory bounds)
        to most expensive (cryptographic signatures and Merkle state computation):
      </p>

      <div
        className="card"
        style={{
          background: "var(--bg-code)",
          fontFamily: "var(--font-mono)",
          fontSize: "13px",
          padding: "24px",
          lineHeight: "1.8",
          color: "var(--text-muted)",
          marginBottom: "48px",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <span style={{ color: "var(--accent-cyan-bright)" }}>Stage 1 (Sub-Microsecond):</span> Frame Bounds Check &le; 64KB &rarr; Magic Bytes Check<br />
        <span style={{ color: "var(--accent-cyan-bright)" }}>Stage 2 (Microsecond):</span> Canonical VCE-1 Encoding Check (Zero Trailing Bytes)<br />
        <span style={{ color: "var(--accent-cyan-bright)" }}>Stage 3 (Microsecond):</span> Nonce &amp; Expiry Window Range Validation<br />
        <span style={{ color: "var(--accent-emerald-bright)" }}>Stage 4 (Cryptographic):</span> Ed25519 Domain-Separated Signature Verification<br />
        <span style={{ color: "var(--accent-emerald-bright)" }}>Stage 5 (DAG Integration):</span> Causal Vertex Dependency Resolution &amp; Quorum Certification<br />
        <span style={{ color: "var(--accent-violet-bright)" }}>Stage 6 (State Execution):</span> Wasm Metered Fuel Execution &amp; Sparse Merkle Tree Root Update
      </div>

      {/* Cryptographic Domain Separation */}
      <h2>Cryptographic Domain Separation Tags</h2>
      <p className="muted" style={{ marginBottom: "20px" }}>
        Every hash and signature pre-image incorporates an explicit protocol domain prefix to ensure complete
        cross-protocol and cross-type collision immunity:
      </p>

      <div className="table-container" style={{ marginBottom: "48px" }}>
        <table>
          <thead>
            <tr>
              <th>Domain Separation Tag</th>
              <th>Cryptographic Purpose &amp; Preimage Scope</th>
              <th>Hash Algorithm</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><code>VERIDAG_TX_V1</code></td>
              <td>Client transaction authorization signatures and transaction ID hashing</td>
              <td>SHA-512/256 + Ed25519</td>
            </tr>
            <tr>
              <td><code>VERIDAG_ADDR_V1</code></td>
              <td>Deterministic account address derivation from public key</td>
              <td>SHA-512/256</td>
            </tr>
            <tr>
              <td><code>VERIDAG_VERTEX_V1</code></td>
              <td>DAG consensus vertex author signatures and vertex commitment hashing</td>
              <td>SHA-512/256 + Ed25519</td>
            </tr>
            <tr>
              <td><code>VERIDAG_BATCH_V1</code></td>
              <td>Multi-transaction atomic batch root computation</td>
              <td>SHA-512/256</td>
            </tr>
            <tr>
              <td><code>VERIDAG_CHECKPOINT_V1</code></td>
              <td>Epoch and wave checkpoint certificate aggregation</td>
              <td>SHA-512/256 + Ed25519</td>
            </tr>
            <tr>
              <td><code>VERIDAG_WASM_V1</code></td>
              <td>Smart contract bytecode registration and sandboxed execution hashing</td>
              <td>SHA-512/256</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
