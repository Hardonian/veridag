# Performance Model

We measure; we do not advertise theoretical TPS.

## Independent measurements (added as components land)

transaction decode/sec, signature verification/sec, state reads/writes per sec,
vertex validation/sec, DAG insertion/sec, consensus commits/sec, sequential and
parallel execution/sec, checkpoint generation/verification, network throughput,
consensus/finality latency, Wasm calls/sec, proof generation/verification.

## Benchmark levels

microbenchmark, single-node, multi-node localhost, multi-machine LAN,
fault-injected, WAN emulation. Every recorded benchmark notes CPU, RAM, storage,
network parameters, validator count, payload, protocol version, and git commit.

## Current status

No performance claims are made yet. Benches land with Phase 8+ and live under
`benches/` with reproducible harnesses.
