# 21 — Normative Specification: Bitcoin Infrastructure Substrate

**Status:** Normative  
**Version:** 1.0  
**Scope:** Veridag as Bitcoin Execution, UTXO Settlement, and SPV Substrate  

---

## 1. Overview

Veridag extends its sovereign DAG-BFT consensus substrate to provide **iron-clad, zero-reorg, trustless interoperability and settlement for the Bitcoin network**.

By pairing Veridag's sub-100ms deterministic wave commits with Bitcoin's global PoW store-of-value liquidity, this architecture delivers:

1. **Lightweight On-Chain SPV Verification:** Continuous verification of canonical 80-byte Bitcoin block headers, compact difficulty target expansion (`nBits`), and Proof-of-Work validation without trusted third-party oracles.
2. **Deterministic Transaction Inclusion Proofs:** Cryptographic verification of partial Merkle tree branches (`txid` inclusion in block `merkle_root`) via Double-SHA256 (`hash256`).
3. **Two-Way UTXO Bridge Primitives:** Cryptographically locked Bitcoin deposits mapped to native Veridag tokens and deterministic consensus-verified withdrawal authorization requests.
4. **Native Bitcoin JSON-RPC Compatibility:** Direct emulation of Bitcoin core RPC interfaces (`getblockcount`, `getblockhash`, `getblockheader`) allowing existing Bitcoin enterprise tooling, custody vaults, and indexers to interface directly with Veridag.

---

## 2. Bitcoin SPV Verification Architecture

```text
┌────────────────────────────────────────────────────────┐
│                     Bitcoin Network                    │
│                                                        │
│   ┌────────────────────────┐  ┌────────────────────┐   │
│   │   80-Byte Block Header │  │   UTXO Transaction │   │
│   │   • Version, Time, nBits│  │   • OP_RETURN / P2WSH  │   │
│   │   • Previous Block Hash│  │   • Bridge Deposit     │   │
│   │   • Merkle Root & Nonce│  │   • Multi-Sig Escrow   │   │
│   └───────────▲────────────┘  └─────────▲──────────┘   │
│               │                         │              │
│               └────────────┬────────────┘              │
│                            │                           │
│                 Double-SHA256 SPV Proof                │
│                            │                           │
└────────────────────────────┼───────────────────────────┘
                             │
             Block Headers & Merkle Audit Trails
                             │
┌────────────────────────────▼───────────────────────────┐
│              Veridag DAG-BFT Engine (L2)               │
│                                                        │
│  • BtcSpvHeaderTracker (Continuous Header Chain)       │
│  • Target Difficulty & POW Verification                │
│  • BitcoinMerkleProof Validation                       │
│  • CrossChainBtcDeposit & Withdrawal Processing        │
│  • BtcJsonRpcProvider (getblockheader, getblockcount)  │
└────────────────────────────────────────────────────────┘
```

---

## 3. Cryptographic Primitives

### 3.1 Double-SHA256 (`hash256`)

All Bitcoin hashing operations within Veridag MUST strictly use double-application of the standard SHA-256 compression function:

$$\text{hash256}(m) = \text{SHA256}(\text{SHA256}(m))$$

Digest output is 32 bytes in length.

### 3.2 80-Byte Header Serialization

Canonical Bitcoin block headers MUST conform strictly to the 80-byte binary layout:

| Offset | Length (bytes) | Field | Encoding | Description |
| :--- | :--- | :--- | :--- | :--- |
| 0 | 4 | `version` | Little-Endian `i32` | Block version number |
| 4 | 32 | `prev_block` | Little-Endian `[u8; 32]` | SHA256d hash of previous header |
| 36 | 32 | `merkle_root` | Little-Endian `[u8; 32]` | SHA256d root of transaction tree |
| 68 | 4 | `time` | Little-Endian `u32` | Block timestamp (Unix epoch) |
| 72 | 4 | `bits` | Little-Endian `u32` | Compact difficulty target (`nBits`) |
| 76 | 4 | `nonce` | Little-Endian `u32` | Proof-of-work nonce |

### 3.3 Compact Difficulty Expansion (`nBits`)

The 32-bit compact representation `0xEECCCC` expands into a 256-bit scalar target $T$:

$$E = \text{bits} \gg 24$$
$$C = \text{bits} \ \& \ \text{0x00FFFFFF}$$
$$T = C \times 256^{(E - 3)}$$

A block header is valid if and only if:

$$\text{hash256}(\text{header}_{80}) \le T$$

---

## 4. SPV Merkle Proofs

A `BitcoinMerkleProof` proves that transaction $T_x$ with transaction identifier `txid` is committed within the block represented by `merkle_root`:

```rust
pub struct BitcoinMerkleProof {
    pub txid: [u8; 32],
    pub index: u32,
    pub siblings: Vec<[u8; 32]>,
}
```

Verification iteratively combines the current hash with its sibling based on whether the index is even or odd:

- If `current_index % 2 == 0`: `parent = hash256(current_hash || sibling)`
- If `current_index % 2 == 1`: `parent = hash256(sibling || current_hash)`
- `current_index = current_index / 2`

The root of the computation MUST exactly match the 32-byte `merkle_root` in the validated header.

---

## 5. Cross-Chain Deposit and Withdrawal Protocol

### 5.1 CrossChainBtcDeposit

Cross-chain BTC inflows are initiated by locking Bitcoin UTXOs into a verifiably monitored multi-signature escrow address:

| Field | Type | Description |
| :--- | :--- | :--- |
| `btc_tx_hash` | `[u8; 32]` | Bitcoin transaction hash (`txid`) |
| `vout` | `u32` | Output index containing the escrow lock |
| `amount_satoshis`| `u64` | Satoshis deposited ($10^{-8}$ BTC) |
| `veridag_recipient` | `[u8; 32]` | Recipient address on Veridag DAG |
| `btc_block_height` | `u64` | Bitcoin block height anchoring the deposit |
| `sequence` | `u64` | Monotonically increasing cross-chain nonce |

### 5.2 CrossChainBtcWithdrawal

Withdrawals burn native wrapped BTC or stable collateral on Veridag, emitting an immutable signed withdrawal event:

| Field | Type | Description |
| :--- | :--- | :--- |
| `veridag_sender` | `[u8; 32]` | Initiating account on Veridag |
| `btc_destination` | `String` | Base58Check or Bech32/Bech32m destination address |
| `amount_satoshis` | `u64` | Satoshis requested for settlement |
| `sequence` | `u64` | Monotonically increasing withdrawal nonce |

---

## 6. Bitcoin JSON-RPC Gateway Mapping

| Bitcoin RPC Method | Veridag DAG Substrate Mapping |
| :--- | :--- |
| `getblockcount` | Returns the total count of verified Bitcoin block headers in the SPV tracker. |
| `getblockhash(height)` | Returns the canonical 32-byte hex hash of the block header at height. |
| `getblockheader(hash, verbose)` | Returns parsed JSON object containing `version`, `previousblockhash`, `merkleroot`, `time`, `bits`, `nonce`, and computed `confirmations`. |

---

## 7. Security and Invariants

1. **PoW Integrity:** Every header ingested into the SPV tracker MUST strictly satisfy difficulty target $T$.
2. **Chain Continuity:** Each header's `prev_block` pointer MUST match the preceding header in the local tracker unless configured as the genesis anchor.
3. **Replay Protection:** Every deposit and withdrawal MUST enforce strictly monotonically increasing `sequence` nonces per recipient/sender.
4. **Safety Under Reorgs:** Bitcoin deposits are finalized on Veridag only after exceeding a configurable confirmation depth (default: 6 confirmations).
