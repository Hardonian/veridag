export default function Architecture() {
  return (
    <div>
      <h1>Architecture</h1>
      <p className="tagline">
        How the reference implementation (Rust) realizes the protocol — built for
        universal edge-grade deployability and deterministic correctness.
      </p>

      <div className="alert">
        <div className="alert-title">Core Design Hierarchy</div>
        <p style={{ margin: 0, fontFamily: "var(--font-mono)", fontSize: "13px", color: "var(--accent-cyan-bright)" }}>
          correctness &gt; determinism &gt; security &gt; implementation independence &gt; modularity &gt; verification &gt; operability &gt; performance
        </p>
      </div>

      <h2>Design Commitments</h2>
      <div className="card-grid">
        <div className="card">
          <span className="card-icon">🛡️</span>
          <h3>Zero Unsafe Code</h3>
          <p>All crates strictly enforce <code>#![forbid(unsafe_code)]</code>. Memory-safety vulnerabilities cannot reach the BFT core.</p>
        </div>
        <div className="card">
          <span className="card-icon">📐</span>
          <h3>Deterministic Execution</h3>
          <p>Sequential oracle guarantees bit-for-bit reproducible state roots across different CPUs, OSs, and compiler optimization levels.</p>
        </div>
        <div className="card">
          <span className="card-icon">📦</span>
          <h3>Zero External Services</h3>
          <p>Integrated embedded <code>sled</code> database. No external database servers, brokers, or cloud sidecars needed.</p>
        </div>
        <div className="card">
          <span className="card-icon">⚡</span>
          <h3>QUIC Transport Fast Path</h3>
          <p>Independent multiplexed streams prevent TCP head-of-line blocking; TLS 1.3 authentication from byte zero.</p>
        </div>
      </div>

      <h2>Data Flow Pipeline</h2>
      <pre><code>{`client tx
  -> validate (veridag-transaction)
  -> batch commitment (VCE-1 canonical encoding)
  -> signed DAG vertex (veridag-dag)
  -> authenticated QUIC gossip (veridag-net)
  -> BaselineDagBft pure commit rule (veridag-consensus)
  -> canonical causal wave ordering
  -> conflict-aware execution (veridag-execution: parallel prefix + sequential suffix)
  -> BMH-1 Merkle state root (veridag-merkle)
  -> 2f+1 quorum checkpoint (veridag-checkpoint)
  -> persist to disk (veridag-storage: sled)`}</code></pre>

      <h2>Crate Map</h2>
      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Crate</th>
              <th>Layer</th>
              <th>Responsibility</th>
            </tr>
          </thead>
          <tbody>
            <tr><td><code>veridag-protocol-types</code></td><td>Types</td><td>Canonical IDs, address types, domain tags</td></tr>
            <tr><td><code>veridag-codec</code></td><td>Encoding</td><td>VCE-1 canonical encoder/decoder</td></tr>
            <tr><td><code>veridag-crypto</code></td><td>Crypto</td><td>BLAKE3 hashing, Ed25519 signing, domain separators</td></tr>
            <tr><td><code>veridag-merkle</code></td><td>State</td><td>BMH-1 state commitments + inclusion proofs</td></tr>
            <tr><td><code>veridag-transaction</code></td><td>Tx</td><td>Transaction model, anti-replay, nonce discipline</td></tr>
            <tr><td><code>veridag-capabilities</code></td><td>Auth</td><td>Capability objects and scoped authorization</td></tr>
            <tr><td><code>veridag-object-state</code></td><td>State</td><td>Version-disciplined object store &amp; balances</td></tr>
            <tr><td><code>veridag-execution</code></td><td>Execution</td><td>Sequential deterministic oracle + parallel scheduler</td></tr>
            <tr><td><code>veridag-dag</code></td><td>DAG</td><td>VCE-1 vertex wire form, validity &amp; equivocation check</td></tr>
            <tr><td><code>veridag-consensus</code></td><td>Consensus</td><td>BaselineDagBft pure commit rule &amp; wave ordering</td></tr>
            <tr><td><code>veridag-checkpoint</code></td><td>Finality</td><td>Quorum finality proofs (2f+1) &amp; checkpoint chain</td></tr>
            <tr><td><code>veridag-storage</code></td><td>Storage</td><td>Sled persistent &amp; Memory storage engines</td></tr>
            <tr><td><code>veridag-net</code></td><td>Network</td><td>QUIC authenticated validator transport and selective libp2p discovery</td></tr>
            <tr><td><code>veridag-stablecoin</code></td><td>Sovereign</td><td>USMCA/G8 USDV stablecoin, Proof of Reserves, and Settler reconciliation anchors</td></tr>
            <tr><td><code>veridag-bitcoin</code></td><td>Multi-Chain</td><td>Bitcoin SPV client, 80-byte header parsing, compact nBits PoW, and UTXO bridge codecs</td></tr>
            <tr><td><code>veridag-ethereum</code></td><td>Multi-Chain</td><td>EVM JSON-RPC provider, Solidity L1 light client, and cross-chain portal</td></tr>
            <tr><td><code>veridag-metrics</code></td><td>Telemetry</td><td>OpenMetrics / Prometheus exposition exporter and operational gauges</td></tr>
            <tr><td><code>veridag-wasm-runtime</code></td><td>Runtime</td><td>Deterministic Wasm execution engine with capability-scoped host ABI</td></tr>
            <tr><td><code>veridag-light-client</code></td><td>Client</td><td>2f+1 quorum checkpoint tracking and object Merkle inclusion proofs</td></tr>
            <tr><td><code>veridag-zkvm</code></td><td>ZK</td><td>Pluggable zkVM state validity proofs (SP1, RISC Zero) decoupled from consensus</td></tr>
            <tr><td><code>veridag-da</code></td><td>DA</td><td>2D Reed-Solomon tensor erasure coding with SIMD vector acceleration</td></tr>
            <tr><td><code>veridag-sdk</code></td><td>SDK</td><td>Idiomatic client with bit-for-bit Rust, TypeScript, and Python conformance</td></tr>
            <tr><td><code>veridag-testkit</code></td><td>Testing</td><td>Vector generation &amp; malformed fuzz suites</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
