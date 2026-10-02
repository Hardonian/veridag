# Veridag Architecture

This document describes how the **reference implementation** (Rust,
`implementations/rust`) realizes the Veridag protocol. The normative layered
view and the three-level authority model (spec > formal model > implementation)
live in `protocol/specification/00-overview.md`.

Veridag is built for **universal usability**: a developer on a laptop, a team
running validators on commodity cloud VMs, or an embedded operator on a single
board computer should all be able to run the same deterministic core. The
design targets **low latency, low energy, and small binary footprint** without
sacrificing correctness or safety.

## Design priorities (in order)

```
correctness > determinism > security > implementation independence
> modularity > verification > operability > performance > developer usability
```

"Performance" here means *throughput per watt and per dollar*, not peak
benchmark numbers. Every hot-path choice is made to stay predictable and cheap:

* **No `unsafe` in the consensus/execution core.** All crates
  `#![forbid(unsafe_code)]`. Memory-safety bugs cannot reach the BFT core.
* **Deterministic by construction.** No reliance on hash-map iteration order,
  wall-clock time, thread scheduling, floating point, OS randomness, or
  filesystem order. Two nodes with the same inputs produce byte-identical
  state roots.
* **Small, dependency-light stack.** `blake3` (fast, parallel, constant-time),
  `ed25519-dalek` (fast signature verification), `quinn` (QUIC, no userspace
  TCP head-of-line blocking), `sled` (embedded, lock-free, no external
  database process). No Kubernetes, no message broker, no sidecar required.
* **Crash-safe persistence.** State and DAG are append-friendly and
  restart-safe; a validator that dies mid-commit recovers identically (proven
  by the `crash_recovery` test).
* **Release profile tuned for the edge.** `opt-level = 3`, `lto = "thin"`,
  `codegen-units = 1`, `panic = "abort"`, `strip = true` → small, fast
  binaries that fail loudly and restart cleanly.

## Crates (implementations/rust/crates)

| Crate | Responsibility |
|-------|----------------|
| `veridag-protocol-types` | Canonical identifiers, core types, domain tags |
| `veridag-codec` | VCE-1 encoder/decoder (canonical wire form) |
| `veridag-crypto` | BLAKE3 hashing, Ed25519 sign/verify, domain preimages, and a fail-closed `KeySigner` integration boundary; cloud KMS/HSM providers are not implemented |
| `veridag-merkle` | BMH-1 state commitments + inclusion proofs |
| `veridag-transaction` | Transaction model, validation, anti-replay |
| `veridag-capabilities` | Object-capability authorization tokens and enforcement |
| `veridag-object-state` | Object set, version discipline, account/balance |
| `veridag-execution` | Sequential deterministic executor, conflict-aware parallel scheduler, multi-chain asset typing, batch compaction |
| `veridag-dag` | VCE-1 vertex wire form, validity, equivocation, quorum |
| `veridag-consensus` | BaselineDagBft pure-function commit rule, leader schedule, dynamic committee reconfiguration, epoch handovers |
| `veridag-checkpoint` | Quorum finality, checkpoint construction/verification |
| `veridag-storage` | StateStore/DagStore/CheckpointStore traits + Memory + Sled, state snapshotting, archival pruning, fast sync |
| `veridag-net` | QUIC authenticated links + vertex/batch gossip + selective libp2p public discovery plane |
| `veridag-wasm-runtime` | Deterministic Wasmtime sandbox, capability-scoped host ABI, fuel metering |
| `veridag-zkvm` | Pluggable zkVM adapters (Mock, SP1, RiscZero) for state validity proofs |
| `veridag-da` | 2D Reed-Solomon tensor erasure coding, validator replication, SIMD hardware acceleration |
| `veridag-light-client` | 2f+1 quorum checkpoint verification, epoch tracking, BMH-1 Merkle inclusion proofs |
| `veridag-stablecoin` | USDV sovereign dollar, ISO 20022 engine, Proof-of-Reserves, OFAC sanctions compliance |
| `veridag-ethereum` | EVM JSON-RPC provider, BMH-1 inclusion proofs, L1 bridge primitives |
| `veridag-bitcoin` | Bitcoin SPV client, PoW validation, Merkle proofs, UTXO bridge codecs |
| `veridag-metrics` | Zero-overhead observability: Prometheus/OpenMetrics exporter, counter/gauge/histogram telemetry |
| `veridag-sdk` | Native Rust client SDK |
| `veridag-qa` | Property-test harness, adversarial fuzzing, criterion benchmarks |
| `veridag-testkit` | Golden vector generation/validation, malformed suite, cross-language conformance |

### Binaries (`implementations/rust/bins`)

| Binary | Purpose |
|--------|---------|
| `veridag-node` | Full validator daemon (demo mode, health probe, networked QUIC daemon with HTTP/JSON RPC) |
| `veridag-cli` | Key management, dev-ledger execution, USDV institutional management, Ethereum tooling |
| `veridag-genesis` | Deterministic genesis generation, inspection, and commitment verification |

## Data flow

```
client tx
  -> validate (transaction crate)
  -> batch commitment (VCE-1)
  -> DAG vertex (veridag-dag, signed)
  -> gossip over QUIC (veridag-net, validator fast path)
  -> BaselineDagBft commit (veridag-consensus, pure function)
  -> canonical causal ordering
  -> conflict-aware execution (veridag-execution: parallel prefix + sequential suffix)
  -> optional: Wasm smart contract execution (veridag-wasm-runtime, metered)
  -> BMH-1 state root (veridag-merkle)
  -> checkpoint with 2f+1 finality proof (veridag-checkpoint)
  -> persist (veridag-storage: sled)
  -> optional: DA erasure coding (veridag-da)
  -> optional: zkVM state validity proof (veridag-zkvm)
  -> optional: L1 settlement (veridag-ethereum / veridag-bitcoin)
```

Every step is a deterministic function of its inputs. The commit rule
(`veridag-consensus::commit`) is a pure function: given an identical DAG, every
node computes an identical committed anchor and ordering.

## Why QUIC (not raw TCP or libp2p)

* **No head-of-line blocking** within a connection (independent streams).
* **Authenticated from byte 0** via TLS 1.3 with self-signed Ed25519 certs;
  the verifier enforces a domain-separated preimage so a cert minted for one
  purpose cannot be reused elsewhere.
* **Low setup latency**: 1-RTT handshake, connection migration, built-in
  congestion control. Suitable for validators on flaky or mobile links.

libp2p is used *only* for the public discovery/relay plane, strictly isolated
from the consensus-critical QUIC mesh.

## Why sled (not Postgres/Redis)

* **Zero external services.** The database *is* a local file. A validator is a
  single static binary plus a data directory.
* **Append-friendly, crash-safe** by design — matches the DAG's
  never-rewrite-history model.
* **Tiny footprint** → runs on a Raspberry Pi-class node.

## Safety posture

* All crates `#![forbid(unsafe_code)]` by default.
* Attacker-facing parsers are canonical (VCE-1) and fuzz-targeted.
* Every consensus-visible value round-trips through VCE-1.
* Signatures use domain-separated preimages (`VERIDAG_TX_V1`,
  `VERIDAG_VERTEX_V1`, …) so a signature for one purpose cannot be replayed
  for another.
* Crash recovery is test-proven: drop all memory, reopen from disk, rebuild
  the DAG, re-run consensus, re-execute → byte-identical state.
* Wasm runtime is default-deny: non-deterministic imports (WASI time/random/
  sockets) are rejected; host calls require capability handles.

## Capabilities summary

The reference implementation contains artifacts associated with Phases 0–26 of
the protocol roadmap, at capability levels ranging from integrated beta to
interface-only. See `capability-matrix.md` for deployment status, `ROADMAP.md`
for milestone history, and `CHANGELOG.md` for the release history.
