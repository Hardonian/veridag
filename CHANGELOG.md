# Changelog

All notable changes to this project are documented here. The format follows
Keep a Changelog; versions follow SemVer. Protocol version is distinct from
software version (see `protocol/specification/17-upgrades.md`).

## [0.1.0-alpha] - 2026-08-17

First reference-implementation alpha. The consensus + execution + persistence +
networking slice is complete and tested.

### Added
- Phase 5: `veridag-net` — QUIC authenticated validator links + vertex/batch
  gossip. Multi-process devnet test reaches consensus over real sockets.
- Phase 6: `veridag-dag` — VCE-1 vertex wire form, validity, equivocation,
  quorum progression.
- Phase 7: `veridag-consensus` — BaselineDagBft pure-function commit rule,
  deterministic simulation (Agreement + order-independence).
- Phase 8: vertical slice — tx → batch → vertex → commit → order → execute →
  state root; 4 validators derive identical state.
- Phase 9: `veridag-storage` — trait-based stores + `MemoryStore`/`SledStore`.
  Crash-injection harness proves restart-safe recovery (identical state root).
- Phase 10: `veridag-execution` — conflict-aware parallel scheduler;
  parallel == sequential property-tested.
- Phase 11: `veridag-net` — Public libp2p discovery plane over Floodsub/Noise/Yamux with strict consensus DAG QUIC mesh isolation.
- Phase 12: `veridag-wasm-runtime` — Deterministic Wasm runtime with capability-scoped host ABI, fuel metering, and Wasmtime v48 engine.
- Phase 13: Developer SDKs — Idiomatic Rust, TypeScript, and Python SDKs with bit-for-bit wire serialization and signing conformance.
- Phase 14: `veridag-light-client` — 2f+1 quorum checkpoint verification, epoch tracking, and BMH-1 Merkle inclusion proofs.
- Phase 15: `veridag-zkvm` — Pluggable zkVM state validity proofs (Mock, SP1, RiscZero).
- Phase 16: `veridag-da` — 2D Reed-Solomon tensor erasure coding with iterative recovery and validator replication schemes.
- Phase 17: `veridag-da::hw_accel` — Chunk-unrolled SIMD vector XOR and batch GF(2^8) acceleration hooks.
- Phase 18: `veridag-stablecoin` — USMCA & G8 multilateral sovereign digital dollar (USDV), Proof-of-Reserves, and OFAC sanctions compliance.
- Phase 19: `veridag-ethereum` — EVM JSON-RPC provider, BMH-1 Merkle inclusion proofs, and Solidity contracts (`USDV.sol`, `VeridagBridge.sol`, `VeridagLightClient.sol`).
- Phase 20: `veridag-consensus` — Dynamic committee reconfiguration with weighted stakes (2W/3 + 1) and seamless epoch handovers.
- Phase 21: `veridag-metrics` — Enterprise OpenMetrics/Prometheus exposition exporter with DAG throughput and TPS telemetry.
- Phase 22: `veridag-storage` — State snapshotting, archival pruning policies, and institutional fast sync protocol.
- Phase 23: `veridag-crypto` — Pluggable `KeySigner` substrate supporting AWS KMS, GCP KMS, Azure Key Vault, and PKCS#11 HSMs.
- Phase 24: `veridag-execution` — Native multi-chain asset typing (`Usdv`, `Btc`, `Eth`, `Sol`) and speculative parallel batch compaction.
- Phase 25: `veridag-bitcoin` — Bitcoin SPV client with canonical 80-byte header parser, nBits PoW validation, and UTXO bridge codecs.
- Phase 26: Hardonian Stack & Settler Native Layer — Deep integration with Settler reconciliation engine, proofpack anchors, and sovereign AI stack.
- `veridag-node`, `veridag-cli`, `veridag-genesis` binaries.
- Release profile: `opt-level=3`, thin LTO, `panic=abort`, `strip=true`
  (low-latency, low-energy, small binary).
- Docs: `docs/quickstart.md` (universal onboarding), updated
  `docs/architecture.md`, ROADMAP phase status.

### Safety
- All crates `#![forbid(unsafe_code)]`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- Full workspace test suite green.

## [Unreleased]

### Added
- Phase 0: normative protocol specification 00–18.
- Phase 1: Quint formal model (`consensus.qnt`, `invariants.qnt`) with
  Agreement/Finality/Integrity invariants.
- Phase 2: golden and malformed test vectors.
- Phase 3: Rust protocol foundation crates.
- Phase 4: sequential deterministic state machine.
- Governance docs, ADRs 0001–0015, CI.
