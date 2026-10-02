# Contributing

## Ground rules

1. Protocol before implementation. A consensus-visible change starts in
   `protocol/specification/`, then the formal model, then test vectors, then code.
2. No placeholder completion. A milestone with `TODO`, `unimplemented!`,
   `panic!("not implemented")`, dummy signatures, fake state roots, mocked
   consensus, hardcoded finality, or always-true verification on its critical
   path is not done.
3. No blockchain theatre. Do not fake multiple validators in one object, mock
   consensus success in end-to-end tests, bypass signatures, centralize ordering
   behind one coordinator, hard-code privileged accounts, or invent benchmarks.
4. Keep the tree green. Run `just check` before pushing.
5. No `unsafe` — all crates `#![forbid(unsafe_code)]`. PRs adding unsafe are rejected.
6. No `unwrap()` on attacker-controlled input. Panics from external data are security bugs.

## Branching

Branch from `main`. Naming: `feat/<slug>`, `fix/<slug>`, `docs/<slug>`. Open a PR
against `main`; all CI gates must be green before merge.

## Quick start

```bash
just setup          # install rustfmt, clippy, rust-src, cargo-deny, cargo-audit
just check          # fmt + clippy + test + doc — must pass before any PR
just demo           # verify 4-validator agreement in-process
just daemon         # launch local dev node on :8080 with Prometheus + readiness probes
just release-gate   # full pre-release: check + vectors + audit + SDK tests + site build
```

## CI gates

Every PR must pass all of the following:

| Gate | Workflow | What it checks |
|------|----------|----------------|
| `fmt + clippy + test + doc` | `ci.yml / rust` | Rust lint, type safety, full test suite |
| `edge target` | `ci.yml / cross-edge` | `aarch64` cross-compilation |
| `sdk-typescript` | `ci.yml / sdk-typescript` | TypeScript SDK conformance |
| `sdk-python` | `ci.yml / sdk-python` | Python SDK conformance |
| `site` | `ci.yml / site` | Next.js site build |
| `conformance` | `conformance.yml` | Golden + malformed vector tests |
| `security` | `security.yml` | `cargo deny` + `cargo audit` |

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo deny check
cargo audit
```

Plus, where relevant: Quint typecheck/run, protocol conformance (golden +
malformed vectors), simulator regression, fuzz smoke tests, Wasm determinism
tests.

## ADRs

Major decisions need an ADR in `docs/adr/`. Use the existing template format:
Context, Decision, Alternatives, Security consequences, Performance consequences,
Complexity consequences, Interoperability consequences, Revisit conditions.

## Security issues

Do **not** file public GitHub issues for security vulnerabilities. See `SECURITY.md`
for the responsible disclosure process.

## Code of Conduct

See `CODE_OF_CONDUCT.md`.
