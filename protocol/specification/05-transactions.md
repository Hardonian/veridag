# 05 — Transactions

Status: NORMATIVE

A transaction is a signed request to apply a state transition. Its canonical form,
`SignedTransaction`, is the only form that may be hashed, signed, or included in a
batch.

## Structure (field order is normative)

```text
Transaction
├── protocol_version   ProtocolVersion
├── chain_id           ChainId
├── sender             Address
├── nonce              u64                  (anti-replay, see below)
├── expiry_epoch       Epoch                (anti-replay bound)
├── declared_reads     Vec<ObjectRef>
├── declared_writes    Vec<ObjectRef>
├── capabilities       Vec<CapabilityId>
├── operation          Operation            (tagged union)
├── resource_budget    ResourceBudget
└── metadata           byte string (<= 256 bytes, opaque to consensus)

SignedTransaction
├── tx                 Transaction
└── signature          Ed25519Signature     over VERIDAG_TX_V1 domain
```

`ObjectRef` is `(ObjectId, expected ObjectVersion)`. `expected` binds the
transaction to a specific object version (16-anti-replay via versions).

`ResourceBudget` is `(compute_units u64, memory_units u64, storage_units u64,
bandwidth_units u64)`; see 37-resource-metering (abstract units, no token price).

## Operations (variant index is normative)

| Index | Operation | Fields |
|-------|-----------|--------|
| 0 | `CreateObject` | `object_type u32, ownership Ownership, payload byte string` |
| 1 | `UpdateObject` | `object ObjectRef, new_payload byte string` |
| 2 | `DeleteObject` | `object ObjectRef` |
| 3 | `TransferObject` | `object ObjectRef, new_owner Ownership` |
| 4 | `TransferValue` | `from ObjectRef(Balance), to Address, amount u64` |
| 5 | `GrantCapability` | `capability Capability` |
| 6 | `RevokeCapability` | `capability_id CapabilityId` |
| 7 | `InvokeApplication` | `app ApplicationId, input byte string` (post-v0.1 runtime) |

`TransferValue` in v0.1 operates on a native **Balance object** (06-object-model);
it does not imply a public token.

## Validation rules (MUST), in cheap-to-expensive order

1. VCE-1 decode succeeds and is canonical (03).
2. `protocol_version` equals the node's active protocol version.
3. `chain_id` equals the node's chain id.
4. Declared reads/writes counts within limits; no duplicate `ObjectId` across
   `declared_writes`.
5. `expiry_epoch >= current_epoch` (not yet expired).
6. Signature verifies under `VERIDAG_TX_V1` against `sender`'s key.
7. `nonce` equals the sender account's current nonce (anti-replay).
8. For each write `(id, v)`: object exists, current version equals `v`, and the
   sender holds authority (ownership or capability; 07).
9. Operation-specific checks (e.g., sufficient balance for `TransferValue`).

A transaction failing any rule MUST NOT be included in a batch by an honest
validator, and MUST be rejected if it appears in a committed batch (11-execution
treats invalid-in-batch as an execution error receipt, never a state change).

## Anti-replay (MUST)

Replay protection is protocol-defined and uses three deterministic mechanisms:

* `nonce` — per-sender strictly incrementing counter; a transaction is valid only
  when `nonce == sender_account.nonce`, and successful execution increments it.
* `expiry_epoch` — transaction invalid once `current_epoch > expiry_epoch`.
* `ObjectRef.expected` version binding — consuming a specific object version.

Transactions are bound to `(protocol_version, chain_id)` through the signed
preimage, so they cannot silently replay on a different chain or protocol
version.

## TransactionId

`TransactionId = H("VERIDAG_TX_V1" || [SignedTransaction])` (02-identifiers).
