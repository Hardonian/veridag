# VeriDAG Bank Secrecy Act & Anti-Money Laundering (BSA/AML) Compliance Manual
## FinCEN Money Services Business (MSB) Operating Policy

**Issuing Entity**: VeriDAG Sovereign Settlement Labs, Inc.  
**Effective Date**: September 2026  
**Regulatory Reference**: 31 U.S.C. §§ 5311–5332 (Bank Secrecy Act); 31 CFR Part 1022 (Rules for Money Services Businesses)  
**Designated Compliance Officer**: Chief Compliance Officer (compliance@veridag.network)

---

## 1. Policy Statement & Scope

VeriDAG Sovereign Settlement Labs, Inc. ("VeriDAG" or "the Company") is registered (or in the statutory grace period for filing) as a **Money Services Business (MSB)** with the Financial Crimes Enforcement Network (FinCEN) under 31 CFR § 1022.380 as a money transmitter and virtual currency issuer with respect to the Sovereign Digital Dollar (USDV).

VeriDAG strictly enforces anti-money laundering (AML), counter-terrorist financing (CFT), and Office of Foreign Assets Control (OFAC) sanctions compliance. This manual governs:
1. All institutional minting and redemption of USDV against fiat reserves.
2. All consortium tenant onboarding (`CONSORTIUM_TENANT` object state).
3. The cryptographic screening and enforcement performed by the `veridag-stablecoin` consensus execution layer.

---

## 2. Customer Identification Program (CIP) & Institutional Due Diligence (EDD)

In accordance with 31 CFR § 1022.210, VeriDAG does not offer anonymous retail fiat on/off ramps. USDV issuance and redemption is restricted exclusively to vetted institutional entities:

### A. Required Documentation for Onboarding
Before an address is granted authorization or permitted to mint/redeem USDV:
- **Certificate of Incorporation / Formation** from an approved jurisdiction (US, Canada, Mexico, or G8 member states).
- **Certificate of Good Standing** issued within the preceding 60 days.
- **FinCEN Form 8300 / W-9 / W-8BEN-E** tax identification.
- **Beneficial Ownership Disclosure**: Identification and verification of all natural persons who directly or indirectly own 25% or more of the equity interests, and one individual with significant managerial control (FinCEN Beneficial Ownership Rule).
- **Government-issued photo identification** (Passport / Driver's License) for all authorized signatories and beneficial owners.

### B. Prohibited Counterparties
VeriDAG strictly refuses onboarding to:
- Shell banks or entities without a physical commercial presence.
- Politically Exposed Persons (PEPs) without explicit written CCO exception approval.
- Entities domiciled in Financial Action Task Force (FATF) blacklisted or high-risk jurisdictions.
- Any natural or legal person listed on OFAC's Specially Designated Nationals (SDN) list.

---

## 3. Suspicious Activity Reports (SAR) & Currency Transaction Reports (CTR)

### A. SAR Reporting (31 CFR § 1022.320)
VeriDAG files a Suspicious Activity Report (FinCEN Form 111) within **30 calendar days** of becoming aware of any suspicious transaction or pattern of transactions conducted or attempted by, at, or through the protocol involving **$2,000 or more**, where the Company knows, suspects, or has reason to suspect that:
1. The funds involve illegal activity or are intended to disguise proceeds of crime.
2. The transaction is designed to evade BSA reporting requirements (structuring).
3. The transaction has no commercial rationale or economic purpose.
4. The transaction involves the use of money laundering or terrorist financing channels.

*Confidentiality Notice*: 31 U.S.C. § 5318(g)(2) strictly prohibits disclosing the existence or filing of a SAR to any person involved in the transaction.

### B. CTR Reporting (31 CFR § 1022.310)
VeriDAG files a Currency Transaction Report (FinCEN Form 112) for each deposit, withdrawal, or transfer of currency involving **more than $10,000** in a single business day by or on behalf of the same customer.

---

## 4. On-Chain Screening & Consensus Blacklisting Integration

The `veridag-stablecoin` crate natively enforces compliance at the deterministic state machine level:

```rust
// veridag-stablecoin execution enforcement
if compliance.is_blacklisted(&recipient) || compliance.is_blacklisted(&sender) {
    return Err(ExecutionError::ComplianceViolation("OFAC / FinCEN address sanction enforced"));
}
```

1. **Automated Feed Sync**: The VeriDAG compliance daemon continuously ingests OFAC SDN list updates, FinCEN advisories, and international sanctions registries.
2. **Deterministic State Commitment**: Sanctioned addresses are committed to the `COMPLIANCE_FREEZE` state registry via authorized capability multisig.
3. **Impossibility of Settlement**: Once flagged, the pure-function execution engine deterministically aborts any transaction referencing the sanctioned address, preventing illicit value transfer prior to checkpoint finalization.

---

## 5. Record Retention & Audit Requirements

Under 31 CFR § 1022.400, all records relating to:
- CIP/KYC verification files,
- Beneficial ownership certifications,
- Fiat deposit/withdrawal wire logs,
- On-chain reserve attestation signatures,
- SAR/CTR filings and supporting workpapers,

must be retained for a **minimum period of five (5) years** in secure, tamper-evident digital storage subject to annual independent BSA/AML compliance audits.
