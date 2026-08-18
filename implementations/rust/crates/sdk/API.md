# Veridag SDK & Client API

This document is the canonical contract for every Veridag client integration
(Rust `veridag-sdk`, TypeScript, Python, Go). Keeping one documented surface
means a wallet, exchange adapter, or light client works against any node that
implements it.

## Transport

Two transports are supported:

1. **In-process** (Rust only): the `veridag-sdk` `InProcessClient` wraps an
   `ObjectState` directly for tests and single-binary demos.
2. **HTTP/JSON** (all languages): a node exposes the REST endpoints below.
   The Rust SDK provides an `http` feature implementing `VeridagClient` over
   `reqwest`; the TS/Python/Go clients implement the same calls over `fetch` /
   `requests` / `net/http`.

## Wire types (canonical field names)

All payloads are JSON. Bytes are hex-encoded strings.

```jsonc
Address   = string  // 32-byte hex
Hash      = string  // 32-byte hex
ObjectId  = string  // 32-byte hex
BatchId   = string  // 32-byte hex
Signature = string  // 64-byte hex

Transaction {
  protocol_version: number,
  chain_id: number,
  sender: Address,
  nonce: number,
  expiry_epoch: number,
  operation: { "TransferValue": { from: {id, expected}, to: Address, amount: number } }
             | { "CreateObject": {...} }
             | { "CallWasm": {...} },
  signature: Signature
}

Checkpoint {
  sequence: number,
  state_root: Hash,
  transaction_root: Hash,
  dag_commitment: Hash,
  validator_set_commitment: Hash,
  id: Hash,            // commitment over the above
  votes: number        // count of distinct validator finality votes
}
```

## Endpoints

| Method | Path                  | Body              | Returns                |
|--------|-----------------------|-------------------|------------------------|
| POST   | `/v1/submit`          | `Transaction`     | `{ tx_id: string }`    |
| GET    | `/v1/state-root`      | —                 | `{ root: Hash | null }`|
| GET    | `/v1/checkpoint/latest`| —                | `Checkpoint | null`    |
| GET    | `/v1/balance/{addr}`  | —                 | `{ balance: number }`  |
| GET    | `/v1/object/{id}`     | —                 | `{ data: string }`     |

## Errors

HTTP `4xx` with `{ "error": string }`. Client libraries map these to their
native error type (e.g. Rust `ClientError::Transport`).

## Determinism guarantees

* Transaction signatures use the domain tag `VERIDAG_TX_V1`.
* Keypairs are seed-derived (`from_seed([n; 32])`) — no RNG — so test fixtures
  are reproducible across languages.
* Batch commitments are order-sensitive (canonical serialization order).
