# Veridag Architecture

See `protocol/specification/00-overview.md` for the normative layered view and the
three-level authority model (spec > formal model > implementation).

This document describes how the reference implementation realizes the layers.

## Crates (implementations/rust/crates)

| Crate | Responsibility |
|-------|----------------|
| `veridag-protocol-types` | Canonical identifiers, core types, domain tags |
| `veridag-codec` | VCE-1 encoder/decoder |
| `veridag-crypto` | BLAKE3 hashing, Ed25519 sign/verify, domain preimages |
| `veridag-merkle` | BMH-1 state commitments + inclusion proofs |
| `veridag-transaction` | Transaction model, validation, anti-replay |
| `veridag-capabilities` | Capability objects and enforcement |
| `veridag-object-state` | Object set, version discipline, account/balance |
| `veridag-execution` | Sequential deterministic executor, receipts |
| `veridag-storage` | StateStore/DagStore/CheckpointStore traits + MemoryStore |
| `veridag-testkit` | Vector generation/validation, malformed suite |

Post-v0.1: `veridag-dag`, `veridag-consensus`, `veridag-network-*`,
`veridag-execution-parallel`, `veridag-runtime-*`, `veridag-proof-*`,
`veridag-sdk-*`, `veridag-simulator`.

## Data flow (Phase 8 vertical slice)

```
client -> RPC -> validate -> mempool -> batch -> DAG -> consensus
      -> ordering -> execution -> state -> checkpoint
```

## Safety posture

* All crates `#![forbid(unsafe_code)]` by default.
* Attacker-facing parsers are canonical and fuzz-targeted.
* Every consensus-visible value round-trips through VCE-1.
