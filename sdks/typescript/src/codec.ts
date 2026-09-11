/**
 * Canonical VCE-1 encoder and decoder for Veridag.
 */

import type { ObjectRef, Operation, ResourceBudget, SignedTransaction, Transaction } from "./types.ts";

export class Encoder {
  private buffer: number[] = [];

  public u8(value: number): this {
    this.buffer.push(value & 0xff);
    return this;
  }

  public u32(value: number): this {
    this.buffer.push((value >>> 24) & 0xff);
    this.buffer.push((value >>> 16) & 0xff);
    this.buffer.push((value >>> 8) & 0xff);
    this.buffer.push(value & 0xff);
    return this;
  }

  public u64(value: bigint | number): this {
    const v = BigInt(value);
    for (let i = 7; i >= 0; i--) {
      this.buffer.push(Number((v >> BigInt(i * 8)) & 0xffn));
    }
    return this;
  }

  public fixed(bytes: Uint8Array): this {
    for (let i = 0; i < bytes.length; i++) {
      this.buffer.push(bytes[i]);
    }
    return this;
  }

  public bytes(bytes: Uint8Array): this {
    this.u32(bytes.length);
    this.fixed(bytes);
    return this;
  }

  public toBytes(): Uint8Array {
    return new Uint8Array(this.buffer);
  }
}

export function encodeObjRef(e: Encoder, ref: ObjectRef): void {
  e.fixed(ref.id);
  e.u64(ref.expected);
}

export function encodeBudget(e: Encoder, budget: ResourceBudget): void {
  e.u64(budget.computeUnits);
  e.u64(budget.memoryBytes);
  e.u64(0n); // storage
  e.u64(0n); // bandwidth
}

export function encodeTransaction(tx: Transaction): Uint8Array {
  const e = new Encoder();
  e.u64(tx.protocolVersion);
  e.u64(tx.chainId);
  e.fixed(tx.sender);
  e.u64(tx.nonce);
  e.u64(tx.expiryEpoch);

  // declared_reads: Vec<ObjectRef>
  e.u32(tx.declaredReads.length);
  for (const r of tx.declaredReads) {
    encodeObjRef(e, r);
  }

  // declared_writes: Vec<ObjectRef>
  e.u32(tx.declaredWrites.length);
  for (const w of tx.declaredWrites) {
    encodeObjRef(e, w);
  }

  // capabilities: Vec<CapabilityId>
  e.u32(tx.capabilities.length);
  for (const c of tx.capabilities) {
    e.fixed(c);
  }

  // Operation
  switch (tx.operation.type) {
    case "TransferValue": {
      e.u8(4);
      encodeObjRef(e, tx.operation.from);
      e.fixed(tx.operation.to);
      e.u64(tx.operation.amount);
      break;
    }
    case "TransferObject": {
      e.u8(3);
      encodeObjRef(e, tx.operation.object);
      e.u8(0); // Ownership::Address
      e.fixed(tx.operation.newOwner);
      break;
    }
  }

  encodeBudget(e, tx.resourceBudget);
  e.bytes(tx.metadata);

  return e.toBytes();
}

export function encodeSignedTransaction(stx: SignedTransaction): Uint8Array {
  const txBytes = encodeTransaction(stx.tx);
  const out = new Uint8Array(txBytes.length + stx.signature.length);
  out.set(txBytes, 0);
  out.set(stx.signature, txBytes.length);
  return out;
}
