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
| `veridag-crypto` | BLAKE3 hashing, Ed25519 sign/verify, domain preimages |
| `veridag-merkle` | BMH-1 state commitments + inclusion proofs |
| `veridag-transaction` | Transaction model, validation, anti-replay |
| `veridag-capabilities` | Capability objects and enforcement |
| `veridag-object-state` | Object set, version discipline, account/balance |
| `veridag-execution` | Sequential deterministic executor + parallel scheduler |
| `veridag-dag` | VCE-1 vertex wire form, validity, equivocation, quorum |
| `veridag-consensus` | BaselineDagBft: pure-function commit rule + leader schedule |
| `veridag-checkpoint` | Quorum finality, checkpoint construction/verification |
| `veridag-storage` | StateStore/DagStore/CheckpointStore traits + Memory + Sled |
| `veridag-net` | QUIC authenticated links + vertex/batch gossip |
| `veridag-testkit` | Vector generation/validation, malformed suite |

### Binaries (`implementations/rust/bins`)

| Binary | Purpose |
|--------|---------|
| `veridag-node` | Reference validator node (in-process demo + devnet entrypoint) |
| `veridag-cli` | Key management, ledger inspection, dev tooling |
| `veridag-genesis` | Genesis state generation |

## Data flow

```
client tx
  -> validate (transaction crate)
  -> batch commitment (VCE-1)
  -> DAG vertex (veridag-dag, signed)
  -> gossip over QUIC (veridag-net)
  -> BaselineDagBft commit (veridag-consensus, pure function)
  -> canonical causal ordering
  -> conflict-aware execution (veridag-execution: parallel prefix + sequential suffix)
  -> BMH-1 state root (veridag-merkle)
  -> checkpoint (veridag-checkpoint)
  -> persist (veridag-storage: sled)
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

## What is NOT in 0.1.0-alpha (see ROADMAP.md)

Public libp2p P2P, the deterministic Wasm runtime, TypeScript/Python/Go SDKs,
light-client proofs, and zk proof adapters are explicitly deferred. The core
consensus + execution + persistence + networking slice is complete and tested.
