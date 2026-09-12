# 18 — Light Clients

Status: NORMATIVE  
Version: 1.0  
Scope: 2f+1 Quorum Checkpoint Verification, Continuous Epoch Tracking, and BMH-1 Merkle Object Inclusion Proofs

A light client verifies state and transaction execution without downloading or replaying the full DAG history.

## 1. Trust & Verification Primitives

A light client requires only:
1. **Out-of-Band Trust Anchor**: A trusted checkpoint `C_0` or genesis root.
2. **2f+1 Finality Proof Verification**: Checking Ed25519 signatures of the validator committee against the committed stake weight threshold (`quorum_threshold = 2f + 1`).
3. **Continuous Epoch Tracking (`LightClientTracker`)**: Tracking sequential monotonic checkpoints across epoch handovers.
4. **BMH-1 Merkle Object Inclusion Proofs**: Sibling path verification against `C.state_root` for specific objects (`ObjectId`, `ObjectBytes`).

## 2. Verification Protocol (NORMATIVE)

Given trusted checkpoint $C_t$ and a candidate checkpoint $C_{t+k}$:

1. **Epoch Continuity**: Verify that $C_{t+k}$ forms a valid monotonic transition from $C_t$. If an epoch handover occurred, verify the outgoing committee's signed transition to the incoming committee's `validator_set_commitment` (16 §3).
2. **Quorum Signature Verification**: Verify that the cumulative weight of valid validator signatures on $C_{t+k}$ meets or exceeds $\lfloor \frac{2W}{3} \rfloor + 1$.
3. **Object State Proof**: For any query regarding an object $(id, bytes)$, verify the BMH-1 inclusion proof:
   $$\text{leaf} = \text{BLAKE3}(\text{"VERIDAG\_BMH\_LEAF\_V1"} \,\|\, id \,\|\, bytes)$$
   and verify sibling hashes upwards to $C_{t+k}.\text{state\_root}$.

A light client MUST reject:
- Checkpoints lacking quorum signatures ($< 2f+1$).
- Non-monotonic epoch transitions.
- Invalid or mismatched Merkle inclusion proof paths.

## 3. Implementations

- **Rust Substrate**: `veridag-light-client` crate (`LightClientTracker`, `verify_checkpoint_finality`, `verify_object_inclusion`).
- **EVM L1 Contract**: `contracts/VeridagLightClient.sol` implementing continuous on-chain checkpoint tracking and Merkle verification for Ethereum L1 bridges.

