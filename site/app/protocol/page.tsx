export default function Protocol() {
  return (
    <div>
      <h1>Protocol</h1>
      <p className="tagline">
        The mental model in one paragraph.
      </p>
      <p>
        A <strong>transaction</strong> is signed and batched. A <strong>vertex</strong>{" "}
        references a batch and its parents in the DAG, and is signed by its author
        validator. Vertices <strong>gossip</strong> over QUIC. The{" "}
        <strong>consensus commit rule</strong> is a <em>pure function</em> of the DAG:
        given the same vertices, every node picks the same commit point and the same
        canonical ordering. That ordering is <strong>executed</strong> deterministically
        (parallel where conflict-free, sequential as the oracle). The resulting{" "}
        <strong>state root</strong> is committed into a <strong>checkpoint</strong> and{" "}
        <strong>persisted</strong>. If a node restarts, it rebuilds from disk and lands
        on the exact same state.
      </p>
      <p className="muted">Determinism is the product. Consensus is how we get it without trusting anyone.</p>

      <h2>Non-negotiable invariants</h2>
      <p>
        No consensus-visible behavior may depend on memory layout, compiler version, CPU
        architecture, thread scheduling, hash-map order, filesystem order, wall-clock
        time, OS randomness, floating point, database iteration quirks, or network timing.
      </p>

      <h2>Layered design</h2>
      <table>
        <thead>
          <tr><th>Layer</th><th>Responsibility</th></tr>
        </thead>
        <tbody>
          <tr><td>Protocol</td><td>Normative spec, schemas, test vectors, conformance</td></tr>
          <tr><td>Formal model</td><td>Quint executable model + invariants</td></tr>
          <tr><td>Implementation</td><td>Reference Rust crates (all <code>#![forbid(unsafe_code)]</code>)</td></tr>
          <tr><td>Runtime</td><td>Deterministic Wasm component runtime (post-v0.1)</td></tr>
          <tr><td>Proofs</td><td>Optional proof-system interface + adapters (post-v0.1)</td></tr>
          <tr><td>SDK</td><td>Rust, TypeScript, Python, Go SDKs</td></tr>
        </tbody>
      </table>

      <h2>Reference crates</h2>
      <p>
        <code>veridag-protocol-types</code>, <code>veridag-codec</code>,{" "}
        <code>veridag-crypto</code>, <code>veridag-merkle</code>,{" "}
        <code>veridag-transaction</code>, <code>veridag-capabilities</code>,{" "}
        <code>veridag-object-state</code>, <code>veridag-storage</code>,{" "}
        <code>veridag-execution</code>, <code>veridag-net</code>, <code>veridag-dag</code>,{" "}
        <code>veridag-consensus</code>.
      </p>
    </div>
  );
}
