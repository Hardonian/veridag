# ADR-0013: Ordering policy (CanonicalWaveOrder)

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Ordering must be deterministic and resist proposer manipulation, censorship, grinding, and strategic withholding.

## Decision
v0.1 adopts CanonicalWaveOrder: seed = H(domain || anchor_id || wave); order committed vertices by (round, H(seed || author || vertex_id)); expand batches in that order; dedupe by TransactionId keeping first occurrence.

## Alternatives
Validator-index order (rejected: manipulable and economically meaningful); arrival-time (rejected: non-deterministic); lexical txid (rejected: gameable).

## Security consequences
Seed depends on the committed anchor, so it is unpredictable before commit; withholding cannot reorder others' transactions.

## Performance consequences
A hash-based permutation is cheap relative to signature verification.

## Complexity consequences
One clear rule; documented threat analysis.

## Interoperability consequences
Fully specified so independent implementations derive identical order.

## Revisit conditions
If MEV-style manipulation is demonstrated against it, adopt threshold encryption or batch auctions via ADR.
