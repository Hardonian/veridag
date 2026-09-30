# Release Readiness

This is the promotion checklist for Veridag. A green repository proves the
software gates below; it does not substitute for an independent audit,
production operating history, or a regulator/operator authorization.

## Executable gates

| Gate | Command or workflow | Required evidence |
| :--- | :--- | :--- |
| Rust correctness | `just check` | Format, strict Clippy, all-feature tests, and docs pass. |
| Protocol vectors | `just vectors` | Generated vectors match the committed cross-language corpus. |
| Dependency policy | `just audit` | No denied advisory, license, source, or ban finding. |
| Formal model | `just formal` | Quint typecheck/tests and Agreement, Finality, and Integrity runs pass. |
| Solidity | `just solidity` | Format, unit, 1,000-run fuzz, and stateful invariant campaigns pass. |
| Solidity static analysis | `just slither` | Slither completes with zero unsuppressed findings. |
| Decoder fuzzing | `just fuzz 60` and `.github/workflows/fuzz.yml` | Nightly libFuzzer/ASan campaign completes without a crash or timeout artifact. |
| Container devnet | `docker compose up -d --build` | Four validators report healthy and agree on a state root. |
| Restart/soak | `just docker-soak 300` | JSON evidence shows health, agreement, progress, and restart recovery. |
| SDK/package checks | `scripts/publish-sdks.*` and release workflows | Built artifacts, package inspection, checksums, SBOM, and provenance. |
| Industry adapters | `just industry` | Every versioned manifest and bounded adapter validates. |

`just release-gate` runs the bounded, non-Docker checks. The Docker soak and
nightly fuzz campaign are explicit because their duration and resource
requirements are operator-selected. CI runs a bounded Docker build, agreement
check, and validator restart on every change; decoder fuzzing runs on relevant
pull requests, on demand, and on a daily schedule.

## Independent security gate

Production promotion requires an assessor independent of the implementation
team. The release record must link all of the following:

1. The exact audited commit and artifact hashes.
2. Scope covering consensus, canonical encoding, cryptography, network identity,
   persistence/recovery, RPC authorization, and Solidity bridge contracts.
3. A signed final report and an issue register mapping every finding to a fix,
   explicit risk acceptance, or release blocker.
4. Retest evidence for every critical or high-severity finding.
5. A declared residual-risk owner and approval date.

Repository authors may prepare this evidence but cannot self-issue the
independence attestation.

## Multi-region operating gate

Before GA, run at least four validators across at least three failure domains
for a sustained review window selected by the operator (72 hours is the default
acceptance window). Use `scripts/devnet-soak.py` against the regional endpoints
and retain:

- deployment region/provider and build provenance for each validator;
- continuous health, wave, checkpoint, and state-root agreement evidence;
- validator restart, regional isolation, packet loss/latency, and recovery logs;
- proof that no two healthy validators reported conflicting finalized state;
- incident notes and operator sign-off for every alert during the window.

A local Docker run validates the mechanism and recovery path, but is not
multi-region operating evidence.

## Regulatory and asset-operation gate

USDV remains an experimental software profile until an authorized operator
supplies jurisdiction-specific approval. The release record must identify:

1. The issuing/operating legal entity and permitted jurisdictions.
2. Counsel-approved classification, terms, disclosures, and launch decision.
3. Custody, reserve, redemption, reconciliation, and independent attestation
   arrangements.
4. Sanctions, AML/KYC, transaction monitoring, freeze/seizure, privacy, and
   record-retention controls with accountable owners.
5. Key-management and governance signers, thresholds, rotation, emergency
   pause, incident response, and recovery procedures.
6. Signed approvals from legal, compliance, security, finance, and operations.

No code change, test result, or internal checklist can create these legal facts.
Until the signed evidence exists, documentation and product surfaces must retain
the pre-GA/experimental labels in the capability matrix.
