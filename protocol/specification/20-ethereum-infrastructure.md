# 20 — Normative Specification: Ethereum Infrastructure Substrate

**Status:** Normative  
**Version:** 1.0  
**Scope:** Veridag as Ethereum L2 / High-Throughput Execution Substrate, EVM Interoperability, and L1 Settlement  

---

## 1. Overview

Veridag functions as an **iron-clad, high-performance execution, sequencing, and settlement substrate for the Ethereum ecosystem**.

By combining Veridag's pure-function DAG-BFT consensus with Ethereum's global liquidity and settlement guarantees, this architecture delivers:
1. **Zero-Reorg Finality:** Veridag wave commits eliminate transaction reorganizations, MEV re-ordering, and front-running on Ethereum L2 applications.
2. **Trustless L1 Light Client:** Ethereum mainnet smart contracts verify Veridag Quorum Checkpoints ($2f+1$ Ed25519 signatures over `VERIDAG_CHECKPOINT_V1`) and BMH-1 Merkle inclusion proofs directly in Solidity.
3. **Native EVM JSON-RPC Layer:** Standard Ethereum wallets (MetaMask), development frameworks (Foundry, Hardhat), and libraries (ethers.js, alloy, viem) interact seamlessly with Veridag through standard `eth_*` RPC endpoints.
4. **Two-Way Cross-Chain Bridge:** Cryptographically secured lock/mint and burn/unlock primitives for USDV and Ethereum assets with monotonic message sequence nonces.

---

## 2. L1 Verification Architecture

```
┌────────────────────────────────────────────────────────┐
│               Ethereum Mainnet (L1)                    │
│                                                        │
│   ┌────────────────────────┐  ┌────────────────────┐   │
│   │  VeridagLightClient.sol│  │      USDV.sol      │   │
│   │  • Quorum Verification │  │  • ERC-20 / EIP-712│   │
│   │  • Checkpoint Anchors  │  │  • ERC-2612 Permit │   │
│   │  • BMH-1 Merkle Proofs │  │  • EIP-3009 Auth   │   │
│   └───────────▲────────────┘  └─────────▲──────────┘   │
│               │                         │              │
│               └────────────┬────────────┘              │
│                            │                           │
│                 ┌──────────▼──────────┐                │
│                 │   VeridagBridge.sol │                │
│                 │   • Deposit / Lock  │                │
│                 │   • Withdraw / Mint │                │
│                 └──────────▲──────────┘                │
└────────────────────────────┼───────────────────────────┘
                             │
            Cryptographic Checkpoints & Proofs
                             │
┌────────────────────────────▼───────────────────────────┐
│              Veridag DAG-BFT Engine (L2)               │
│                                                        │
│  • Sub-100ms Wave Commit                               │
│  • Pure-Function BaselineDagBft Consensus              │
│  • BMH-1 State Root & Quorum Checkpoints               │
│  • Native EVM JSON-RPC Gateway                         │
└────────────────────────────────────────────────────────┘
```

---

## 3. L1 Smart Contract Specification

### 3.1 `VeridagLightClient.sol`
Maintains the canonical state of Veridag on Ethereum L1:
- Stores latest verified sequence number `latestSequence` and checkpoint hash `latestCheckpointId`.
- Verifies $2f+1$ Ed25519 validator signatures over `H("VERIDAG_CHECKPOINT_V1" || checkpoint_body)`.
- Validates sequence monotony ($S_{n+1} > S_n$) and validator set commitment consistency.
- Function `verifyInclusion(bytes32 stateRoot, bytes32 key, bytes value, bytes32[] proof, uint256 index)` evaluates BMH-1 Merkle inclusion on EVM.

### 3.2 `USDV.sol`
Canonical Ethereum-side representation of the Veridag Dollar:
- **Standards:** ERC-20, ERC-2612 (`permit`), EIP-3009 (`receiveWithAuthorization`, `transferWithAuthorization`).
- **Decimals:** 6 (identical to USDC/USDT and Veridag native USDV).
- **Access Control:** Role-based capabilities (`MINTER_ROLE`, `BURNER_ROLE`, `PAUSER_ROLE`, `COMPLIANCE_ROLE`).
- **Bridge Integration:** Only `VeridagBridge.sol` holds `MINTER_ROLE` and `BURNER_ROLE` on L1.

### 3.3 `VeridagBridge.sol`
Two-way cross-chain gateway:
- **L1 -> Veridag (Deposit):**
  - Caller transfers USDV to `VeridagBridge`.
  - Bridge burns (or locks) tokens and emits `DepositInitiated(address sender, bytes32 veridagRecipient, uint256 amount, uint64 sequence)`.
  - Veridag validators observe event or client submits deposit proof; Veridag mints corresponding native USDV.
- **Veridag -> L1 (Withdrawal):**
  - Caller burns native USDV on Veridag, generating a withdrawal receipt in the checkpoint state root.
  - Caller submits `(checkpointBytes, receiptBytes, merkleProof)` to `VeridagBridge.sol`.
  - Bridge checks `VeridagLightClient.isCheckpointFinal(checkpointId)` and validates the BMH-1 Merkle proof.
  - Bridge verifies the withdrawal has not been previously claimed (`claimedWithdrawals[withdrawalHash] == false`).
  - Bridge mints/releases USDV to the recipient address on Ethereum L1.

---

## 4. EVM JSON-RPC Mapping

The Veridag EVM interface translates standard Ethereum JSON-RPC calls into deterministic Veridag queries:

| Ethereum RPC Method | Veridag DAG Implementation |
| :--- | :--- |
| `eth_chainId` | Returns the configured EVM chain identifier (e.g., `0x5645` for Veridag). |
| `eth_blockNumber` | Maps directly to the highest committed DAG wave or checkpoint sequence. |
| `eth_getBalance` | Queries the deterministic BMH-1 state for the given 20-byte address. |
| `eth_sendRawTransaction` | Ingests signed EVM transactions into the Veridag mempool for DAG vertex batching. |
| `eth_getTransactionReceipt` | Looks up execution receipt generated by the deterministic executor. |
| `eth_call` | Executes read-only EVM bytecode against the latest committed state root. |
| `web3_clientVersion` | Returns `Veridag/v0.1.0-ironclad/rust1.85`. |
| `net_version` | Returns configured network id. |

---

## 5. Security Invariants

1. **Bridge Conservation Invariant:**
   $$\text{TotalSupply}_{\text{L1}} + \text{TotalSupply}_{\text{Veridag}} = \text{TotalAttestedReserves}$$
2. **Replay Freedom:** Every cross-chain message specifies `(source_chain, dest_chain, sequence, nonce)`. A message can be executed at most once on either chain.
3. **No Rollback:** A checkpoint accepted by `VeridagLightClient.sol` cannot be overridden or reorganized, guaranteeing instant finality for L1 bridge withdrawals.
