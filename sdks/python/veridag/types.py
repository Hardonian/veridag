"""
Canonical types for Veridag Python SDK.
"""

from dataclasses import dataclass, field
from typing import List, Union

Address = bytes  # 32 bytes
ObjectId = bytes  # 32 bytes
CapabilityId = bytes  # 32 bytes
TransactionId = bytes  # 32 bytes


@dataclass(frozen=True)
class ObjectRef:
    id: ObjectId
    expected: int


@dataclass(frozen=True)
class TransferValueOp:
    from_ref: ObjectRef
    to: Address
    amount: int


@dataclass(frozen=True)
class TransferObjectOp:
    object: ObjectRef
    new_owner: Address


Operation = Union[TransferValueOp, TransferObjectOp]


@dataclass(frozen=True)
class ResourceBudget:
    compute_units: int = 0
    memory_bytes: int = 0
    storage_bytes: int = 0
    bandwidth_bytes: int = 0


@dataclass(frozen=True)
class Transaction:
    protocol_version: int
    chain_id: int
    sender: Address
    nonce: int
    expiry_epoch: int
    declared_reads: List[ObjectRef] = field(default_factory=list)
    declared_writes: List[ObjectRef] = field(default_factory=list)
    capabilities: List[CapabilityId] = field(default_factory=list)
    operation: Operation = None  # type: ignore
    resource_budget: ResourceBudget = field(default_factory=ResourceBudget)
    metadata: bytes = b""


@dataclass(frozen=True)
class SignedTransaction:
    tx: Transaction
    signature: bytes  # 64 bytes
