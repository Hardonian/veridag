# Veridag: Iron-Clad Infrastructure for Ethereum

Veridag functions as an **ultra-high-performance execution, sequencing, and settlement substrate for Ethereum**, delivering zero-reorg finality, EVM JSON-RPC compatibility, and trustless L1 checkpoint verification.

---

## 1. System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 ETHEREUM MAINNET (L1)                       │
│                                                             │
│   ┌──────────────────────────┐   ┌──────────────────────┐   │
│   │  VeridagLightClient.sol  │   │       USDV.sol       │   │
│   │  • Quorum Verification   │   │  • ERC-20 / ERC-2612 │   │
│   │  • BMH-1 Merkle Proofs   │   │  • EIP-3009 Auth     │   │
│   └─────────────▲────────────┘   └──────────▲───────────┘   │
│                 │                           │               │
│                 └─────────────┬─────────────┘               │
│                               │                             │
│                    ┌──────────▼──────────┐                  │
│                    │  VeridagBridge.sol  │                  │
│                    │  • Lock / Mint      │                  │
│                    │  • Burn / Release   │                  │
│                    └──────────▲──────────┘                  │
└───────────────────────────────┼─────────────────────────────┘
                                │
               Cryptographic Checkpoints & Proofs
                                │
┌───────────────────────────────▼─────────────────────────────┐
│                VERIDAG DAG-BFT SUBSTRATE (L2)               │
│                                                             │
│  • Sub-100ms Wave Commit (Zero-Reorg Finality)              │
│  • Pure-Function BaselineDagBft Consensus                   │
│  • Native EVM JSON-RPC Gateway (`eth_*`)                    │
│  • BMH-1 State Root & Quorum Checkpoints                    │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Key Components

### 2.1 L1 Smart Contracts (`contracts/`)
- [`USDV.sol`](../contracts/USDV.sol): Production-grade ERC-20, ERC-2612 (`permit`), and EIP-3009 stablecoin contract with institutional compliance roles (`MINTER_ROLE`, `BURNER_ROLE`, `COMPLIANCE_ROLE`, `PAUSER_ROLE`).
- [`VeridagLightClient.sol`](../contracts/VeridagLightClient.sol): Trustless Ethereum L1 light client verifying Veridag Quorum Checkpoints ($2f+1$ validator signatures) and evaluating BMH-1 Merkle inclusion proofs in EVM bytecode.
- [`VeridagBridge.sol`](../contracts/VeridagBridge.sol): Cross-chain bridge gateway enforcing the Global Conservation Invariant:
  $$\text{TotalSupply}_{\text{L1}} + \text{TotalSupply}_{\text{Veridag}} = \text{TotalAttestedReserves}$$

### 2.2 EVM JSON-RPC Layer
The `veridag-ethereum` crate provides standard Ethereum RPC compatibility:
- `eth_chainId`: Returns configured EVM chain ID (`0x5645`).
- `eth_blockNumber`: Maps to the latest committed DAG wave or checkpoint sequence.
- `eth_getBalance`: Queries account balance directly from the BMH-1 state root.
- `eth_sendRawTransaction`: Ingests EVM transactions into the DAG mempool.
- `web3_clientVersion`: Identifies the client as `Veridag/v0.1.0-ironclad/rust1.85`.

---

## 3. Cross-Chain Workflow

### Deposit (Ethereum L1 -> Veridag L2)
1. User calls `VeridagBridge.depositUSDV(veridagRecipient, amount)`.
2. Bridge burns or locks USDV on L1 and emits `DepositInitiated`.
3. Veridag validators ingest the deposit event and credit the recipient's native USDV balance object.

### Withdrawal (Veridag L2 -> Ethereum L1)
1. User burns native USDV on Veridag, generating a withdrawal object in state.
2. At the next wave checkpoint, the state root commits to the withdrawal object.
3. User generates a BMH-1 inclusion proof using `veridag-cli eth bridge-proof --account <user>`.
4. User submits the proof to `VeridagBridge.finalizeWithdrawal(...)` on Ethereum.
5. Bridge verifies checkpoint finality via `VeridagLightClient`, checks proof, prevents replay, and mints USDV on L1.

---

## 4. CLI Developer Quickstart

```bash
# 1. Execute an EVM JSON-RPC query
veridag-cli eth rpc

# 2. Export an Ethereum L1 Merkle inclusion proof
veridag-cli eth bridge-proof --account alice
```
