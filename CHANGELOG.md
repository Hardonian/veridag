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
