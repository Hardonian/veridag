"""
Ergonomic, deterministic transaction builder for the Veridag Python SDK.
"""

from .types import Transaction, SignedTransaction, ObjectRef, TransferValueOp, Address, ResourceBudget
from .codec import encode_transaction
from .crypto import Keypair, derive_object_id


class TxBuilder:
    """Fluent builder for constructing and signing transactions."""

    def __init__(self, keypair: Keypair):
        self._keypair = keypair
        self._chain_id = 1
        self._nonce = 0
        self._expiry_epoch = 0xFFFFFFFFFFFFFFFF

    def chain(self, chain_id: int) -> "TxBuilder":
        self._chain_id = chain_id
        return self

    def nonce(self, nonce: int) -> "TxBuilder":
        self._nonce = nonce
        return self

    def expiry(self, epoch: int) -> "TxBuilder":
        self._expiry_epoch = epoch
        return self

    def transfer(self, to: Address, amount: int) -> SignedTransaction:
        from_addr = self._keypair.address()
        from_id = derive_object_id(from_addr, 0)

        tx = Transaction(
            protocol_version=1,
            chain_id=self._chain_id,
            sender=from_addr,
            nonce=self._nonce,
            expiry_epoch=self._expiry_epoch,
            declared_reads=[],
            declared_writes=[],
            capabilities=[],
            operation=TransferValueOp(
                from_ref=ObjectRef(id=from_id, expected=self._nonce),
                to=to,
                amount=amount,
            ),
            resource_budget=ResourceBudget(),
            metadata=b"",
        )

        payload = encode_transaction(tx)
        signature = self._keypair.sign("VERIDAG_TX_V1", payload)
        return SignedTransaction(tx=tx, signature=signature)
