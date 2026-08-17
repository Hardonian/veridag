set shell := ["bash", "-Eeuo", "pipefail", "-c"]

default:
    @just --list

# Install rust components used by CI
setup:
    rustup component add rustfmt clippy rust-src
    @echo "setup: ok"

# Full local verification (fast subset used pre-push)
check:
    cargo fmt --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo test --workspace --all-features
    cargo doc --workspace --no-deps

# Protocol conformance: regenerate and validate golden vectors
vectors:
    cargo run -p veridag-testkit --bin veridag-vector-gen
    cargo test -p veridag-testkit --test vectors

# Quint formal model (requires quint on PATH)
formal:
    cd formal/quint && quint typecheck consensus.qnt
    cd formal/quint && quint typecheck instance4.qnt
    cd formal/quint && quint test consensus_test.qnt
    cd formal/quint && quint run instance4.qnt --invariant=Agreement --max-steps=30 --max-samples=200
    cd formal/quint && quint run instance4.qnt --invariant=Finality --max-steps=30 --max-samples=200
    cd formal/quint && quint run instance4.qnt --invariant=Integrity --max-steps=30 --max-samples=200

# 4-validator devnet (available after Phase 8)
devnet:
    @echo "devnet is available after Phase 8 (vertical slice)."
    @echo "Run the Phase 4 CLI demo instead: cargo run -p veridag-cli -- --help"

# Deterministic simulator smoke (available after Phase 7)
sim:
    @echo "simulator is available after Phase 7."
