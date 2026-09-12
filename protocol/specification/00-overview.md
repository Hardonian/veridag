# Veridag Protocol Specification — Overview

Status: NORMATIVE v1.0
Protocol version: `1`
Chain of custody: any change to consensus-visible behavior requires a change to this
specification first (see `17-upgrades.md`).

## What Veridag is

Veridag is an **implementation-independent protocol for deterministic,
Byzantine-resilient, capability-secured, verifiable distributed computation**.

It defines, as protocol (Level 1), independently of any implementation (Level 3):

1. Canonical identifiers and encoding for every consensus-visible object.
2. A deterministic state-transition function over an object-centric state.
3. Capability-based authorization, enforced by protocol state.
4. A DAG-based Byzantine fault-tolerant consensus that finalizes ordered
   transaction batches.
5. Deterministic checkpoints binding state, DAG, and validator-set commitments.

Conceptually:

```
consensus
+ verifiable state
+ deterministic computation
+ capability security
+ data availability
+ cryptographic proofs
= distributed trust fabric
```

Money is only one possible application. There is no protocol-mandated token.

## What Veridag is not

* Not primarily a cryptocurrency.
* Not a blockchain clone. It is a DAG-BFT protocol with object state.
* Not a Rust framework. Rust is implementation #1, not the protocol.
* Not dependent on any particular VM, database, proof system, cloud vendor,
  networking framework, or implementation language.

## Three explicit levels

| Level | Artifact | Authority |
|-------|----------|-----------|
| 1 | This normative specification (`protocol/specification/`) | Highest. Defines correctness. |
| 2 | Formal executable model (`formal/quint/`) | Checked model of Level 1 safety invariants. |
| 3 | Reference implementations (`implementations/rust/`, …) | Correct only if it satisfies Levels 1 and 2. |

An implementation is **correct** only if it satisfies Levels 1 and 2. Rust code must
never become an undocumented source of protocol truth. Every consensus-visible
behavioral change requires, in order: (1) specification change, (2) protocol version
assessment, (3) formal model review, (4) test-vector change if applicable,
(5) conformance test, (6) implementation change.

## Non-negotiable determinism rule

No consensus-visible behavior may depend on:

* struct/enum memory layout or compiler version;
* CPU architecture, host endianness;
* thread scheduling, hash-map iteration order;
* filesystem ordering, wall-clock time, OS randomness;
* floating-point behavior;
* database iteration quirks;
* network arrival timing.

Any required randomness must be explicit protocol input (see `10-ordering.md`).

## Design principles (priority order)

```
correctness > determinism > security > implementation independence
> modularity > verification > operability > performance > developer usability
```

## Layered architecture

```
Applications / AI Agents / Services
SDK + Component Interface
Deterministic Wasm Application Runtime      (post-v0.1)
Object + Capability State Model
Deterministic (later Parallel) Execution
Optional Verifiable Execution / zkVM        (post-v0.1)
State Commitments + Checkpoints
DAG-BFT Consensus / Finality
Data Availability
Validator QUIC / Public P2P
Persistent Storage
```

The formal specification sits alongside all layers and feeds conformance vectors to
all implementations.

## Specification map

| File | Scope | Status |
|------|-------|--------|
| 00-overview.md | Protocol architectural overview & invariants | Normative |
| 01-terminology.md | Terms, mathematical notation, RFC 2119 semantics | Normative |
| 02-identifiers.md | Fixed-width canonical identifiers (Blake3/Ed25519) | Normative |
| 03-canonical-encoding.md | VCE-1 canonical codec & malformed rejection rules | Normative |
| 04-cryptography.md | Hash domains, Ed25519 signatures, zero non-determinism | Normative |
| 05-transactions.md | Transaction model, payload boundaries, anti-replay nonces | Normative |
| 06-object-model.md | Object-centric state, class isolation, versioning | Normative |
| 07-capabilities.md | Object capability authorization & delegation rules | Normative |
| 08-dag.md | Vertex DAG structure, causal parents, equivocation defense | Normative |
| 09-consensus.md | BaselineDagBft consensus, wave commits, leader rules | Normative |
| 10-ordering.md | Deterministic batch & transaction causal ordering | Normative |
| 11-execution.md | Sequential & parallel deterministic state execution | Normative |
| 12-state.md | State commitments (BMH-1 Blake3 Merkle-Hash tree) | Normative |
| 13-checkpoints.md | Monotonic checkpoint structure & 2f+1 finality proofs | Normative |
| 14-networking.md | Validator QUIC fast-path & selective libp2p discovery | Normative |
| 15-data-availability.md | 2D Reed-Solomon DA, tensor commitments & reconstruction | Normative |
| 16-validator-membership.md | Dynamic consortium committee & epoch handover | Normative |
| 17-upgrades.md | Checkpoint-activated upgrades & version boundaries | Normative |
| 18-light-clients.md | 2f+1 quorum checkpoint tracking & BMH-1 inclusion proofs | Normative |
| 19-us-stablecoin.md | USMCA sovereign settlement & USDV stablecoin engine | Normative |
| 20-ethereum-infrastructure.md | Ethereum L2 & EVM settlement substrate | Normative |
| 21-bitcoin-infrastructure.md | Bitcoin UTXO & SPV verification substrate | Normative |
| 22-settler-integration.md | Settler cross-border reconciliation & dual-leg atomic swap | Normative |

Sections marked NORMATIVE use RFC 2119 language (MUST/SHOULD/MAY).

