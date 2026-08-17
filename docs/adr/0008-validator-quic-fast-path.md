# ADR-0008: Validator QUIC fast path

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Validators need authenticated, low-latency, bounded transport for consensus-critical traffic, independent of the public P2P plane.

## Decision
Purpose-built QUIC protocol for validator traffic: vertices, sync, batch exchange, checkpoints, control.

## Alternatives
TCP+TLS (rejected: head-of-line blocking, weaker stream model); libp2p for validator traffic (rejected as a consensus dependency; kept for the public plane).

## Security consequences
Authenticated peer identity bound to ValidatorId; bounded frames/messages/queues; backpressure; no unbounded tasks.

## Performance consequences
QUIC streams and 0-RTT-style resumption help latency; measured before claims.

## Complexity consequences
Maintaining a narrow validator protocol is work; offset by independence from libp2p internals.

## Interoperability consequences
Wire format is versioned; consensus semantics are transport-independent.

## Revisit conditions
If QUIC ecosystem support regresses or a better transport emerges with clear wins.
