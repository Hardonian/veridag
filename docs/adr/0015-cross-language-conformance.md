# ADR-0015: Cross-language conformance

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Independent implementations must agree on canonical bytes, digests, and signature preimages for every consensus object.

## Decision
Publish machine-readable golden vectors and malformed vectors; every implementation must reproduce/accept golden vectors and reject malformed ones. A conformance runner checks this before an implementation is called compliant.

## Alternatives
Informal agreement (rejected: drift); testing only against Rust (rejected: Rust becomes the spec).

## Security consequences
Catches parser differentials and canonicalization bugs that undermine consensus.

## Performance consequences
Test-time cost only.

## Complexity consequences
Maintaining vectors as the protocol evolves.

## Interoperability consequences
This is the mechanism that makes veridag-go/zig/cpp possible.

## Revisit conditions
Never optional; expand coverage as the protocol grows.
