# ADR-0003: Quint formal model

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
We need an executable, model-checkable spec that engineers can read and that can emit counterexample traces.

## Decision
Use Quint for the formal model. Keep it isolated so TLA+/Apalache/Alloy/Lean/Coq can augment later.

## Alternatives
TLA+ (deferred: powerful but steeper for contributors); Alloy (deferred); Lean/Coq (deferred: high assurance, high effort).

## Security consequences
Lets us prove Agreement/Finality/Integrity on small networks before optimizing Rust.

## Performance consequences
Model checking is offline; no runtime cost.

## Complexity consequences
Adds a second language to maintain; mitigated by keeping the model small and focused.

## Interoperability consequences
Model traces feed conformance vectors, keeping the model tied to implementations.

## Revisit conditions
If invariants outgrow Quint's checker performance, or a theorem-prover proof is required.
