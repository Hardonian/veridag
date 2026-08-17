# 07 — Capabilities

Status: NORMATIVE

Capabilities are **protocol-native authorization objects**. Authority is not
"owns key → can do everything". A transaction's mutations are limited to the union
of ownership and the capabilities it validly carries.

## Capability object

```text
Capability
├── id              CapabilityId = H("VERIDAG_CAPABILITY_V1" || [fields below])
├── issuer          Address
├── holder          Address
├── kind            CapabilityKind (tagged union)
├── constraints     Constraints
├── delegable       bool
├── revoked         bool
├── expiry          ExpiryCondition
└── parent          Option<CapabilityId>   (delegation chain)
```

## CapabilityKind (variant index normative)

| Index | Kind | Scope fields |
|-------|------|--------------|
| 0 | `Spend` | `max_per_epoch u64, current_epoch_spent u64` |
| 1 | `ModifyObject` | `object_class u32` |
| 2 | `Delegate` | (may create child capabilities) |
| 3 | `Application` | `app ApplicationId` |
| 4 | `Validator` | (validator authority; managed by membership) |
| 5 | `Agent` | `max_spend u64, allowed_apps Vec<ApplicationId>, allowed_counterparties Vec<Address>, allowed_object_classes Vec<u32>` |
| 6 | `Session` | `max_calls u32, calls_used u32` |

## Constraints

```text
Constraints
├── expiry_epoch     Epoch
├── rate_limit       Option<(u32 per_epoch)>
└── resource_limit   Option<ResourceBudget>
```

## Enforcement rules (MUST)

A mutation is authorized iff at least one holds:

1. the object's `owner` is `Address(sender)`;
2. the object's `owner` is `Capability(cid)` and the transaction carries a valid
   capability `cid` whose kind covers the mutation;
3. the transaction carries an `Agent`/`Application` capability covering the
   operation's application, object class, and counterparty within limits.

For `Spend`/`Agent` capabilities, enforcement MUST check `current_epoch_spent +
amount <= max_per_epoch` and update `current_epoch_spent` deterministically at
execution. An over-limit attempt MUST be rejected with a capability-exceeded
receipt. Delegation of a non-delegable capability MUST be rejected. Revoked or
expired capabilities MUST be rejected.

## Agent-native authorization (MUST)

Autonomous agents never require custody of a master key. An `Agent` capability
constrains: total/per-epoch spend, allowed applications, allowed counterparties,
allowed object classes, expiry checkpoint, and delegation prohibition. Example
policy (from the architecture brief) enforced by protocol state:

```text
Agent A may spend <= 20 units/epoch, call application X,
interact with Y and Z, modify objects of type T, until checkpoint C.
Agent A may not delegate, change policy, or transfer its parent credential.
```

These limits are checked by the executor (11), not by SDK convention.
