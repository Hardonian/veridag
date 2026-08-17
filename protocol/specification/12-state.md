# 12 — State Commitments (BMH-1)

Status: NORMATIVE

State commitments use a versioned, deterministic authenticated structure so that
checkpoints and light clients can verify state without replaying history.

## BMH-1: Binary Merkle Hash-map commitment, version 1

The state is a set of `(ObjectId -> Object)` pairs. BMH-1 commits to it as a
binary Merkle tree over canonically ordered leaf hashes.

* **Leaf.** For each object, `leaf_key = ObjectId` (32 bytes, already uniform),
  `leaf_hash = H("VERIDAG_BMH_LEAF_V1" || [Object])`.
* **Ordering.** Leaves are sorted by `leaf_key` bytewise ascending. This ordering
  is total and deterministic; it replaces any hash-map iteration order.
* **Internal node.** For children `(L, R)` with `L` before `R` by position:
  `node_hash = H("VERIDAG_BMH_NODE_V1" || L || R)`. Odd leaf at a level is
  promoted unchanged (no duplication).
* **Root.** The root of the single resulting tree is the `state_root` of the
  empty tree as `H("VERIDAG_BMH_NODE_V1" || 0x00 || 0x00)` by convention.

## Proofs

* **Inclusion proof** for `ObjectId`: the sibling path plus the leaf. Encoding is
  versioned (`proof_version = 1`) and VCE-1.
* **Verification** recomputes the root and compares bytewise.
* **Exclusion proofs** are post-v0.1 (a sparse Merkle variant may replace BMH-1
  without changing checkpoints, via ADR and proof versioning).

## Abstraction

```text
StateCommitment
  root(state) -> Hash
  prove_inclusion(state, ObjectId) -> Proof
  verify_inclusion(root, ObjectId, Object, Proof) -> bool
  proof_version -> u8
```

Subsystems MUST depend on this interface, not on BMH-1 concretely.

## Determinism requirement (MUST)

Given the same set of objects, every implementation MUST compute the identical
`state_root`. This is tested by golden vectors (protocol/test-vectors/state/) and
by the parallel==sequential property (11).
