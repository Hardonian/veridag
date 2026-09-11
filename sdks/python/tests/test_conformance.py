"""
Cross-language conformance test for Veridag Python SDK.
Verifies bit-for-bit equivalence with Rust and TypeScript SDKs.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from veridag.crypto import Keypair, hash_domain
from veridag.builder import TxBuilder
from veridag.codec import encode_transaction, encode_signed_transaction


def test_conformance():
    vector_path = Path(__file__).resolve().parents[3] / "protocol" / "test-vectors" / "sdk_conformance.json"
    with open(vector_path, "r", encoding="utf-8") as f:
        vector = json.load(f)

    print("Running Veridag Python SDK conformance test against golden vectors...")

    # 1. Derive keypair from secret seed
    seed = bytes.fromhex(vector["secret_seed"].removeprefix("0x"))
    keypair = Keypair.from_seed(seed)

    # 2. Public key check
    expected_pk = vector["public_key"].removeprefix("0x")
    assert keypair.public().hex() == expected_pk, f"Public key mismatch: {keypair.public().hex()} vs {expected_pk}"
    print("  [PASS] Public key matches golden vector")

    # 3. Address derivation check
    expected_address = vector["sender_address"].removeprefix("0x")
    assert keypair.address().hex() == expected_address, f"Address mismatch: {keypair.address().hex()} vs {expected_address}"
    print("  [PASS] Sender address matches golden vector")

    # 4. Build transaction
    recipient = bytes.fromhex(vector["recipient_address"].removeprefix("0x"))
    signed_tx = (
        TxBuilder(keypair)
        .chain(vector["chain_id"])
        .nonce(vector["nonce"])
        .expiry(vector["expiry_epoch"])
        .transfer(recipient, vector["amount"])
    )

    # 5. Unsigned payload check
    unsigned_bytes = encode_transaction(signed_tx.tx)
    expected_unsigned = vector["unsigned_tx_payload"].removeprefix("0x")
    assert unsigned_bytes.hex() == expected_unsigned, f"Unsigned payload mismatch"
    print("  [PASS] Canonical VCE-1 unsigned payload matches golden vector")

    # 6. Signature check
    expected_sig = vector["signature"].removeprefix("0x")
    assert signed_tx.signature.hex() == expected_sig, f"Signature mismatch"
    print("  [PASS] Ed25519 domain-separated signature matches golden vector")

    # 7. Signed wire payload check
    signed_wire = encode_signed_transaction(signed_tx)
    expected_signed = vector["signed_tx_wire"].removeprefix("0x")
    assert signed_wire.hex() == expected_signed, f"Signed wire payload mismatch"
    print("  [PASS] Signed wire bytes match golden vector")

    # 8. Transaction ID / Hash check
    tx_hash = hash_domain("VERIDAG_TX_V1", signed_wire)
    expected_hash = vector["tx_hash"].removeprefix("0x")
    assert tx_hash.hex() == expected_hash, f"Transaction hash mismatch"
    print("  [PASS] Transaction hash matches golden vector")

    print("\nALL PYTHON SDK CONFORMANCE TESTS PASSED (100% BIT-FOR-BIT EQUIVALENCE WITH RUST & TS)")


if __name__ == "__main__":
    test_conformance()
