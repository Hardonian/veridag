import unittest
import json
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler
from veridag import VeridagClient, Keypair, TxBuilder


class MockRpcHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()

        if self.path == "/v1/health":
            resp = {
                "status": "healthy",
                "version": "0.1.0-alpha",
                "protocol_version": 1,
                "chain_id": 1,
                "validator_seed": 1,
                "committee_n": 4,
                "committee_quorum": 3,
                "max_round": 10,
                "highest_wave": 2,
                "state_root": "0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
                "checkpoints_count": 1,
            }
            self.wfile.write(json.dumps(resp).encode("utf-8"))
        elif self.path == "/v1/state/root":
            resp = {
                "state_root": "0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
                "highest_wave": 2,
            }
            self.wfile.write(json.dumps(resp).encode("utf-8"))
        elif self.path.startswith("/v1/state/account/"):
            resp = {
                "address": "0x1111111111111111111111111111111111111111111111111111111111111111",
                "object_id": "0x2222222222222222222222222222222222222222222222222222222222222222",
                "balance": 1000000,
                "exists": True,
            }
            self.wfile.write(json.dumps(resp).encode("utf-8"))
        elif self.path == "/v1/checkpoints/latest":
            resp = {
                "id": "0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40",
                "sequence": 1,
                "epoch": 0,
                "state_root": "0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
                "transaction_root": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "dag_commitment": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "validator_set_commitment": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "votes_count": 4,
            }
            self.wfile.write(json.dumps(resp).encode("utf-8"))
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path == "/v1/tx/submit":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            data = json.loads(body.decode("utf-8"))
            assert "raw_tx_hex" in data

            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            resp = {
                "status": "admitted",
                "tx_id": "0x8888888888888888888888888888888888888888888888888888888888888888",
                "sender": "0x1111111111111111111111111111111111111111111111111111111111111111",
            }
            self.wfile.write(json.dumps(resp).encode("utf-8"))


class TestClient(unittest.TestCase):
    def test_python_client(self):
        server = HTTPServer(("127.0.0.1", 0), MockRpcHandler)
        port = server.server_port
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()

        try:
            client = VeridagClient(f"http://127.0.0.1:{port}")

            # 1. Health
            h = client.health()
            self.assertEqual(h["status"], "healthy")
            self.assertEqual(h["chain_id"], 1)

            # 2. State root
            r = client.get_state_root()
            self.assertTrue(r["state_root"].startswith("0xac04"))

            # 3. Account balance
            bal = client.get_account_balance("0x1111111111111111111111111111111111111111111111111111111111111111")
            self.assertEqual(bal["balance"], 1000000)
            self.assertTrue(bal["exists"])

            # 4. Checkpoint
            ckpt = client.get_latest_checkpoint()
            self.assertEqual(ckpt["sequence"], 1)
            self.assertEqual(ckpt["votes_count"], 4)

            # 5. Submit transaction
            sender = Keypair.from_seed(b"\x01" * 32)
            recipient = Keypair.from_seed(b"\x02" * 32).address()
            stx = (
                TxBuilder(sender)
                .nonce(0)
                .transfer(recipient, 500)
            )
            res = client.submit_transaction(stx, sender.public())
            self.assertEqual(res["status"], "admitted")
            self.assertTrue(res["tx_id"].startswith("0x"))
        finally:
            server.shutdown()
            server.server_close()


if __name__ == "__main__":
    unittest.main()

