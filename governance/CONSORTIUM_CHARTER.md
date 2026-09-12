# VeriDAG Institutional Validator Consortium Charter
## Dynamic Committee Governance & Operating Standards

**Governing Entity**: VeriDAG Sovereign Validator Consortium  
**Effective Date**: September 2026  
**Consensus Engine**: BaselineDagBft with Dynamic Committee Weighting (Phase 20 / Spec 16)  
**Stake Quorum Formula**: $\lfloor \frac{2W}{3} \rfloor + 1$ (where $W$ is total active committee weight)

---

## 1. Consortium Purpose & Sovereign Principles

The VeriDAG Validator Consortium is an alliance of institutional participants, qualified custodians, trade logistics clearinghouses, and AI agent networks dedicated to maintaining the liveness, safety, and regulatory integrity of the VeriDAG settlement substrate.

Consortium operations adhere to four non-negotiable principles:
1. **Mathematical Byzantine Fault Tolerance**: Quorum agreement is a pure function of the Narwhal DAG; no unilateral central party can alter transaction order or reverse committed checkpoints.
2. **Zero Unsafe Execution**: All validator nodes must execute the official release binaries compiled under `#![forbid(unsafe_code)]`.
3. **Hardware Isolation**: Consensus traffic is strictly isolated to the authenticated QUIC mesh using self-signed Ed25519 TLS 1.3 certificates.
4. **Conservation of Sovereign Value**: Validators deterministically reject any proposed transaction or block that violates parity or attempts illicit balance mutation.

---

## 2. Validator Node Hardware & Operational Requirements

To guarantee sub-300ms finality and 99.999% network uptime, each validator node must satisfy:

| Component | Minimum Specification | Recommended Production Standard |
|---|---|---|
| **CPU** | 16 Cores (x86_64 with AVX-512 / ARM64 Graviton 3) | 32 Cores, 3.5 GHz+ base clock |
| **RAM** | 64 GB ECC DDR5 | 128 GB ECC DDR5 |
| **Storage** | 2 TB NVMe PCIe 4.0 (100k+ IOPS) | 4 TB Enterprise U.2 NVMe in RAID 1 |
| **Network** | 1 Gbps symmetric unmetered fiber | 10 Gbps redundant multi-homed BGP |
| **Key Signing** | FIPS 140-2 Level 3 HSM / Cloud KMS | AWS KMS, GCP Cloud KMS, or YubiHSM 2 |
| **Redundancy** | Dual hot-standby nodes with automatic failover | Multi-region geographic distribution |

---

## 3. Dynamic Committee Reconfiguration & Epoch Transitions

As specified in Normative Specification 16 (`16-validator-membership.md`):

1. **Epoch Handovers**: The consortium operates on 24-hour epochs ($E$). Committee membership changes take effect strictly at the epoch checkpoint boundary:
   ```rust
   // Dynamic committee quorum requirement
   let total_weight: u64 = committee.validators().iter().map(|v| v.weight).sum();
   let quorum_threshold = (2 * total_weight) / 3 + 1;
   ```
2. **Weighted Stake Voting**: Voting power is proportional to bonded stake. No single institutional validator or affiliated corporate group may hold more than **25% of total committee weight ($W$)**, ensuring a minimum of $3f+1$ decentralization across distinct sovereign jurisdictions.
3. **Graceful Handover**: The outgoing committee certifies the initial root of the incoming committee within the final epoch checkpoint (`EpochHandoverTracker`), ensuring zero liveness interruption during validator onboarding or decommissioning.

---

## 4. Slashing & Disciplinary Rules

To ensure economic security, bonded validator stakes are subject to deterministic slashing for protocol violations:

| Violation | Severity | Evidence Requirement | Slashing Penalty |
|---|---|---|---|
| **Equivocation** (Double-proposing conflicting vertices in the same round) | Critical | Two cryptographically valid vertices signed by the same node for round $r$ | **100% of Bonded Stake Slashed** + Permanent Ban |
| **Spurious Forking / L1 Bridge Tampering** | Critical | Invalid Merkle inclusion proof or conflicting state root attestation | **100% of Bonded Stake Slashed** |
| **Extended Unscheduled Downtime** (> 2 hours without handover) | Moderate | Missed round anchor proposals for $\ge 500$ consecutive rounds | **5% Penalty** + Temporary Committee Suspension |
| **Sanctions Non-Compliance** (Attempting to validate blacklisted addresses) | Severe | Consensus rejection receipt showing deliberate inclusion of SDN address | **50% Penalty** + Referral to Legal Council |

---

## 5. Consortium Governance Council

The Consortium is governed by a **Seven-Member Governance Council**:
- Three (3) seats elected by active validator node operators.
- Two (2) seats held by Qualified Custodian & Reserve Trust representatives.
- One (1) seat held by USMCA Commercial Settlement Clearinghouse partners.
- One (1) seat held by VeriDAG Sovereign Settlement Labs engineering team.

*Quorum for Governance Actions*: Amendments to this Charter, slashing penalty executions, or protocol parameter upgrades require a supermajority vote of at least **five (5) out of seven (7) Council votes**.
