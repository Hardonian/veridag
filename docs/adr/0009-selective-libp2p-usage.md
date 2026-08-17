# ADR-0009: Selective libp2p usage

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Public nodes need discovery, NAT traversal, and broad interoperability, but libp2p must not become a consensus dependency.

## Decision
Use libp2p only on the public P2P plane (discovery, bootstrapping, NAT, public tx propagation, general sync). The validator fast path does not depend on it.

## Alternatives
libp2p everywhere (rejected: couples consensus to a large dependency); no libp2p (rejected: reinvents discovery/NAT poorly).

## Security consequences
Limits libp2p's blast radius; public-plane attacks cannot directly reach the validator fast path.

## Performance consequences
Pays libp2p cost only on the public plane.

## Complexity consequences
Two planes to operate; narrow internal interfaces keep them decoupled.

## Interoperability consequences
Public plane can interoperate with the libp2p ecosystem.

## Revisit conditions
If the public plane's requirements shrink or a lighter discovery mechanism suffices.
