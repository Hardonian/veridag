# USDV: Institutional US Sovereign Stablecoin Architecture

**USDV (Veridag Dollar)** is an institutional-grade, reserve-backed, capability-governed US sovereign digital dollar built natively on the Veridag DAG-BFT consensus substrate.

---

## 1. Executive Summary

Traditional stablecoins (USDT, USDC) rely either on centralized off-chain databases or high-fee, reorg-vulnerable smart contracts. USDV transforms the US stablecoin paradigm by providing:
- **100% Backed Proof of Reserves (PoR):** Guaranteed by short-term US Treasury Bills ($\le 90$ days), overnight reverse repurchase agreements (RRP), and FDIC-insured cash deposits, attested cryptographically into every state root.
- **Microsecond DAG Wave Settlement:** Finalized in sub-100ms waves via pure-function DAG-BFT consensus.
- **Zero MEV & Sandwich Attack Protection:** Invariant causal ordering eliminates front-running and toxic value extraction.
- **Capability-Gated Compliance:** Programmable, multi-sig capability keys enforce real-time OFAC sanctions screening, address freeze/unfreeze, and court-ordered fund seizure without backdoors.

```
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
