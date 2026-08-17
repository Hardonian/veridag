# ADR-0004: Canonical protocol encoding (VCE-1) + BLAKE3 + Ed25519

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Consensus encoding must be deterministic and implementation-independent; general-purpose codecs (bincode, CBOR, MessagePack, protobuf) admit multiple byte forms for one semantic value or depend on library defaults.

## Decision
Define VCE-1: fixed-width big-endian integers, no varints, length-prefixed strings/bytes with limits, sequences with counts, explicit tagged unions, maps as sorted key-value sequences, rejection of any non-canonical form. Hash = BLAKE3 with explicit domain separation. Signatures = Ed25519.

## Alternatives
bincode (rejected: implementation-defined); CBOR (rejected: multiple canonical forms in the wild); protobuf (rejected: field ordering/unknown fields complicate canonicity); SHA-256 (deferred: BLAKE3 chosen for speed with equal security margin for our use); BLS (deferred: needs benchmarks and security analysis first).

## Security consequences
Domain separation prevents cross-context signature/hash reuse; canonical rejection removes parser differential attacks.

## Performance consequences
Fixed-width integers cost a few bytes; BLAKE3 is fast; Ed25519 verification is cheap and batched later if profiling demands it.

## Complexity consequences
We own a small codec; offset by a conformance vector suite.

## Interoperability consequences
Any language can implement VCE-1 from the spec alone; golden vectors prove it.

## Revisit conditions
If a cryptographic break or a demonstrated interoperability requirement forces a change; changes are protocol-version bumps.
