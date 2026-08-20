const phases = [
  { n: "Phase 0", t: "Specification skeleton", d: "Normative spec 00–13, scoped drafts 14–18.", done: true },
  { n: "Phase 1", t: "Formal consensus model", d: "Quint model + invariants (Agreement/Finality/Integrity).", done: true },
  { n: "Phase 2", t: "Protocol vectors", d: "Encoding/hash/sig/tx golden vectors + malformed must-reject.", done: true },
  { n: "Phase 3", t: "Rust protocol foundation", d: "8 core crates, all forbid(unsafe_code), all pass vectors.", done: true },
  { n: "Phase 4", t: "Sequential state machine", d: "Native transfer, capabilities, state roots, receipts.", done: true },
  { n: "Phase 5", t: "Validator networking", d: "QUIC links, self-signed Ed25519, authenticated gossip. 4-validator devnet over real sockets.", done: true },
  { n: "Phase 6", t: "DAG", d: "VCE-1 vertex wire form, equivocation detection, quorum progression.", done: true },
  { n: "Phase 7", t: "Baseline consensus", d: "StaticCommittee schedule, pure-function commit rule, deterministic ordering.", done: true },
  { n: "Phase 8", t: "Vertical slice", d: "tx → batch → vertex → commit → ordering → execution → state root. Identical across 4 validators.", done: true },
  { n: "Phase 9", t: "Crash recovery", d: "SledStore persistence; simulated crash → reopen → identical state root + balances.", done: true },
  { n: "Phase 10", t: "Parallel execution", d: "Conflict-aware scheduler; parallel == sequential (property-tested).", done: true },
  { n: "Phase 11", t: "Public P2P", d: "Selective libp2p; no change to consensus semantics.", done: false },
  { n: "Phase 12", t: "Deterministic Wasm runtime", d: "Component loading, capability-scoped host API, metering.", done: false },
  { n: "Phase 13", t: "SDKs", d: "Rust then TypeScript; shared conformance vectors.", done: false },
  { n: "Phase 14", t: "Light client", d: "Checkpoint verification + object proofs.", done: false },
  { n: "Phase 15", t: "Proof adapters", d: "One zkVM behind feature flags; proving never required for consensus.", done: false },
  { n: "Phase 16", t: "Advanced DA", d: "Validator-replicated, then erasure-coded DA experiments.", done: false },
  { n: "Phase 17", t: "Optimization", d: "Profile first. Zig/C/CUDA only where evidence justifies.", done: false },
];

export default function Roadmap() {
  return (
    <div>
      <h1>Roadmap</h1>
      <p className="tagline">
        Built phase by phase, always keeping the tree green. A phase is not "done"
        until its Definition of Done is met with real artifacts.
      </p>
      <p>
        <strong className="status-done">Phases 0–10 complete</strong> — the reference
        implementation compiles clean, all workspace tests green, and the release
        binary produces identical state roots + checkpoints across 4 validators.
      </p>
      {phases.map((p) => (
        <div className="phase" key={p.n}>
          <span className="p">{p.n}</span> — <strong>{p.t}</strong>{" "}
          {p.done ? <span className="status-done">[DONE]</span> : <span className="muted">[planned]</span>}
          <div className="muted">{p.d}</div>
        </div>
      ))}
    </div>
  );
}
