import assert from "node:assert";
import http from "node:http";
import { VeridagClient } from "../src/client.ts";
import { Keypair } from "../src/crypto.ts";
import { TxBuilder } from "../src/builder.ts";

console.log("Running Veridag TypeScript SDK Client tests...");

// Setup mock HTTP server
const server = http.createServer((req, res) => {
  res.setHeader("Content-Type", "application/json");

  if (req.method === "GET" && req.url === "/v1/health") {
    res.writeHead(200);
    res.end(
      JSON.stringify({
        status: "healthy",
        version: "0.1.0-alpha",
        protocol_version: 1,
        chain_id: 1,
        validator_seed: 1,
        committee_n: 4,
        committee_quorum: 3,
        max_round: 12,
        highest_wave: 3,
        state_root: "0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
        checkpoints_count: 1,
      })
    );
    return;
  }

  if (req.method === "GET" && req.url === "/v1/state/root") {
    res.writeHead(200);
    res.end(
      JSON.stringify({
        state_root: "0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
        highest_wave: 3,
      })
    );
    return;
  }

  if (req.method === "GET" && req.url?.startsWith("/v1/state/account/")) {
    res.writeHead(200);
    res.end(
      JSON.stringify({
        address: "0x1111111111111111111111111111111111111111111111111111111111111111",
        object_id: "0x2222222222222222222222222222222222222222222222222222222222222222",
        balance: 5000000,
        exists: true,
      })
    );
    return;
  }

  if (req.method === "GET" && req.url === "/v1/checkpoints/latest") {
    res.writeHead(200);
    res.end(
      JSON.stringify({
        id: "0x3a875556df63e5ff02303aa0971018d2a69e3d1bd344b50cbb459b160daade40",
        sequence: 1,
        epoch: 0,
        state_root: "0xac049e6fdadc2840ff5d3a9ee9e4598a4eebf86e68861bccedcab8f46942cb4e",
        transaction_root: "0x4444444444444444444444444444444444444444444444444444444444444444",
        dag_commitment: "0x5555555555555555555555555555555555555555555555555555555555555555",
        validator_set_commitment: "0x6666666666666666666666666666666666666666666666666666666666666666",
        votes_count: 4,
      })
    );
    return;
  }

  if (req.method === "POST" && req.url === "/v1/tx/submit") {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      const parsed = JSON.parse(body);
      assert.ok(parsed.raw_tx_hex, "Expected raw_tx_hex");
      res.writeHead(200);
      res.end(
        JSON.stringify({
          status: "admitted",
          tx_id: "0x9999999999999999999999999999999999999999999999999999999999999999",
          sender: "0x1111111111111111111111111111111111111111111111111111111111111111",
        })
      );
    });
    return;
  }

  res.writeHead(404);
  res.end(JSON.stringify({ error: "not found" }));
});

server.listen(0, "127.0.0.1", async () => {
  const addr = server.address();
  const port = typeof addr === "object" && addr ? addr.port : 8080;
  const client = new VeridagClient(`http://127.0.0.1:${port}`);

  try {
    // 1. Health
    const health = await client.health();
    assert.strictEqual(health.status, "healthy");
    assert.strictEqual(health.chainId, 1);
    console.log("  [PASS] client.health() returned expected status");

    // 2. State root
    const root = await client.getStateRoot();
    assert.ok(root.stateRoot.startsWith("0xac04"));
    console.log("  [PASS] client.getStateRoot() returned expected root");

    // 3. Account balance
    const bal = await client.getAccountBalance("0x1111111111111111111111111111111111111111111111111111111111111111");
    assert.strictEqual(bal.balance, 5000000);
    assert.strictEqual(bal.exists, true);
    console.log("  [PASS] client.getAccountBalance() returned expected balance");

    // 4. Checkpoint
    const ckpt = await client.getLatestCheckpoint();
    assert.strictEqual(ckpt.sequence, 1);
    assert.strictEqual(ckpt.votesCount, 4);
    console.log("  [PASS] client.getLatestCheckpoint() returned expected checkpoint");

    // 5. Submit transaction
    const sender = Keypair.fromSeed(new Uint8Array(32).fill(1));
    const recipient = Keypair.fromSeed(new Uint8Array(32).fill(2)).address();
    const stx = new TxBuilder(sender)
      .nonce(0n)
      .transfer(recipient, 100n);


    const submitRes = await client.submitTransaction(stx, sender.public());
    assert.strictEqual(submitRes.status, "admitted");
    assert.ok(submitRes.txId.startsWith("0x"));
    console.log("  [PASS] client.submitTransaction() admitted successfully");

    console.log("ALL VERIDAG TYPESCRIPT SDK CLIENT TESTS PASSED!");
  } finally {
    server.close();
  }
});
