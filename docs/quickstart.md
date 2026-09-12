# Veridag Quickstart

Veridag is an implementation-independent protocol and lightweight Rust engine for **deterministic, Byzantine-resilient, capability-secured distributed execution**.

This guide takes you from zero to a running 4-validator consensus demo in under three minutes, verifies multi-process network agreement, tests crash consistency, and provides role-based next steps.

> **Universal by design.** The same Rust core runs on a developer laptop, a commodity cloud VM, or a single-board computer. No external database, no message broker, no orchestrator. One static binary + a local data directory.

---

## 1. Prerequisites & Toolchain Setup

### Linux & macOS
```bash
# 1. Install Rust via official installer (requires Rust >= 1.85, edition 2021)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable

# 2. (Optional) Install 'just' command runner
cargo install just
```

### Windows (PowerShell or WSL2)
```powershell
# In PowerShell:
winget install Rustlang.Rustup
rustup default stable

# Or under WSL2 (Ubuntu / Debian):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

---

## 2. Clone the Repository

```bash
git clone https://github.com/Hardonian/veridag.git
cd veridag
```

---

## 3. Your First Consensus Run (In-Process Demo)

The fastest way to see the protocol work: a single process that initializes a 4-validator committee, executes a capability-checked transfer (`alice -> bob 40`), builds a DAG across rounds, computes the pure-function commit rule, and emits a cryptographic checkpoint.

Using `cargo`:
```bash
cargo run -p veridag-node -- demo
```
Or using `just`:
```bash
just demo
```

### Expected Output:
```text
veridag-node demo: 4-validator committee, in-process
submitted transfer alice->bob 40 to all mempools
round 1: max round reached 1
round 2: max round reached 2
round 3: max round reached 3
...
round 11: max round reached 11
validator 0: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
  checkpoint seq=1 id=0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40
validator 1: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
  checkpoint seq=1 id=0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40
validator 2: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
  checkpoint seq=1 id=0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40
validator 3: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
  checkpoint seq=1 id=0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40
AGREEMENT OK: identical state root across 4 validators
bob balance: 40 (expected 40)
```

> **Why this matters:** Identical roots across validators = exact mathematical agreement. That is the fundamental guarantee of Byzantine Fault Tolerance.

---

## 4. The Real Thing: 4 Independent Validator Processes Over QUIC

The demo above is in-process. To prove the network layer, run four *actual* OS processes that communicate over real QUIC sockets:

```bash
cargo test -p veridag-net --test devnet -- --nocapture
```
Or:
```bash
just devnet
```

This spins up four independent validators, connects them over authenticated QUIC with self-signed Ed25519 TLS certificates, propagates vertices and transaction batches, and asserts they all reach the same committed wave with the same state root.

---

## 5. Crash Recovery & Restart-Safety

Veridag persists every vertex and the full object state to an embedded `sled` database. If a validator crashes between a commit and a checkpoint, it recovers byte-for-byte:

```bash
cargo test -p veridag-storage --features persistent
```

The `crash_recovery` harness builds a DAG, persists it, **drops all in-memory state** (simulated crash), reopens from disk, rebuilds the DAG, re-runs consensus, and asserts the recovered state root equals the pre-crash root.

---

## 6. Validator Node Health Check

The node binary includes a self-test command (`health`) that validates consensus, checkpoints, and state agreement, emitting human-readable or machine-parseable JSON:

```bash
# Human-readable summary
cargo run -p veridag-node -- health

# Machine-readable JSON (ideal for ops monitors, Prometheus exporters, and dashboards)
cargo run -p veridag-node -- health --json
```

Sample JSON output:
```json
{
  "binary": "veridag-node",
  "version": "0.1.0-alpha",
  "protocol_version": 1,
  "chain_id": 1,
  "n_validators": 4,
  "committee_n": 4,
  "committee_quorum": 3,
  "highest_complete_wave": 2,
  "max_round": 11,
  "state_root": "ac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
  "committed_tx_count": 4,
  "checkpoint_count": 1,
  "checkpoint_ids": ["3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40"],
  "agreement": true
}
```

---

## 7. Developer Toolchain Reference

| Command | `just` Alias | What it does |
|---------|--------------|--------------|
| `cargo fmt --check` | `just check` | Verify syntax and canonical code formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `just check` | Enforce zero-warning lint gate and `#![forbid(unsafe_code)]` |
| `cargo test --workspace --all-features` | `just check` | Execute full test suite across all 15 crates |
| `cargo build --release` | `cargo build --release` | Produce stripped, `panic=abort` optimized release binary |
| `cargo run -p veridag-node -- demo` | `just demo` | Run the in-process 4-validator consensus demo |
| `cargo test -p veridag-net --test devnet -- --nocapture` | `just devnet` | Run the multi-process QUIC network devnet |
| `cargo test -p veridag-consensus --test simulation` | `just sim` | Run the deterministic simulation harness |
| `cargo test -p veridag-testkit --test vectors` | `just vectors` | Run protocol golden vector tests |

---

## 8. Where to Go Next, by Role

* **Protocol Engineers & Researchers:**
  * Read the normative protocol specifications in [`protocol/specification/00-overview.md`](../protocol/specification/00-overview.md).
  * Inspect the executable formal model in [`formal/quint/consensus.qnt`](../formal/quint/consensus.qnt).
* **Application & AI Agent Developers:**
  * Learn how to construct signed transactions with [`veridag-transaction`](../implementations/rust/crates/transaction).
  * Explore the account and balance state model in [`veridag-object-state`](../implementations/rust/crates/object-state).
  * Use the developer CLI via `cargo run -p veridag-cli -- --help`.
* **Node Operators & Infrastructure Teams:**
  * Review the validator runbook in [`implementations/rust/RUNBOOK.md`](../implementations/rust/RUNBOOK.md).
  * Read the threat and adversarial assumptions in [`docs/threat-model.md`](threat-model.md).
* **Contributors:**
  * Check [`CONTRIBUTING.md`](../CONTRIBUTING.md) and [`ROADMAP.md`](../ROADMAP.md) for phased development guidelines.

---

## 9. Mental Model in One Paragraph

A **transaction** is signed and batched. A **vertex** references a batch and its parents in the DAG, and is signed by its author validator. Vertices **gossip** over QUIC. The **consensus commit rule** is a *pure function* of the DAG: given the same vertices, every node computes the exact same commit point and the same canonical ordering. That ordering is **executed** deterministically (parallel where conflict-free, sequential as the oracle). The resulting **state root** is committed into a **checkpoint** and **persisted**. If a node restarts, it rebuilds from disk and lands on the exact same state.

*Determinism is the product. Consensus is how we achieve it without trusting anyone.*
