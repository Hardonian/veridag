# USDV: USMCA & G8 Sovereign Multilateral Settlement Architecture

**USDV (Veridag Dollar)** is an institutional sovereign digital dollar engineered for **USMCA** (United States-Mexico-Canada Agreement) cross-border trade corridors and **G8 economic forum** multilateral settlement, anchored by direct US Treasury collateral and built natively on the Veridag DAG-BFT consensus substrate.

---

## 1. Executive Summary

Traditional cross-border settlement and commercial stablecoins (USDT, USDC) rely either on opaque centralized databases or high-fee, reorg-vulnerable smart contracts prone to settlement delay and counterparty exposure. USDV establishes an institutional sovereign financial rail by providing:

- **Direct US Strategic & Treasury Alignment:** 100% backed by short-term US Treasury Bills ($\le 90$ days), overnight reverse repurchase agreements (RRP), and FDIC-insured cash deposits, with cryptographic Proof of Reserves (PoR) committed into every state root.
- **USMCA Cross-Border Trade Corridor Clearing:** Sub-100ms wave finality removes foreign exchange settlement risk (Herstatt risk), cross-border clearing friction, and correspondent banking delays across United States, Canada, and Mexico trade flows.
- **G8-Grade Multilateral Settlement Invariants:** Compliant with CPMI-IOSCO Principles for Financial Market Infrastructures (PFMI) and Basel III liquidity standards, guaranteeing deterministic settlement without MEV re-ordering.
- **Capability-Gated Institutional Compliance:** Programmable, multi-sig capability keys enforce real-time OFAC sanctions screening, FATF Travel Rule compliance, address freeze/unfreeze, and court-ordered fund quarantine without unilateral backdoors.

```text
┌─────────────────────────────────────────────────────────────┐
│                    USDV TRUST HIERARCHY                     │
├──────────────────────────────┬──────────────────────────────┤
│ 🏛️ Collateral Reserves       │ • US Treasury Bills (<=90d)  │
│                              │ • Overnight Reverse Repo     │
│                              │ • FDIC-Insured Cash Deposits │
├──────────────────────────────┼──────────────────────────────┤
│ 📜 Proof of Reserves Oracle  │ • Signed Attestation Epochs  │
│                              │ • BNY Mellon / State Street  │
├──────────────────────────────┼──────────────────────────────┤
│ 🛡️ Invariant State Engine    │ • Supply <= Reserves         │
│                              │ • Supply == Sum(Balances)    │
│                              │ • 6-Decimal Fixed Precision  │
├──────────────────────────────┼──────────────────────────────┤
│ ⚡ Settlement Substrate       │ • Sub-100ms Wave Finality    │
│                              │ • Pure-Function DAG-BFT      │
└──────────────────────────────┴──────────────────────────────┘
```

---

## 2. Core Mathematical Invariants

Every state transition enforces strict invariants checked by `StablecoinLedger::verify_invariants`:

### Invariant 1: Proof of Reserves Upper Bound

$$\text{TotalCirculatingSupply} \le \text{TotalAttestedReserves}$$

The protocol forbids minting any token not backed 1:1 by verified collateral.

### Invariant 2: Conservation of Value

$$\text{TotalSupply} \equiv \sum_{a \in \text{Accounts}} a.\text{balance}$$

Value cannot leak, double-count, or generate out of thin air.

### Invariant 3: Sanctions Freeze Invariance

$$\forall a \in \text{Accounts}, a.\text{frozen} = \text{true} \implies \Delta a.\text{balance} = 0$$

Frozen accounts cannot send or receive funds under any non-compliance transaction.

---

## 3. Institutional Governance & Capabilities

Access control is governed by cryptographic **Object Capabilities**:

- `MintCapability`: Granted to authorized treasury minters; constrained by available unbacked reserves.
- `BurnCapability`: Granted to redemption accounts; burns USDV and emits verifiable redemption receipts for fiat wire settlement.
- `ComplianceCapability`: Granted to compliance officers; executes address freeze, unfreeze, and fund seizure into escrow.
- `OracleCapability`: Granted to custodian institutions; submits signed reserve attestations.

---

## 4. CLI Developer Quickstart

```bash
# 1. Attest $100M in reserves from institutional custodian
veridag-cli usdv attest-reserves --tbills 80000000 --cash 15000000 --repo 5000000

# 2. Mint $5M USDV to Alice
veridag-cli usdv mint --to alice --amount 5000000

# 3. Instant DAG transfer from Alice to Bob
veridag-cli usdv transfer --from alice --to bob --amount 1500000

# 4. Check Bob's balance
veridag-cli usdv balance --account bob

# 5. Enforce compliance freeze on Bob
veridag-cli usdv freeze --target bob

# 6. Verify attempt to transfer from Bob fails
veridag-cli usdv transfer --from bob --to alice --amount 500000 # REJECTED

# 7. Run mathematical invariant audit
veridag-cli usdv audit
```
