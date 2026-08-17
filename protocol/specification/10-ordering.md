# 10 — Ordering

Status: NORMATIVE

Ordering is a first-class protocol concern. We do not use validator index, arrival
time, local receipt order, or lexical transaction id as economically meaningful
tie-breakers without analysis.

## Objectives

* determinism;
* proposer manipulation resistance;
* censorship resistance;
* bounded ordering influence;
* resistance to grinding and strategic withholding.

## OrderingPolicy: `CanonicalWaveOrder` (v0.1 normative)

Inputs: the committed anchor `A(w)` and the deterministic causal vertex set `V(w)`
selected by the commit rule (09), plus the ordering seed.

Steps (all deterministic):

1. **Seed.** `seed(w) = H("VERIDAG_ORDER_SEED_V1" || VertexId(A(w)) || u64be(w))`.
   The seed is unpredictable before `A(w)` is committed, because it depends on the
   anchor's id and the anchor is not known to be committed until the commit rule
   fires.
2. **Vertex order.** Order `V(w)` by `(round, H(seed || author || VertexId))`.
   This is a deterministic permutation that no single validator controls, because
   it depends on the committed anchor and the full vertex set.
3. **Transaction order.** For each vertex in vertex order, expand its batch
   commitments to the ordered list of transaction ids (batch order is fixed by
   the batch's own canonical encoding; a batch is an ordered sequence of
   `TransactionId`). Concatenate in vertex order. Deduplicate by `TransactionId`,
   keeping first occurrence (a transaction referenced by multiple vertices is
   executed once).
4. **Execution order.** The resulting total order is the input to `Apply` (11).

## Threat analysis (summary; full model in docs/threat-model.md)

* A proposer cannot pick a favorable position for its own transaction once the
  anchor is fixed, because the seed depends on the committed anchor id.
* Withholding a vertex can at most exclude its own transactions from this wave;
  it cannot reorder other validators' transactions, because their relative order
  is fixed by the seed-based permutation.
* Grinding on the seed requires re-deriving the anchor id after seeing the
  commit, which is impossible without equivocation (detected and excluded).

## Future policies

`FairOrder`, `ThresholdEncryptedMempool`, and batch auctions are explicitly
post-v0.1 and require ADRs (17-upgrades).
