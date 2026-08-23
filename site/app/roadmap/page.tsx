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
  { n: "Phase 11", t: "Public P2P Plane", d: "Selective libp2p discovery without altering consensus security semantics.", done: false },
  { n: "Phase 12", t: "Deterministic Wasm Runtime", d: "Wasm component loading, capability-scoped host API, and deterministic fuel metering.", done: false },
  { n: "Phase 13", t: "Developer SDKs", d: "Idiomatic Rust, TypeScript, and Python SDKs with cross-language conformance.", done: false },
  { n: "Phase 14", t: "Light Client Protocol", d: "2f+1 quorum checkpoint verification and Merkle object inclusion proofs.", done: false },
  { n: "Phase 15", t: "Zero-Knowledge Proof Adapters", d: "Pluggable zkVM state validity proofs behind feature flags.", done: false },
  { n: "Phase 16", t: "Advanced Data Availability", d: "Validator-replicated and erasure-coded 2D Reed-Solomon DA schemes.", done: false },
  { n: "Phase 17", t: "Hardware Acceleration", d: "Targeted SIMD/AVX-512, Zig/C fallbacks, and optional CUDA acceleration.", done: false },
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

      <div className="alert">
        <div className="alert-title">🎯 v0.1.0-alpha Milestone Achieved ({progressPercent}% Complete)</div>
        <p style={{ margin: 0, fontSize: "14px", color: "#c7d2e0" }}>
          Phases 0 through 10 are completely implemented, formally verified, and tested end-to-end.
          The reference implementation compiles with zero warnings, zero unsafe code, and achieves 4-validator BFT agreement.
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
