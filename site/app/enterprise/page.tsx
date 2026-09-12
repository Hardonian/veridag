import Link from "next/link";

export default function EnterprisePage() {
  return (
    <div>
      <h1>Enterprise &amp; Institutional Gateway</h1>
      <p className="tagline">
        Sovereign multilateral settlement, 100% Treasury-backed digital dollars (USDV), and cross-border trade clearing across the USMCA trade corridor.
      </p>

      {/* Hero Pillars */}
      <div className="card-grid" style={{ margin: "28px 0" }}>
        <div className="card">
          <span className="card-icon">🏛️</span>
          <h3>100% US Treasury Backed</h3>
          <p>
            USDV is backed exclusively by short-term US Treasury Bills (4–13 weeks), Federal Reserve Overnight Reverse Repo,
            and FDIC-insured cash held in segregated bankruptcy-remote trust accounts.
          </p>
        </div>
        <div className="card">
          <span className="card-icon">🚛</span>
          <h3>USMCA Trade Clearing</h3>
          <p>
            Compliant under USMCA Chapter 19 (Digital Trade). Replaces 72-hour correspondent banking wires with
            sub-300ms atomic gross settlement and zero currency slippage.
          </p>
        </div>
        <div className="card">
          <span className="card-icon">🔐</span>
          <h3>FIPS 140-2 Cloud KMS</h3>
          <p>
            Hardware Security Module (HSM) signing substrate supporting AWS KMS, GCP Cloud KMS, Azure Key Vault,
            and PKCS#11 devices for regulated banking institutions.
          </p>
        </div>
        <div className="card">
          <span className="card-icon">🛡️</span>
          <h3>Real-Time OFAC Screening</h3>
          <p>
            Autonomous consensus-level sanctions filtering built directly into the deterministic state machine,
            blocking prohibited counterparties before checkpoint commitment.
          </p>
        </div>
      </div>

      {/* Institutional Reserve Float Yield Economics */}
      <h2>Sovereign Reserve Float &amp; Seigniorage Economics</h2>
      <p>
        Unlike retail tokens, USDV operates as an institutional treasury asset where short-term Treasury yield
        generates counter-cyclical, risk-free cash flows for consortium validators and the protocol treasury:
      </p>

      <div style={{ overflowX: "auto", margin: "16px 0 32px" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", textAlign: "left", fontSize: "14px" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)", color: "var(--text-muted)" }}>
              <th style={{ padding: "12px 16px" }}>USDV Float Scale</th>
              <th style={{ padding: "12px 16px" }}>Blended T-Bill Yield</th>
              <th style={{ padding: "12px 16px" }}>Annual Gross Revenue</th>
              <th style={{ padding: "12px 16px" }}>Validator Staking Rebate (30%)</th>
              <th style={{ padding: "12px 16px" }}>Net Protocol Operating Cashflow</th>
            </tr>
          </thead>
          <tbody>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)" }}>
              <td style={{ padding: "12px 16px", color: "var(--text-main)", fontWeight: 700 }}>$50,000,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-muted)" }}>4.50%</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-emerald-bright)", fontWeight: 600 }}>$2,250,000 / yr</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-cyan-bright)" }}>$675,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-main)" }}>$1,350,000</td>
            </tr>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)" }}>
              <td style={{ padding: "12px 16px", color: "var(--text-main)", fontWeight: 700 }}>$100,000,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-muted)" }}>4.50%</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-emerald-bright)", fontWeight: 600 }}>$4,500,000 / yr</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-cyan-bright)" }}>$1,350,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-main)" }}>$2,700,000</td>
            </tr>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)" }}>
              <td style={{ padding: "12px 16px", color: "var(--text-main)", fontWeight: 700 }}>$500,000,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-muted)" }}>4.25%</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-emerald-bright)", fontWeight: 600 }}>$21,250,000 / yr</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-cyan-bright)" }}>$6,375,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-main)" }}>$12,750,000</td>
            </tr>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)" }}>
              <td style={{ padding: "12px 16px", color: "var(--text-main)", fontWeight: 700 }}>$1,000,000,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-muted)" }}>4.00%</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-emerald-bright)", fontWeight: 600 }}>$40,000,000 / yr</td>
              <td style={{ padding: "12px 16px", color: "var(--accent-cyan-bright)" }}>$12,000,000</td>
              <td style={{ padding: "12px 16px", color: "var(--text-main)" }}>$24,000,000</td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* Institutional Virtual Data Room & Diligence */}
      <h2>Institutional Due Diligence &amp; Data Room</h2>
      <p>
        The complete institutional diligence archive is accessible to qualified investors, banking partners, and consortium candidates:
      </p>

      <div className="card-grid">
        <div className="card">
          <h3>📁 Legal &amp; Regulatory Moat</h3>
          <ul style={{ paddingLeft: "18px", fontSize: "14px", color: "var(--text-muted)", marginTop: "8px", lineHeight: "1.7" }}>
            <li>FinCEN MSB BSA/AML Compliance Manual</li>
            <li>SEC / Howey Test Non-Security Legal Opinion</li>
            <li>OFAC Sanctions Filtering &amp; Freeze Policy</li>
            <li>USMCA Chapter 19 Digital Trade Memorandum</li>
            <li>Enterprise Master Services Agreement (MSA) &amp; 99.999% SLA</li>
          </ul>
        </div>
        <div className="card">
          <h3>📊 Treasury &amp; Financial Model</h3>
          <ul style={{ paddingLeft: "18px", fontSize: "14px", color: "var(--text-muted)", marginTop: "8px", lineHeight: "1.7" }}>
            <li>Treasury Reserve Investment Policy (100% T-Bills)</li>
            <li>5-Year Unit Economics Pro-Forma Forecast</li>
            <li>Qualified Custodian RFP (BNY / State Street)</li>
            <li>Monthly Independent CPA Attestation Framework</li>
          </ul>
        </div>
        <div className="card">
          <h3>💼 Investor Onboarding</h3>
          <ul style={{ paddingLeft: "18px", fontSize: "14px", color: "var(--text-muted)", marginTop: "8px", lineHeight: "1.7" }}>
            <li>15-Slide Series A Institutional Pitch Deck</li>
            <li>Series A Preferred Stock Term Sheet Template</li>
            <li>Virtual Data Room Master Index</li>
            <li>Cap Table &amp; Delaware C-Corp Corporate Charter</li>
          </ul>
        </div>
        <div className="card">
          <h3>🤝 Consortium Governance</h3>
          <ul style={{ paddingLeft: "18px", fontSize: "14px", color: "var(--text-muted)", marginTop: "8px", lineHeight: "1.7" }}>
            <li>Validator Consortium Operating Charter</li>
            <li>Dynamic Committee Stake Thresholds (2W/3 + 1)</li>
            <li>USMCA Trade Corridor Pilot Partner LOI</li>
            <li>Automated Slashing &amp; Hardware Requirements</li>
          </ul>
        </div>
      </div>

      <div className="alert" style={{ marginTop: "32px", border: "1px solid rgba(139, 92, 246, 0.4)", background: "rgba(18, 26, 40, 0.85)" }}>
        <div className="alert-title" style={{ color: "var(--accent-violet-bright)" }}>
          Institutional Inquiries &amp; Consortium Applications
        </div>
        <p style={{ margin: "4px 0 16px", fontSize: "14px", color: "#c7d2e0" }}>
          Qualified institutions applying for consortium validator seats, pilot clearing integration, or Series A diligence access may contact the treasury and investor relations committee directly.
        </p>
        <div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
          <a href="mailto:capital@veridag.network" className="btn-primary">
            Request VDR Access
          </a>
          <Link href="/faucet" className="btn-secondary">
            Developer Testnet Faucet
          </Link>
          <Link href="/explorer" className="btn-secondary">
            Live DAG Explorer
          </Link>
        </div>
      </div>
    </div>
  );
}
