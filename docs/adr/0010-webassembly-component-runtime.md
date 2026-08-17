# ADR-0010: WebAssembly Component runtime

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Applications must be language-independent and deterministic; general host capabilities must be denied by default.

## Decision
Adopt the WebAssembly Component Model with a deterministic Wasmtime-based reference runtime. Default-deny filesystem, network, wall clock, OS entropy, environment, process creation, threads. Applications get explicit capability handles only.

## Alternatives
EVM-style VM (rejected: language/ecosystem lock-in and non-determinism pitfalls); native plugins (rejected: unsafe); eBPF (deferred).

## Security consequences
Sandboxed execution with explicit capabilities prevents ambient authority; determinism rules are protocol-defined.

## Performance consequences
Wasmtime metering adds overhead; measured per host call before optimization.

## Complexity consequences
Maintaining a deterministic host API is ongoing work.

## Interoperability consequences
Any Wasm-capable language can target the component interface.

## Revisit conditions
If the Component Model cannot express a needed deterministic API, extend the host API via ADR.
