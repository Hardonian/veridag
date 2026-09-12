# Veridag Quickstart

Veridag is an implementation-independent protocol and lightweight Rust engine for **deterministic, Byzantine-resilient, capability-secured distributed execution**.

This guide takes you from zero to a running 4-validator consensus demo, launching an HTTP JSON-RPC node daemon, querying it via TypeScript and Python SDKs, and verifying crash consistency in under three minutes.

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
round 1..11: max round reached 11
validator 0: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
  checkpoint seq=1 id=0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40
validator 1: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
validator 2: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
validator 3: state_root=0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e checkpoints=1
AGREEMENT OK: identical state root across 4 validators
bob balance: 40 (expected 40)
```

---

## 4. Launching the HTTP / JSON-RPC Node Daemon

Start the production validator node daemon with the built-in HTTP RPC server listening on port `8080`:

```bash
# Launch validator node daemon with HTTP RPC enabled
cargo run -p veridag-node -- run --validator-seed 1 --rpc 0.0.0.0:8080

# In another terminal, query node health & state root
curl http://127.0.0.1:8080/v1/health
curl http://127.0.0.1:8080/v1/state/root
curl http://127.0.0.1:8080/v1/checkpoints/latest
```

---

## 5. Multi-Container Cluster (Docker Compose)

Launch a 4-validator distributed consensus mesh with exposed RPC ports `8081..8084`:

```bash
# Start 4 independent validator nodes over authenticated QUIC mesh
docker compose up -d

# Check cluster logs and consensus agreement
docker compose logs -f

# Query validator node 1 RPC
curl http://localhost:8081/v1/health
```

---

## 6. Cross-Language SDKs (TypeScript & Python)

### TypeScript SDK (`@veridag/sdk`)
```typescript
import { VeridagClient, Keypair, TxBuilder } from "@veridag/sdk";

const client = new VeridagClient("http://127.0.0.1:8080");
const health = await client.health();
console.log("DAG Status:", health.status, "Chain ID:", health.chain_id);

const sender = Keypair.fromSeed(new Uint8Array(32).fill(1));
const recipient = Keypair.fromSeed(new Uint8Array(32).fill(2)).address();
const stx = new TxBuilder(sender).nonce(0).transfer(recipient, 500n);
const res = await client.submitTransaction(stx, sender.public);
console.log("Tx admitted:", res.tx_id);
```

### Python SDK (`veridag`)
```python
from veridag import VeridagClient, Keypair, TxBuilder

client = VeridagClient("http://127.0.0.1:8080")
h = client.health()
print(f"Connected: chain={h['chain_id']} root={h['state_root'][:16]}...")

sender = Keypair.from_seed(b"\x01" * 32)
recipient = Keypair.from_seed(b"\x02" * 32).address()
stx = TxBuilder(sender).nonce(0).transfer(recipient, 500)
res = client.submit_transaction(stx, sender.public())
print("Tx ID:", res["tx_id"])
```

---

## 7. Multi-Process Network Devnet (QUIC)

Spin up 4 distinct OS processes communicating over live authenticated QUIC sockets:
```bash
cargo test -p veridag-net --test devnet -- --nocapture
```

---

## 8. Crash Recovery & Persistence

Verify restart consistency: drop all in-memory state, rebuild from disk, and assert bit-for-bit identical state roots:
```bash
cargo test -p veridag-storage --features persistent
```

---

## 9. Institutional ISO 20022 Banking Bridge

Ingest institutional `pacs.008.001.08` customer credit transfers, automatically apply 1 bps clearing surcharge (80% validator pool, 20% insurance reserve), and receive signed `pacs.002` execution receipts:

```bash
cargo test -p veridag-stablecoin iso20022
```

---

## 10. Developer Toolchain Reference

| Command | Script / Alias | What it does |
| :--- | :--- | :--- |
| `cargo fmt --check` | `just check` | Verify syntax and canonical code formatting |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `just check` | Enforce zero-warning lint gate and `forbid(unsafe_code)` |
| `cargo test --workspace --all-features` | `just check` | Execute full test suite across all 28 crates |
| `pwsh -File scripts/publish-sdks.ps1` | `bash scripts/publish-sdks.sh` | Conformance runner testing Rust, TS, and Python SDKs |
| `cargo build --release` | `cargo build --release` | Produce stripped, `panic=abort` optimized release binary |
| `cargo run -p veridag-node -- demo` | `just demo` | Run the in-process 4-validator consensus demo |
| `cargo test -p veridag-net --test devnet` | `just devnet` | Run the multi-process QUIC network devnet |
| `cargo test -p veridag-consensus --test simulation` | `just sim` | Run the deterministic simulation harness |
| `cargo test -p veridag-testkit --test vectors` | `just vectors` | Run protocol golden vector tests |

---

## 11. Mental Model in One Paragraph

A **transaction** is signed and batched. A **vertex** references a batch and its parents in the DAG, and is signed by its author validator. Vertices **gossip** over QUIC. The **consensus commit rule** is a *pure function* of the DAG: given the same vertices, every node computes the exact same commit point and the same canonical ordering. That ordering is **executed** deterministically (parallel where conflict-free, sequential as the oracle). The resulting **state root** is committed into a **checkpoint** and **persisted**. If a node restarts, it rebuilds from disk and lands on the exact same state.

*Determinism is the product. Consensus is how we achieve it without trusting anyone.*
