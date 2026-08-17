# 11 — Execution

Status: NORMATIVE

## State transition

```text
S(n+1) = Apply(S(n), OrderedTransactions)
```

`Apply` is a pure, total, deterministic function. It MUST NOT perform filesystem,
network, wall-clock, OS-entropy, locale, environment-variable, thread-ordering,
process-id, or floating-point dependent operations. Any required randomness is
explicit protocol input (10-ordering).

## Receipts

`Apply` produces, per input transaction, a `Receipt`:

```text
Receipt
├── transaction_id   TransactionId
├── status           Status (tagged union: Success | Error(TransactionError))
├── writes           Vec<(ObjectId, ObjectVersion)>   (new versions)
├── events           Vec<Event>                        (application-defined)
└── resource_used    ResourceBudget
```

`TransactionError` variants (index normative): `InsufficientFunds`,
`VersionConflict`, `Unauthorized`, `CapabilityExceeded`, `Expired`,
`InvalidOperation`, `BudgetExceeded`, `ApplicationError`.

## Sequential reference executor (v0.1 normative)

Process the ordered transactions one at a time, in order. For each transaction:

1. Re-run the validation rules of 05 against the current state. A transaction
   that fails validation produces an `Error` receipt and no state change.
2. Apply the operation:
   * `CreateObject` — create with version 0; charge budget.
   * `UpdateObject` — check version & authority; write new payload; bump version.
   * `DeleteObject` — check version & authority; remove (not for Immutable).
   * `TransferObject` — check version & authority; change owner; bump version.
   * `TransferValue` — debit sender Balance object, credit/create recipient
     Balance object; check sufficient funds; bump versions; enforce capabilities.
   * `GrantCapability` — create capability object if issuer authorized.
   * `RevokeCapability` — mark revoked if issuer.
   * `InvokeApplication` — post-v0.1; routed to the deterministic runtime (35).
3. Increment sender nonce (once per transaction, even on some error classes per
   the error semantics below).
4. Emit receipt.

**Nonce rule.** A transaction that passes signature and nonce checks increments
the sender nonce exactly once, whether execution later succeeds or returns a
state-level error (e.g., insufficient funds). A transaction that fails signature
or nonce checks does not increment the nonce and is dropped before execution.

## Deterministic conflict semantics

Because every mutation is version-bound (06), two transactions writing the same
object in one batch are resolved by order: the later one observes the incremented
version and fails with `VersionConflict` if its declared `expected` is stale.
This is deterministic and independent of scheduling.

## Parallel execution (post-v0.1; invariant stated now)

A parallel executor is valid only if, for every ordered batch:

```text
parallel_result.state_root   == sequential_result.state_root
parallel_result.receipts     == sequential_result.receipts
parallel_result.versions     == sequential_result.versions
parallel_result.resource     == sequential_result.resource
```

This invariant MUST be property-tested aggressively. The sequential executor is
the oracle.
