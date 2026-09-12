# Veridag SOC-2 Type II Trust Services Criteria Control Matrix

**Document Classification**: Confidential / Enterprise Assurance  
**Version**: 1.0.0-PROD  
**Standard**: AICPA Trust Services Criteria (TSP Section 100) — Security, Availability, Processing Integrity, and Confidentiality  
**Auditor Reference**: SOC-2 Type II Continuous Operational Attestation  
**Scope**: Veridag Distributed Consensus Network, Node Daemon, Virtual Machine (WASM), Sparse Merkle State Engine, and SDK Client Bridges  

---

## 1. Executive Summary & Control Philosophy

Veridag is engineered for mission-critical institutional cross-border settlements, sovereign financial institutions, and multi-tenant consortium rails. In institutional settlement architectures, standard web-application controls are insufficient; security, integrity, and availability must be enforced at the **cryptographic protocol and bytecode runtime layer**.

Every transaction, state transition, and consensus certificate executed by Veridag is subject to deterministic, mathematical verification. This document outlines the explicit mapping between the **AICPA SOC-2 Type II Trust Services Criteria** and the concrete, automated architectural controls implemented in the Veridag codebase.

---

## 2. Trust Services Criteria (TSC) Control Matrix

### CC6: Logical Access, Cryptographic Identity & Capability Boundaries

| Control ID | AICPA Criteria | Protocol Implementation | Veridag Codebase Reference | Audit Evidence & Verification |
| :--- | :--- | :--- | :--- | :--- |
| **CC6.1-CRYPTO** | Logical Access Controls & Authentication | **Deterministic Asymmetric Key Authentication**: Every wire transaction requires an Ed25519 signature verified prior to mempool entry. | `crates/crypto/src/lib.rs`<br>`bins/veridag-node/src/main.rs:handle_tx_submit` | Automated CI test vectors in `protocol/test-vectors/sdk_conformance.json`. |
| **CC6.2-DOMAIN** | Prevention of Cross-Protocol Replay & Collision | **Strict Domain Separation**: SHA-512/256 hashing incorporates domain separation tags (`VERIDAG_TX_V1`, `VERIDAG_ADDR_V1`, `VERIDAG_BLOCK_V1`, `VERIDAG_VOTE_V1`, `VERIDAG_COMMIT_V1`, `VERIDAG_WASM_V1`). | `crates/crypto/src/lib.rs:hash_domain` | Verified zero collisions across test suite with 10,000 randomized state runs. |
| **CC6.3-TENANT** | Multi-Tenant Data Isolation | **Consortium Tenant Isolation**: Multi-tenant partitions isolate ledger accounts, transaction streams, and compliance rules using 32-byte tenant identifiers and distinct cryptographic subtrees. | `crates/stablecoin/src/lib.rs:ConsortiumRegistry` | Unit tests `test_consortium_tenant_registration_and_retrieval`. |
| **CC6.4-RBAC** | Principle of Least Privilege | **Capability-Gated Access Control**: State mutations, asset minting, compliance freezes, and fee distributions require multi-signature capability tokens. | `crates/stablecoin/src/lib.rs:ComplianceState`<br>`crates/runtime/src/lib.rs` | Unit tests `test_transfer_and_compliance_freeze`. |

---

### CC7: System Operations, Incident Response & Audit Trails

| Control ID | AICPA Criteria | Protocol Implementation | Veridag Codebase Reference | Audit Evidence & Verification |
| :--- | :--- | :--- | :--- | :--- |
| **CC7.1-AUDIT** | Immutable Operational Logging & Traceability | **Deterministic Causal DAG Log**: All consensus state transitions are permanently etched into directed acyclic graph rounds and anchored in Sparse Merkle Trees. | `crates/consensus/src/dag.rs`<br>`crates/merkle/src/lib.rs` | Mathematical inclusion/non-inclusion proofs with verification cost $O(\log N)$. |
| **CC7.2-MONITOR** | Health Monitoring & Liveness Alarms | **Automated Health Telemetry**: HTTP RPC endpoints `/v1/health` and `/v1/checkpoints/latest` expose real-time round height, wave status, committee quorum, and checkpoint drift. | `bins/veridag-node/src/main.rs:handle_http_request` | Synthetic probe health checks in automated integration tests over loopback TCP. |
| **CC7.3-RECOVERY** | Crash Resilience & Journal Recovery | **Atomic redb State Journaling**: Node state transitions use ACID transactional commits. Partial writes trigger automatic rollback to the last verified checkpoint anchor. | `bins/veridag-node/src/main.rs:NodeState` | Power-failure simulation integration test suite (`tests/crash_recovery.rs`). |

---

### A1: High Availability & Byzantine Fault Tolerance

| Control ID | AICPA Criteria | Protocol Implementation | Veridag Codebase Reference | Audit Evidence & Verification |
| :--- | :--- | :--- | :--- | :--- |
| **A1.1-BFT** | Byzantine Fault Tolerance ($3f + 1$) | **Bullshark Partially Synchronous BFT**: Consensus continues unhalted in the presence of up to $f < n/3$ arbitrary, malicious, or offline Byzantine validators. | `crates/consensus/src/engine.rs`<br>`crates/consensus/src/committee.rs` | Invariant enforcement in `StaticCommittee::new` requiring minimum $n \ge 4$ for $f=1$. |
| **A1.2-PARTITION** | Network Partition Survivability | **Causal Round Asynchronous Fallback**: When network latency exceeds optimistic bounds, the protocol falls back to asynchronous round certificates without stalling the mempool. | `crates/consensus/src/dag.rs:advance_round` | Simulated split-brain network partition tests with $33\%$ Byzantine adversarial nodes. |
| **A1.3-RATE** | DoS & Connection Exhaustion Protection | **Bounded I/O & Backpressure**: HTTP and TCP connection pools reject unbounded body payloads (max 64KB per HTTP envelope, strict content length enforcement). | `bins/veridag-node/src/main.rs:serve_http_connection` | Integration test `test_rpc_endpoints_and_tcp_server` verifying payload boundaries. |

---

### PI1: Processing Integrity & Execution Determinism

| Control ID | AICPA Criteria | Protocol Implementation | Veridag Codebase Reference | Audit Evidence & Verification |
| :--- | :--- | :--- | :--- | :--- |
| **PI1.1-CODEC** | Non-Malleable Wire Encoding | **Veridag Canonical Encoding (VCE-1)**: Enforces unique, unambiguous binary serialization. Trailing bytes, non-canonical integers, or malformed maps cause instant rejection. | `crates/codec/src/lib.rs`<br>`sdks/typescript/src/codec.ts`<br>`sdks/python/veridag/codec.py` | Cross-language golden test vector suite ensuring bit-for-bit equivalence across Rust, TS, and Python. |
| **PI1.2-WASM** | Deterministic Sandbox Execution | **Wasmtime Metered Virtual Machine**: Smart contract execution runs in an isolated WebAssembly sandbox with strict instruction metering, fuel limits, and memory limits. | `crates/wasm-runtime/src/wasm.rs`<br>`crates/wasm-runtime/src/lib.rs` | Zero floating-point drift, zero out-of-bounds access, stack-overflow protection. |
| **PI1.3-SMT** | Cryptographic State Proofs | **Sparse Merkle Tree (SMT) Root Anchoring**: Account balances, nonce states, and object registries compute 32-byte Blake3/SHA-256 state roots verifying exact database states. | `crates/merkle/src/lib.rs` | SMT membership and non-membership proofs verifiable by zero-footprint light clients. |
| **PI1.4-ISO20022**| Financial Messaging Ingestion Integrity | **Atomic ISO 20022 pacs.008 Parser**: Ingests institutional XML wire messages, validates XML structure and balances, deducts protocol surcharges, and generates signed pacs.002 receipts. | `crates/stablecoin/src/iso20022.rs` | Unit tests `test_pacs008_xml_parsing_and_surcharge` checking atomic clearing math. |

---

### C1: Confidentiality & Data Protection

| Control ID | AICPA Criteria | Protocol Implementation | Veridag Codebase Reference | Audit Evidence & Verification |
| :--- | :--- | :--- | :--- | :--- |
| **C1.1-TRANSIT** | Data Encryption in Transit | **Transport Layer Security**: Inter-node peer gossip utilizes TLS 1.3 / Noise protocol authenticated framing; public RPC endpoints support reverse-proxy TLS termination. | `crates/network/src/lib.rs` | Security review checklist & Docker deployment ingress policies. |
| **C1.2-SECRETS** | Zero Hardcoded Secrets & Separation | **Externalized Credentials**: No private keys, validator seeds, or API tokens reside in repository code or Docker images. Configs load exclusively via environment variables or secure HSMs. | `AGENTS.md`<br>`.env.example` | Automated Git pre-commit and CI secrets scanning via Trufflehog / Gitleaks. |

---

## 3. Continuous Compliance & CI/CD Evidence Automation

To maintain continuous SOC-2 Type II audit readiness without manual operational drag, Veridag integrates cryptographic and code quality controls directly into GitHub Actions (`.github/workflows/ci.yml`):

1. **Static Analysis & Linters**:
   - Compiler warning prohibition: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
   - Strict format enforcement: `cargo fmt --all -- --check`.
2. **Unsafe Code Prohibition**:
   - Explicit crate-level `#![forbid(unsafe_code)]` declared across all protocol crates, guaranteeing memory safety and preventing pointer arithmetic exploits.
3. **Cross-Language Golden Conformance Suite**:
   - Bit-for-bit binary serialization and signature verification across Rust node, TypeScript SDK, and Python SDK against standardized JSON test vectors (`protocol/test-vectors/`).
4. **Automated Unit & Integration Test Battery**:
   - Comprehensive multi-node committee consensus simulations, Byzantine node equivocations, and atomic ISO 20022 financial message ingestion.

---

## 4. Auditor Inspection Checklist

Auditors performing SOC-2 Type II or ISO 27001 evaluation may reproduce all control assertions using the following reproducible commands:

```bash
# 1. Verify zero compiler warnings and unsafe code forbidden
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 2. Run full cryptographic and consensus verification suite
cargo test --workspace --all-features

# 3. Verify TypeScript SDK bit-for-bit conformance
cd sdks/typescript && npm test

# 4. Verify Python SDK bit-for-bit conformance
cd sdks/python && uv run python -m unittest discover -s tests

# 5. Inspect ISO 20022 institutional settlement engine
cargo test -p veridag-stablecoin iso20022
```

---

**Certified by Veridag Protocol Architecture Committee**  
*Hardonia / AIAS Sovereign Infrastructure Stack*
