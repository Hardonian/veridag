export default function Security() {
  return (
    <div>
      <h1>Security</h1>
      <p className="tagline">
        The security model combines the threat model, the capability model, the
        cryptographic domains, the validation pipeline, and the consensus safety
        invariants.
      </p>
      <p className="muted">
        We claim only what is demonstrated by the formal model and the test suite in
        this tree. We do not claim production readiness without external audits.
      </p>

      <h2>Adversary classes</h2>
      <ul>
        <li>Byzantine validators (up to <code>f</code> of <code>n &gt;= 3f+1</code>)</li>
        <li>Malicious clients</li>
        <li>Sybil peers on the public plane</li>
        <li>Malicious applications in the Wasm runtime</li>
        <li>Supply-chain attackers on dependencies</li>
      </ul>

      <h2>Attacks considered &amp; mitigations</h2>
      <table>
        <thead>
          <tr><th>Attack</th><th>Mitigation</th></tr>
        </thead>
        <tbody>
          <tr><td>Equivocation</td><td>Detected; one working vertex per (author, round); safety proved in Quint model.</td></tr>
          <tr><td>Censorship</td><td>DAG proposals from all validators; withholding only delays own txs.</td></tr>
          <tr><td>Ordering manipulation</td><td>CanonicalWaveOrder seed bound to committed anchor (spec 10).</td></tr>
          <tr><td>Replay</td><td>nonce + expiry_epoch + object-version binding + (chain, protocol) in preimage.</td></tr>
          <tr><td>Eclipse</td><td>Validator fast path authenticated to ValidatorId; public plane separate.</td></tr>
          <tr><td>Parser attacks</td><td>VCE-1 canonical rejection; malformed vector suite; fuzz targets on every parser.</td></tr>
          <tr><td>DoS / resource exhaustion</td><td>Bounded frames/messages/queues/requests; validation pipeline cheap-to-expensive; backpressure.</td></tr>
          <tr><td>Key compromise</td><td>Capability scoping limits blast radius; validator keys via keystore/signer abstraction.</td></tr>
          <tr><td>Wasm escape</td><td>Default-deny host API; explicit capability handles; deterministic metering.</td></tr>
          <tr><td>Proof forgery</td><td>Versioned proof envelopes; verification optional and backend-identified.</td></tr>
          <tr><td>Rollback</td><td>Finality invariant; checkpoint chain links validator sets.</td></tr>
          <tr><td>Checkpoint forgery</td><td>Quorum finality proofs (2f+1).</td></tr>
          <tr><td>Supply-chain</td><td>cargo deny + audit in CI; deliberate dependency policy.</td></tr>
          <tr><td>Validator crash/restart</td><td>Persist-before-ack; recovery rebuilds from durable state.</td></tr>
          <tr><td>Partition</td><td>Safety is clock-independent; liveness resumes under eventual synchrony.</td></tr>
        </tbody>
      </table>

      <h2>Validation pipeline (increasing cost)</h2>
      <pre><code>{`frame bounds -> basic format -> protocol version -> canonical encoding
-> duplicate check -> cheap structural checks -> signature verification
-> state-dependent validation -> execution -> proof verification (if required)`}</code></pre>

      <h2>Cryptographic domains</h2>
      <p>
        Signatures use <strong>domain-separated preimages</strong> (
        <code>VERIDAG_TX_V1</code>, <code>VERIDAG_VERTEX_V1</code>, &hellip;) so a
        signature minted for one purpose cannot be replayed for another. Hashing is
        BLAKE3 (fast, parallel, constant-time); signatures are Ed25519
        (ed25519-dalek).
      </p>

      <h2>Safety invariants</h2>
      <ul>
        <li><strong>Agreement</strong> — non-faulty nodes commit the same anchor.</li>
        <li><strong>Finality</strong> — committed state is never reverted.</li>
        <li><strong>Integrity</strong> — only validly-signed, well-formed vertices enter the DAG.</li>
        <li>Plus determinism, no-double-spend, replay-protection, capability-safety, and canonical-interpretation.</li>
      </ul>

      <h2>Reporting</h2>
      <p>
        See <code>SECURITY.md</code> for the disclosure policy. The protocol is
        research-grade; deploy only after external review for your threat profile.
      </p>
    </div>
  );
}
