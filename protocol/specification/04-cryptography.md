# 04 — Cryptography

Status: NORMATIVE

Veridag never implements cryptographic primitives manually. Implementations MUST
use audited libraries. The protocol defines abstractions and concrete algorithm
choices for v0.1.

## Abstractions

```text
Hasher              (input bytes) -> 32-byte digest
UserSignature       (Ed25519) for client transactions
ValidatorSignature  (Ed25519) for vertices and checkpoint votes in v0.1
CommitmentScheme    BMH-1 (see 12-state)
RandomnessBeacon    derived from committed DAG data (see 10-ordering)
```

## v0.1 algorithm choices (ADR-0004)

| Purpose | Algorithm |
|---------|-----------|
| Protocol hash `H` | BLAKE3 (256-bit, keyed or unkeyed as specified) |
| Client signature | Ed25519 |
| Validator signature | Ed25519 |
| State commitment | BMH-1 over BLAKE3 (12-state) |

BLS, threshold signatures, and exotic curves are deliberately excluded from v0.1.
They may be added only after concrete need, benchmarks, security analysis, and an
ADR (17-upgrades).

## Domain separation (MUST)

Every hashed or signed protocol object is prefixed with a unique ASCII domain
string terminated by a single zero byte, followed by the VCE-1 encoding of the
object:

```text
preimage = domain_bytes || 0x00 || VCE1(object)
digest   = BLAKE3(preimage)
```

Current domains:

| Domain string | Object |
|---------------|--------|
| `VERIDAG_TX_V1` | SignedTransaction |
| `VERIDAG_VERTEX_V1` | Vertex (signature excluded) |
| `VERIDAG_CHECKPOINT_V1` | Checkpoint (finality proof excluded) |
| `VERIDAG_CHECKPOINT_VOTE_V1` | Checkpoint vote |
| `VERIDAG_CAPABILITY_V1` | Capability |
| `VERIDAG_GENESIS_V1` | Genesis |
| `VERIDAG_ADDRESS_V1` | address derivation |
| `VERIDAG_OBJECT_ID_V1` | object id derivation |
| `VERIDAG_VALIDATOR_V1` | validator id derivation |
| `VERIDAG_APP_V1` | application id derivation |
| `VERIDAG_BATCH_V1` | batch of transaction ids |
| `VERIDAG_BMH_LEAF_V1` | BMH-1 leaf |
| `VERIDAG_BMH_NODE_V1` | BMH-1 internal node |
| `VERIDAG_ORDER_SEED_V1` | ordering seed derivation |

The same byte payload MUST NOT be interpretable under two different domains.

## Signature verification

* Ed25519 verification MUST follow RFC 8032 with canonical (S < L) signatures
  enforced by the underlying library.
* Signature verification is performed only after cheap structural validation
  (52-validation-pipeline in `../docs/threat-model.md`).

## Randomness

Consensus-visible randomness MUST be derived from committed protocol data via
`H("VERIDAG_ORDER_SEED_V1" || committed_anchor_id || round)`. No OS entropy and
no wall-clock input may enter consensus-visible computation.
