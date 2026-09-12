export default function Quickstart() {
  return (
    <div>
      <h1>Quickstart</h1>
      <p className="tagline">
        From zero to a running 4-validator Byzantine consensus demo, HTTP JSON-RPC daemon,
        and cross-language SDK client in under three minutes.
      </p>

      <div className="alert">
        <div className="alert-title">⚡ Universal Single-Binary Footprint</div>
        <p style={{ margin: 0, fontSize: "14px", color: "#c7d2e0" }}>
          The exact same deterministic Rust core runs on a developer laptop, commodity cloud VM, or Raspberry Pi single-board computer.
          No Docker, no Kubernetes, no Postgres, no Redis, and no message brokers required.
        </p>
      </div>

      <h2>1. Prerequisites &amp; Setup</h2>
      <p>Veridag requires only standard <strong>Rust 1.85+</strong> (edition 2021):</p>
      <pre><code>{`# Linux, macOS, WSL2:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Windows PowerShell:
winget install Rustlang.Rustup
rustup default stable`}</code></pre>

      <h2>2. Get the Code</h2>
      <pre><code>{`git clone https://github.com/Hardonian/veridag.git
cd veridag`}</code></pre>

      <h2>3. First Consensus Run (In-Process Demo)</h2>
      <p>
        Execute a complete 4-validator committee in one process: initializes genesis balances,
        executes an <code>alice -&gt; bob 40</code> transfer, constructs DAG rounds, computes the pure-function
        commit rule, and issues a cryptographic checkpoint:
      </p>
      <pre><code>{`cargo run -p veridag-node -- demo`}</code></pre>
      <p>Expected output:</p>
      <div className="terminal-box" style={{ margin: "16px 0" }}>
        <div className="terminal-body" style={{ padding: "16px" }}>
          <div><span className="terminal-prompt">$ </span>cargo run -p veridag-node -- demo</div>
          <div className="muted">veridag-node demo: 4-validator committee, in-process</div>
          <div className="muted">submitted transfer alice-&gt;bob 40 to all mempools</div>
          <div className="terminal-highlight">round 1..11: max round reached 11</div>
          <div>validator 0: state_root=<span className="terminal-success">0xac049e6fdadc2840...</span> checkpoints=1</div>
          <div>&nbsp;&nbsp;checkpoint seq=1 id=0x3a875556df63e5ff...</div>
          <div>validator 1: state_root=<span className="terminal-success">0xac049e6fdadc2840...</span> checkpoints=1</div>
          <div>validator 2: state_root=<span className="terminal-success">0xac049e6fdadc2840...</span> checkpoints=1</div>
          <div>validator 3: state_root=<span className="terminal-success">0xac049e6fdadc2840...</span> checkpoints=1</div>
          <div className="terminal-success">AGREEMENT OK: identical state root across 4 validators</div>
          <div className="terminal-highlight">bob balance: 40 (expected 40)</div>
        </div>
      </div>

      <h2>4. Launching the HTTP / JSON-RPC Node Daemon</h2>
      <p>
        Start the production validator node daemon with the built-in HTTP RPC server listening on port <code>8080</code>:
      </p>
      <pre><code>{`# Launch validator node daemon with HTTP RPC enabled
cargo run -p veridag-node -- run --validator-seed 1 --rpc 0.0.0.0:8080

# In another terminal, query node health & state root
curl http://127.0.0.1:8080/v1/health
curl http://127.0.0.1:8080/v1/state/root
curl http://127.0.0.1:8080/v1/checkpoints/latest`}</code></pre>

      <h2>5. Multi-Container Topology (Docker Compose)</h2>
      <p>
        Launch a 4-validator distributed consensus mesh with exposed RPC ports <code>8081..8084</code>:
      </p>
      <pre><code>{`# Start 4 independent validator nodes over authenticated QUIC mesh
docker compose up -d

# Check cluster logs and consensus agreement
docker compose logs -f

# Query validator node 1 RPC
curl http://localhost:8081/v1/health`}</code></pre>

      <h2>6. Cross-Language SDK Quickstart</h2>
      <div className="card-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))", gap: "20px", margin: "20px 0" }}>
        <div className="card">
          <h3>TypeScript SDK (<code>@veridag/sdk</code>)</h3>
          <p className="muted" style={{ fontSize: "13px" }}>Native Web Crypto / ESM client with zero runtime bloat:</p>
          <pre style={{ margin: "10px 0" }}><code>{`import { VeridagClient, Keypair, TxBuilder } from "@veridag/sdk";

const client = new VeridagClient("http://127.0.0.1:8080");
const health = await client.health();
console.log("DAG Status:", health.status, "Chain ID:", health.chain_id);

const sender = Keypair.fromSeed(new Uint8Array(32).fill(1));
const recipient = Keypair.fromSeed(new Uint8Array(32).fill(2)).address();
const stx = new TxBuilder(sender).nonce(0).transfer(recipient, 500n);
const res = await client.submitTransaction(stx, sender.public);
console.log("Tx admitted:", res.tx_id);`}</code></pre>
        </div>

        <div className="card">
          <h3>Python SDK (<code>veridag</code>)</h3>
          <p className="muted" style={{ fontSize: "13px" }}>Zero-dependency standard library client for AI agents and quants:</p>
          <pre style={{ margin: "10px 0" }}><code>{`from veridag import VeridagClient, Keypair, TxBuilder

client = VeridagClient("http://127.0.0.1:8080")
h = client.health()
print(f"Connected: chain={h['chain_id']} root={h['state_root'][:16]}...")

sender = Keypair.from_seed(b"\\x01" * 32)
recipient = Keypair.from_seed(b"\\x02" * 32).address()
stx = TxBuilder(sender).nonce(0).transfer(recipient, 500)
res = client.submit_transaction(stx, sender.public())
print("Tx ID:", res["tx_id"])`}</code></pre>
        </div>
      </div>

      <h2>7. Multi-Process Network Devnet (QUIC)</h2>
      <p>
        Spin up 4 distinct OS processes communicating over real authenticated QUIC sockets
        with self-signed TLS 1.3 certificates:
      </p>
      <pre><code>{`cargo test -p veridag-net --test devnet -- --nocapture`}</code></pre>

      <h2>8. Crash Recovery &amp; Persistence</h2>
      <p>
        Verify restart consistency: build a DAG, persist to embedded <code>sled</code>, drop all in-memory state
        (simulated crash), rebuild from disk, and assert bit-for-bit identical state roots:
      </p>
      <pre><code>{`cargo test -p veridag-storage --features persistent`}</code></pre>

      <h2>9. Institutional ISO 20022 Banking Bridge</h2>
      <p>
        Ingest institutional <code>pacs.008.001.08</code> customer credit transfers, automatically apply 1 bps
        clearing surcharge (80% validator pool, 20% insurance reserve), and receive signed <code>pacs.002</code> execution receipts:
      </p>
      <pre><code>{`# Run ISO 20022 unit and integration tests
cargo test -p veridag-stablecoin iso20022`}</code></pre>

      <h2>10. Developer Toolchain Cheatsheet</h2>
      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Command</th>
              <th>Script / Alias</th>
              <th>What It Does</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><code>cargo fmt --check</code></td>
              <td><code>just check</code></td>
              <td>Verify canonical formatting</td>
            </tr>
            <tr>
              <td><code>cargo clippy --workspace --all-targets --all-features -- -D warnings</code></td>
              <td><code>just check</code></td>
              <td>Zero-warning lint gate + <code>#![forbid(unsafe_code)]</code></td>
            </tr>
            <tr>
              <td><code>cargo test --workspace --all-features</code></td>
              <td><code>just check</code></td>
              <td>Run entire test suite across all 28 crates</td>
            </tr>
            <tr>
              <td><code>pwsh -File scripts/publish-sdks.ps1</code></td>
              <td><code>bash scripts/publish-sdks.sh</code></td>
              <td>Cross-language test runner verifying Rust, TS, and Python SDKs</td>
            </tr>
            <tr>
              <td><code>cargo run -p veridag-node -- demo</code></td>
              <td><code>just demo</code></td>
              <td>Run in-process 4-validator consensus demo</td>
            </tr>
            <tr>
              <td><code>cargo test -p veridag-net --test devnet</code></td>
              <td><code>just devnet</code></td>
              <td>Run multi-process QUIC network devnet</td>
            </tr>
            <tr>
              <td><code>cargo test -p veridag-consensus --test simulation</code></td>
              <td><code>just sim</code></td>
              <td>Run deterministic simulation test harness</td>
            </tr>
            <tr>
              <td><code>cargo build --release</code></td>
              <td><code>cargo build --release</code></td>
              <td>Build stripped, <code>panic=abort</code> optimized release binary</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
