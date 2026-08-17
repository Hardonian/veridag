# 06 — Object Model

Status: NORMATIVE

Persistent state is a set of **objects**. There is no global key-value blob visible
to consensus; every mutation names explicit objects.

## Object

```text
Object
├── id              ObjectId
├── version         ObjectVersion (u64, starts 0, +1 per successful mutation)
├── object_type     u32
├── owner           Ownership
├── payload_commit  Hash          H("VERIDAG_OBJECT_PAYLOAD_V1" || payload)
├── payload         byte string   (<= 2^20 bytes in v0.1)
└── metadata        byte string   (<= 256 bytes, opaque to consensus)
```

`ObjectId = H("VERIDAG_OBJECT_ID_V1" || creator_address || u64be(nonce))` where
`nonce` is the creating transaction's sender nonce. This makes object ids
deterministic and collision-resistant without wall-clock or OS randomness.

## Ownership (tagged union, variant index normative)

| Index | Mode | Meaning |
|-------|------|---------|
| 0 | `Address(Address)` | Controlled by a single address's key. |
| 1 | `Shared` | Mutable by any transaction carrying a valid capability. |
| 2 | `Immutable` | Never mutated or deleted after creation. |
| 3 | `System` | Controlled by protocol-native logic only. |
| 4 | `Capability(CapabilityId)` | Mutations require the named capability. |
| 5 | `Application(ApplicationId)` | Mutations only via that application's execution. |

## Version discipline (MUST)

* Every mutable object version may be consumed at most once.
* A mutation transaction MUST declare `expected == current version`.
* Successful mutation sets `version := version + 1`.
* Two transactions in the same ordered batch that write the same object MUST be
  ordered such that the second sees the first's incremented version; if its
  declared `expected` no longer matches, it fails with a version-conflict receipt
  (11-execution). This yields deterministic conflict semantics without relying on
  execution scheduling.

## Built-in object types (v0.1)

| `object_type` | Meaning |
|---------------|---------|
| 0 | Account (nonce, capability refs; `owner = Address`) |
| 1 | Balance (`payload` = u64be amount; `owner = Address`) |
| 2 | Capability object (07-capabilities) |
| 3+ | Application-defined |

## State root

The commitment to the full object set is `BMH-1` (12-state). State roots are part
of checkpoints (13-checkpoints).
