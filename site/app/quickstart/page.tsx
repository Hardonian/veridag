export default function Quickstart() {
  return (
    <div>
      <h1>Quickstart</h1>
      <p className="tagline">
        From zero to a running 4-validator consensus demo in under five minutes.
      </p>
      <p className="muted">
        Universal by design: the same Rust core runs on a laptop, a cloud VM, or a
        single-board computer. One static binary + a data directory. No database, no
        broker, no orchestrator.
      </p>

      <h2>1. Prerequisites</h2>
      <ul className="muted">
        <li>Rust edition 2021, rust-version ≥ 1.85 (install via rustup)</li>
        <li>A UNIX-like shell (Linux, macOS, WSL2, FreeBSD)</li>
      </ul>
      <pre><code>{`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable`}</code></pre>

      <h2>2. Get the code</h2>
      <pre><code>{`git clone https://github.com/Hardonian/veridag.git
cd veridag/implementations/rust`}</code></pre>

      <h2>3. First consensus run (in-process demo)</h2>
      <pre><code>{`cargo run -p veridag-node -- demo`}</code></pre>
      <p>You will see four validators print the same state root and checkpoint id:</p>
      <pre><code>{`validator 0: state_root=0xf7aa1731... checkpoints=1
validator 1: state_root=0xf7aa1731... checkpoints=1
validator 2: state_root=0xf7aa1731... checkpoints=1
validator 3: state_root=0xf7aa1731... checkpoints=1`}</code></pre>
      <p className="muted">Identical roots across validators = agreement. That is the whole point of BFT.</p>

      <h2>4. The real thing: 4 processes over QUIC</h2>
      <pre><code>{`cargo test -p veridag-net --test devnet -- --nocapture`}</code></pre>
      <p className="muted">
        Four independent validators gossip over authenticated QUIC and assert the same
        committed wave + state root. A real multi-process network — not a simulation.
      </p>

      <h2>5. Crash recovery (restart-safety)</h2>
      <pre><code>{`cargo test -p veridag-storage --features persistent`}</code></pre>

      <h2>6. Toolchain</h2>
      <table>
        <thead><tr><th>Command</th><th>What it does</th></tr></thead>
        <tbody>
          <tr><td><code>cargo fmt --all -- --check</code></td><td>Verify formatting</td></tr>
          <tr><td><code>cargo clippy --workspace --all-targets --all-features -- -D warnings</code></td><td>Zero-warning lint gate</td></tr>
          <tr><td><code>cargo test --workspace --all-features</code></td><td>Full test suite</td></tr>
          <tr><td><code>cargo build --release</code></td><td>Optimized, stripped binary</td></tr>
        </tbody>
      </table>

      <h2>Health check</h2>
      <p>
        The node exposes a self-test (<code>veridag-node health</code> /{" "}
        <code>health --json</code>) that probes consensus + checkpoint and asserts
        agreement with machine-readable JSON — usable by operators and dashboards.
      </p>
    </div>
  );
}
