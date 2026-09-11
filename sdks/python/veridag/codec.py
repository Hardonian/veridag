"""
Canonical VCE-1 encoder and decoder for Veridag.
"""

import struct
from .types import Transaction, SignedTransaction, ObjectRef, ResourceBudget, TransferValueOp, TransferObjectOp


class Encoder:
    def __init__(self):
        self._buf = bytearray()

    def u8(self, val: int) -> "Encoder":
        self._buf.append(val & 0xFF)
        return self

    def u32(self, val: int) -> "Encoder":
        self._buf.extend(struct.pack(">I", val & 0xFFFFFFFF))
        return self

    def u64(self, val: int) -> "Encoder":
        self._buf.extend(struct.pack(">Q", val & 0xFFFFFFFFFFFFFFFF))
        return self

    def fixed(self, data: bytes) -> "Encoder":
        self._buf.extend(data)
        return self

    def bytes(self, data: bytes) -> "Encoder":
        self.u32(len(data))
        self.fixed(data)
        return self

    def to_bytes(self) -> bytes:
        return bytes(self._buf)


def encode_objref(e: Encoder, ref: ObjectRef) -> None:
    e.fixed(ref.id)
    e.u64(ref.expected)


def encode_budget(e: Encoder, budget: ResourceBudget) -> None:
    e.u64(budget.compute_units)
    e.u64(budget.memory_bytes)
    e.u64(budget.storage_bytes)
    e.u64(budget.bandwidth_bytes)


def encode_transaction(tx: Transaction) -> bytes:
    e = Encoder()
    e.u64(tx.protocol_version)
    e.u64(tx.chain_id)
    e.fixed(tx.sender)
    e.u64(tx.nonce)
    e.u64(tx.expiry_epoch)

    # declared_reads
    e.u32(len(tx.declared_reads))
    for r in tx.declared_reads:
        encode_objref(e, r)

    # declared_writes
    e.u32(len(tx.declared_writes))
    for w in tx.declared_writes:
        encode_objref(e, w)

    # capabilities
    e.u32(len(tx.capabilities))
    for c in tx.capabilities:
        e.fixed(c)

    # Operation
    if isinstance(tx.operation, TransferValueOp):
        e.u8(4)
        encode_objref(e, tx.operation.from_ref)
        e.fixed(tx.operation.to)
        e.u64(tx.operation.amount)
    elif isinstance(tx.operation, TransferObjectOp):
        e.u8(3)
        encode_objref(e, tx.operation.object)
        e.u8(0)  # Ownership::Address
        e.fixed(tx.operation.new_owner)
    else:
        raise ValueError(f"unsupported operation type: {type(tx.operation)}")

    encode_budget(e, tx.resource_budget)
    e.bytes(tx.metadata)

    return e.to_bytes()


def encode_signed_transaction(stx: SignedTransaction) -> bytes:
    tx_bytes = encode_transaction(stx.tx)
    return tx_bytes + stx.signature
