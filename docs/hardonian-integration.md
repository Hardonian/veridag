# Hardonian Sovereign AI & Systems Architecture Integration

## Overview

Veridag is the **deterministic distributed trust and monetary settlement substrate** within the **Hardonian / AIAS Sovereign Platform**. It provides pure-function DAG-BFT consensus, zero-reorg wave finality, capability-gated execution, and stable sovereign currency (USDV) to power enterprise operations, autonomous agent swarms, and financial infrastructure.

---

## The Sovereign Stack Connective Tissue

```text
                                  ┌─────────────────────────────────────────────────────────┐
                                  │            HARDONIAN SOVEREIGN CONTROL PLANE            │
                                  └────────────────────────────┬────────────────────────────┘
                                                               │
                 ┌─────────────────────────────┬───────────────┴─────────────┬─────────────────────────────┐
                 │                             │                             │                             │
    ┌────────────▼────────────┐   ┌────────────▼────────────┐   ┌────────────▼────────────┐   ┌────────────▼────────────┐
    │         Settler         │   │      MissionLedger      │   │       ReadyLayer        │   │         mcpwall         │
    │  Reconciliation & Audit │   │  Governed Agent Missions│   │  Delivery Governance CI │   │  Zero-Cloud Policy Wall │
    │  • Multi-Source Recon   │   │  • Strict Policy Gates  │   │  • Release Attestations │   │  • Human Approval Gates │
    │  • TigerBeetle Ledger   │   │  • Budget Guardrails    │   │  • Provenance Tracking  │   │  • Stdio Sandboxing     │
    │  • Evidence Manifests   │   │  • Execution Proofpacks │   │  • Delivery Proofs      │   │  • Policy Audits        │
    └────────────┬────────────┘   └────────────┬────────────┘   └────────────┬────────────┘   └────────────┬────────────┘
                 │                             │                             │                             │
                 │ Multi-Party Settlement      │ Mission Anchors             │ Release Attestation Hash    │ Security Approvals
                 │ & Proofpack Commitments     │ & Budget Depletion          │ & Provenance Anchors        │ & Revocation Broadcast
                 │                             │                             │                             │
                 └─────────────────────────────┼─────────────────────────────┴─────────────────────────────┘
                                               │
                                  ┌────────────▼────────────┐
                                  │         veridag         │
                                  │ Pure DAG-BFT Consensus  │
                                  │ • Sub-100ms Wave Commit │
                                  │ • USDV Stablecoin Engine│
                                  │ • BMH-1 State Root Tree │
                                  │ • Quorum Checkpoints    │
                                  │ • BTC/ETH Cross-Chain   │
                                  └─────────────────────────┘
```

---

## Inter-System Connective Tissue Matrix

| Hardonian Repo | Role in Stack | Veridag Integration Contract |
| :--- | :--- | :--- |
| **[Settler](https://github.com/Hardonian/Settler)** | Reconciliation Intelligence & Audit OS | **SettlerBatchSettlement & SettlerReconciliationAnchor (`object_type::SETTLER_ANCHOR = 5`)**: Executes atomic multi-party payouts in native USDV backed by Settler's Merkle proofpacks. |
| **[MissionLedger](https://github.com/Hardonian/MissionLedger)** | Governed Agent Missions & Execution Substrate | **Mission Proofpack Anchoring**: Commits autonomous agent mission outcomes, compliance validations, and budget expenditures directly to Veridag wave checkpoints. |
| **[ReadyLayer](https://github.com/Hardonian/ReadyLayer)** | Software Delivery Governance CI/CD | **Release Provenance Commitments**: Anchors cryptographically signed software release digests and audit approvals into Veridag's immutable DAG state. |
| **[nlsqlc](https://github.com/Hardonian/nlsqlc)** | Multi-Tenant Query IR Compiler | **Tenant Audit Logs & State Commitments**: Cryptographically verifies query plans and schema migrations across tenant boundaries on Veridag. |
| **[mcpwall](https://github.com/Hardonian/mcpwall)** | Zero-Cloud Policy Firewall for MCP Agents | **Capability Revocation & Policy Sync**: Distributes instantaneous, Byzantine-fault-tolerant capability revocations and policy updates across all nodes and agent runtimes. |
| **[TokenGoblin](https://github.com/Hardonian/TokenGoblin)** | Real-Time AI Token Cost & FinOps Guardrails | **Micro-Settlement & Credit Grants**: Real-time programmatic settlement of LLM inference compute credits and budget caps using native USDV micro-units ($10^{-6}$ USD). |

---

## Operational Truth & Deployments

- **Revenue DB:** `/home/scott/ai-lab/revenue-os/revenue-os.db`
- **Deploy/Verify Pipeline:** `/home/scott/ai-lab/scripts/bin/deploy-all.sh`
- **Service Supervision:** `systemctl --user status veridag.*`
- **Ops Truth Monitor:** `python3 /home/scott/.hermes/scripts/ops-nerve-center.py`
