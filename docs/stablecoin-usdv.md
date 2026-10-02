# USDV: Experimental Regulated-Settlement Architecture

**USDV (Veridag Dollar)** is an experimental asset and control model for regulated-settlement pilots on Veridag. The code does not create reserve backing, legal authorization, custody, or regulatory compliance; those are deployment responsibilities of an authorized operator.

---

## 1. Executive Summary

The prototype demonstrates the following technical building blocks:

- **Reserve evidence commitments:** Signed reserve figures can be committed to state and used to bound protocol-side minting. The software does not verify custodians or reserve assets.
- **Deterministic settlement:** Fixed-precision transitions and checkpoint commitments support measurable corridor pilots; no latency or risk-elimination claim is made without deployment evidence.
- **Policy controls:** Separate capabilities govern minting, burning, freezing, pausing, and reserve updates. Sanctions data and legal decisions are supplied by the operator.
- **Control mapping:** Architecture can be assessed against PFMI, Basel, and jurisdiction-specific requirements, but no conformance or approval is implied.

```text
┌─────────────────────────────────────────────────────────────┐
│                    USDV TRUST HIERARCHY                     │
├──────────────────────────────┬──────────────────────────────┤
│ 🏛️ Collateral Reserves       │ • US Treasury Bills (<=90d)  │
│                              │ • Overnight Reverse Repo     │
│                              │ • FDIC-Insured Cash Deposits │
├──────────────────────────────┼──────────────────────────────┤
│ 📜 Proof of Reserves Oracle  │ • Signed Attestation Epochs  │
│                              │ • Operator-selected custodian│
├──────────────────────────────┼──────────────────────────────┤
│ 🛡️ Invariant State Engine    │ • Supply <= Reserves         │
│                              │ • Supply == Sum(Balances)    │
│                              │ • 6-Decimal Fixed Precision  │
├──────────────────────────────┼──────────────────────────────┤
│ ⚡ Settlement Substrate       │ • Measured per deployment    │
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
