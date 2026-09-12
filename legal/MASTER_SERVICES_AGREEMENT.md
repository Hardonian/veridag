# VeriDAG Institutional Master Services Agreement (MSA) & SLA

**Effective Date**: September 2026  
**Between**: VeriDAG Sovereign Settlement Labs, Inc. ("Provider") and the Contracting Enterprise / Financial Institution ("Client")

---

## 1. Engagement & Platform Access

Subject to the terms and conditions of this Master Services Agreement ("Agreement"), Provider grants Client a non-exclusive, worldwide, institutional license to:
1. Access and utilize the VeriDAG distributed settlement network and EVM JSON-RPC gateways.
2. Mint and redeem Sovereign Digital Dollars (USDV) at par ($1.00 USDV = $1.00 USD) against segregated qualified custodian reserves.
3. Submit Settler reconciliation batches and cryptographic state anchors (`SETTLER_ANCHOR`) into the consensus DAG.
4. Deploy autonomous agents authorized by capability-scoped host ABIs under `veridag-wasm-runtime`.

---

## 2. Service Level Agreement (SLA): 99.999% High Availability

### A. Uptime Commitment
Provider covenants that the VeriDAG consensus network and institutional gateway APIs will achieve **99.999% Service Availability** ("Five Nines") in each calendar month, excluding scheduled maintenance windows announced at least 72 hours in advance.

$$\text{Availability \%} = \frac{\text{Total Operational Minutes} - \text{Unscheduled Downtime Minutes}}{\text{Total Operational Minutes}} \times 100 \ge 99.999\%$$

### B. Finality & Latency Commitments
- **DAG Batch Ingress**: Sub-50ms acknowledgement across authenticated QUIC validator streams.
- **BFT Quorum Wave Commit**: Deterministic finality achieved within **sub-300ms** under normal network conditions ($n=4, f=1$).
- **Zero Settlement Variance**: Every committed transaction guarantees mathematical balance conservation ($\sum \Delta \text{Balances} = 0$).

### C. Service Credits
If monthly availability falls below 99.999%, Client is entitled to the following fee credits against next month's settlement protocol fees:
- **99.90% to 99.99%**: 15% Monthly Protocol Fee Credit
- **99.00% to 99.89%**: 30% Monthly Protocol Fee Credit
- **< 99.00%**: 50% Monthly Protocol Fee Credit

---

## 3. Custody, Reserve Assurance & Parity Guarantee

1. **100% Bankruptcy-Remote Backing**: Provider guarantees that 100% of all outstanding USDV in circulation is backed by eligible reserve assets (short-term US Treasury Bills, Federal Reserve Reverse Repo, and FDIC cash deposits) held by an independent Qualified Custodian in trust for USDV holders.
2. **Monthly CPA Attestation**: Provider warrants that an independent, PCAOB-registered accounting firm will perform monthly examinations of reserve assets and publish formal attestation reports reconciling on-chain state roots with custodian balances.
3. **No Commingling**: Under no circumstances shall reserve assets be pledged, rehypothecated, loaned, or commingled with the operating capital of the Provider.

---

## 4. Representations, Warranties & Compliance

1. **Client BSA/AML Warranty**: Client represents and warrants that it maintains an active AML/CIP compliance program, has completed VeriDAG institutional onboarding, and that no funds transferred originate from sanctioned persons or jurisdictions.
2. **Provider Security Warranty**: Provider warrants that the software reference implementation enforces strict `#![forbid(unsafe_code)]`, passes all automated protocol test vectors, and incorporates hardware-security module (HSM) signing capabilities.

---

## 5. Limitation of Liability & Indemnification

1. **Direct Damages Cap**: Except for gross negligence, willful misconduct, or breach of confidentiality, either party's aggregate liability under this Agreement shall not exceed the total fees paid by Client to Provider during the twelve (12) months preceding the event.
2. **Indemnification by Provider**: Provider shall defend, indemnify, and hold harmless Client against any third-party claims alleging that the VeriDAG core consensus protocol infringes any intellectual property right.

---

## 6. Governing Law & Jurisdiction

This Agreement shall be governed by and construed in accordance with the laws of the **State of Delaware**, without regard to conflict of laws principles. Any legal suit, action, or proceeding arising out of or related to this Agreement shall be instituted exclusively in the federal or state courts located in New Castle County, Delaware.
