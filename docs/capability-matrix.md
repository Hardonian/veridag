# Capability Matrix

This file is the source of truth for product readiness. A capability is not
called production-ready merely because an interface, data type, or unit test
exists.

## Readiness levels

| Level | Meaning |
| :--- | :--- |
| Production | Wired into the validator, exercised end to end, operated through documented recovery procedures, and covered by release gates. |
| Beta | Functional and integrated, but awaiting sustained fault testing, an external security review, or production operating evidence. |
| Experimental | Useful implementation or reference path whose API or security properties may change. |
| Interface only | A typed boundary or deterministic test double; it does not provide the advertised external service. |

## Current status

| Capability | Level | Evidence and remaining gate |
| :--- | :--- | :--- |
| Canonical encoding, signatures, object state | Beta | Golden and malformed vectors pass across the Rust, TypeScript, and Python implementations. External cryptographic review remains. |
| DAG and baseline BFT consensus | Beta | Simulation, vertical-slice, and four-process QUIC tests pass. Multi-region soak, adversarial networking, and independent review remain. |
| Parallel deterministic execution | Beta | Checked against sequential execution in property-style tests. Long-running production workload evidence remains. |
| Validator QUIC transport | Beta | Authenticated committee transport and a four-process test exist. Connection admission, rotation, and chaos evidence remain. |
| Validator daemon and HTTP API | Experimental | Suitable for development networks. Production persistence, external key loading, authenticated RPC, and recovery must be enabled before deployment. |
| Sled persistence and snapshot primitives | Experimental | Reopen and crash-recovery tests exist. Snapshot replacement must be atomic and root-verified, and the daemon must use the store. |
| Wasm runtime | Experimental | Deterministic host-call tests exist. Compatibility, fuzzing, and sandbox review remain. |
| Rust SDK | Beta | Native conformance tests pass. Package/release provenance remains. |
| TypeScript SDK | Experimental | Conformance tests pass; the publish pipeline must build and validate the declared `dist` entry point. |
| Python SDK | Experimental | Conformance tests pass; package metadata and release installation must be validated. |
| Cloud KMS/HSM | Interface only | `KeySigner` is an integration boundary. No cloud or PKCS#11 provider is currently implemented. |
| SP1 and RISC Zero | Interface only | Current adapters produce deterministic test packets, not cryptographic zkVM proofs. |
| Ethereum JSON-RPC | Interface only | A small compatibility facade exists; it is not an EVM implementation or production sequencer. |
| Bitcoin SPV | Experimental | Header and proof primitives are tested; production chain synchronization and reorg operations remain. |
| Solidity contracts and bridge | Experimental | Contracts are unaudited until contract tests, invariant/fuzz gates, static analysis, and independent review pass. |
| USDV and ISO 20022 | Experimental | Deterministic domain logic exists. It is not a licensed financial product, reserve program, sanctions service, or legal authorization. |
| Explorer and faucet | Demonstration | Static product demonstrations; they are not connected to a live network. |

## Promotion rule

Every promotion must link to all applicable evidence: conformance tests,
integration tests, benchmarks with a reproducible environment, threat-model
coverage, recovery drills, deployment documentation, and an independent
review. Unsupported paths fail closed and remain clearly labeled.
