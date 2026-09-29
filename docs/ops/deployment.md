# Operator Deployment Guide

This document covers production deployment of a `veridag-node` validator on a
Linux VM, bare metal, or within the Docker Compose mesh.

---

## Environment Variables

The node binary reads the following environment variables at startup.
All are optional — defaults are production-safe for a local devnet.

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level / filter. E.g. `RUST_LOG=veridag=debug,info` |
| `VERIDAG_LOG_JSON` | *(unset)* | Set to `1` to emit logs as newline-delimited JSON (for log shippers) |
| `VERIDAG_DATA_DIR` | `./data` | Path to the persistent sled storage directory |

### Example: systemd unit

```ini
[Unit]
Description=Veridag Validator Node
After=network.target

[Service]
Type=simple
User=veridag
WorkingDirectory=/opt/veridag
Environment=RUST_LOG=info
Environment=VERIDAG_LOG_JSON=1
ExecStart=/opt/veridag/veridag-node daemon \
    --seed 1 \
    --peers 10.0.0.2:8000,10.0.0.3:8000,10.0.0.4:8000 \
    --bind 0.0.0.0:8000 \
    --rpc 0.0.0.0:8080
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

---

## HTTP Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/health` | GET | Full health JSON: version, state root, wave, checkpoints |
| `/v1/ready` | GET | Readiness probe: 200 = progressing, 503 = not yet |
| `/v1/metrics` | GET | Prometheus text format metrics |
| `/v1/state/root` | GET | Current state root and highest wave |
| `/v1/checkpoints/latest` | GET | Latest checkpoint details |
| `/v1/state/account/{addr}` | GET | Account balance and object for address |
| `/v1/submit` | POST | Submit a signed transaction |

### Health check example

```bash
curl -sf http://localhost:8080/v1/health | jq .
# {
#   "status": "healthy",
#   "version": "1.0.0",
#   "protocol_version": 1,
#   "chain_id": 1,
#   "highest_wave": 14,
#   "state_root": "0xac049e6f...",
#   "checkpoints_count": 3
# }
```

---

## Prometheus Metrics

Metrics are emitted at `/v1/metrics` in the standard Prometheus text format.

| Metric | Type | Description |
|--------|------|-------------|
| `veridag_vertices_proposed_total` | Counter | Vertices this node has proposed |
| `veridag_waves_committed_total` | Counter | Consensus waves committed |
| `veridag_txs_executed_total` | Counter | Transactions executed |
| `veridag_checkpoints_total` | Counter | Checkpoints produced |
| `veridag_highest_wave` | Gauge | Current highest committed wave |
| `veridag_max_round` | Gauge | Current max DAG round |

### Grafana alert rule (example)

```yaml
alert: VeridagNotProgressing
expr: increase(veridag_waves_committed_total[5m]) == 0
for: 10m
labels:
  severity: warning
annotations:
  summary: "Veridag validator {{ $labels.instance }} has not committed a wave in 10 minutes"
```

---

## Docker Compose (4-Validator Mesh)

```bash
# Start the 4-validator cluster
docker compose up -d

# Health check all 4 nodes
for port in 8081 8082 8083 8084; do
  echo "--- Node :$port ---"
  curl -sf http://localhost:$port/v1/health | jq '{status,highest_wave,state_root}'
done

# Stop
docker compose down
```

---

## Key Management

The `--seed` argument in the daemon command derives a deterministic keypair
from a fixed byte pattern (`[seed; 32]`). This is appropriate for
devnet/testnet validators only.

For production deployments, validator keys should be:
- Generated externally and injected via an environment variable or file
- Stored in an HSM or secret manager (e.g. HashiCorp Vault, AWS Secrets Manager)
- Rotated through the protocol's validator-set membership mechanism (spec 16)

> [!CAUTION]
> Never use seed-derived keys (e.g. `--seed 1`) for mainnet validators.
> Anyone who knows the seed value can forge your validator's signatures.

---

## Upgrade Procedure

1. Build the new release binary: `cargo build --release -p veridag-node`
2. Stop the running daemon: `systemctl --user stop veridag.service`
3. Replace the binary in `/opt/veridag/`
4. Restart: `systemctl --user start veridag.service`
5. Verify: `curl http://localhost:8080/v1/health | jq .version`

Rolling upgrades across a 4-validator cluster are safe provided the new binary
is protocol-compatible. Check `CHANGELOG.md` for breaking protocol changes.
