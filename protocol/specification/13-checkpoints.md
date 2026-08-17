# 13 — Checkpoints

Status: NORMATIVE

Consensus periodically finalizes checkpoints. A checkpoint binds state, DAG, and
validator-set commitments and carries a finality proof.

## Checkpoint

```text
Checkpoint
├── protocol_version          ProtocolVersion
├── chain_id                  ChainId
├── epoch                     Epoch
├── sequence                  CheckpointSequence
├── previous_checkpoint       CheckpointId
├── state_root                Hash          (BMH-1)
├── transaction_root          Hash          (Merkle root of ordered tx ids)
├── object_root               Hash          (same as state_root in v0.1)
├── dag_commitment            Hash          (H over committed anchor ids in order)
├── validator_set_commitment  Hash          (H over current ValidatorSet)
└── finality_proof            FinalityProof
```

`CheckpointId = H("VERIDAG_CHECKPOINT_V1" || [Checkpoint without finality_proof])`.

## FinalityProof (v0.1)

```text
FinalityProof
├── kind = 0 (QuorumSignatures)
└── votes = Vec<(ValidatorId, Ed25519Signature over VERIDAG_CHECKPOINT_VOTE_V1
                 || CheckpointId)>
```

A checkpoint is **final** when it carries valid votes from validators with total
weight `>= 2f + 1`. Because `n >= 3f + 1`, two conflicting checkpoints cannot both
be final (quorum intersection contains an honest validator that signs at most one
checkpoint per sequence; the formal model proves Agreement).

## Checkpoint production (MUST)

* Checkpoints are produced at a fixed, deterministic cadence: every
  `CHECKPOINT_INTERVAL_WAVES = 2` committed waves, the executor emits a checkpoint
  from the state after the wave's ordered transactions.
* The checkpoint content is a deterministic function of the committed history.
  Two honest validators that have committed the same waves MUST derive the same
  `CheckpointId`.

## Uses

* **Bootstrap** — a new node starts from a trusted checkpoint plus DA data.
* **Pruning** — history before a final checkpoint MAY be pruned (implementation).
* **Light clients** — verify finality proofs and inclusion proofs (18).
* **Upgrades** — protocol-version changes activate at checkpoint boundaries (17).

## Determinism requirement

`state_root`, `transaction_root`, and `dag_commitment` are consensus-visible and
MUST be identical across implementations for the same committed history.
