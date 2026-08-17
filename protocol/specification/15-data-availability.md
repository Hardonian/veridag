# 15 — Data Availability (Scoped Draft)

Status: SCOPED DRAFT

The protocol separates data availability from consensus. A vertex commits to
batches by `BatchId`; the DA layer guarantees the referenced batch bytes are
retrievable before those batches are used for commit (08 §7).

## Interface

```text
DataAvailabilityBackend
  put(batch_bytes) -> BatchId
  get(BatchId) -> Option<batch_bytes>
  available(BatchId) -> bool
```

## Progressive backends

| Backend | Status | Notes |
|---------|--------|-------|
| `LocalDA` | v0.1 | single-node/dev |
| `ValidatorReplicatedDA` | v0.1 target | batches replicated across validators |
| `ErasureCodedDA` | v0.3 experimental | |
| `ExternalDAAdapter` | future | never required |

Sovereign operation MUST remain possible: no external DA infrastructure is
required for a functioning network.

## Consensus-visible rule (NORMATIVE)

A commit decision that depends on a batch MUST NOT be finalized until the batch
is available via the configured backend. Availability is a precondition to
commit, not an afterthought.
