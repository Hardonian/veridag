# ADR-0014: Data availability abstraction

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Commit must depend on batch availability, but no external DA infrastructure may be required.

## Decision
Define DataAvailabilityBackend (put/get/available). v0.1 provides LocalDA and targets ValidatorReplicatedDA; ErasureCodedDA and ExternalDAAdapter come later. Availability is a precondition to commit.

## Alternatives
Implicit DA via gossip only (rejected: hard to reason about); external DA only (rejected: violates sovereignty).

## Security consequences
Prevents committing to unavailable data; explicit precondition closes a class of attacks.

## Performance consequences
Replication costs bandwidth; erasure coding later improves efficiency.

## Complexity consequences
Simple interface; backends are pluggable.

## Interoperability consequences
Any backend can serve any implementation; external adapters are optional.

## Revisit conditions
When scale demands erasure coding or an external DA integration is concretely needed.
