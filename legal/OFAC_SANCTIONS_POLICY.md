# VeriDAG Office of Foreign Assets Control (OFAC) Sanctions Policy
## Institutional Compliance & Autonomous On-Chain Enforcement Manual

**Entity**: VeriDAG Sovereign Settlement Labs, Inc.  
**Effective Date**: September 2026  
**Applicable Authority**: International Emergency Economic Powers Act (IEEPA), 50 U.S.C. §§ 1701–1706; Trading with the Enemy Act (TWEA), 50 U.S.C. §§ 4301–4341; 31 CFR Chapter V (OFAC Regulations)  
**Supervisory Officer**: Chief Compliance Officer (compliance@veridag.network)

---

## 1. Corporate Policy Mandate

VeriDAG strictly complies with all economic and trade sanctions administered and enforced by the U.S. Department of the Treasury's Office of Foreign Assets Control (OFAC). VeriDAG prohibits the use of its protocol, sovereign digital dollar (USDV), multi-chain bridges, and consortium clearing services by, on behalf of, or for the benefit of:
1. Any jurisdiction subject to comprehensive territorial sanctions (presently including Cuba, Iran, North Korea, Syria, and the sanctioned regions of Ukraine).
2. Any individual, entity, vessel, or aircraft designated on OFAC's Specially Designated Nationals and Blocked Persons List (SDN List).
3. Any entity owned 50% or more, directly or indirectly, in the aggregate by one or more blocked persons ("OFAC 50 Percent Rule").

---

## 2. Dual-Layer Enforcement Architecture

VeriDAG enforces sanctions compliance through a dual-layer strategy combining off-chain institutional KYC with autonomous, cryptographically verified on-chain execution blocking:

```
[OFAC / Treasury SDN Updates]
              │
              ▼
[VeriDAG Compliance Oracle / Multisig]
              │ (Signs COMPLIANCE_FREEZE transaction)
              ▼
[Deterministic State Machine (BMH-1)]
              │
              ├─► Transfer Ingress Screened
              │   (Sender/Receiver checked against state root)
              ▼
    Is Address Sanctioned?
       ├── YES ──► State Machine Aborts Tx (Rejection Receipt)
       └── NO  ──► Settlement Finalized into Checkpoint
```

---

## 3. Protocol Consensus Enforcement

The protocol execution crate (`veridag-stablecoin`) enforces zero-tolerance sanctions filtering during state transition execution:

1. **Pre-Execution Check**: Before mutating any account balance or executing a Settler reconciliation batch, the sequential state machine queries the compliance registry:
   ```rust
   // Enforced across all USDV and multi-asset transfers
   if self.compliance.is_blacklisted(&tx.sender) || self.compliance.is_blacklisted(&tx.recipient) {
       return Err(ExecutionError::SanctionedEntityBlocked);
   }
   ```
2. **Deterministic Rejection**: Transactions referencing sanctioned addresses are rejected across all consensus validators. No validator can include or commit a prohibited transaction without violating consensus rules.
3. **Asset Freezing (`COMPLIANCE_FREEZE`)**: Upon receiving a valid court order, regulatory mandate, or verified OFAC designation, the compliance authority submits a signed capability action that locks the target object state. Frozen funds cannot be moved, transferred, or redeemed until formally cleared by compliance and legal counsel.

---

## 4. Reporting Blocked & Rejected Transactions

In accordance with 31 CFR § 501.603:
- **Blocked Property Reporting**: VeriDAG must report all blocked virtual currency accounts and property to OFAC within **ten (10) business days** of the blocking action via the OFAC Reporting System (ORS).
- **Annual Report of Blocked Property**: By September 30 of each year, VeriDAG submits a comprehensive annual report of all blocked property held as of June 30.
- **Unblocking Procedure**: Property may only be unblocked pursuant to specific licenses issued by OFAC or formal delisting published in the Federal Register.
