# 19 — Normative Specification: US Sovereign Stablecoin (USDV)

**Status:** Normative  
**Version:** 1.0  
**Scope:** Veridag Native Stablecoin Architecture, Proof of Reserves (PoR), Compliance Engine, and Invariants  

---

## 1. Overview

**USDV (Veridag Dollar)** is an institutional-grade, reserve-backed, capability-governed US sovereign digital dollar built natively on the Veridag DAG-BFT consensus substrate.

Unlike algorithmic or weakly-governed tokens, USDV guarantees:
1. **1:1 Collateral Invariant:** Total circulating supply is mathematically bounded by cryptographically attested institutional reserves (US Treasury bills with maturities $\le 90$ days, overnight reverse repurchase agreements, and FDIC-insured cash deposits).
2. **Deterministic Micro-Unit Accounting:** Fixed 6-decimal precision ($10^{-6}$ USD) stored in 128-bit integers (`u128`), ensuring zero loss of precision across multi-billion-dollar transaction batches.
3. **Capability-Enforced Governance:** Minting, burning, sanctions enforcement, and reserve updates require distinct, unforgeable cryptographic capabilities. No single key or backdoor can inflate supply or bypass compliance.
4. **Sub-100ms Wave Settlement:** Transfers achieve irreversible finality upon DAG wave commitment, eliminating MEV, sandwich attacks, and settlement counterparty risk.

---

## 2. Object Definitions

### 2.1 Stablecoin Object (`object_type = 3`)

A USDV balance object represents sovereign stablecoin value held by an address:

```text
USDV Account Object:
  id:          ObjectId (derived from Owner Address || Token Domain)
  object_type: 3 (STABLECOIN)
  owner:       Ownership::Address(owner_address)
  payload:     VCE-1 encoded StablecoinAccountPayload
```

#### Schema: `StablecoinAccountPayload`
| Field | Type | Description |
| :--- | :--- | :--- |
| `balance` | `u128` | Balance in micro-dollars ($1 \text{ USD} = 1{,}000{,}000$). |
| `frozen` | `bool` | True if address is frozen under compliance order. |
| `nonce` | `u64` | Monotonic account mutation nonce. |

---

### 2.2 Reserve Attestation Object (`object_type = 4`)

A cryptographic attestation representing verified institutional reserves backing USDV:

```text
Reserve Attestation Object:
  id:          ObjectId (derived from Oracle Address || Attestation Epoch)
  object_type: 4 (RESERVE_ATTESTATION)
  owner:       Ownership::System
  payload:     VCE-1 encoded ReserveAttestationPayload
```

#### Schema: `ReserveAttestationPayload`
| Field | Type | Description |
| :--- | :--- | :--- |
| `oracle_id` | `Address` | Public key of the verified institutional custodian oracle. |
| `epoch` | `u64` | Protocol epoch of the attestation. |
| `timestamp` | `u64` | Unix timestamp of reserve verification. |
| `treasury_bills_cents` | `u128` | Reserve amount in short-term US Treasuries. |
| `cash_deposits_cents` | `u128` | Reserve amount in FDIC-insured bank deposits. |
| `reverse_repo_cents` | `u128` | Reserve amount in overnight reverse repurchase agreements. |
| `total_reserves_cents` | `u128` | Aggregate attested collateral value. |
| `custodian_signature` | `Ed25519Signature` | Signature over `VERIDAG_POR_V1 || payload_hash`. |

---

## 3. Mathematical Invariants

Every state transition involving USDV must satisfy these non-negotiable invariants:

### Invariant 1: Proof of Reserves Upper Bound
$$\text{TotalCirculatingSupply} \le \text{TotalAttestedReserves}$$
No transaction or mint operation may cause total circulating supply to exceed the latest verified reserve attestation.

### Invariant 2: Conservation of Value
$$\text{TotalSupply} = \sum_{a \in \text{Accounts}} a.\text{balance}$$
Stablecoin value cannot be created or destroyed except through authorized `Mint` or `Burn` operations carrying verified capabilities.

### Invariant 3: Sanctions & Freeze Invariance
$$\forall a \in \text{Accounts}, a.\text{frozen} = \text{true} \implies \Delta a.\text{balance} = 0$$
Frozen accounts cannot send, receive, or transfer funds. Any transaction referencing a frozen account must fail deterministically with `TxExecError::Unauthorized`.

---

## 4. Capability Governance Matrix

| Capability Kind | Authorized Actions | Constraints & Safety Rules |
| :--- | :--- | :--- |
| **`MintCapability`** | Mint new USDV to designated KYC address. | Enforces Invariant 1 ($\text{Supply} + \text{Amount} \le \text{Reserves}$). Bounded by maximum per-epoch mint quota. |
| **`BurnCapability`** | Burn USDV in exchange for fiat wire redemption receipt. | Decrements circulating supply atomically. Produces cryptographic receipt with redemption hash. |
| **`ComplianceCapability`** | Freeze, unfreeze, or quarantine illicit funds. | Subject to multi-sig quorum. Requires reference to valid regulatory / judicial enforcement identifier. |
| **`OracleCapability`** | Commit new Proof-of-Reserves attestation. | Restricted to whitelisted institutional custodian addresses (e.g. State Street, BNY Mellon, BlackRock). |
| **`PauseCapability`** | Halt all non-compliance transfers globally. | Emergency circuit-breaker; expires automatically after $N$ epochs unless renewed. |

---

## 5. Transaction Operations

### 5.1 `Mint`
1. Verify caller presents valid `MintCapability`.
2. Verify `latest_reserves >= total_supply + amount`.
3. Increment recipient balance by `amount`.
4. Increment `total_supply` by `amount`.
5. Emit `Receipt` with updated state root.

### 5.2 `Burn`
1. Verify caller owns source balance and presents `BurnCapability` (or caller is owner).
2. Verify source `balance >= amount`.
3. Decrement source balance by `amount`.
4. Decrement `total_supply` by `amount`.
5. Emit `Receipt` containing redemption voucher hash.

### 5.3 `Transfer`
1. Verify neither sender nor recipient is frozen.
2. Verify system is not paused.
3. Verify sender `balance >= amount`.
4. Debit sender, credit recipient atomically.
5. Invariant check: $\text{sender}.\text{pre} + \text{recipient}.\text{pre} = \text{sender}.\text{post} + \text{recipient}.\text{post}$.
