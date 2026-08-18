# Veridag go-live runbook (v0.2.0)

## What this is
A deterministic, Byzantine-resilient, capability-secured DAG-BFT execution
substrate. One binary (`veridag-node`) runs the full vertical slice in-process:
mempool → proposal → DAG → BaselineDagBft commit → parallel executor →
checkpoint. Multi-process P2P and persistent crash recovery are wired behind
the same crates (Phase 5 / Phase 9) but this alpha ships the consensus-critical
path in-process so the whole pipeline is exercisable end-to-end.

## Prerequisites
- Rust 1.85+ (workspace `rust-version`).
- No external services required for the alpha: everything is local/in-process.
- Heavy optional backends (libp2p, risc0, wasmtime) are feature-gated and NOT
  compiled in the default build. Do NOT enable them unless you have network
  access and want the optional feature; they are excluded from CI on purpose.

## Build
```bash
cd implementations/rust
cargo build --release
```
Release profile: opt-level 3, thin LTO, codegen-units 1, panic=abort, strip.
Binaries: `target/release/veridag-{node,cli,genesis}`.

## Quality gate (run before any release)
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings   # default features
cargo test --workspace
```
These pass for the default-feature build. The three optional heavy backends
(libp2p/wasmtime/risc0) are feature-gated and intentionally excluded from the
default CI job; each has its own documented feature-gated build path and is
verified only when its dependency is available.

## Health probe (go-live)
```bash
./target/release/veridag-node health        # human-readable summary
./target/release/veridag-node health --json # one JSON object, machine-parseable
```
The health probe is a self-test: it spins up a fresh 4-validator in-process
committee, submits a transfer (alice -> bob, 40), runs enough rounds for a wave
to commit and a checkpoint to fire, then asserts all validators agree on the
state root. Exit 0 = pipeline healthy; exit non-zero = consensus/pipeline broke.
JSON fields: version, protocol_version, chain_id, committee_n/quorum,
highest_complete_wave, max_round, state_root, committed_tx_count,
checkpoint_count, checkpoint_ids, proposal_nonce, epoch, keyset_fingerprint,
agreement.

## Demo (manual smoke test)
```bash
./target/release/veridag-node demo --validators 4
```
Expected: AGREEMENT OK, identical state root across all validators, bob balance
40, one checkpoint. Deterministic — same input always produces the same output.

## Genesis
```bash
./target/release/veridag-genesis   # inspect/verify genesis tool
```
Genesis is deterministic: identical input -> identical commitment
(`VERIDAG_GENESIS_V1`).

## Deterministic keys
All crypto is deterministic / RNG-free on consensus paths:
`Keypair::from_seed(&[n;32])`. Never RNG for consensus-visible derivation.

## Domain separation
Every signature/hash uses an explicit domain tag:
`VERIDAG_TX_V1`, `VERIDAG_VERTEX_V1`, `VERIDAG_BATCH_V1`,
`VERIDAG_CHECKPOINT_V1`, `VERIDAG_DA_BLOB_V1`, `VERIDAG_BMH_LEAF_V1`,
`VERIDAG_BMH_NODE_V1`, `VERIDAG_GENESIS_V1`. Never mix domains.

## Capabilities
Capabilities gate writes and spends. Spend requires a `Capability` with
`authorize_spend` covering the amount + epoch. Write requires a capability
covering the object class / application. Revoked/expired capabilities fail.

## Deployment (alpha)
Single node, single process, no persistence yet (in-memory Dag + ObjectState).
For a persistent devnet you wire `veridag-storage` (sled, feature `persistent`)
behind the same crates; crash recovery requires adding vertices in ROUND order
on reload (see api-quirks.md "Crash recovery ordering").

## Architecture snapshot
- `veridag-protocol-types`: canonical types (Hash=[u8;32], Address, ObjectId,
  ValidatorId, Ed25519PublicKey/Signature, CapabilityId, etc.)
- `veridag-crypto`: BLAKE3 domain-separated hash + Ed25519 sign/verify.
- `veridag-codec`: VCE-1 encoder/decoder (no serde in the hot path).
- `veridag-capabilities`: capability scoping + spend authorization.
- `veridag-transaction`: tx body, nonce (anti-replay), signing, structural check.
- `veridag-object-state`: ObjectState, state_root (BMH-1), balance.
- `veridag-merkle`: BMH-1 Merkle tree, inclusion proofs (leaf binds id||bytes).
- `veridag-dag`: vertex DAG, equivocation book, round progression.
- `veridag-consensus`: BaselineDagBft commit rule (2f+1 quorum, n>=3f+1,
  deterministic, pure function of DAG).
- `veridag-execution`: sequential + parallel executor, transaction root.
- `veridag-checkpoint`: checkpoint + finality proof (distinct-validator quorum,
  cryptographic vote verification, separate vote domain).
- `veridag-da`: Reed-Solomon GF(2^8) erasure coding (encode/reconstruct), hash-
  bound shares.
- `veridag-proof`: ProofSystem trait + NoOpProof (default) + risc0 adapter
  (feature-gated).
- `veridag-wasm-runtime`: HostAbi + NativeEngine (default) + wasmtime WasmEngine
  (feature-gated).
- `veridag-net`: Transport trait + QuicTransport + libp2p (feature-gated).
- `veridag-sdk` + clients: Rust/TS/Python/Go SDKs for the REST contract.
- `veridag-metrics` (go-live): deterministic observability facade, instrumented
  on crypto::hash, merkle leaf/node, da encode/reconstruct (opt-in `metrics`
  feature).
- `veridag-light-client`: verification-only; depends only on crypto/codec/
  merkle/checkpoint/protocol-types.

## Alerts / failure modes
- Health probe exits non-zero -> consensus/pipeline broken. Investigate.
- Checkpoint not advancing -> check quorum (need 2f+1 votes in vote round).
- State root mismatch across validators -> deterministic deviation; check
  domain tags, equivocation, round-ordering on any reload.
- `StaticCommittee` panics on construction if f=0 or n < 3f+1 — that is the
  intended fail-fast; fix the config.

## Optimization seams (documented, not touched in this release)
- crypto::hash (every tx/vertex/batch/checkpoint/DA blob).
- merkle leaf_hash + node_hash.
- da encode/reconstruct (GF multiply inner loop).
- ed25519 verify (batch-verify candidate).
See `references/optimization.md`. No unsafe/SIMD in this release.

## Feature flags that are NOT in CI (read before enabling)
- `veridag-net`: `libp2p` (libp2p 0.54).
- `veridag-wasm-runtime`: `wasmtime` (wasmtime 21).
- `veridag-proof`: `risc0` (risc0-zkvm + bincode).
Each is documented in `references/{libp2p-0.54,wasmtime-21}.md` and the
veridag-engineering skill. Enable only with network access; they are excluded
from the default build and CI on purpose.
