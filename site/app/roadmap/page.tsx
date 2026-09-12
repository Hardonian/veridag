const phases = [
  { n: "Phase 0", t: "Specification Skeleton", d: "Normative spec 00–13, scoped drafts 14–18. VCE-1 encoding, object model, capability security.", done: true },
  { n: "Phase 1", t: "Formal Consensus Model", d: "Quint model + invariants (Agreement, Finality, Integrity) machine-checked.", done: true },
  { n: "Phase 2", t: "Protocol Vectors", d: "Encoding, hash, signature, and transaction golden vectors + malformed reject suite.", done: true },
  { n: "Phase 3", t: "Rust Protocol Foundation", d: "8 core foundation crates, all forbid(unsafe_code), all passing golden test vectors.", done: true },
  { n: "Phase 4", t: "Sequential State Machine", d: "Native transfer, capability enforcement, deterministic state roots, and receipts.", done: true },
  { n: "Phase 5", t: "Validator Networking (QUIC)", d: "Authenticated QUIC links, self-signed TLS 1.3 certs, and 4-validator multi-process devnet.", done: true },
  { n: "Phase 6", t: "Directed Acyclic Graph (DAG)", d: "VCE-1 vertex wire form, equivocation detection, and round quorum progression.", done: true },
  { n: "Phase 7", t: "Baseline Consensus (DAG-BFT)", d: "StaticCommittee leader schedule, pure-function commit rule, and deterministic causal ordering.", done: true },
  { n: "Phase 8", t: "Vertical Slice Integration", d: "End-to-end pipeline from client tx to checkpoint; identical state roots across all validators.", done: true },
  { n: "Phase 9", t: "Crash Recovery & Durability", d: "SledStore persistence; crash simulation with full memory drop and exact state replay.", done: true },
  { n: "Phase 10", t: "Parallel Execution", d: "Conflict-aware parallel prefix scheduler property-tested against sequential oracle.", done: true },
  { n: "Phase 11", t: "Public P2P Plane", d: "Selective libp2p discovery over Floodsub/Noise/Yamux without altering consensus security semantics.", done: true },
  { n: "Phase 12", t: "Deterministic Wasm Runtime", d: "Wasm component loading, capability-scoped host API, and deterministic fuel metering.", done: true },
  { n: "Phase 13", t: "Developer SDKs", d: "Idiomatic Rust, TypeScript, and Python SDKs with bit-for-bit cross-language conformance.", done: true },
  { n: "Phase 14", t: "Light Client Protocol", d: "2f+1 quorum checkpoint verification, continuous state tracking, and Merkle object inclusion proofs.", done: true },
  { n: "Phase 15", t: "Zero-Knowledge Proof Adapters", d: "Pluggable zkVM state validity proofs (Mock, SP1, RiscZero) decoupled from critical consensus path.", done: true },
  { n: "Phase 16", t: "Advanced Data Availability", d: "Validator-replicated and erasure-coded 2D Reed-Solomon DA schemes with iterative row/column recovery.", done: true },
  { n: "Phase 17", t: "Hardware Acceleration", d: "Targeted SIMD vector XOR, batch Galois Field multiplication, and foreign acceleration hooks.", done: true },
  { n: "Phase 18", t: "USMCA & G8 Multilateral Settlement (USDV)", d: "100% US Treasury-backed sovereign digital dollar, cryptographic Proof-of-Reserves, and OFAC compliance.", done: true },
  { n: "Phase 19", t: "Iron-Clad Ethereum Infrastructure", d: "High-throughput EVM sequencer, JSON-RPC provider, Solidity light client, and trustless bridge contracts.", done: true },
  { n: "Phase 20", t: "Dynamic Validator Membership", d: "Weighted dynamic committee reconfiguration, stake threshold calculation (2W/3 + 1), and seamless epoch handovers.", done: true },
  { n: "Phase 21", t: "Enterprise Prometheus Observability", d: "OpenMetrics/Prometheus exposition exporter providing sorted metrics, DAG commit rates, and validator TPS telemetry.", done: true },
  { n: "Phase 22", t: "State Archival & Fast Snapshot Sync", d: "Historical pruning policies, cryptographic state snapshots, and fast sync protocol for rapid node onboarding.", done: true },
  { n: "Phase 23", t: "Enterprise Cloud KMS & HSM Signer", d: "Pluggable KeySigner abstraction with support for AWS KMS, GCP Cloud KMS, Azure Key Vault, and PKCS#11 HSMs.", done: true },
  { n: "Phase 24", t: "Multi-Chain Asset & Batch Compactor", d: "Native multi-asset typing (USDV, BTC, ETH, SOL), disjoint dependency partitioning, and parallel batch compaction.", done: true },
  { n: "Phase 25", t: "Bitcoin Native Substrate & SPV", d: "80-byte header parser, compact nBits PoW target validation, Double-SHA256, Merkle proofs, and UTXO bridge codecs.", done: true },
  { n: "Phase 26", t: "Hardonian Stack & Settler Native Layer", d: "Deep integration with Settler reconciliation engine (EvidenceManifest anchors), MissionLedger, ReadyLayer, nlsqlc, and mcpwall.", done: true },
];

export default function Roadmap() {
  const completed = phases.filter((p) => p.done).length;
  const progressPercent = Math.round((completed / phases.length) * 100);

  return (
    <div>
      <h1>Phased Roadmap</h1>
      <p className="tagline">
        Engineered phase by phase, always maintaining a green build. A phase is not claimed &quot;done&quot;
        until its Definition of Done is fully verified with executable artifacts and tests.
      </p>

      <div className="alert" style={{ border: "1px solid rgba(52, 211, 153, 0.4)", background: "linear-gradient(180deg, rgba(16, 185, 129, 0.1) 0%, rgba(10, 16, 26, 0.8) 100%)" }}>
        <div className="alert-title" style={{ color: "var(--accent-emerald-bright)" }}>🎯 Complete Enterprise Roadmap Delivered ({progressPercent}% Complete — {completed}/{phases.length} Phases)</div>
        <p style={{ margin: 0, fontSize: "14px", color: "#c7d2e0" }}>
          Phases 0 through 26 are completely implemented, formally verified, and tested end-to-end across the Rust workspace.
          The reference implementation compiles with zero warnings, strictly forbids unsafe code, and provides native settlement
          infrastructure for USMCA/G8 sovereign trade (USDV), Ethereum L1/L2, Bitcoin SPV, and the Hardonian Sovereign AI Stack (Settler, MissionLedger, ReadyLayer).
        </p>
      </div>

      <h2>Phase Breakdown</h2>
      <div>
        {phases.map((p) => (
          <div className="phase-card" key={p.n}>
            <div className="phase-header">
              <div>
                <span className="phase-num">{p.n}</span> — <span className="phase-title">{p.t}</span>
              </div>
              {p.done ? (
                <span className="brand-badge">✓ COMPLETED</span>
              ) : (
                <span style={{ fontSize: "12px", color: "var(--text-dim)", fontFamily: "var(--font-mono)" }}>PLANNED</span>
              )}
            </div>
            <p style={{ margin: 0, fontSize: "13.5px", color: "var(--text-muted)" }}>{p.d}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
