/**
 * Ergonomic, deterministic transaction builder matching the Rust TxBuilder.
 */

import { encodeTransaction } from "./codec.ts";
import { Keypair, deriveObjectId } from "./crypto.ts";
import type { Address, SignedTransaction, Transaction } from "./types.ts";

export class TxBuilder {
  private keypair: Keypair;
  private chainIdVal: bigint = 1n;
  private nonceVal: bigint = 0n;
  private expiryEpochVal: bigint = 0xffffffffffffffffn;

  constructor(from: Keypair) {
    this.keypair = from;
  }

  public chain(chainId: bigint | number): this {
    this.chainIdVal = BigInt(chainId);
    return this;
  }

  public nonce(nonce: bigint | number): this {
    this.nonceVal = BigInt(nonce);
    return this;
  }

  public expiry(epoch: bigint | number): this {
    this.expiryEpochVal = BigInt(epoch);
    return this;
  }

  public transfer(to: Address, amount: bigint | number): SignedTransaction {
    const fromAddr = this.keypair.address();
    const fromId = deriveObjectId(fromAddr, 0n);

    const tx: Transaction = {
      protocolVersion: 1n,
      chainId: this.chainIdVal,
      sender: fromAddr,
      nonce: this.nonceVal,
      expiryEpoch: this.expiryEpochVal,
      declaredReads: [],
      declaredWrites: [],
      capabilities: [],
      operation: {
        type: "TransferValue",
        from: {
          id: fromId,
          expected: this.nonceVal,
        },
        to,
        amount: BigInt(amount),
      },
      resourceBudget: {
        computeUnits: 0n,
        memoryBytes: 0n,
      },
      metadata: new Uint8Array(0),
    };

    const payload = encodeTransaction(tx);
    const signature = this.keypair.sign("VERIDAG_TX_V1", payload);

    return { tx, signature };
  }
}
