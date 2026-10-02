#!/usr/bin/env python3
"""Continuously verify health, agreement, progress, and restart recovery.

The default endpoints target the four-validator Docker Compose mesh. Repeated
``--endpoint NAME=URL`` arguments can instead point at validators in distinct
regions without changing the verification logic.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DEFAULT_ENDPOINTS = {
    "node1": "http://127.0.0.1:8081",
    "node2": "http://127.0.0.1:8082",
    "node3": "http://127.0.0.1:8083",
    "node4": "http://127.0.0.1:8084",
}


@dataclass(frozen=True)
class Health:
    status: str
    wave: int
    state_root: str


def parse_endpoint(value: str) -> tuple[str, str]:
    if "=" not in value:
        raise argparse.ArgumentTypeError("endpoint must be NAME=URL")
    name, url = value.split("=", 1)
    if not name or not url.startswith(("http://", "https://")):
        raise argparse.ArgumentTypeError("endpoint must be NAME=http(s)://host:port")
    return name, url.rstrip("/")


def fetch_health(base_url: str, timeout: float) -> Health:
    request = urllib.request.Request(f"{base_url}/v1/health", headers={"Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        if response.status != 200:
            raise RuntimeError(f"HTTP {response.status}")
        payload: dict[str, Any] = json.load(response)
    return Health(
        status=str(payload.get("status", "")),
        wave=int(payload["highest_wave"]),
        state_root=str(payload["state_root"]),
    )


def sample(endpoints: dict[str, str], timeout: float) -> dict[str, Health]:
    states = {name: fetch_health(url, timeout) for name, url in endpoints.items()}
    unhealthy = [name for name, state in states.items() if state.status != "healthy"]
    if unhealthy:
        raise RuntimeError(f"unhealthy validators: {', '.join(unhealthy)}")
    roots = {state.state_root for state in states.values()}
    if "" in roots or len(roots) != 1:
        raise RuntimeError(f"state-root disagreement: {sorted(roots)}")
    return states


def wait_until_healthy(endpoints: dict[str, str], timeout: float, request_timeout: float) -> dict[str, Health]:
    deadline = time.monotonic() + timeout
    last_error = "no samples collected"
    while time.monotonic() < deadline:
        try:
            return sample(endpoints, request_timeout)
        except (KeyError, OSError, ValueError, RuntimeError, urllib.error.URLError) as error:
            last_error = str(error)
            time.sleep(1)
    raise RuntimeError(f"validators did not become healthy and converged within {timeout:.0f}s: {last_error}")


def restart_service(repo_root: Path, service: str) -> None:
    subprocess.run(
        ["docker", "compose", "restart", service],
        cwd=repo_root,
        check=True,
        text=True,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", action="append", type=parse_endpoint, help="validator NAME=URL (repeatable)")
    parser.add_argument("--duration", type=float, default=300, help="steady-state sample duration in seconds")
    parser.add_argument("--interval", type=float, default=5, help="seconds between samples")
    parser.add_argument("--warmup", type=float, default=90, help="maximum seconds to wait for initial convergence")
    parser.add_argument("--request-timeout", type=float, default=3, help="per-request timeout in seconds")
    parser.add_argument("--restart-service", help="Docker Compose service to restart once at the soak midpoint")
    parser.add_argument("--recovery-timeout", type=float, default=60, help="maximum seconds for post-restart recovery")
    parser.add_argument("--output", type=Path, help="optional JSON evidence output path")
    args = parser.parse_args()

    if args.duration <= 0 or args.interval <= 0 or args.warmup <= 0 or args.request_timeout <= 0:
        parser.error("duration, interval, warmup, and request-timeout must be positive")

    endpoints = dict(args.endpoint or DEFAULT_ENDPOINTS.items())
    if len(endpoints) < 2:
        parser.error("at least two validator endpoints are required to prove agreement")

    repo_root = Path(__file__).resolve().parent.parent
    started_at = datetime.now(timezone.utc)
    initial = wait_until_healthy(endpoints, args.warmup, args.request_timeout)
    initial_waves = {name: state.wave for name, state in initial.items()}
    final = initial
    samples = 0
    restart_completed = False
    deadline = time.monotonic() + args.duration
    restart_at = time.monotonic() + (args.duration / 2)

    while time.monotonic() < deadline:
        if args.restart_service and not restart_completed and time.monotonic() >= restart_at:
            restart_service(repo_root, args.restart_service)
            final = wait_until_healthy(endpoints, args.recovery_timeout, args.request_timeout)
            restart_completed = True
        else:
            final = sample(endpoints, args.request_timeout)
        samples += 1
        time.sleep(min(args.interval, max(0, deadline - time.monotonic())))

    final = sample(endpoints, args.request_timeout)
    final_waves = {name: state.wave for name, state in final.items()}
    stalled = [name for name in endpoints if final_waves[name] <= initial_waves[name]]
    if stalled:
        raise RuntimeError(f"validators made no wave progress: {', '.join(stalled)}")

    evidence = {
        "schema": "veridag.devnet-soak.v1",
        "started_at": started_at.isoformat(),
        "completed_at": datetime.now(timezone.utc).isoformat(),
        "duration_seconds": args.duration,
        "sample_count": samples + 2,
        "endpoints": endpoints,
        "initial_waves": initial_waves,
        "final_waves": final_waves,
        "state_root": next(iter({state.state_root for state in final.values()})),
        "restart_service": args.restart_service,
        "restart_recovered": restart_completed if args.restart_service else None,
        "result": "pass",
    }

    rendered = json.dumps(evidence, indent=2, sort_keys=True)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"devnet soak failed: {error}", file=sys.stderr)
        raise SystemExit(1)
