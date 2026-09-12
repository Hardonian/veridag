"use client";

import { useState } from "react";

const rounds = [
  { round: 11, wave: 3, leader: "V3", vertices: ["0x3a87... (Anchor)", "0x77bc...", "0x89fd...", "0x12ea..."], committed: true },
  { round: 10, wave: 3, leader: "V2", vertices: ["0xca12...", "0xfe33...", "0x09da...", "0xbb76..."], committed: true },
  { round: 9, wave: 2, leader: "V1", vertices: ["0xac04... (Anchor)", "0x11ee...", "0x99aa...", "0x54cc..."], committed: true },
  { round: 8, wave: 2, leader: "V0", vertices: ["0x23aa...", "0x77dd...", "0xcc55...", "0x44ee..."], committed: true },
  { round: 7, wave: 2, leader: "V3", vertices: ["0x98bb...", "0x65aa...", "0x33dd...", "0x11ff..."], committed: true },
  { round: 6, wave: 1, leader: "V2", vertices: ["0x55aa...", "0x44bb...", "0x88cc...", "0x99dd..."], committed: true },
  { round: 5, wave: 1, leader: "V1", vertices: ["0x11aa... (Anchor)", "0x22bb...", "0x33cc...", "0x44dd..."], committed: true },
  { round: 4, wave: 1, leader: "V0", vertices: ["0xaaaa...", "0xbbbb...", "0xcccc...", "0xdddd..."], committed: true },
];

const mockTxs = [
  { id: "0x8f12cb7e...", type: "USDV Transfer", from: "alice (0x9a4f...)", to: "bob (0x7e22...)", amount: "40.00 USDV", status: "Finalized" },
  { id: "0xca49aa10...", type: "Settler Anchor", from: "settler-us", to: "State Anchor", amount: "rec_01 (0-var)", status: "Finalized" },
  { id: "0x33ee91ca...", type: "Bitcoin SPV", from: "BtcTracker", to: "Block 840,000", amount: "Header Valid", status: "Verified" },
  { id: "0x11d8820c...", type: "PoR Attestation", from: "Custodian BNY", to: "Reserve Root", amount: "$100M Treasuries", status: "Audited" },
];

export default function ExplorerPage() {
  const [selectedRound, setSelectedRound] = useState(rounds[0]);

  return (
    <div>
      <h1>VeriDAG Consensus &amp; State Explorer</h1>
      <p className="tagline">
        Live inspection of Narwhal-style DAG rounds, BaselineDagBft commit anchors, and BMH-1 Merkle state roots.
      </p>

      {/* Network Metrics Header */}
      <div className="card-grid" style={{ marginBottom: "28px" }}>
        <div className="card" style={{ padding: "20px" }}>
          <div className="muted" style={{ fontSize: "12px", textTransform: "uppercase", letterSpacing: "0.5px" }}>Consensus State Root</div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: "14px", fontWeight: 700, color: "var(--accent-emerald-bright)", marginTop: "6px", wordBreak: "break-all" }}>
            0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e
          </div>
        </div>
        <div className="card" style={{ padding: "20px" }}>
          <div className="muted" style={{ fontSize: "12px", textTransform: "uppercase", letterSpacing: "0.5px" }}>Checkpoint Seq #1</div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: "14px", fontWeight: 700, color: "var(--accent-cyan-bright)", marginTop: "6px", wordBreak: "break-all" }}>
            0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40
          </div>
        </div>
        <div className="card" style={{ padding: "20px" }}>
          <div className="muted" style={{ fontSize: "12px", textTransform: "uppercase", letterSpacing: "0.5px" }}>Active Committee</div>
          <div style={{ fontSize: "20px", fontWeight: 800, color: "var(--text-main)", marginTop: "6px" }}>
            n = 4 <span style={{ fontSize: "13px", color: "var(--accent-emerald-bright)", fontWeight: 500 }}>(Quorum 3 / 2W+1)</span>
          </div>
        </div>
        <div className="card" style={{ padding: "20px" }}>
          <div className="muted" style={{ fontSize: "12px", textTransform: "uppercase", letterSpacing: "0.5px" }}>BFT Agreement</div>
          <div style={{ fontSize: "20px", fontWeight: 800, color: "var(--accent-emerald-bright)", marginTop: "6px" }}>
            100% Deterministic
          </div>
        </div>
      </div>

      {/* Interactive DAG Visualizer */}
      <h2>1. Narwhal DAG Wave Progression</h2>
      <p>
        Vertices are produced in parallel by validators and linked causal round-by-round.
        Select a round to inspect wave anchoring and quorum proofs:
      </p>

      <div className="card" style={{ padding: "24px", marginBottom: "32px" }}>
        <div style={{ display: "flex", gap: "10px", overflowX: "auto", paddingBottom: "12px", marginBottom: "16px" }}>
          {rounds.map((r) => {
            const isSelected = selectedRound.round === r.round;
            return (
              <button
                key={r.round}
                type="button"
                onClick={() => setSelectedRound(r)}
                style={{
                  padding: "10px 16px",
                  borderRadius: "var(--radius-md)",
                  border: isSelected ? "1px solid var(--accent-cyan-bright)" : "1px solid var(--border-subtle)",
                  background: isSelected ? "rgba(56, 189, 248, 0.15)" : "var(--bg-code)",
                  color: isSelected ? "var(--accent-cyan-bright)" : "var(--text-muted)",
                  cursor: "pointer",
                  fontFamily: "var(--font-mono)",
                  fontSize: "13px",
                  minWidth: "110px",
                  textAlign: "center",
                }}
              >
                <div>Round {r.round}</div>
                <div style={{ fontSize: "11px", color: isSelected ? "var(--text-main)" : "var(--text-dim)", marginTop: "2px" }}>Wave {r.wave}</div>
              </button>
            );
          })}
        </div>

        <div style={{ background: "var(--bg-code)", padding: "16px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-subtle)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "12px", alignItems: "center" }}>
            <span style={{ fontWeight: 700, color: "var(--accent-cyan-bright)", fontSize: "14px" }}>
              DAG Round {selectedRound.round} Detail (Wave {selectedRound.wave})
            </span>
            <span style={{ fontSize: "12px", background: "rgba(52, 211, 153, 0.15)", color: "var(--accent-emerald-bright)", padding: "2px 8px", borderRadius: "4px" }}>
              Committed by 2f+1 Quorum
            </span>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "12px" }}>
            {selectedRound.vertices.map((v, i) => (
              <div key={i} style={{ padding: "12px", border: "1px solid var(--border-subtle)", borderRadius: "var(--radius-sm)", background: "var(--bg-surface)" }}>
                <div style={{ fontSize: "11px", color: "var(--text-dim)", textTransform: "uppercase" }}>Validator {i} Vertex</div>
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "13px", color: v.includes("Anchor") ? "var(--accent-emerald-bright)" : "var(--text-main)", marginTop: "4px" }}>
                  {v}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Recent Transactions Table */}
      <h2>2. Recent Cleared Settlements</h2>
      <div style={{ overflowX: "auto", margin: "16px 0 32px" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", textAlign: "left", fontSize: "14px" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)", color: "var(--text-muted)" }}>
              <th style={{ padding: "12px 16px" }}>Tx ID</th>
              <th style={{ padding: "12px 16px" }}>Type</th>
              <th style={{ padding: "12px 16px" }}>From / Tenant</th>
              <th style={{ padding: "12px 16px" }}>To / Target</th>
              <th style={{ padding: "12px 16px" }}>Value / Details</th>
              <th style={{ padding: "12px 16px" }}>Status</th>
            </tr>
          </thead>
          <tbody>
            {mockTxs.map((tx, idx) => (
              <tr key={idx} style={{ borderBottom: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)", fontSize: "13px" }}>
                <td style={{ padding: "12px 16px", color: "var(--accent-cyan-bright)" }}>{tx.id}</td>
                <td style={{ padding: "12px 16px", color: "var(--text-main)", fontFamily: "var(--font-sans)", fontWeight: 600 }}>{tx.type}</td>
                <td style={{ padding: "12px 16px", color: "var(--text-muted)" }}>{tx.from}</td>
                <td style={{ padding: "12px 16px", color: "var(--text-muted)" }}>{tx.to}</td>
                <td style={{ padding: "12px 16px", color: "var(--accent-emerald-bright)", fontWeight: 600 }}>{tx.amount}</td>
                <td style={{ padding: "12px 16px" }}>
                  <span style={{ background: "rgba(52, 211, 153, 0.12)", color: "var(--accent-emerald-bright)", padding: "2px 8px", borderRadius: "4px", fontSize: "12px" }}>
                    {tx.status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2>3. Node Observability Telemetry</h2>
      <p>
        Live Prometheus / OpenMetrics scrape output exposed by <code>veridag-metrics</code>:
      </p>
      <pre><code>{`# HELP veridag_consensus_round Current round height of the Narwhal DAG
# TYPE veridag_consensus_round gauge
veridag_consensus_round{chain_id="1"} 11

# HELP veridag_state_root Committed BMH-1 cryptographic state root
# TYPE veridag_state_root gauge
veridag_state_root{seq="1"} 0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e

# HELP veridag_tps Instantaneous settlement transactions per second
# TYPE veridag_tps gauge
veridag_tps 4250.00`}</code></pre>
    </div>
  );
}
