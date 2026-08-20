export default function Architecture() {
  return (
    <div>
      <h1>Architecture</h1>
      <p className="tagline">
        How the reference implementation (Rust) realizes the protocol — built for
        universal usability and deterministic correctness.
      </p>

      <p>
        Veridag targets <strong>low latency, low energy, and small binary
        footprint</strong> without sacrificing correctness or safety. Design
        priorities, in order:
      </p>
      <pre><code>{`correctness > determinism > security > implementation independence
> modularity > verification > operability > performance > developer usability`}</code></pre>
      <p className="muted">
        &ldquo;Performance&rdquo; means throughput per watt and per dollar — not
        peak benchmark numbers. Every hot-path choice stays predictable and cheap.
      </p>

      <h2>Design commitments</h2>
      <ul>
        <li>
          <strong>No <code>unsafe</code> in the consensus/execution core.</strong>{" "}
          All crates <code>#![forbid(unsafe_code)]</code>. Memory-safety bugs
          cannot reach the BFT core.
        </li>
        <li>
          <strong>Deterministic by construction.</strong> No reliance on hash-map
          iteration order, wall-clock time, thread scheduling, floating point, OS
          randomness, or filesystem order. Two nodes with the same inputs produce
          byte-identical state roots.
        </li>
        <li>
          <strong>Small, dependency-light stack.</strong>{" "}
          <code>blake3</code> (fast, parallel, constant-time),{" "}
          <code>ed25519-dalek</code> (fast verification), <code>quinn</code> (QUIC,
          no userspace TCP head-of-line blocking), <code>sled</code> (embedded,
          lock-free). No Kubernetes, no message broker, no sidecar.
        </li>
        <li>
          <strong>Crash-safe persistence.</strong> State and DAG are
          append-friendly and restart-safe; a validator that dies mid-commit
          recovers identically.
        </li>
        <li>
          <strong>Release profile tuned for the edge.</strong>{" "}
          <code>opt-level = 3</code>, <code>lto = &quot;thin&quot;</code>,{" "}
          <code>codegen-units = 1</code>, <code>panic = &quot;abort&quot;</code>,{" "}
          <code>strip = true</code>.
        </li>
      </ul>

      <h2>Crates</h2>
      <table>
        <thead>
          <tr><th>Crate</th><th>Responsibility</th></tr>
        </thead>
        <tbody>
          <tr><td><code>veridag-protocol-types</code></td><td>Canonical identifiers, core types, domain tags</td></tr>
          <tr><td><code>veridag-codec</code></td><td>VCE-1 encoder/decoder (canonical wire form)</td></tr>
          <tr><td><code>veridag-crypto</code></td><td>BLAKE3 hashing, Ed25519 sign/verify, domain preimages</td></tr>
          <tr><td><code>veridag-merkle</code></td><td>BMH-1 state commitments + inclusion proofs</td></tr>
          <tr><td><code>veridag-transaction</code></td><td>Transaction model, validation, anti-replay</td></tr>
          <tr><td><code>veridag-capabilities</code></td><td>Capability objects and enforcement</td></tr>
          <tr><td><code>veridag-object-state</code></td><td>Object set, version discipline, account/balance</td></tr>
          <tr><td><code>veridag-execution</code></td><td>Sequential deterministic executor + parallel scheduler</td></tr>
          <tr><td><code>veridag-dag</code></td><td>VCE-1 vertex wire form, validity, equivocation, quorum</td></tr>
          <tr><td><code>veridag-consensus</code></td><td>BaselineDagBft: pure-function commit rule + leader schedule</td></tr>
          <tr><td><code>veridag-checkpoint</code></td><td>Quorum finality, checkpoint construction/verification</td></tr>
          <tr><td><code>veridag-storage</code></td><td>StateStore/DagStore/CheckpointStore traits + Memory + Sled</td></tr>
          <tr><td><code>veridag-net</code></td><td>QUIC authenticated links + vertex/batch gossip</td></tr>
          <tr><td><code>veridag-testkit</code></td><td>Vector generation/validation, malformed suite</td></tr>
        </tbody>
      </table>

      <h2>Data flow</h2>
      <pre><code>{`client tx
  -> validate (transaction crate)
  -> batch commitment (VCE-1)
  -> DAG vertex (veridag-dag, signed)
  -> gossip over QUIC (veridag-net)
  -> BaselineDagBft commit (veridag-consensus, pure function)
  -> canonical causal ordering
  -> conflict-aware execution (veridag-execution: parallel prefix + sequential suffix)
  -> BMH-1 state root (veridag-merkle)
  -> checkpoint (veridag-checkpoint)
  -> persist (veridag-storage: sled)`}</code></pre>
      <p className="muted">
        Every step is a deterministic function of its inputs. The commit rule is a
        pure function: given an identical DAG, every node computes an identical
        committed anchor and ordering.
      </p>

      <h2>Why QUIC</h2>
      <ul>
        <li>No head-of-line blocking within a connection (independent streams).</li>
        <li>Authenticated from byte 0 via TLS 1.3 with self-signed Ed25519 certs; a domain-separated preimage prevents cross-purpose reuse.</li>
        <li>1-RTT handshake, connection migration, built-in congestion control — suitable for validators on flaky or mobile links.</li>
      </ul>

      <h2>Why sled</h2>
      <ul>
        <li>Zero external services — the database is a local file. A validator is one static binary plus a data directory.</li>
        <li>Append-friendly, crash-safe — matches the DAG&rsquo;s never-rewrite-history model.</li>
        <li>Tiny footprint → runs on a Raspberry Pi-class node.</li>
      </ul>

      <h2>Safety posture</h2>
      <ul>
        <li>All crates <code>#![forbid(unsafe_code)]</code> by default.</li>
        <li>Attacker-facing parsers are canonical (VCE-1) and fuzz-targeted.</li>
        <li>Every consensus-visible value round-trips through VCE-1.</li>
        <li>Signatures use domain-separated preimages (<code>VERIDAG_TX_V1</code>, <code>VERIDAG_VERTEX_V1</code>, &hellip;) so a signature for one purpose cannot be replayed for another.</li>
      </ul>

      <h2>Not in 0.1.0-alpha</h2>
      <p className="muted">
        Public libp2p P2P, the deterministic Wasm runtime, TypeScript/Python/Go
        SDKs, light-client proofs, and zk proof adapters are explicitly deferred.
        The core consensus + execution + persistence + networking slice is complete
        and tested.
      </p>
    </div>
  );
}
