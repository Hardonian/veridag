export default function Protocol() {
  return (
    <div>
      <h1>Protocol Specification</h1>
      <p className="tagline">
        The formal mental model, authority hierarchy, and non-negotiable invariants.
      </p>

      <div className="alert">
        <div className="alert-title">Mental Model in One Paragraph</div>
        <p style={{ margin: 0, fontSize: "14.5px", color: "#e2e8f0" }}>
          A <strong>transaction</strong> is signed and batched. A <strong>vertex</strong> references a batch and its parents in the DAG, and is signed by its author validator. Vertices <strong>gossip</strong> over QUIC. The <strong>consensus commit rule</strong> is a <em>pure function</em> of the DAG: given the same vertices, every node computes the exact same commit point and the same canonical ordering. That ordering is <strong>executed</strong> deterministically (parallel where conflict-free, sequential as the oracle). The resulting <strong>state root</strong> is committed into a <strong>checkpoint</strong> and <strong>persisted</strong>. If a node restarts, it rebuilds from disk and lands on the exact same state.
        </p>
      </div>

      <h2>Three-Level Authority Ladder</h2>
      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Level</th>
              <th>Artifact</th>
              <th>Path</th>
              <th>Authority Scope</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><strong>Level 1</strong></td>
              <td>Normative Protocol Specification</td>
              <td><code>protocol/specification/</code></td>
              <td>Canonical wire formats, mathematical models, state rules</td>
            </tr>
            <tr>
              <td><strong>Level 2</strong></td>
              <td>Formal Executable Model (Quint)</td>
              <td><code>formal/quint/</code></td>
              <td>Machine-checked Agreement, Finality, and Integrity invariants</td>
            </tr>
            <tr>
              <td><strong>Level 3</strong></td>
              <td>Reference Implementation (Rust)</td>
              <td><code>implementations/rust/</code></td>
              <td>Production Rust engine; correct only if it satisfies Levels 1 and 2</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2>Non-Negotiable Invariants</h2>
      <div className="card-grid">
        <div className="card">
          <span className="card-icon">⚡</span>
          <h3>Agreement</h3>
          <p>All non-faulty validators commit the exact same anchor vertices in the exact same wave sequence.</p>
        </div>
        <div className="card">
          <span className="card-icon">🔒</span>
          <h3>Finality</h3>
          <p>Committed state is cryptographically immutable and permanently irrevocable across epochs.</p>
        </div>
        <div className="card">
          <span className="card-icon">🛡️</span>
          <h3>Integrity</h3>
          <p>Only validly signed, well-formed vertices matching VCE-1 canonical wire format enter the DAG.</p>
        </div>
        <div className="card">
          <span className="card-icon">📐</span>
          <h3>Zero Non-Determinism</h3>
          <p>No consensus behavior may depend on hash-map iteration, clock time, OS random, floating point, or thread scheduling.</p>
        </div>
      </div>

      <h2>Specification Modules</h2>
      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Doc</th>
              <th>Topic</th>
              <th>Key Standard</th>
            </tr>
          </thead>
          <tbody>
            <tr><td>00-overview.md</td><td>Protocol Overview &amp; Architecture</td><td>Normative 3-level model</td></tr>
            <tr><td>02-identifiers.md</td><td>Domain Separators &amp; IDs</td><td>BLAKE3 domain preimages</td></tr>
            <tr><td>03-canonical-encoding.md</td><td>VCE-1 Canonical Encoding</td><td>Deterministic binary serialization</td></tr>
            <tr><td>04-cryptography.md</td><td>Cryptographic Primitives</td><td>Ed25519 + BLAKE3</td></tr>
            <tr><td>05-transactions.md</td><td>Transaction Anatomy &amp; Nonces</td><td>Anti-replay &amp; capability checks</td></tr>
            <tr><td>06-object-model.md</td><td>Object-Centric State</td><td>Version-disciplined state trees</td></tr>
            <tr><td>08-dag.md</td><td>Directed Acyclic Graph</td><td>Equivocation detection &amp; round quorum</td></tr>
            <tr><td>09-consensus.md</td><td>BaselineDagBft Commit Rule</td><td>Pure-function Shoal-style pipelining</td></tr>
            <tr><td>11-execution.md</td><td>Deterministic State Machine</td><td>Conflict-aware parallel scheduler</td></tr>
            <tr><td>13-checkpoints.md</td><td>Epochs &amp; Quorum Finality</td><td>2f+1 signed checkpoint proofs</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
