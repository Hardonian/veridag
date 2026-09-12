"""
Veridag HTTP RPC Client.
"""

from __future__ import annotations
import json
import urllib.request
import urllib.error
from typing import Any, Dict, Optional, Union
from .codec import encode_signed_transaction
from .types import SignedTransaction


class VeridagClient:
    """Client for querying Veridag RPC nodes and submitting signed transactions."""

    def __init__(self, rpc_url: str = "http://127.0.0.1:8080"):
        self.rpc_url = rpc_url.rstrip("/")

    def _request(self, method: str, path: str, body: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        url = f"{self.rpc_url}{path}"
        data = json.dumps(body).encode("utf-8") if body is not None else None
        headers = {"Content-Type": "application/json"} if body is not None else {}
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req) as resp:
                resp_bytes = resp.read()
                return json.loads(resp_bytes.decode("utf-8"))
        except urllib.error.HTTPError as e:
            err_content = e.read().decode("utf-8")
            try:
                err_json = json.loads(err_content)
                msg = err_json.get("error", err_content)
            except Exception:
                msg = err_content
            raise RuntimeError(f"Veridag RPC error ({e.code}): {msg}") from e

    def health(self) -> Dict[str, Any]:
        """Get node health and consensus status."""
        return self._request("GET", "/v1/health")

    def get_state_root(self) -> Dict[str, Any]:
        """Get the latest committed state root."""
        return self._request("GET", "/v1/state/root")

    def get_account_balance(self, address: Union[bytes, str]) -> Dict[str, Any]:
        """Get account balance and object existence."""
        if isinstance(address, bytes):
            addr_hex = address.hex()
        else:
            addr_hex = address.removeprefix("0x")
        return self._request("GET", f"/v1/state/account/{addr_hex}")

    def get_latest_checkpoint(self) -> Dict[str, Any]:
        """Get latest finalized checkpoint with quorum proof."""
        return self._request("GET", "/v1/checkpoints/latest")

    def submit_transaction(
        self, stx: SignedTransaction, sender_public_key: Optional[bytes] = None
    ) -> Dict[str, Any]:
        """Submit a signed transaction to the validator node."""
        tx_bytes = encode_signed_transaction(stx)
        body: Dict[str, str] = {
            "raw_tx_hex": tx_bytes.hex(),
        }
        if sender_public_key is not None:
            body["public_key"] = sender_public_key.hex()
        return self._request("POST", "/v1/tx/submit", body)
