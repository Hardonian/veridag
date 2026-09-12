#!/usr/bin/env bash
# ==============================================================================
# VeriDAG Universal Multi-Language SDK Release & Conformance Verification Script
# Publishes:
#   1. Rust SDK -> crates.io (veridag-sdk)
#   2. TypeScript SDK -> npm (@veridag/sdk)
#   3. Python SDK -> PyPI (veridag)
# ==============================================================================
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "================================================================="
echo " VeriDAG Multi-Language SDK Conformance & Release Gatekeeper"
echo " Root: $ROOT_DIR"
echo "================================================================="

# 1. Rust SDK
echo "[1/3] Verifying Rust SDK conformance..."
cd "$ROOT_DIR"
cargo test -p veridag-sdk
echo "  [PASS] Rust SDK conformance verified."

# 2. TypeScript SDK
echo "[2/3] Verifying TypeScript SDK conformance..."
cd "$ROOT_DIR/sdks/typescript"
npm test
echo "  [PASS] TypeScript SDK conformance verified."

# 3. Python SDK
echo "[3/3] Verifying Python SDK conformance..."
cd "$ROOT_DIR/sdks/python"
if command -v uv >/dev/null 2>&1; then
    uv run tests/test_conformance.py
else
    python3 tests/test_conformance.py
fi
echo "  [PASS] Python SDK conformance verified."

echo "================================================================="
echo " ALL 3 SDKS MATCH GOLDEN VECTORS 100% BIT-FOR-BIT!"
echo " Ready for distribution to crates.io, npm, and PyPI."
echo "================================================================="
