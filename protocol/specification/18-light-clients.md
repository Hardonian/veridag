# 18 — Light Clients (Scoped Draft)

Status: SCOPED DRAFT (designed early; full implementation post-v0.1)

A light client verifies without replaying the chain. It needs only:

1. a trusted genesis or checkpoint (out-of-band trust anchor);
2. validator transition proofs (checkpoint-to-checkpoint);
3. checkpoint finality proofs (13);
4. state inclusion proofs (12).

## Verification sketch (NORMATIVE once implemented)

Given trusted checkpoint `C_t` and a claimed later checkpoint `C`:

* verify `C.finality_proof` against the validator set committed by `C_t` (or by
  an intermediate linked sequence of validator-set commitments at epoch
  boundaries);
* for an object claim, verify the BMH-1 inclusion proof against `C.state_root`.

A light client MUST NOT trust: unauthenticated peer claims, unversioned proofs,
or checkpoints without quorum finality.

## Non-goals for v0.1

Full header chains, fraud proofs, and data-availability sampling are future work.
