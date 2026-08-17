# Veridag Quickstart

Veridag is a deterministic, Byzantine-resilient, capability-secured distributed
execution substrate. This guide gets you from zero to a running 4-validator
consensus demo in under five minutes, then points you at the right depth for
your role.

> **Universal by design.** The same Rust core runs on a developer laptop, a
> commodity cloud VM, or a single-board computer. No external database, no
> message broker, no orchestrator. One static binary + a data directory.

---

## 1. Prerequisites

* **Rust** (edition 2021, `rust-version >= 1.85`). Install with
  [rustup](https://rustup.rs):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup default stable
  ```
* A UNIX-like shell (Linux, macOS, WSL2, or FreeBSD). Windows works under WSL2.

That is the entire dependency list. There is nothing else to install.

---

## 2. Get the code

```bash
git clone https://github.com/Hardonian/veridag.git
cd veridag/implementations/rust
```

---

## 3. Your first consensus run (in-process demo)

The fastest way to *see* the protocol work: a single process that simulates a
4-validator committee, executes one transfer, commits a wave, and checkpoints.

```bash
cargo run -p veridag-node -- demo
```

You will see four validators print the **same** state root and the **same**
checkpoint id:

```
validator 0: state_root=0xf7aa1731... checkpoints=1
validator 1: state_root=0xf7aa1731... checkpoints=1
validator 2: state_root=0xf7aa1731... checkpoints=1
validator 3: state_root=0xf7aa1731... checkpoints=1
```

Identical roots across validators = agreement. That is the whole point of BFT.

---

## 4. The real thing: 4 separate validator processes over QUIC

The demo above is in-process. To prove the network layer, run four *actual*
OS processes that gossip over real QUIC sockets:

```bash
cargo test -p veridag-net --test devnet -- --nocapture
```

This spins up four independent validators, connects them over authenticated
QUIC, propagates vertices and transaction batches, and asserts they all reach
the same committed wave with the same state root. (It is a test, but it is a
real multi-process network — not a simulation.)

---

## 5. Crash recovery (restart-safety)

Veridag persists every vertex and the full object state to an embedded `sled`
database. If a validator dies between a commit and a checkpoint, it recovers
byte-for-byte:

```bash
cargo test -p veridag-storage --features persistent
```

The `crash_recovery` test builds a DAG, persists it, **drops all in-memory
state** (simulated crash), reopens from disk, rebuilds the DAG, re-runs
consensus, and asserts the recovered state root equals the pre-crash root.

---

## 6. The developer toolchain

| Command | What it does |
|---------|--------------|
| `cargo fmt --all -- --check` | Verify formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Zero-warning lint gate |
| `cargo test --workspace --all-features` | Full test suite (consensus, execution, net, storage) |
| `cargo build --release` | Optimized, stripped, `panic=abort` binary |

A clean tree means: `clippy` is warning-free and **every** test target is green.

---

## 7. Where to go next, by role

**I want to understand the protocol.**
→ `protocol/specification/00-overview.md` (normative) and
`formal/quint/consensus.qnt` (executable model). The implementation is correct
only if it satisfies both.

**I want to build an app on Veridag.**
→ Start with `veridag-transaction` (how to construct a signed transfer) and
`veridag-object-state` (the account/balance model). The crates are SDK-ready:
no `unsafe`, stable VCE-1 wire form, and `veridag-cli`/`veridag-node` as
reference clients. TypeScript/Python/Go SDKs are on the roadmap (Phase 13).

**I want to run a validator.**
→ `veridag-node` is the entrypoint. Keys are managed via `veridag-cli key
generate <name>`. State lives in a local `sled` directory — back that up; you
do not need a separate database.

**I want to contribute.**
→ `CONTRIBUTING.md` + `ROADMAP.md`. Every phase has a Definition of Done backed
by real artifacts. We do not claim "done" without tests.

**I want the architecture.**
→ `docs/architecture.md` (crate map, data flow, why QUIC + sled, safety
posture). `docs/threat-model.md` and `docs/security-model.md` for the
adversarial assumptions.

---

## 8. Mental model in one paragraph

A **transaction** is signed and batched. A **vertex** references a batch and its
parents in the DAG, and is signed by its author validator. Vertices **gossip**
over QUIC. The **consensus commit rule** is a *pure function* of the DAG: given
the same vertices, every node picks the same commit point and the same canonical
ordering. That ordering is **executed** deterministically (parallel where
conflict-free, sequential as the oracle). The resulting **state root** is
committed into a **checkpoint** and **persisted**. If a node restarts, it
rebuilds from disk and lands on the exact same state.

Determinism is the product. Consensus is how we get it without trusting anyone.
