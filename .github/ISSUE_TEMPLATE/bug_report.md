---
name: Bug report
about: Something is broken or behaves incorrectly
title: "[bug] "
labels: bug
assignees: ""
---

## Component
<!-- Which crate / subsystem: consensus, execution, codec, storage, net, rpc, sdk-ts, sdk-py, site, formal model -->

## Protocol version & binary version
<!-- Output of: cargo run -p veridag-node -- --version -->

## What happened
<!-- Clear description of the incorrect behaviour -->

## Steps to reproduce
```
# Minimal steps or command sequence
```

## Expected behaviour
<!-- What should have happened instead -->

## Logs / trace
```
# Paste relevant tracing output (set RUST_LOG=debug or RUST_LOG=veridag=trace)
```

## Is this consensus-visible?
<!-- Could this cause validators to diverge on state root or checkpoint? -->
- [ ] Yes — potential safety or liveness issue (consider using [Security Advisory](../../security/advisories/new) instead)
- [ ] No — incorrect output or crash only
- [ ] Unsure
