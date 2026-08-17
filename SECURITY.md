# Security Policy

## Reporting

Report vulnerabilities privately to the maintainers. Do not open public issues for
security reports. Include: affected component, protocol version, reproduction or
trace, impact assessment, and whether the issue is consensus-visible.

## Scope

In scope: canonical codec, transaction/vertex/checkpoint parsers, capability
enforcement, consensus commit rule, state commitments, validator networking
authentication, anti-replay, crash consistency, Wasm runtime isolation,
dependency supply chain.

Out of scope for v0.1 (do not report as though they were promised): zk proof
soundness, production mainnet economics, public token behavior, GPU acceleration
safety.

## Guarantees we do and do not claim

We claim only what is demonstrated by the formal model and tests in this tree.
We do not claim production security without external audits. See
`docs/threat-model.md` for the threat model and explicit non-goals.

## Attacker-facing paths

Every attacker-facing path enforces bounded resource consumption (see
`protocol/specification/14-networking.md` and `docs/threat-model.md` §validation
pipeline). Attacker-controlled input must never cause a process panic.
