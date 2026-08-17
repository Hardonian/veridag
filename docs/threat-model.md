# Veridag Threat Model

Status: living document. Scope: protocol v0.1 and reference implementation.

## Adversary classes

* Byzantine validators (up to `f` of `n >= 3f+1`).
* Malicious clients.
* Sybil peers on the public plane.
* Malicious applications in the Wasm runtime.
* Supply-chain attackers on dependencies.

## Attacks considered and mitigations

| Attack | Mitigation |
|--------|-----------|
| Equivocation | Detected; one working vertex per (author, round); safety proved in Quint model. |
| Censorship | DAG proposals from all validators; withholding only delays own txs. |
| Ordering manipulation | CanonicalWaveOrder seed bound to committed anchor (spec 10). |
| Replay | nonce + expiry_epoch + object-version binding + (chain, protocol) in preimage. |
| Eclipse | Validator fast path authenticated to ValidatorId; public plane separate. |
| Parser attacks | VCE-1 canonical rejection; malformed vector suite; fuzz targets on every parser. |
| DoS / resource exhaustion | Bounded frames/messages/queues/requests; validation pipeline cheap-to-expensive; backpressure. |
| Key compromise | Capability scoping limits blast radius; validator keys via keystore/signer abstraction. |
| Wasm escape | Default-deny host API; explicit capability handles; deterministic metering. |
| Proof forgery | Versioned proof envelopes; verification optional and backend-identified. |
| Rollback | Finality invariant; checkpoint chain links validator sets. |
| Checkpoint forgery | Quorum finality proofs (2f+1). |
| Supply-chain | cargo deny + audit in CI; deliberate dependency policy. |
| Validator crash/restart | Persist-before-ack; recovery rebuilds from durable state (34-crash-consistency). |
| Partition | Safety is clock-independent; liveness resumes under eventual synchrony. |

## Validation pipeline (increasing cost)

```
frame bounds -> basic format -> protocol version -> canonical encoding
-> duplicate check -> cheap structural checks -> signature verification
-> state-dependent validation -> execution -> proof verification (if required)
```

Expensive work is never done before cheap rejection when avoidable.

## Explicit non-goals (v0.1)

* Public token economics / proof-of-stake security.
* zk proof soundness (proofs are optional and experimental).
* Protection against a comprome of > f validators.
* Defense against physical-side-channel attacks on validator hardware.
* Guaranteed liveness under permanent partition.

## Panic policy

Attacker-controlled input must never trigger process panic. No `unwrap`/`expect`/
`panic!`/`unreachable!` on externally reachable paths. Typed errors everywhere.
A validator may terminate only for genuinely unrecoverable local integrity
failures or deliberate operator action.
