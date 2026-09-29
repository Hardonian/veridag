set shell := ["bash", "-Eeuo", "pipefail", "-c"]

default:
    @just --list

# Install rust components used by CI
setup:
    rustup component add rustfmt clippy rust-src
    cargo install cargo-deny cargo-audit || true
    @echo "setup: ok"

# Full local verification (fast subset used pre-push)
check:
    cargo fmt --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo test --workspace --all-features
    cargo doc --workspace --no-deps

# Security audit (advisory + license + banned deps)
audit:
    cargo deny check
    cargo audit

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

# In-process 4-validator consensus demo
demo:
    cargo run -p veridag-node -- demo

# 4-validator multi-process QUIC network devnet
devnet:
    cargo test -p veridag-net --test devnet -- --nocapture

# Deterministic simulation harness (agreement & causal ordering)
sim:
    cargo test -p veridag-consensus --test simulation -- --nocapture

# Validator node health probe (checks agreement & emits status)
health:
    cargo run -p veridag-node -- health

# Launch a single daemon node for local dev (seed 1, RPC on :8080)
daemon:
    cargo run -p veridag-node -- daemon --seed 1 --rpc 0.0.0.0:8080

# Spin up 4-validator Docker Compose mesh
docker-up:
    docker compose up -d
    @echo "Nodes starting — wait ~10s then: curl http://localhost:8081/v1/health"

# Tear down Docker Compose mesh
docker-down:
    docker compose down

# Hot-path benchmark suite
bench:
    cargo bench -p veridag-qa

# Developer CLI help
cli:
    cargo run -p veridag-cli -- --help

# Run web portal documentation app locally
site-dev:
    cd site && npm run dev

# Build static production bundle for web portal
site-build:
    cd site && npm run build

# Run TypeScript SDK conformance tests
ts-test:
    cd sdks/typescript && npm test

# Run Python SDK conformance tests
py-test:
    cd sdks/python && uv run python -m unittest discover -s tests

# Full pre-release gate: check + vectors + audit + ts-test + py-test + site-build
release-gate: check vectors audit ts-test py-test site-build
    @echo "release-gate: all checks passed"
