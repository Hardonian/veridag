# Security Model

The security model is the combination of:

* the threat model (`threat-model.md`);
* the capability model (`../protocol/specification/07-capabilities.md`);
* the cryptographic domains (`../protocol/specification/04-cryptography.md`);
* the validation pipeline and panic policy (threat-model.md);
* the consensus safety invariants (Agreement, Finality, Integrity, plus the
  determinism, no-double-spend, replay-protection, capability-safety, and
  canonical-interpretation invariants in `../protocol/specification/00-overview.md`).

We claim only what is demonstrated by the formal model and the test suite in this
tree. Production deployment guidance is in `ops/deployment.md`. The threat model
and attack surface validation pipeline are in `threat-model.md`.
