# 14 — Networking

Status: NORMATIVE  
Version: 1.0  
Scope: Authenticated Validator QUIC Fast-Path, Selective libp2p Discovery Plane, Bounded Resource Defense, and Consensus Decoupling

The Veridag networking substrate cleanly decouples latency-critical validator consensus traffic from open public peer discovery and transaction submission.

---

## 1. Dual-Plane Architecture

```text
               +-------------------------------------------+
               |        Public P2P Plane (libp2p)          |
               | - Peer discovery (Kademlia DHT)           |
               | - Public transaction submission & gossip  |
               | - NAT traversal & bootstrap relays        |
               +---------------------+---------------------+
                                     |
                                     v
                        [ Ingress Rate Limiter ]
                                     |
+------------------------------------+------------------------------------+
|                                                                         |
|                 Validator Fast Path Plane (QUIC / TLS 1.3)              |
|                                                                         |
|  Validator A <==== Authenticated Bi-directional Streams ====> Validator B |
|  - Ed25519 TLS 1.3 mutual auth bound to ValidatorId                     |
|  - Sub-millisecond DAG vertex propagation                               |
|  - Checkpoint finality signature aggregation                            |
|  - Bounded buffers, strict frame limits, zero unbounded allocation      |
+-------------------------------------------------------------------------+
```

---

## 2. Validator Fast Path Plane (QUIC / TLS 1.3)

The validator fast path is purpose-built over QUIC (ALPN `veridag/v1`) using TLS 1.3 with mutually authenticated Ed25519 certificates.

### 2.1 Identity Binding & Mutual Authentication
1. Each validator generates an ephemeral or persistent TLS certificate where the public key matches its registered consensus `ValidatorId`.
2. Connections from unknown or non-committee public keys on the fast path port MUST be rejected immediately during TLS handshake.
3. Fast path sockets MUST NOT be exposed to untrusted public clients.

### 2.2 Wire Framing & Bounded Resource Invariants
To prevent memory exhaustion attacks and slowloris vulnerabilities:
* **Max Frame Size:** Strict limit of 4 MiB (`4,194,304` bytes) per frame. Frames exceeding this limit MUST cause immediate stream termination.
* **Max Message Size:** Maximum decoded message buffer of 16 MiB.
* **Connection Limits:** Maximum of 2 concurrent QUIC connections per active peer validator; redundant connection attempts are deduped.
* **Stream Backpressure:** Channel buffers between QUIC streams and the consensus DAG pool are strictly bounded (default 1,024 items). Excess proposals trigger backpressure at the transport layer without dropping or panicking.
* **Timeouts:** Handshake timeout = 5s; RPC request timeout = 10s; idle keepalive = 30s.

---

## 3. Public P2P Plane (Selective libp2p)

Used exclusively for open peer discovery, bootstrap routing, and unprivileged transaction gossip.

1. **Sovereign Decoupling:** libp2p is strictly an auxiliary plane. The consensus committee and fast path MUST remain fully functional even if the libp2p network is partitioned or disabled.
2. **Admission Ingress:** Transactions arriving via public gossip pass through strict VCE-1 format parsing, anti-replay nonces, and capability checks before being batched into local memory pools.
3. **Sybil & Flooding Protection:** Per-IP connection and rate limits enforce that no single peer can consume more than 100 requests/sec.

---

## 4. Normative Boundary

The only consensus-visible networking facts are:
1. **Eventual Delivery:** Vertices and checkpoints are eventually delivered to honest validators (liveness assumption).
2. **Cryptographic Authenticity:** Every delivered object is canonically decodable under VCE-1 and signed by an authorized key.
3. **Transport Independence:** Delivery order, transport packetization, latency jitter, and stream multiplexing are NOT consensus-visible and do not affect deterministic state commitment.

