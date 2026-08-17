# 16 — Validator Membership

Status: NORMATIVE (StaticCommittee) / SCOPED DRAFT (dynamic)

Consensus is separate from validator membership.

## ValidatorSet

```text
ValidatorSet
├── epoch        Epoch
├── validators   Vec<(ValidatorId, Ed25519PublicKey, weight u32)>
└── quorum_threshold() -> u64   // = 2f + 1 computed from total weight
```

## v0.1: StaticCommittee (NORMATIVE)

* The validator set is fixed at genesis.
* `n >= 3f + 1` with uniform weight 1.
* Validator-set changes occur ONLY at explicit epoch boundaries, are committed in
  a checkpoint's `validator_set_commitment`, and the old and new sets are
  cryptographically linked via the checkpoint chain.

## Later membership modes (SCOPED DRAFT)

`StakeWeighted`, `PermissionedCommittee`, `FederatedMembership`,
`ExternalMembership`, and `ReputationWeighted` (experimental). None are required
to prove consensus correctness; no token is required. Proof-of-work is out of
scope by design.
