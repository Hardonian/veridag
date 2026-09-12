# Security Policy

## Reporting

Report vulnerabilities privately to the maintainers. Do not open public issues for
security reports. Include: affected component, protocol version, reproduction or
trace, impact assessment, and whether the issue is consensus-visible.

## Scope

In scope:
- Canonical VCE-1 codec, transaction/vertex/checkpoint serialization, and malformed vector rejection.
- Object-capability model, class boundaries, spending limits, and cryptographic delegation.
- BaselineDagBft consensus commit rules, causal DAG ordering, wave anchors, and equivocation defense.
- BMH-1 Blake3 Merkle-Hash state commitments, verifiable checkpoints, and light-client inclusion proofs.
- Validator QUIC fast-path mutual TLS authentication, selective libp2p discovery, and anti-DDoS backpressure.
- Wasm component isolation, gas/instruction metering, memory limits, and host call capability gating.
- 2D Reed-Solomon data availability, tensor commitments, and iterative erasure reconstruction.
- zkVM verification adapters (Circom, Groth16, Halo2) and verifiable state transitions.
- USMCA sovereign settlement engine (USDV stablecoin), Treasury reserve attestation, and OFAC sanctions enforcement.
- Settler atomic dual-leg cross-border reconciliation and multi-currency routing.
- Bitcoin SPV verification and Ethereum light-client bridge substrates.
- Dependency supply chain and `#![forbid(unsafe_code)]` enforcement across all workspace crates.

## Non-Goals & Architecture Boundaries

1. **Zero Retail Gas Speculation:** By protocol design, Veridag operates as an enterprise USMCA institutional consortium. There are no volatile, speculative retail native gas tokens. Transactions are settled deterministically in USDV or enterprise credits.
2. **Consortium Fault Bounds:** The system guarantees safety and liveness provided Byzantine weight satisfies $f < W/3$. Compromise of greater than $f$ validators falls outside BFT mathematical bounds.

## Guarantees We Claim

We claim:
1. Complete deterministic consensus and state execution across all conforming implementations.
2. Full bit-for-bit test vector agreement across Rust, TypeScript, and Python SDKs.
3. Zero-warning compilations under `cargo clippy --all-features -D warnings` and `#![forbid(unsafe_code)]`.
See `docs/threat-model.md` for the threat model and attack surface validation pipeline.


## Attacker-facing paths

Every attacker-facing path enforces bounded resource consumption (see
`protocol/specification/14-networking.md` and `docs/threat-model.md` §validation
pipeline). Attacker-controlled input must never cause a process panic.
