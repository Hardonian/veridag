# 14 — Networking (Scoped Draft for v0.1)

Status: SCOPED DRAFT — wire details are implementation-level in v0.1; the rules
below that affect consensus safety are NORMATIVE.

Two conceptual planes:

## Validator fast path (purpose-built QUIC)

Used for: DAG vertices, consensus-critical sync, validator batch exchange,
checkpoints, validator control. Implementations MUST:

* authenticate each peer's networking identity bound to its `ValidatorId`;
* enforce maximum frame size, maximum message size, maximum outstanding requests,
  per-peer request rate, connection limits, queue bounds, timeouts, duplicate
  suppression, and protocol-version checks;
* use backpressure; never spawn unbounded tasks; never let one malicious peer
  allocate arbitrary memory.

## Public P2P plane (selective libp2p)

Used for: peer discovery, bootstrapping, NAT traversal, non-validator peer
identity, public transaction propagation, general sync. libp2p is NOT a consensus
dependency: the validator fast path MUST function without it.

## Normative boundary

The only consensus-visible networking facts are: (a) vertices/batches are
eventually delivered to honest validators (liveness assumption), and (b) every
delivered object is authenticated and canonically decodable. Delivery order,
timing, and duplication are NOT consensus-visible.
