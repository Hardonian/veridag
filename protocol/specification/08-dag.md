# 08 — DAG

Status: NORMATIVE

Validators propose vertices independently. The DAG is the data structure from
which consensus derives a totally ordered history (09-consensus). There is no
single permanent leader serializing proposals.

## Vertex

```text
Vertex
├── protocol_version   ProtocolVersion
├── chain_id           ChainId
├── epoch              Epoch
├── round              Round
├── author             ValidatorId
├── parents            Vec<VertexId>        (see rules below)
├── batch_commitments  Vec<BatchId>         (commitments to tx batches; DA layer)
├── metadata           byte string (<= 128 bytes, opaque)
└── signature          ValidatorSignature   over VERIDAG_VERTEX_V1 (signature excluded)
```

`VertexId = H("VERIDAG_VERTEX_V1" || [Vertex without signature])`.

## Validity rules (MUST)

A vertex is valid iff:

1. VCE-1 canonical decode succeeds.
2. `protocol_version`/`chain_id` match the local node's active values.
3. `epoch` equals the node's current epoch.
4. `author` is in the current validator set, and `signature` verifies.
5. `round >= 1`; for `round == 1`, `parents` MUST be the genesis vertex ids
   (or empty per genesis definition); for `round > 1`, `parents` MUST contain
   at least `quorum_threshold()` distinct vertices of round `round - 1`.
6. All referenced parents are known or fetched before the vertex is processed
   (implementation buffers out-of-order vertices; this is an implementation
   concern, not a consensus rule).
7. Each `batch_commitment` refers to a batch available via the DA layer
   (15-data-availability); a vertex referencing unavailable batches MUST NOT be
   used for commit until the batches are available.

## Equivocation (MUST)

Two distinct valid vertices `(v, v')` with `author(v) == author(v')` and
`round(v) == round(v')` are an **equivocation**. Honest validators:

* retain at most one vertex per `(author, round)` as their working vertex;
* MUST treat the equivocating author as faulty for commit purposes;
* MUST NOT let equivocation violate safety (see 09-consensus invariants; the
  formal model proves agreement in the presence of equivocation).

## Round progression

A validator advances to round `r + 1` only after its local DAG contains at least
`quorum_threshold()` valid vertices of round `r` from distinct authors. This is
the only round-advance rule; it depends on quorum certificates, not clocks.

## Persistence

Vertices MUST be persisted before being acknowledged to peers (34-crash-
consistency). A restarted validator MUST rebuild its DAG from durable storage and
resume without finalizing incompatible state.
