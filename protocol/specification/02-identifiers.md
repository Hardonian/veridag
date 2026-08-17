# 02 — Canonical Identifiers

Status: NORMATIVE

All protocol identifiers are fixed-width, explicitly typed byte strings. No
identifier has UUID semantics. No identifier is encoded with a host-native integer
layout.

## Integer identifiers (8-byte big-endian)

| Type | Width | Encoding |
|------|-------|----------|
| `ProtocolVersion` | 8 bytes | u64be |
| `ChainId` | 8 bytes | u64be |
| `Epoch` | 8 bytes | u64be |
| `Round` | 8 bytes | u64be |
| `CheckpointSequence` | 8 bytes | u64be |

The current `ProtocolVersion` is `1`. `ChainId` is fixed at genesis and MUST be
bound into every transaction and vertex (05, 08).

## Hash-derived identifiers (32 bytes)

These are `H(domain || [payload])` for the stated domain:

| Type | Domain | Payload |
|------|--------|---------|
| `TransactionId` | `VERIDAG_TX_V1` | `SignedTransaction` |
| `ObjectId` | `VERIDAG_OBJECT_ID_V1` | `creator_address || nonce_u64be` |
| `CapabilityId` | `VERIDAG_CAPABILITY_V1` | `Capability` |
| `VertexId` | `VERIDAG_VERTEX_V1` | `Vertex` (without signature field) |
| `CheckpointId` | `VERIDAG_CHECKPOINT_V1` | `Checkpoint` (without finality proof) |
| `BatchId` | `VERIDAG_BATCH_V1` | `ordered list of TransactionId` |
| `ValidatorId` | `VERIDAG_VALIDATOR_V1` | `validator Ed25519 public key` |
| `ApplicationId` | `VERIDAG_APP_V1` | `application module commitment` |

`ObjectVersion` is a `u64be` starting at `0` at object creation and incrementing
by exactly `1` per successful mutation of that object (06-object-model).

## Addresses (32 bytes)

An `Address` is `H("VERIDAG_ADDRESS_V1" || ed25519_public_key)`. There is no
human-readable form at protocol level; SDKs MAY define bech32-style display forms
but MUST NOT use them in consensus bytes.

## ValidatorId note

`ValidatorId` identifies the consensus key. A validator's networking identity MUST
be cryptographically bound to its `ValidatorId` at connection time
(14-networking).

## Rules

1. Identifiers MUST be compared bytewise.
2. Identifiers MUST NOT be re-derived from display forms inside consensus paths.
3. All identifier widths are part of the protocol; changing a width is a
   protocol-version change (17-upgrades).
