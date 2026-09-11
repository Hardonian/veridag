import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { Keypair, hash } from "../src/crypto.ts";
import { TxBuilder } from "../src/builder.ts";
import { encodeTransaction, encodeSignedTransaction } from "../src/codec.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const vectorPath = path.resolve(__dirname, "../../../protocol/test-vectors/sdk_conformance.json");
const vector = JSON.parse(fs.readFileSync(vectorPath, "utf8"));

function hexToBytes(hex) {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  return new Uint8Array(Buffer.from(clean, "hex"));
}

function bytesToHex(bytes) {
  return Buffer.from(bytes).toString("hex");
}

console.log("Running Veridag TypeScript SDK conformance test against golden vectors...");

// 1. Derive keypair
const seed = hexToBytes(vector.secret_seed);
const keypair = Keypair.fromSeed(seed);

// 2. Public key check
const expectedPk = vector.public_key.replace(/^0x/, "");
assert.strictEqual(bytesToHex(keypair.public()), expectedPk, "Public key mismatch");
console.log("  [PASS] Public key matches golden vector");

// 3. Address derivation check
const expectedAddress = vector.sender_address.replace(/^0x/, "");
assert.strictEqual(bytesToHex(keypair.address()), expectedAddress, "Sender address mismatch");
console.log("  [PASS] Sender address matches golden vector");

// 4. Build transaction
const recipient = hexToBytes(vector.recipient_address);
const signedTx = new TxBuilder(keypair)
  .chain(vector.chain_id)
  .nonce(vector.nonce)
  .expiry(vector.expiry_epoch)
  .transfer(recipient, vector.amount);

// 5. Unsigned payload check
const unsignedBytes = encodeTransaction(signedTx.tx);
const expectedUnsigned = vector.unsigned_tx_payload.replace(/^0x/, "");
assert.strictEqual(bytesToHex(unsignedBytes), expectedUnsigned, "Unsigned payload mismatch");
console.log("  [PASS] Canonical VCE-1 unsigned payload matches golden vector");

// 6. Signature check
const expectedSig = vector.signature.replace(/^0x/, "");
assert.strictEqual(bytesToHex(signedTx.signature), expectedSig, "Signature mismatch");
console.log("  [PASS] Ed25519 domain-separated signature matches golden vector");

// 7. Signed wire bytes check
const signedWire = encodeSignedTransaction(signedTx);
const expectedSigned = vector.signed_tx_wire.replace(/^0x/, "");
assert.strictEqual(bytesToHex(signedWire), expectedSigned, "Signed wire payload mismatch");
console.log("  [PASS] Signed wire bytes match golden vector");

// 8. Transaction ID / Hash check
const txHash = hash("VERIDAG_TX_V1", signedWire);
const expectedHash = vector.tx_hash.replace(/^0x/, "");
assert.strictEqual(bytesToHex(txHash), expectedHash, "Transaction ID hash mismatch");
console.log("  [PASS] Transaction hash matches golden vector");

console.log("\nALL TYPESCRIPT SDK CONFORMANCE TESTS PASSED (100% BIT-FOR-BIT EQUIVALENCE WITH RUST)");
