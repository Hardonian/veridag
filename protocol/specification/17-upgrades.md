# 17 — Upgrades and Versioning

Status: NORMATIVE

Distinct versions, never inferred from each other:

```text
software version    (implementation release)
protocol version    (this spec; current = 1)
wire version        (network framing)
state version       (state encoding)
runtime ABI version (Wasm component interface)
proof version       (proof system/proof encoding)
SDK version         (client library)
```

## Rules (MUST)

1. Historical bytes never change meaning.
2. A consensus-visible behavioral change requires: spec change, protocol version
   assessment, formal model review, test-vector update if applicable,
   conformance test, implementation change — in that order.
3. Protocol upgrades activate only at explicit checkpoint boundaries. An upgrade
   record specifies: current version, new version, activation checkpoint, state
   migration (if any), runtime ABI changes, consensus changes.
4. There is no permanent privileged upgrade key. Upgrades are explicit protocol
   transitions validated by the same BFT finality as other state.
5. Nodes that do not support an activated protocol version MUST halt rather than
   risk interpreting bytes under the wrong rules.
