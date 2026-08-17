# ADR-0011: Storage abstraction

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Consensus semantics must not depend on a storage backend's iteration order or quirks; we need crash consistency and atomic writes.

## Decision
Define StateStore/DagStore/CheckpointStore/MetadataStore traits. Provide MemoryStore and a PersistentStore chosen by benchmark and reliability criteria. Iteration order is never consensus-visible; canonical ordering is defined by the protocol.

## Alternatives
Hard-wiring to RocksDB (rejected: backend lock-in); SQL (deferred: heavier than needed).

## Security consequences
Crash-consistency tests cover the commit path; corruption handling is explicit.

## Performance consequences
Persistent engine chosen by measured performance; abstraction avoids lock-in.

## Complexity consequences
Trait indirection is small.

## Interoperability consequences
Any backend satisfying the traits can serve an implementation.

## Revisit conditions
If the chosen engine fails reliability/performance requirements in production.
