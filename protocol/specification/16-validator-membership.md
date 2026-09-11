# 16 — Validator Membership and Consortium Governance

Status: NORMATIVE  
Version: 1.0  
Scope: Static Committee, Dynamic Consortium Membership, Weight Rules, and Epoch Handover  

Consensus is cleanly decoupled from validator membership. Membership defines *who* participates in the DAG and with what voting power; consensus defines *how* valid vertices are causally ordered into committed waves.

---

## 1. ValidatorSet Schema

```text
ValidatorSet
├── epoch:               Epoch (u64)
├── validators:          Vec<WeightedValidator>
│   ├── id:              ValidatorId
│   ├── public_key:      Ed25519PublicKey
│   └── weight:          u64
├── total_weight:        u64  (sum of weights)
├── byzantine_weight:    u64  (f = (total_weight - 1) / 3)
└── quorum_threshold:    u64  (2f + 1 of total weight)
```

---

## 2. Invariants & Weight Rules

1. **BFT Quorum Bound:**
   $$\text{total\_weight} \ge 3f + 1 \implies \text{quorum\_threshold} = 2f + 1$$
   Any subset of validators whose cumulative weight meets or exceeds $2f + 1$ constitutes a valid quorum.
2. **Deterministic Canonical Ordering:**
   Validators in the committee are strictly sorted by `ValidatorId`.
3. **Commitment Invariance:**
   The cryptographic commitment to the validator set is:
   $$\text{valset\_commitment} = \text{BLAKE3}(\text{"VERIDAG\_VALSET\_V1"} \,\|\, \text{epoch} \,\|\, \text{sorted\_entries})$$

---

## 3. Epoch Handover Protocol

Validator-set transitions occur exclusively at explicit epoch boundaries committed in checkpoints:

1. **Epoch Anchor:** When wave $W$ reaches epoch boundary $E$, the state machine processes membership adjustments (stake rebalancing, consortium bank additions/removals).
2. **Checkpoint Commitment:** Checkpoint $CP_E$ embeds the `validator_set_commitment` of epoch $E+1$.
3. **Quorum Handover:** Current committee $C_E$ signs $CP_E$ with $2f_E + 1$ weight.
4. **Activation:** Starting at round $R_{\text{start}}(E+1)$, vertices and wave anchors are governed by committee $C_{E+1}$.
5. **No Fork Guarantee:** Light clients and peers verify the single monotonic checkpoint chain linking $C_E \to C_{E+1}$.
