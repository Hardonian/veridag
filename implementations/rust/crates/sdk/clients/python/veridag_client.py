"""Veridag Python client — typed wrapper over the Veridag REST API.

Mirrors the Rust `veridag-sdk` `VeridagClient` trait. Uses only the standard
library (urllib) so it runs anywhere Python 3.8+ is available with no deps.

Example:
    from veridag_client import HttpClient
    client = HttpClient("http://localhost:8080")
    print(client.balance_of("abcd" * 16))
"""

from __future__ import annotations

import json
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

Hex = str  # hex-encoded string (address / hash / object id / signature)
Address = Hex
Hash = Hex
ObjectId = Hex
Signature = Hex


class VeridagClientError(Exception):
    """Transport / validation failure raised by the client."""

    def __init__(self, kind: str, message: str) -> None:
        super().__init__(message)
        self.kind = kind


@dataclass
class Transaction:
    protocol_version: int
    chain_id: int
    sender: Address
    nonce: int
    expiry_epoch: int
    operation: Dict[str, Any]
    signature: Signature


@dataclass
class Checkpoint:
    sequence: int
    state_root: Hash
    transaction_root: Hash
    dag_commitment: Hash
    validator_set_commitment: Hash
    id: Hash
    votes: int


class VeridagClient:
    """Abstract client surface (mirrors the SDK trait)."""

    def submit(self, tx: Transaction) -> str:  # pragma: no cover - interface
        raise NotImplementedError

    def state_root(self) -> Optional[Hash]:  # pragma: no cover
        raise NotImplementedError

    def latest_checkpoint(self) -> Optional[Checkpoint]:  # pragma: no cover
        raise NotImplementedError

    def balance_of(self, owner: Address) -> int:  # pragma: no cover
        raise NotImplementedError

    def get_object(self, id: ObjectId) -> Optional[str]:  # pragma: no cover
        raise NotImplementedError


class HttpClient(VeridagClient):
    """HTTP/JSON implementation against a Veridag node."""

    def __init__(self, base_url: str) -> None:
        self.base_url = base_url.rstrip("/")

    def _get(self, path: str) -> Any:
        url = f"{self.base_url}{path}"
        req = urllib.request.Request(url, headers={"accept": "application/json"})
        try:
            with urllib.request.urlopen(req) as resp:  # noqa: S310 - http only
                return json.loads(resp.read().decode())
        except urllib.error.HTTPError as e:
            if e.code == 404:
                raise VeridagClientError("not_found", e.reason) from e
            raise VeridagClientError("transport", str(e.reason)) from e

    def _post(self, path: str, body: Dict[str, Any]) -> Any:
        url = f"{self.base_url}{path}"
        data = json.dumps(body).encode()
        req = urllib.request.Request(
            url,
            data=data,
            headers={"content-type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req) as resp:  # noqa: S310
                return json.loads(resp.read().decode())
        except urllib.error.HTTPError as e:
            if e.code == 400:
                raise VeridagClientError("rejected", e.reason) from e
            raise VeridagClientError("transport", str(e.reason)) from e

    def submit(self, tx: Transaction) -> str:
        r = self._post("/v1/submit", tx.__dict__)
        return str(r["tx_id"])

    def state_root(self) -> Optional[Hash]:
        r = self._get("/v1/state-root")
        return r.get("root")

    def latest_checkpoint(self) -> Optional[Checkpoint]:
        r = self._get("/v1/checkpoint/latest")
        return Checkpoint(**r) if r else None

    def balance_of(self, owner: Address) -> int:
        r = self._get(f"/v1/balance/{owner}")
        return int(r["balance"])

    def get_object(self, id: ObjectId) -> Optional[str]:
        r = self._get(f"/v1/object/{id}")
        return r.get("data")
