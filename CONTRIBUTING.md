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

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo deny check   # when configured
cargo audit        # when configured
```

Plus, where relevant: Quint typecheck/run, protocol conformance (golden +
malformed vectors), simulator regression, fuzz smoke tests, Wasm determinism
tests.

## ADRs

Major decisions need an ADR in `docs/adr/`. Use the existing template format:
Context, Decision, Alternatives, Security consequences, Performance consequences,
Complexity consequences, Interoperability consequences, Revisit conditions.

## Code of Conduct

See `CODE_OF_CONDUCT.md`.
