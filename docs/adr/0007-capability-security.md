# ADR-0007: Capability security

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Key-owns-everything is unsafe for agents and services. We need least-authority, delegation, revocation, expiry, and resource limits as protocol state.

## Decision
Capabilities are protocol-native objects with scope, expiry, constraints, delegation flags, revocation, and deterministic enforcement by the executor. Agent/Session/Application/Spend kinds are built in.

## Alternatives
Pure signature ACLs (rejected: ambient authority); off-chain policy engines (rejected: not consensus-enforced).

## Security consequences
Agent keys can be narrowly scoped; a compromised agent key has bounded blast radius.

## Performance consequences
Capability checks are cheap state reads; enforced in the executor.

## Complexity consequences
Adds a capability subsystem; justified by the agent-native requirement.

## Interoperability consequences
Capabilities are consensus objects; any implementation enforces them identically.

## Revisit conditions
If real usage shows the model cannot express a needed policy; evolve via ADR + version bump.
