# ADR-0012: Proof system abstraction

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
We want optional verifiable execution without making a vendor's prover consensus-mandatory.

## Decision
Define ProofBackend (prove_transition, verify_transition, backend_id, proof_version). v0.1 ships NoProof; SP1/RISC Zero adapters are feature-gated experiments. Consensus never requires a specific prover.

## Alternatives
Mandating one zkVM (rejected: vendor lock-in and immature tooling); no proofs (deferred: we want the option).

## Security consequences
Proof bytes identify system/version/program commitment/protocol version; verification is optional and versioned.

## Performance consequences
Proving is expensive and off the critical path; only used where its value is proven.

## Complexity consequences
Abstraction layer to maintain; minimal in v0.1.

## Interoperability consequences
Any backend behind the interface can be used by any implementation.

## Revisit conditions
When a concrete zkVM integration is benchmarked and a security analysis is done.
