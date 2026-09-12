"use client";

import { useState } from "react";
import Link from "next/link";

export default function PricingPage() {
  // ROI Calculator State
  const [dailyVolume, setDailyVolume] = useState<number>(50_000_000); // $50M/day
  const [avgTxSize, setAvgTxSize] = useState<number>(50_000); // $50k average
  const [wireCost, setWireCost] = useState<number>(25); // $25 per traditional wire

  // Derived Calculations
  const dailyTxCount = Math.max(1, Math.round(dailyVolume / avgTxSize));
  const annualTxCount = dailyTxCount * 365;
  const legacyAnnualCost = annualTxCount * wireCost;
  const veridagFixedCost = annualTxCount * 0.001; // $0.001 fixed gas per tx
  const wireSavings = legacyAnnualCost - veridagFixedCost;

  // Capital Drag: T+2 settlement ties up 2 days of gross volume in collateral buffers
  // Veridag provides instant sub-second T+0 finality, freeing 2 days of float
  // At 5.25% risk-free rate, the freed float generates:
  const floatCapitalFreed = dailyVolume * 2;
  const floatYieldAnnual = floatCapitalFreed * 0.0525;

  const totalAnnualValue = wireSavings + floatYieldAnnual;

  return (
    <div>
      <div style={{ textAlign: "center", marginBottom: "48px" }}>
        <div className="hero-eyebrow">Institutional Commercial Rails &amp; Licensing</div>
        <h1 className="hero-headline" style={{ marginBottom: "16px" }}>
          Predictable Economics. Zero Float Drag.
        </h1>
        <p className="hero-tagline">
          Replace legacy Fedwire, SWIFT, and correspondent banking fees with deterministic sub-second
          settlement. Model your institution&apos;s annual cost savings and float yield below.
        </p>
      </div>

      {/* ROI & Float Yield Calculator */}
      <div
        className="card"
        style={{
          background: "linear-gradient(180deg, rgba(14, 24, 40, 0.95) 0%, rgba(8, 14, 24, 0.95) 100%)",
          border: "1px solid var(--accent-cyan)",
          boxShadow: "0 0 35px rgba(6, 182, 212, 0.15)",
          padding: "36px",
          marginBottom: "64px",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "16px", marginBottom: "28px" }}>
          <div>
            <h2 style={{ margin: 0, fontSize: "26px", color: "var(--text-main)" }}>
              Interactive Enterprise Settlement ROI Calculator
            </h2>
            <p className="muted" style={{ margin: "4px 0 0" }}>
              Calculate net annual cost reductions, T+0 liquidity unlocking, and Treasury reserve yield.
            </p>
          </div>
          <span
            style={{
              padding: "6px 14px",
              borderRadius: "9999px",
              background: "rgba(16, 185, 129, 0.15)",
              color: "var(--accent-emerald-bright)",
              border: "1px solid rgba(16, 185, 129, 0.3)",
              fontSize: "12px",
              fontWeight: 700,
              fontFamily: "var(--font-mono)",
            }}
          >
            REAL-TIME MODEL
          </span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "28px", marginBottom: "36px" }}>
          {/* Slider 1: Daily Volume */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
              <label style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-main)" }}>
                Daily Settlement Volume
              </label>
              <span style={{ fontFamily: "var(--font-mono)", color: "var(--accent-cyan-bright)", fontWeight: 700 }}>
                ${(dailyVolume / 1_000_000).toFixed(1)}M / day
              </span>
            </div>
            <input
              type="range"
              min={5_000_000}
              max={500_000_000}
              step={5_000_000}
              value={dailyVolume}
              onChange={(e) => setDailyVolume(Number(e.target.value))}
              style={{ width: "100%", accentColor: "var(--accent-cyan)" }}
            />
            <div className="muted" style={{ fontSize: "12px", marginTop: "4px" }}>
              Scale: $5M/day to $500M/day gross flow
            </div>
          </div>

          {/* Slider 2: Average Wire Value */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
              <label style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-main)" }}>
                Average Wire Size
              </label>
              <span style={{ fontFamily: "var(--font-mono)", color: "var(--accent-emerald-bright)", fontWeight: 700 }}>
                ${avgTxSize.toLocaleString()}
              </span>
            </div>
            <input
              type="range"
              min={10_000}
              max={250_000}
              step={5_000}
              value={avgTxSize}
              onChange={(e) => setAvgTxSize(Number(e.target.value))}
              style={{ width: "100%", accentColor: "var(--accent-emerald)" }}
            />
            <div className="muted" style={{ fontSize: "12px", marginTop: "4px" }}>
              Implies ~{dailyTxCount.toLocaleString()} transactions daily ({annualTxCount.toLocaleString()} / year)
            </div>
          </div>

          {/* Slider 3: Legacy Wire Cost */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
              <label style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-main)" }}>
                Legacy Wire Fee (Fedwire/SWIFT)
              </label>
              <span style={{ fontFamily: "var(--font-mono)", color: "var(--accent-violet-bright)", fontWeight: 700 }}>
                ${wireCost}.00 / wire
              </span>
            </div>
            <input
              type="range"
              min={10}
              max={50}
              step={1}
              value={wireCost}
              onChange={(e) => setWireCost(Number(e.target.value))}
              style={{ width: "100%", accentColor: "var(--accent-violet)" }}
            />
            <div className="muted" style={{ fontSize: "12px", marginTop: "4px" }}>
              Industry median: $20 to $35 per cross-border wire
            </div>
          </div>
        </div>

        {/* Results Matrix */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
            gap: "20px",
            background: "rgba(6, 12, 20, 0.8)",
            padding: "24px",
            borderRadius: "var(--radius-md)",
            border: "1px solid var(--border-subtle)",
          }}
        >
          <div>
            <div className="muted" style={{ fontSize: "13px", marginBottom: "6px" }}>
              Annual Wire Cost (Legacy)
            </div>
            <div style={{ fontSize: "24px", fontWeight: 800, color: "#f87171" }}>
              ${(legacyAnnualCost / 1_000_000).toFixed(2)}M
            </div>
            <div className="muted" style={{ fontSize: "11px", marginTop: "2px" }}>
              At ${wireCost}/wire via SWIFT/correspondents
            </div>
          </div>

          <div>
            <div className="muted" style={{ fontSize: "13px", marginBottom: "6px" }}>
              Veridag Annual Rail Cost
            </div>
            <div style={{ fontSize: "24px", fontWeight: 800, color: "var(--accent-emerald-bright)" }}>
              ${(veridagFixedCost / 1_000).toFixed(1)}k
            </div>
            <div className="muted" style={{ fontSize: "11px", marginTop: "2px" }}>
              Fixed sub-cent gas (~$0.001/tx)
            </div>
          </div>

          <div>
            <div className="muted" style={{ fontSize: "13px", marginBottom: "6px" }}>
              T+0 Float Capital Unlocked
            </div>
            <div style={{ fontSize: "24px", fontWeight: 800, color: "var(--accent-cyan-bright)" }}>
              ${(floatCapitalFreed / 1_000_000).toFixed(1)}M
            </div>
            <div className="muted" style={{ fontSize: "11px", marginTop: "2px" }}>
              2-day collateral buffer freed immediately
            </div>
          </div>

          <div>
            <div className="muted" style={{ fontSize: "13px", marginBottom: "6px" }}>
              Float Yield (5.25% T-Bills)
            </div>
            <div style={{ fontSize: "24px", fontWeight: 800, color: "var(--accent-violet-bright)" }}>
              +${(floatYieldAnnual / 1_000_000).toFixed(2)}M / yr
            </div>
            <div className="muted" style={{ fontSize: "11px", marginTop: "2px" }}>
              Yield earned on overnight freed float
            </div>
          </div>
        </div>

        <div
          style={{
            marginTop: "24px",
            padding: "18px 24px",
            borderRadius: "var(--radius-md)",
            background: "rgba(16, 185, 129, 0.1)",
            border: "1px solid rgba(52, 211, 153, 0.3)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            flexWrap: "wrap",
            gap: "12px",
          }}
        >
          <div>
            <span style={{ fontSize: "15px", fontWeight: 600, color: "var(--text-main)" }}>
              Total Net Annual Economic Value Created:
            </span>
            <div className="muted" style={{ fontSize: "12px" }}>
              Direct fee elimination + 48-hour capital velocity gain
            </div>
          </div>
          <div style={{ fontSize: "32px", fontWeight: 900, color: "var(--accent-emerald-bright)", fontFamily: "var(--font-mono)" }}>
            +${(totalAnnualValue / 1_000_000).toFixed(2)}M / yr
          </div>
        </div>
      </div>

      {/* Commercial Tiers */}
      <h2 style={{ textAlign: "center", marginBottom: "12px", fontSize: "32px" }}>
        Enterprise Commercial Tiers
      </h2>
      <p className="muted" style={{ textAlign: "center", maxWidth: "600px", margin: "0 auto 48px" }}>
        Transparent, auditable licensing for financial institutions, fintech platforms, and sovereign clearers.
      </p>

      <div className="card-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(310px, 1fr))", gap: "28px", marginBottom: "64px" }}>
        {/* Tier 1: Community / Developer */}
        <div className="card" style={{ display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
              <span className="badge">DEVELOPER &amp; SANDBOX</span>
              <span style={{ fontSize: "12px", color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>FREE</span>
            </div>
            <h3 style={{ fontSize: "24px", marginBottom: "8px" }}>Pilot Sandbox</h3>
            <p className="muted" style={{ fontSize: "14px", marginBottom: "20px" }}>
              For fintech engineering teams evaluating VCE-1, WebAssembly contracts, and SDK integrations.
            </p>
            <div style={{ fontSize: "36px", fontWeight: 800, marginBottom: "24px", color: "var(--text-main)" }}>
              $0 <span style={{ fontSize: "14px", fontWeight: 400, color: "var(--text-muted)" }}>/ forever</span>
            </div>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "14px", display: "flex", flexDirection: "column", gap: "12px" }}>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Public Testnet &amp; Devnet Access
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> TypeScript, Python &amp; Rust SDKs
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Standard RPC Endpoints (100 req/s)
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Open-Source MIT/Apache-2.0 Engine
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Community Discord &amp; GitHub Discussions
              </li>
            </ul>
          </div>
          <div style={{ marginTop: "32px" }}>
            <Link href="/quickstart" className="btn-secondary" style={{ width: "100%", justifyContent: "center" }}>
              Launch Quickstart
            </Link>
          </div>
        </div>

        {/* Tier 2: Consortium Member (Featured) */}
        <div
          className="card"
          style={{
            display: "flex",
            flexDirection: "column",
            justifyContent: "space-between",
            background: "linear-gradient(180deg, rgba(20, 36, 60, 0.8) 0%, rgba(10, 18, 30, 0.95) 100%)",
            border: "2px solid var(--accent-emerald-bright)",
            boxShadow: "0 0 30px rgba(52, 211, 153, 0.2)",
            position: "relative",
          }}
        >
          <div
            style={{
              position: "absolute",
              top: "-12px",
              left: "50%",
              transform: "translateX(-50%)",
              background: "var(--accent-emerald-bright)",
              color: "#042f1a",
              padding: "4px 16px",
              borderRadius: "9999px",
              fontSize: "11px",
              fontWeight: 800,
              letterSpacing: "0.5px",
            }}
          >
            MOST POPULAR FOR BANKS
          </div>

          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px", marginTop: "8px" }}>
              <span className="badge" style={{ background: "rgba(52, 211, 153, 0.15)", color: "var(--accent-emerald-bright)", borderColor: "var(--accent-emerald-bright)" }}>
                INSTITUTIONAL
              </span>
              <span style={{ fontSize: "12px", color: "var(--accent-emerald-bright)", fontFamily: "var(--font-mono)" }}>SLA 99.999%</span>
            </div>
            <h3 style={{ fontSize: "24px", marginBottom: "8px" }}>Consortium Member</h3>
            <p className="muted" style={{ fontSize: "14px", marginBottom: "20px" }}>
              For regulated banks, broker-dealers, and cross-border payment processors requiring consensus participation.
            </p>
            <div style={{ fontSize: "36px", fontWeight: 800, marginBottom: "24px", color: "var(--text-main)" }}>
              $250,000 <span style={{ fontSize: "14px", fontWeight: 400, color: "var(--text-muted)" }}>/ year</span>
            </div>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "14px", display: "flex", flexDirection: "column", gap: "12px" }}>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Dedicated Validator Seat in Bullshark DAG
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> ISO 20022 pacs.008 / pacs.002 Automated Clearing
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> 80% Revenue Share on Network 1 bps Surcharge
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Multi-Tenant Account Registry &amp; Compliance Freezes
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> 24/7 Dedicated Protocol Engineering Slack Bridge
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> SOC-2 Type II Attestation Evidence Package
              </li>
            </ul>
          </div>
          <div style={{ marginTop: "32px" }}>
            <Link href="/enterprise" className="btn-primary" style={{ width: "100%", justifyContent: "center" }}>
              Request Consortium Member Seat
            </Link>
          </div>
        </div>

        {/* Tier 3: Sovereign / Anchor */}
        <div className="card" style={{ display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
              <span className="badge">SOVEREIGN &amp; CLEARINGHOUSE</span>
              <span style={{ fontSize: "12px", color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>CUSTOM</span>
            </div>
            <h3 style={{ fontSize: "24px", marginBottom: "8px" }}>Anchor Clearinghouse</h3>
            <p className="muted" style={{ fontSize: "14px", marginBottom: "20px" }}>
              For central banks, national clearinghouses, and global liquidity networks operating dedicated subnets.
            </p>
            <div style={{ fontSize: "36px", fontWeight: 800, marginBottom: "24px", color: "var(--text-main)" }}>
              $1,000,000 <span style={{ fontSize: "14px", fontWeight: 400, color: "var(--text-muted)" }}>/ year</span>
            </div>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, fontSize: "14px", display: "flex", flexDirection: "column", gap: "12px" }}>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Private Sovereign Consensus Subnet Orchestration
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> On-Premise Cloud HSM (AWS CloudHSM / Google KMS) Key Gating
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Custom Wasm Precompiles &amp; Tailored Execution Logic
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Multi-Jurisdictional Cross-Border Gateway Routing
              </li>
              <li style={{ display: "flex", gap: "8px" }}>
                <span style={{ color: "var(--accent-emerald-bright)" }}>✓</span> Dedicated Core Engineering Pod &amp; 15-min Incident Response
              </li>
            </ul>
          </div>
          <div style={{ marginTop: "32px" }}>
            <a href="mailto:consortium@veridag.dev" className="btn-secondary" style={{ width: "100%", justifyContent: "center" }}>
              Contact Sovereign Cleared Desk
            </a>
          </div>
        </div>
      </div>

      {/* Compliance & Trust Guarantees */}
      <div className="card" style={{ padding: "32px", marginBottom: "64px" }}>
        <h3 style={{ fontSize: "22px", marginBottom: "16px" }}>Enterprise Compliance &amp; Regulatory Alignment</h3>
        <p className="muted" style={{ marginBottom: "24px" }}>
          Veridag software architecture is engineered to satisfy stringent US, EU, and global institutional banking regulations:
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: "20px" }}>
          <div>
            <h4 style={{ color: "var(--accent-cyan-bright)", fontSize: "16px", marginBottom: "6px" }}>
              FinCEN Non-Custodial
            </h4>
            <p className="muted" style={{ fontSize: "13px" }}>
              Operates as non-custodial communication protocol. Clearers retain direct control over reserve accounts; Veridag never takes custody of funds.
            </p>
          </div>
          <div>
            <h4 style={{ color: "var(--accent-emerald-bright)", fontSize: "16px", marginBottom: "6px" }}>
              SOC-2 Type II Certified
            </h4>
            <p className="muted" style={{ fontSize: "13px" }}>
              Full AICPA Trust Services Criteria mapping covering CC6 Logical Access, A1 Byzantine Availability, and PI1 Canonical VCE-1 Processing Integrity.
            </p>
          </div>
          <div>
            <h4 style={{ color: "var(--accent-violet-bright)", fontSize: "16px", marginBottom: "6px" }}>
              USMCA Digital Trade
            </h4>
            <p className="muted" style={{ fontSize: "13px" }}>
              Engineered in alignment with Chapter 19 cross-border electronic transmission guarantees and zero forced localization mandates.
            </p>
          </div>
          <div>
            <h4 style={{ color: "#fbbf24", fontSize: "16px", marginBottom: "6px" }}>
              OFAC Real-Time Screening
            </h4>
            <p className="muted" style={{ fontSize: "13px" }}>
              Built-in capability gating enables consortium institutions to enforce instantaneous SDN list sanctions filtering prior to DAG ingestion.
            </p>
          </div>
        </div>
      </div>

      {/* FAQ */}
      <h2>Frequently Asked Questions</h2>
      <div style={{ display: "flex", flexDirection: "column", gap: "16px", marginTop: "24px" }}>
        <div className="card">
          <h4 style={{ fontSize: "16px", marginBottom: "8px" }}>How does Veridag eliminate T+2 float drag?</h4>
          <p className="muted" style={{ fontSize: "14px" }}>
            In legacy banking, counterparties hold precautionary capital reserves because settlement is decoupled from messaging and takes 24–48 hours to finalize. Veridag merges messaging (ISO 20022) and atomic state settlement into sub-second deterministic DAG waves. Funds settle at T+0, immediately freeing up collateral float.
          </p>
        </div>
        <div className="card">
          <h4 style={{ fontSize: "16px", marginBottom: "8px" }}>Who holds custody of the USD reserve assets?</h4>
          <p className="muted" style={{ fontSize: "14px" }}>
            USD reserves are held directly by Qualified Custodians (FDIC-insured chartered trust banks) in bankruptcy-remote omnibus accounts. Veridag acts solely as the deterministic synchronization and clearing rail.
          </p>
        </div>
        <div className="card">
          <h4 style={{ fontSize: "16px", marginBottom: "8px" }}>Can our institution integrate existing core banking systems?</h4>
          <p className="muted" style={{ fontSize: "14px" }}>
            Yes. The Veridag node natively ingests standard ISO 20022 <code>pacs.008</code> XML messages and emits cryptographically signed <code>pacs.002</code> execution status reports, making it a drop-in replacement for existing SWIFT and Fedline interfaces.
          </p>
        </div>
      </div>
    </div>
  );
}
