# VeriDAG USDV Treasury Management & Reserve Investment Policy

**Entity**: VeriDAG Sovereign Settlement Labs, Inc. / USDV Reserve Trust  
**Effective Date**: September 2026  
**Supervisory Authority**: Treasury Management Committee (treasury@veridag.network)  
**Mandate**: Absolute Capital Preservation, Continuous Liquidity, and 100% Mathematical Solvency

---

## 1. Permitted Reserve Assets (Eligible Investments)

To eliminate credit risk and ensure immediate liquidity under extreme market volatility, 100% of all USDV reserve assets must be held in the following three eligible asset classes:

| Asset Class | Allocation Target | Maximum Maturity | Safety / Insurance Profile |
|---|---|---|---|
| **Short-Term US Treasury Bills** | 70% – 85% | $\le 90$ Days (4-week & 13-week bills) | Backed by the full faith and credit of the US Government |
| **Federal Reserve Overnight Reverse Repo (ON RRP)** | 10% – 20% | Overnight (1 business day) | Direct Federal Reserve System collateralized transaction |
| **FDIC-Insured Cash & Demand Deposits** | 5% – 10% | Instant / Same-Day | Insured cash in segregated accounts at qualified US banks |

### Strictly Prohibited Assets
Under no circumstances may reserve assets include:
- Commercial paper or corporate debt instruments.
- Unsecured interbank deposits.
- Municipal or foreign sovereign debt.
- Digital assets, cryptocurrencies, or algorithmic balancing tokens.
- Leveraged derivatives, futures, or credit default swaps.

---

## 2. Custodial Segregation & Bankruptcy-Remote Trust Structure

1. **Segregated Trust Account**: All reserve assets are held in segregated trust accounts at approved Qualified Custodians (e.g., BNY Mellon, State Street, Anchorage Digital) in the name of the **USDV Reserve Trust** for the sole and exclusive benefit of USDV token holders.
2. **Bankruptcy Remoteness**: The trust is structured so that in the event of an insolvency, bankruptcy, or restructuring of VeriDAG Sovereign Settlement Labs, Inc., the reserve assets do not form part of the general bankruptcy estate and cannot be attached by general creditors.
3. **No Rehypothecation**: Custodians are legally forbidden from lending, hypothecating, or re-pledging any portion of the reserve assets.

---

## 3. Independent Proof-of-Reserves (PoR) & CPA Attestations

VeriDAG enforces a dual-track reserve verification protocol:

```
[Off-Chain Custodian Balances]
             │
             ├─► API Telemetry / Custodian Sig
             ▼
[VeriDAG Proof-of-Reserves Substrate] ──► Committed to Consensus State Root
             │
             ▼
[Independent Top-Tier Accounting Firm (RSM / Grant Thornton)]
             │
             ▼ (Monthly Public Examination under AICPA AT-C 205)
[Public Attestation Report Published on-chain and web portal]
```

1. **Daily On-Chain Attestation**: Authorized custodians submit cryptographic signatures certifying aggregate USD balances, which are verified and committed into the DAG state root.
2. **Monthly Independent CPA Examination**: An accredited, PCAOB-registered public accounting firm conducts monthly examinations under AICPA Attestation Standards (AT-C Section 205) and issues public opinion letters confirming reserve assets equal or exceed outstanding USDV tokens.

---

## 4. Redemption SLA & Emergency Liquidity Operations

1. **Standard Institutional Redemptions**: Same-day redemption processing for wires submitted prior to 3:00 PM EST; T+1 for wires submitted post-cutoff.
2. **24/7 Primary Liquidity**: The 5–10% cash and ON RRP buffer provides instant automated liquidity for consortium settlement clearing around the clock.
3. **Emergency Circuit Breaker**: If redemption requests exceed 30% of total float within a 2-hour window, the consensus engine invokes an orderly pacing buffer to liquidate underlying Treasury bills without secondary market slippage.
