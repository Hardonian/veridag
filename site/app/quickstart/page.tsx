export default function Quickstart() {
  return (
    <div>
      <h1>Quickstart</h1>
      <p className="tagline">
        From zero to a running 4-validator Byzantine consensus demo in under three minutes.
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
          <div className="terminal-highlight">round 1..9: max round reached 9</div>
          <div>validator 0: state_root=<span className="terminal-success">0xf7aa17319c5c1653...</span> checkpoints=1</div>
          <div>&nbsp;&nbsp;checkpoint seq=1 id=0x2c0f6f0ba82cb46a...</div>
          <div>validator 1: state_root=<span className="terminal-success">0xf7aa17319c5c1653...</span> checkpoints=1</div>
          <div>validator 2: state_root=<span className="terminal-success">0xf7aa17319c5c1653...</span> checkpoints=1</div>
          <div>validator 3: state_root=<span className="terminal-success">0xf7aa17319c5c1653...</span> checkpoints=1</div>
          <div className="terminal-success">AGREEMENT OK: identical state root across 4 validators</div>
          <div className="terminal-highlight">bob balance: 40 (expected 40)</div>
        </div>
      </div>

      <h2>4. Multi-Process Network Devnet (QUIC)</h2>
      <p>
        Spin up 4 distinct OS processes communicating over real authenticated QUIC sockets
        with self-signed TLS 1.3 certificates:
      </p>
      <pre><code>{`cargo test -p veridag-net --test devnet -- --nocapture`}</code></pre>

      <h2>5. Crash Recovery &amp; Persistence</h2>
      <p>
        Verify restart consistency: build a DAG, persist to embedded <code>sled</code>, drop all in-memory state
        (simulated crash), rebuild from disk, and assert bit-for-bit identical state roots:
      </p>
      <pre><code>{`cargo test -p veridag-storage --features persistent`}</code></pre>

      <h2>6. Validator Node Health Check</h2>
      <p>
        Emit human-readable or machine-parseable JSON status for ops monitoring and dashboard integration:
      </p>
      <pre><code>{`cargo run -p veridag-node -- health --json`}</code></pre>

      <h2>7. Developer Toolchain Cheatsheet</h2>
      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Command</th>
              <th>Just Alias</th>
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
              <td>Run entire test suite across all 15 crates</td>
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

      <h2>8. Institutional Substrate Operations (USDV, Settler, Bitcoin SPV)</h2>
      <p>
        Interact directly with sovereign USDV stablecoin reserves, execute atomic Settler reconciliation batches, and verify Bitcoin SPV block headers:
      </p>
      <pre><code>{`# 1. Attest institutional USDV reserves (US Treasuries & Cash)
cargo run -p veridag-cli -- usdv attest-reserves --tbills 80000000 --cash 15000000 --repo 5000000

# 2. Settle Settler reconciliation batch atomically with zero variance
cargo run -p veridag-cli -- usdv settle --tenant settler-us --run-id rec_01 --manifest-hash 0xca49... --from alice --to bob --amount 1000000

# 3. Verify Bitcoin SPV block header and proof-of-work
cargo run -p veridag-cli -- btc verify-header --header-hex 010000000000...

# 4. Export Prometheus / OpenMetrics telemetry
cargo run -p veridag-cli -- metrics`}</code></pre>
    </div>
  );
}
