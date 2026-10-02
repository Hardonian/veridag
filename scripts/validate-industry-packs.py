"""Validate the repository's industry-pack manifests without dependencies."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = {
    "schema_version",
    "id",
    "name",
    "standard",
    "normative_url",
    "profile",
    "media_types",
    "rust_adapter",
    "data_policy",
    "status",
}


def main() -> None:
    manifests = sorted((ROOT / "industry-packs").glob("*/pack.json"))
    if not manifests:
        raise SystemExit("no industry pack manifests found")

    identifiers: set[str] = set()
    for path in manifests:
        data = json.loads(path.read_text(encoding="utf-8"))
        missing = REQUIRED - data.keys()
        if missing:
            raise SystemExit(f"{path}: missing {sorted(missing)}")
        if data["schema_version"] != 1:
            raise SystemExit(f"{path}: unsupported schema_version")
        if data["id"] in identifiers:
            raise SystemExit(f"{path}: duplicate id {data['id']}")
        identifiers.add(data["id"])
        if data["status"] not in {"experimental", "beta", "production"}:
            raise SystemExit(f"{path}: invalid status")
        if not data["normative_url"].startswith("https://"):
            raise SystemExit(f"{path}: normative_url must use HTTPS")

    print(f"validated {len(manifests)} industry packs")


if __name__ == "__main__":
    main()
