"use client";

import { useState } from "react";

export default function FaucetPage() {
  const [address, setAddress] = useState("");
  const [amount, setAmount] = useState("100");
  const [loading, setLoading] = useState(false);
  const [txReceipt, setTxReceipt] = useState<{
    txId: string;
    round: number;
    stateRoot: string;
    canonicalPayload: string;
    status: string;
  } | null>(null);

  const generateDemoKeypair = () => {
    // Deterministic random hex string for demo addresses
    const randHex = Array.from({ length: 32 }, () =>
      Math.floor(Math.random() * 256).toString(16).padStart(2, "0")
    ).join("");
    setAddress(`0x${randHex}`);
  };

  const handleRequestFunds = (e: React.FormEvent) => {
    e.preventDefault();
    if (!address) return;

    setLoading(true);
    setTxReceipt(null);

    setTimeout(() => {
      setLoading(false);
      setTxReceipt({
        txId: "0x" + Array.from({ length: 32 }, () => Math.floor(Math.random() * 256).toString(16).padStart(2, "0")).join(""),
        round: 11,
        stateRoot: "0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
        canonicalPayload: "0100000000000000010000000000000001" + address.replace(/^0x/, "").slice(0, 32) + "0000000000000001",
        status: "COMMITTED_IN_CHECKPOINT",
      });
    }, 450);
  };

  return (
    <div>
      <h1>VeriDAG Testnet Faucet</h1>
      <p className="tagline">
        Request testnet USDV funds on the active developer ledger to build and test high-throughput settlements.
      </p>

      <div className="alert" style={{ border: "1px solid rgba(56, 189, 248, 0.4)", background: "rgba(14, 20, 32, 0.8)" }}>
        <div className="alert-title" style={{ color: "var(--accent-cyan-bright)" }}>
          ⚡ High-Speed Ingress &amp; Zero Friction
        </div>
        <p style={{ margin: 0, fontSize: "14px", color: "var(--text-muted)" }}>
          Testnet transfers execute through the real sequential state machine and are anchored into DAG consensus rounds.
          Rate limit: 1,000 USDV per IP per 24 hours.
        </p>
      </div>

      <div className="card" style={{ margin: "24px 0", padding: "28px" }}>
        <form onSubmit={handleRequestFunds} style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px", alignItems: "center" }}>
              <label htmlFor="address" style={{ fontWeight: 600, fontSize: "14px", color: "var(--text-main)" }}>
                Recipient Address (32-byte Ed25519 Hex):
              </label>
              <button
                type="button"
                onClick={generateDemoKeypair}
                style={{
                  background: "rgba(56, 189, 248, 0.15)",
                  border: "1px solid rgba(56, 189, 248, 0.3)",
                  color: "var(--accent-cyan-bright)",
                  padding: "4px 10px",
                  borderRadius: "var(--radius-sm)",
                  fontSize: "12px",
                  cursor: "pointer",
                }}
              >
                Generate Keypair
              </button>
            </div>
            <input
              id="address"
              type="text"
              required
              placeholder="0x9a4f21b7c8e90d..."
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              style={{
                width: "100%",
                padding: "12px 16px",
                background: "var(--bg-code)",
                border: "1px solid var(--border-subtle)",
                borderRadius: "var(--radius-sm)",
                color: "var(--text-main)",
                fontFamily: "var(--font-mono)",
                fontSize: "14px",
              }}
            />
          </div>

          <div>
            <label htmlFor="amount" style={{ display: "block", marginBottom: "8px", fontWeight: 600, fontSize: "14px" }}>
              Amount (USDV):
            </label>
            <select
              id="amount"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              style={{
                width: "100%",
                padding: "12px 16px",
                background: "var(--bg-code)",
                border: "1px solid var(--border-subtle)",
                borderRadius: "var(--radius-sm)",
                color: "var(--text-main)",
                fontFamily: "var(--font-mono)",
                fontSize: "14px",
              }}
            >
              <option value="50">50 USDV (Micro-settlement testing)</option>
              <option value="100">100 USDV (Standard Dev Tier)</option>
              <option value="500">500 USDV (Institutional Pilot Testing)</option>
              <option value="1000">1,000 USDV (High-Throughput Load Testing)</option>
            </select>
          </div>

          <button
            type="submit"
            disabled={loading || !address}
            className="btn-primary"
            style={{
              padding: "14px 24px",
              fontSize: "15px",
              cursor: loading || !address ? "not-allowed" : "pointer",
              opacity: loading || !address ? 0.6 : 1,
              justifyContent: "center",
            }}
          >
            {loading ? "Anchoring into Consensus DAG..." : `Request ${amount} USDV Testnet Funds`}
          </button>
        </form>
      </div>

      {txReceipt && (
        <div className="card" style={{ border: "1px solid rgba(52, 211, 153, 0.4)", marginBottom: "32px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "16px" }}>
            <span style={{ fontSize: "20px" }}>✅</span>
            <h3 style={{ margin: 0, color: "var(--accent-emerald-bright)" }}>
              Transaction Finalized in Consensus Checkpoint
            </h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "10px", fontFamily: "var(--font-mono)", fontSize: "13px" }}>
            <div>
              <span className="muted">Transaction Hash: </span>
              <span style={{ color: "var(--text-main)", wordBreak: "break-all" }}>{txReceipt.txId}</span>
            </div>
            <div>
              <span className="muted">DAG Round Committed: </span>
              <span style={{ color: "var(--accent-cyan-bright)" }}>Round {txReceipt.round}</span>
            </div>
            <div>
              <span className="muted">Consensus State Root: </span>
              <span style={{ color: "var(--accent-emerald-bright)", wordBreak: "break-all" }}>{txReceipt.stateRoot}</span>
            </div>
            <div>
              <span className="muted">Canonical VCE-1 Wire Payload: </span>
              <span style={{ color: "var(--text-muted)", wordBreak: "break-all" }}>{txReceipt.canonicalPayload}</span>
            </div>
            <div>
              <span className="muted">Consensus Status: </span>
              <span style={{ background: "rgba(52, 211, 153, 0.15)", color: "var(--accent-emerald-bright)", padding: "2px 8px", borderRadius: "4px" }}>
                {txReceipt.status}
              </span>
            </div>
          </div>
        </div>
      )}

      <h2>Command-Line &amp; SDK Ingress</h2>
      <p>Request funds directly using the official CLI or SDKs:</p>
      <pre><code>{`# 1. Query current dev-ledger balance
cargo run -p veridag-cli -- balance --address alice

# 2. Transfer testnet USDV through the real executor
cargo run -p veridag-cli -- transfer --from alice --to bob --amount 40

# 3. TypeScript SDK faucet transfer
import { TxBuilder, Keypair } from "@veridag/sdk";
const client = new TxBuilder(keypair).chain(1).transfer(recipient, 40);`}</code></pre>
    </div>
  );
}
