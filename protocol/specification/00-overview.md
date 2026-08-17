# Veridag Protocol Specification — Overview

Status: NORMATIVE DRAFT v0.1.0
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

| File | Scope | v0.1 status |
|------|-------|-------------|
| 00-overview.md | This document | Normative |
| 01-terminology.md | Terms, notations | Normative |
| 02-identifiers.md | Fixed-width canonical identifiers | Normative |
| 03-canonical-encoding.md | VCE-1 codec | Normative |
| 04-cryptography.md | Hash, signatures, domains | Normative |
| 05-transactions.md | Transaction model, anti-replay | Normative |
| 06-object-model.md | Object-centric state | Normative |
| 07-capabilities.md | Capability authorization | Normative |
| 08-dag.md | Vertex model, DAG rules | Normative |
| 09-consensus.md | BaselineDagBft | Normative |
| 10-ordering.md | Deterministic ordering policy | Normative |
| 11-execution.md | Sequential deterministic executor | Normative |
| 12-state.md | State commitments (BMH-1) | Normative |
| 13-checkpoints.md | Checkpoint structure & finality | Normative |
| 14-networking.md | Validator QUIC / public P2P | Scoped draft (impl detail for v0.1 wire) |
| 15-data-availability.md | DA backends | Scoped draft |
| 16-validator-membership.md | StaticCommittee | Normative (static) |
| 17-upgrades.md | Versioning & upgrades | Normative |
| 18-light-clients.md | Light-client verification | Scoped draft |

Sections marked NORMATIVE use RFC 2119 language (MUST/SHOULD/MAY).
