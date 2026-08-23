export default function Security() {
  return (
    <div>
      <h1>Security &amp; Threat Model</h1>
      <p className="tagline">
        The Veridag security posture integrates capability isolation, domain-separated cryptography,
        cheap-to-expensive validation pipelines, and formal Quint consensus invariants.
      </p>

      <div className="alert">
        <div className="alert-title">🔒 Zero-Unsafe Guarantee</div>
        <p style={{ margin: 0, fontSize: "14px", color: "#c7d2e0" }}>
          All consensus, execution, cryptographic, and state crates in the Rust reference implementation
          strictly forbid unsafe code (<code>#![forbid(unsafe_code)]</code>).
        </p>
      </div>

      <h2>Adversary Classes Handled</h2>
      <div className="card-grid">
        <div className="card">
          <span className="card-icon">🦹</span>
          <h3>Byzantine Validators</h3>
          <p>Tolerates up to <code>f</code> Byzantine/faulty validators in any committee of <code>n &gt;= 3f + 1</code> nodes.</p>
        </div>
        <div className="card">
          <span className="card-icon">⚡</span>
          <h3>Equivocating Authors</h3>
          <p>Equivocation is strictly detected and quarantined. At most one vertex per author-round is admitted.</p>
        </div>
        <div className="card">
          <span className="card-icon">🔄</span>
          <h3>Replay Attackers</h3>
          <p>Anti-replay via nonce, epoch expiry, object version binding, and domain-separated preimages.</p>
        </div>
        <div className="card">
          <span className="card-icon">💣</span>
          <h3>DoS &amp; Parser Attacks</h3>
          <p>Bounded frame sizes, canonical VCE-1 format, cheap-to-expensive checks, and fuzz testing.</p>
        </div>
      </div>

      <h2>Attacks Considered &amp; Mitigations</h2>
      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Attack Vector</th>
              <th>Protocol &amp; Implementation Mitigation</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><strong>Equivocation</strong></td>
              <td>Detected on ingress; exactly one working vertex per (author, round); safety proved in Quint.</td>
            </tr>
            <tr>
              <td><strong>Censorship</strong></td>
              <td>DAG multi-proposer model; any honest validator can propose batches; withholding hurts only the withholding node.</td>
            </tr>
            <tr>
              <td><strong>Ordering Manipulation</strong></td>
              <td>Deterministic <code>CanonicalWaveOrder</code> with seed bound to committed anchor vertex.</td>
            </tr>
            <tr>
              <td><strong>Replay Attacks</strong></td>
              <td>Nonce tracking, epoch bounds, object version binding, and protocol domain separators.</td>
            </tr>
            <tr>
              <td><strong>Parser Exploitation</strong></td>
              <td>Strict VCE-1 canonical decoder rejection; malformed fuzz vector suite in CI.</td>
            </tr>
            <tr>
              <td><strong>Resource Exhaustion</strong></td>
              <td>Progressive validation pipeline; cheap checks execute before expensive cryptographic verify.</td>
            </tr>
            <tr>
              <td><strong>Crash &amp; Restarts</strong></td>
              <td>Persist-before-ack discipline; recovery rebuilds state bit-for-bit from durable Sled logs.</td>
            </tr>
            <tr>
              <td><strong>Network Partitions</strong></td>
              <td>Safety is clock-independent; liveness smoothly recovers upon eventual synchrony.</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2>Progressive Validation Pipeline</h2>
      <pre><code>{`frame bounds -> basic format -> protocol version -> canonical VCE-1 encoding
-> duplicate check -> cheap structural checks -> signature verification
-> state-dependent validation -> execution -> proof verification`}</code></pre>
      <p className="muted">
        Expensive signature and Merkle calculations are never executed before cheap structural filters pass.
      </p>

      <h2>Cryptographic Domain Separation</h2>
      <p>
        All cryptographic operations use explicit BLAKE3 / Ed25519 domain separators to prevent cross-context replay:
      </p>
      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Domain Tag</th>
              <th>Scope</th>
            </tr>
          </thead>
          <tbody>
            <tr><td><code>VERIDAG_TX_V1</code></td><td>Client transaction signatures</td></tr>
            <tr><td><code>VERIDAG_VERTEX_V1</code></td><td>DAG vertex validator signatures</td></tr>
            <tr><td><code>VERIDAG_BATCH_V1</code></td><td>Transaction batch commitment hashing</td></tr>
            <tr><td><code>VERIDAG_CHECKPOINT_V1</code></td><td>Checkpoint finality quorum voting</td></tr>
            <tr><td><code>VERIDAG_TLS_CERT_V1</code></td><td>QUIC node authentication certificates</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
