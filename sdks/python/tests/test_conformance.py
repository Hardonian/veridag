import unittest
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from veridag.crypto import Keypair, hash_domain
from veridag.builder import TxBuilder
from veridag.codec import encode_transaction, encode_signed_transaction


class TestConformance(unittest.TestCase):
    def test_conformance(self):
        vector_path = Path(__file__).resolve().parents[3] / "protocol" / "test-vectors" / "sdk_conformance.json"
        with open(vector_path, "r", encoding="utf-8") as f:
            vector = json.load(f)

        # 1. Derive keypair from secret seed
        seed = bytes.fromhex(vector["secret_seed"].removeprefix("0x"))
        keypair = Keypair.from_seed(seed)

        # 2. Public key check
        expected_pk = vector["public_key"].removeprefix("0x")
        self.assertEqual(keypair.public().hex(), expected_pk, "Public key mismatch")

        # 3. Address derivation check
        expected_address = vector["sender_address"].removeprefix("0x")
        self.assertEqual(keypair.address().hex(), expected_address, "Address mismatch")

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
        self.assertEqual(unsigned_bytes.hex(), expected_unsigned, "Unsigned payload mismatch")

        # 6. Signature check
        expected_sig = vector["signature"].removeprefix("0x")
        self.assertEqual(signed_tx.signature.hex(), expected_sig, "Signature mismatch")

        # 7. Signed wire payload check
        signed_wire = encode_signed_transaction(signed_tx)
        expected_signed = vector["signed_tx_wire"].removeprefix("0x")
        self.assertEqual(signed_wire.hex(), expected_signed, "Signed wire payload mismatch")

        # 8. Transaction ID / Hash check
        tx_hash = hash_domain("VERIDAG_TX_V1", signed_wire)
        expected_hash = vector["tx_hash"].removeprefix("0x")
        self.assertEqual(tx_hash.hex(), expected_hash, "Transaction hash mismatch")


if __name__ == "__main__":
    unittest.main()

