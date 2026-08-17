# ADR-0006: Object-centric state

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
A shared key-value blob makes dependencies implicit and parallelization/determinism hard. Explicit objects with versions give deterministic conflict semantics.

## Decision
State is a set of objects with id, version, type, ownership, payload commitment, payload, metadata. Mutable versions are consumed at most once; transactions declare reads/writes with expected versions.

## Alternatives
Account-only model (rejected: hides dependencies); UTXO-only (rejected: too narrow for general computation).

## Security consequences
Explicit dependencies enable capability-scoped mutation and deterministic conflict resolution.

## Performance consequences
Enables conflict-aware parallel execution later; BMH-1 root is cheap to maintain.

## Complexity consequences
Slightly more transaction boilerplate (declared reads/writes).

## Interoperability consequences
Object model is specified independently of storage backend.

## Revisit conditions
If application patterns prove the model too rigid; change requires a protocol-version bump.
