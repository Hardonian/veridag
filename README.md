# Veridag

**Veridag** is an implementation-independent protocol for deterministic,
Byzantine-resilient, capability-secured, verifiable distributed computation.

It is not a cryptocurrency, not a blockchain clone, and not a Rust framework. It is
a neutral distributed execution substrate for mutually distrustful humans,
organizations, machines, services, AI agents, devices, and applications.

```
consensus + verifiable state + deterministic computation
+ capability security + data availability + cryptographic proofs
= distributed trust fabric
```

## Status

Early development. The three levels are maintained explicitly:

| Level | Artifact | Where |
|-------|----------|-------|
| 1 | Normative protocol specification | `protocol/specification/` |
| 2 | Formal executable model (Quint) | `formal/quint/` |
| 3 | Reference implementation (Rust) | `implementations/rust/` |

An implementation is correct only if it satisfies Levels 1 and 2.

## Layout

```
protocol/           normative spec, schemas, test vectors, conformance
formal/quint/       formal model + invariants
implementations/    reference implementations (rust first)
runtime/            deterministic Wasm component runtime (post-v0.1)
proofs/             optional proof-system interface and adapters (post-v0.1)
acceleration/       optional Zig/C/C++/CUDA behind narrow interfaces (post-v0.1)
sdk/                Rust, TypeScript, Python, Go SDKs
conformance/        golden + malformed vectors, cross-implementation runner
simulations/        deterministic simulation assets
fuzz/               fuzz targets
docs/               architecture, threat model, operations, ADRs
```

## Quickstart (once toolchain is present)

```bash
just setup      # install rust toolchain components
just check      # fmt, clippy, tests, doc
just devnet     # 4-validator local devnet (after Phase 8)
```

See `protocol/specification/00-overview.md` for the protocol, and `ROADMAP.md`
for the phased plan.

## Principles (priority order)

```
correctness > determinism > security > implementation independence
> modularity > verification > operability > performance > developer usability
```

Non-negotiable: no consensus-visible behavior may depend on memory layout,
compiler version, CPU architecture, thread scheduling, hash-map order, filesystem
order, wall-clock time, OS randomness, floating point, database iteration quirks,
or network timing.

## License

Dual-licensed under Apache-2.0 and MIT. See `LICENSE-APACHE` and `LICENSE-MIT`.
