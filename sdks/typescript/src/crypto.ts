/**
 * Cryptographic routines for the Veridag TypeScript SDK.
 * BLAKE3 domain-separated hashing and Ed25519 signing.
 */

import crypto from "node:crypto";

const IV = new Uint32Array([
  0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
  0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
]);

const MSG_PERM = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

function rotr(w: number, c: number): number {
  return ((w >>> c) | (w << (32 - c))) >>> 0;
}

function g(state: Uint32Array, a: number, b: number, c: number, d: number, mx: number, my: number): void {
  state[a] = (state[a] + state[b] + mx) >>> 0;
  state[d] = rotr(state[d] ^ state[a], 16);
  state[c] = (state[c] + state[d]) >>> 0;
  state[b] = rotr(state[b] ^ state[c], 12);
  state[a] = (state[a] + state[b] + my) >>> 0;
  state[d] = rotr(state[d] ^ state[a], 8);
  state[c] = (state[c] + state[d]) >>> 0;
  state[b] = rotr(state[b] ^ state[c], 7);
}

function round(state: Uint32Array, m: Uint32Array): void {
  g(state, 0, 4, 8, 12, m[0], m[1]);
  g(state, 1, 5, 9, 13, m[2], m[3]);
  g(state, 2, 6, 10, 14, m[4], m[5]);
  g(state, 3, 7, 11, 15, m[6], m[7]);
  g(state, 0, 5, 10, 15, m[8], m[9]);
  g(state, 1, 6, 11, 12, m[10], m[11]);
  g(state, 2, 7, 8, 13, m[12], m[13]);
  g(state, 3, 4, 9, 14, m[14], m[15]);
}

function compress(
  cv: Uint32Array,
  blockBytes: Uint8Array,
  blockLen: number,
  counter: number,
  flags: number
): Uint32Array {
  const state = new Uint32Array(16);
  state.set(cv, 0);
  state.set(IV, 8);
  state[12] = counter >>> 0;
  state[13] = 0;
  state[14] = blockLen >>> 0;
  state[15] = flags >>> 0;

  const m = new Uint32Array(16);
  const buf = new Uint8Array(64);
  buf.set(blockBytes);
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  for (let i = 0; i < 16; i++) {
    m[i] = view.getUint32(i * 4, true);
  }

  for (let r = 0; r < 7; r++) {
    round(state, m);
    const mNext = new Uint32Array(16);
    for (let i = 0; i < 16; i++) {
      mNext[i] = m[MSG_PERM[i]];
    }
    m.set(mNext);
  }

  const out = new Uint32Array(8);
  for (let i = 0; i < 8; i++) {
    out[i] = (state[i] ^ state[i + 8]) >>> 0;
  }
  return out;
}

/**
 * Pure BLAKE3 cryptographic hash implementation.
 */
export function blake3(input: Uint8Array): Uint8Array {
  let cv = new Uint32Array(IV);
  const CHUNK_START = 1;
  const CHUNK_END = 2;
  const ROOT = 8;
  const numBlocks = Math.ceil(input.length / 64) || 1;

  for (let b = 0; b < numBlocks; b++) {
    const start = b * 64;
    const end = Math.min(start + 64, input.length);
    const block = input.subarray(start, end);
    let flags = 0;
    if (b === 0) flags |= CHUNK_START;
    if (b === numBlocks - 1) flags |= CHUNK_END | ROOT;
    cv = compress(cv, block, block.length, 0, flags);
  }

  const res = new Uint8Array(32);
  const view = new DataView(res.buffer, res.byteOffset, res.byteLength);
  for (let i = 0; i < 8; i++) {
    view.setUint32(i * 4, cv[i], true);
  }
  return res;
}

/**
 * Protocol domain-separated hash: `BLAKE3(domain || 0x00 || payload)`.
 */
export function hash(domain: string, payload: Uint8Array): Uint8Array {
  const domainBytes = new TextEncoder().encode(domain);
  const buf = new Uint8Array(domainBytes.length + 1 + payload.length);
  buf.set(domainBytes, 0);
  buf[domainBytes.length] = 0;
  buf.set(payload, domainBytes.length + 1);
  return blake3(buf);
}

/**
 * Derive an account address from an Ed25519 public key.
 */
export function addressOf(pubKey: Uint8Array): Uint8Array {
  return hash("VERIDAG_ADDRESS_V1", pubKey);
}

/**
 * Derive a canonical ObjectId from a creator address and nonce.
 */
export function deriveObjectId(creator: Uint8Array, nonce: bigint): Uint8Array {
  const buf = new Uint8Array(40);
  buf.set(creator, 0);
  for (let i = 7; i >= 0; i--) {
    buf[32 + (7 - i)] = Number((nonce >> BigInt(i * 8)) & 0xffn);
  }
  return hash("VERIDAG_OBJECT_ID_V1", buf);
}

/**
 * Ed25519 Keypair wrapper.
 */
export class Keypair {
  private privKey: crypto.KeyObject;
  private pubKeyBytes: Uint8Array;
  private addressBytes: Uint8Array;

  private constructor(privKey: crypto.KeyObject, pubKeyBytes: Uint8Array) {
    this.privKey = privKey;
    this.pubKeyBytes = pubKeyBytes;
    this.addressBytes = addressOf(pubKeyBytes);
  }

  /**
   * Derive a Keypair from a 32-byte secret seed.
   */
  public static fromSeed(seed: Uint8Array): Keypair {
    if (seed.length !== 32) {
      throw new Error("seed must be 32 bytes");
    }
    const pkcs8Prefix = Buffer.from("302e020100300506032b657004220420", "hex");
    const pkcs8Der = Buffer.concat([pkcs8Prefix, Buffer.from(seed)]);
    const privKey = crypto.createPrivateKey({ key: pkcs8Der, format: "der", type: "pkcs8" });
    const pubKeyDer = crypto.createPublicKey(privKey).export({ format: "der", type: "spki" });
    const pubKeyBytes = new Uint8Array(pubKeyDer.subarray(pubKeyDer.length - 32));
    return new Keypair(privKey, pubKeyBytes);
  }

  public public(): Uint8Array {
    return this.pubKeyBytes;
  }

  public address(): Uint8Array {
    return this.addressBytes;
  }

  /**
   * Sign payload under a domain separator: `Sign(domain || 0x00 || payload)`.
   */
  public sign(domain: string, payload: Uint8Array): Uint8Array {
    const domainBytes = new TextEncoder().encode(domain);
    const msg = Buffer.concat([Buffer.from(domainBytes), Buffer.from([0]), Buffer.from(payload)]);
    const sig = crypto.sign(null, msg, this.privKey);
    return new Uint8Array(sig);
  }
}
