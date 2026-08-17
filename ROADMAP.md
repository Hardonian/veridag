# Roadmap

The protocol is built phase by phase, always keeping the tree green. We do not
claim a phase is done until its Definition of Done is met with real artifacts.

## Phase 0 — Specification skeleton (DONE in this tree)
Normative spec 00–13, scoped drafts 14–18. Terminology, identifiers, VCE-1
canonical encoding, transactions, object model, capabilities, DAG, BaselineDagBft,
ordering, execution, state (BMH-1), checkpoints, membership, upgrades.

## Phase 1 — Formal consensus model (DONE in this tree)
`formal/quint/consensus.qnt` + `invariants.qnt`: validators, rounds, vertices,
parents, equivocation, quorum commit rule, Agreement/Finality/Integrity
invariants. Checked with `quint typecheck` and `quint run --invariant`.

## Phase 2 — Protocol vectors (DONE in this tree)
`protocol/test-vectors/`: encoding, hash, signature, transaction, genesis golden
vectors. `conformance/malformed/`: must-reject byte strings. Generated and
re-validated by the Rust workspace.

## Phase 3 — Rust protocol foundation (DONE in this tree)
Crates: `veridag-protocol-types`, `veridag-codec`, `veridag-crypto`,
`veridag-merkle`, `veridag-transaction`, `veridag-capabilities`,
`veridag-object-state`, `veridag-storage`. All `#![forbid(unsafe_code)]`, all
pass golden + malformed vectors.

## Phase 4 — Sequential state machine (DONE in this tree)
`veridag-execution`: native transfer, create/update/delete, capabilities,
resource accounting, state roots, receipts. Deterministic state tests.

## Phase 5 — Validator networking (NEXT)
QUIC validator links, authentication, bounded framing, peer management, reconnect,
backpressure.

## Phase 6 — DAG
Vertices, parent validation, round progression, persistence, propagation.

## Phase 7 — Baseline consensus
Connect implementation to the formal model; validate Agreement/Finality via
deterministic simulation.

## Phase 8 — Vertical slice
client tx → RPC → validation → mempool → batch → DAG → consensus → ordering →
execution → state → checkpoint. Four-validator transfer end-to-end.

## Phase 9 — Crash recovery
Persistent store; crash injection across commit path; restart-safe verification.

## Phase 10 — Parallel execution
Conflict-aware scheduler; sequential executor as oracle; parallel==sequential
property-tested.

## Phase 11 — Public P2P
Selective libp2p; no change to consensus semantics.

## Phase 12 — Deterministic Wasm runtime
Component loading, capability-scoped host API, resource metering, determinism.

## Phase 13 — SDKs
Rust then TypeScript; shared conformance vectors.

## Phase 14 — Light client
Checkpoint verification + object proofs.

## Phase 15 — Proof adapters
One zkVM behind feature flags; proving never required for ordinary consensus.

## Phase 16 — Advanced DA
Validator-replicated, then erasure-coded DA experiments.

## Phase 17 — Optimization
Profile first. Zig/C/CUDA only where evidence justifies, always behind a safe
portable fallback.

See `docs/adr/` for the decisions behind these phases.
