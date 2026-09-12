# Performance Model

We measure empirically; we do not advertise unverified theoretical TPS.

---

## 1. Active Benchmark Suites

All performance benchmarks are reproducible using `cargo bench` and Criterion.

### 1.1 Cryptographic Microbenchmarks (`veridag-crypto`)
* **Path:** `implementations/rust/crates/crypto/benches/hash.rs`
* **Metrics:**
  - BLAKE3 32-byte hash latency (~120 ns/op) and multi-megabyte streaming throughput (> 3.5 GB/s).
  - SHA-256 fallback comparison.
  - Ed25519 signature verification throughput (> 25,000 verifications/sec on standard x86_64 AVX2 cores).

### 1.2 Consensus & Execution Hotpath Benchmarks (`veridag-qa`)
* **Path:** `implementations/rust/crates/qa/benches/hotpath.rs`
* **Metrics:**
  - VCE-1 canonical transaction parsing and serialization throughput (> 250,000 tx/sec).
  - DAG vertex causal link validation and insertion latency (< 15 µs/vertex).
  - BMH-1 Merkle-Hash tree recalculation across 10,000 modified objects (< 8 ms).
  - Sequential deterministic execution throughput.

### 1.3 Multi-Node QUIC Network Integration
* **Path:** `implementations/rust/crates/net/tests/devnet.rs`
* **Test:** 4 independent validator processes communicating over live QUIC network sockets with TLS 1.3 mutual authentication, verifying wave consensus commit and checkpoint finality under concurrent transaction loads.

---

## 2. Real-Time Observability (`veridag-metrics`)

Production nodes expose Prometheus metrics over `/metrics`:
* `veridag_dag_round`: Current round of the local validator DAG.
* `veridag_committed_waves_total`: Cumulative BFT wave anchors finalized.
* `veridag_mempool_size`: Current queued transactions awaiting vertex packaging.
* `veridag_checkpoint_index`: Latest monotonic finalized checkpoint index.
* `veridag_execution_duration_seconds`: Histogram of batch execution and state root computation latency.

