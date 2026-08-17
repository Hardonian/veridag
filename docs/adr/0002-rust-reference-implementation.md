# ADR-0002: Rust reference implementation

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
We need memory safety, a strong type system, good Wasm/QUIC ecosystem, and stable tooling for implementation #1.

## Decision
Rust stable toolchain is implementation #1. Crates default to `#![forbid(unsafe_code)]`.

## Alternatives
Go (rejected: GC pauses and weaker type-level guarantees for consensus code); C++ (rejected: memory safety burden); Zig (deferred: useful later for acceleration, not the reference).

## Security consequences
Memory-safety bugs are a major class of consensus failures; Rust removes most by construction.

## Performance consequences
Good optimizing backend; zero-cost abstractions; no GC on the hot path.

## Complexity consequences
Steeper learning curve; offset by strong compiler diagnostics.

## Interoperability consequences
Rust is an implementation detail; the protocol never depends on Rust layout or representation.

## Revisit conditions
If a second implementation overtakes it in production usage and the community shifts.
