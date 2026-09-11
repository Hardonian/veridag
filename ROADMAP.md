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

## Phase 5 — Validator networking (DONE in this tree)
`veridag-net`: QUIC validator links with self-signed Ed25519 certs and a
domain-separated verifier (no MITM, no plaintext). `gossip` module broadcasts
tagged messages (vertices + batches) over authenticated uni-streams.
`tests/devnet.rs`: four OS-process validators reach consensus over **real
QUIC sockets** — identical state roots and checkpoints across the network.

## Phase 6 — DAG (DONE in this tree)
`veridag-dag`: VCE-1 vertex wire form, domain-separated ids/signatures,
validity rules, equivocation detection, quorum round progression, causal
traversal. 14 unit tests. `Dag::iter_vertex_ids()` added for recovery.

## Phase 7 — Baseline consensus (DONE in this tree)
`veridag-consensus`: StaticCommittee leader schedule, pure-function commit rule
with Shoal-style pipelining, deterministic causal ordering. Validated by
deterministic simulation (`tests/simulation.rs`): n=4/f=1 Agreement +
delivery-order independence.

## Phase 8 — Vertical slice (DONE in this tree)
`tests/vertical_slice.rs`: client tx → batch commitment → DAG vertex →
BaselineDagBft commit → canonical ordering → sequential execution → state root.
Four validators derive identical committed ordering and identical final state;
committed double-spend resolves deterministically.

## Phase 9 — Crash recovery (DONE in this tree)
`veridag-storage`: trait-based `StateStore`/`DagStore`/`CheckpointStore` with
`MemoryStore` and `SledStore` (feature `persistent`). `sled_tests`:
crash-injection harness builds a 4-validator DAG through a committed wave,
persists every vertex + final state to sled, **drops all in-memory state**
(simulated crash), reopens, rebuilds the DAG from persisted bytes in round
order, re-runs the commit rule, and re-executes — asserting identical state
root + balances. The commit → checkpoint boundary is restart-safe.

## Phase 10 — Parallel execution (DONE in this tree)
`veridag-execution`: conflict-aware scheduler partitions the committed ordering
into a non-conflicting parallel prefix + sequential suffix; the sequential
executor is the oracle. `tests/parallel.rs` property-tests
parallel == sequential across randomized workloads.

## Phase 11 — Public P2P Plane (DONE in this tree)
`veridag-net`: selective libp2p discovery (`DiscoveryPolicy::Open`, `DiscoveryPolicy::Allowlist`)
over Floodsub/TCP/Noise/Yamux without altering consensus security semantics. Consensus DAG
strictly isolated on authenticated QUIC mesh; public gossip plane handles ingress and relay.

## Phase 12 — Deterministic Wasm Runtime (DONE in this tree)
`veridag-wasm-runtime`: `ComponentLoader` validates Wasm bytecode and `WasmComponentManifest`,
rejecting non-deterministic imports (WASI/time/random/sockets) and enforcing capability-scoped
host ABI (`host_read`, `host_write`, `host_spend`, `host_epoch`, `host_log`) and deterministic
fuel metering. Native and Wasmtime backends.

## Phase 13 — Developer SDKs (DONE in this tree)
Idiomatic developer SDKs in Rust (`veridag-sdk`), TypeScript (`sdks/typescript`), and Python
(`sdks/python`). All three languages validated for bit-for-bit wire serialization, Ed25519 signing,
and transaction hashing against `protocol/test-vectors/sdk_conformance.json`.

## Phase 14 — Light Client Protocol (DONE in this tree)
`veridag-light-client`: 2f+1 quorum checkpoint verification, continuous epoch state tracking
(`LightClientTracker`), and BMH-1 Merkle object inclusion proofs. Matched on EVM L1 by
`contracts/VeridagLightClient.sol`.

## Phase 15 — Zero-Knowledge Proof Adapters (DONE in this tree)
`veridag-zkvm`: pluggable `ZkvmAdapter` trait with `MockZkvmAdapter`, `Sp1Adapter`, and
`RiscZeroAdapter` behind feature flags. Proving is decoupled from the critical consensus path
and utilized for L1 settlement compression and fast light client sync.

## Phase 16 — Advanced Data Availability (DONE in this tree)
`veridag-da`: 2D Reed-Solomon tensor erasure coding (`Da2DConfig`, `encode_2d`, `reconstruct_2d`)
with alternating row/column iterative recovery under scattered erasures, independent row/col
Merkle commitments, and deterministic validator replication assignment (`ValidatorReplicationScheme`).

## Phase 17 — Hardware Acceleration (DONE in this tree)
`veridag-da::hw_accel`: `HwAccelEngine` with chunk-unrolled SIMD vector XOR, batch GF(2^8)
multiplication, and C/Zig foreign acceleration hooks under strict `#![forbid(unsafe_code)]`.

## Phase 18 — Institutional US Sovereign Stablecoin (USDV) (DONE in this tree)
`veridag-stablecoin`: 100% reserve-backed (US Treasuries, FDIC cash deposits, Reverse
Repo), cryptographically verified Proof-of-Reserves (PoR) in state roots, capability-gated
mint/burn/pause, and real-time OFAC compliance sanctions screening. Normative spec 19.
Mathematical conservation-of-value invariant checked across all transitions.

## Phase 19 — Iron-Clad Ethereum Infrastructure Substrate (DONE in this tree)
`veridag-ethereum`: EVM JSON-RPC provider (`eth_chainId`, `eth_blockNumber`, `eth_getBalance`),
BMH-1 Merkle inclusion proof generator for L1 contracts, and two-way cross-chain bridge
primitives. Production Solidity contracts: `USDV.sol`, `VeridagLightClient.sol`, and
`VeridagBridge.sol`. Normative spec 20.

## Release status

`0.1.0-alpha` — reference implementation compiles clean (`cargo clippy
--workspace --all-targets --all-features -- -D warnings`), all workspace tests
green, release binary builds (`panic = "abort"`, `strip = true`) and the demo
produces identical state roots + checkpoints across 4 validators. See
`docs/quickstart.md` for universal onboarding and `docs/architecture.md` for
the system design.
