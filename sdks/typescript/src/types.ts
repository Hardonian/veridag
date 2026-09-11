/**
 * Canonical protocol types for the Veridag TypeScript SDK.
 */

export type Address = Uint8Array; // 32 bytes
export type ObjectId = Uint8Array; // 32 bytes
export type CapabilityId = Uint8Array; // 32 bytes
export type TransactionId = Uint8Array; // 32 bytes

export interface ObjectRef {
  id: ObjectId;
  expected: bigint;
}

export type Operation =
  | {
      type: "TransferValue";
      from: ObjectRef;
      to: Address;
      amount: bigint;
    }
  | {
      type: "TransferObject";
      object: ObjectRef;
      newOwner: Uint8Array;
    };

export interface ResourceBudget {
  computeUnits: bigint;
  memoryBytes: bigint;
}

export interface Transaction {
  protocolVersion: bigint;
  chainId: bigint;
  sender: Address;
  nonce: bigint;
  expiryEpoch: bigint;
  declaredReads: ObjectRef[];
  declaredWrites: ObjectRef[];
  capabilities: CapabilityId[];
  operation: Operation;
  resourceBudget: ResourceBudget;
  metadata: Uint8Array;
}

export interface SignedTransaction {
  tx: Transaction;
  signature: Uint8Array; // 64 bytes
}
