# 01 — Terminology and Notation

Status: NORMATIVE

## Terms

* **Protocol** — the implementation-independent rules in this specification.
* **Implementation** — software claiming conformance to the protocol.
* **Validator** — a member of the active `ValidatorSet` entitled to author vertices.
* **Honest (correct) validator** — one following the protocol.
* **Byzantine validator** — one deviating arbitrarily.
* **`n`, `f`** — validator-set size and Byzantine tolerance; the protocol assumes
  `n >= 3f + 1` (see 09-consensus).
* **Quorum** — any set of validators with total weight `>= quorum_threshold()`;
  for uniform weight, `2f + 1`.
* **Vertex** — a signed unit of the DAG (08-dag).
* **Round** — a monotonically increasing DAG layer index.
* **Anchor** — a vertex designated by the deterministic leader schedule for
  ordering (09-consensus).
* **Checkpoint** — a finalized snapshot commitment (13-checkpoints).
* **Object** — a unit of persistent state (06-object-model).
* **Capability** — a protocol-native authorization object (07-capabilities).
* **Transaction** — a signed state-transition request (05-transactions).
* **Canonical encoding** — the unique byte form defined by VCE-1
  (03-canonical-encoding).
* **Domain** — a fixed ASCII string prefixed to every hash/sign preimage
  (04-cryptography).
* **Epoch** — a bounded span of checkpoints at whose boundary validator-set or
  protocol changes activate (16-validator-membership).
* **Execution context** — `(protocol_version, chain_id, epoch)` bound into every
  transaction and signature.

## Notation

* `||` — byte concatenation.
* `H(x)` — the protocol hash of byte string `x` (04-cryptography). All protocol
  hashes are computed over `domain || canonical_encoding(x)`; never over a native
  in-memory representation.
* `Sig(sk, domain, x)` — signature over `domain || canonical_encoding(x)`.
* `[T]` — the canonical encoding of value `T` per 03-canonical-encoding.
* `u64be(x)` — 8-byte unsigned big-endian encoding of `x`.
* `S(n) -> S(n+1)` — the state-transition function `Apply(S, OrderedTransactions)`
  defined in 11-execution.
* "MUST", "MUST NOT", "SHOULD", "MAY" — RFC 2119.

## Implementation independence requirement

Every rule stated here MUST be expressible without reference to any programming
language, operating system, database, or network stack. Where a rule could be
satisfied by multiple distinct byte representations, this specification MUST
choose exactly one (see 03-canonical-encoding §canonicity).
