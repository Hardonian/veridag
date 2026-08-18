//! Veridag SDK — the stable developer surface for building on Veridag.
//!
//! This crate intentionally re-exports only the types a third-party integrator
//! needs and provides ergonomic builders + a transport-agnostic client trait.
//! Keeping the surface small and explicit is what makes the protocol
//! "universal": a wallet, exchange adapter, or light client only depends on
//! this facade, never on internal consensus/execution modules.
//!
//! Design priorities (per the low-latency / low-energy / portable mandate):
//!   * no_std-friendly core types (the re-exported crates forbid unsafe_code),
//!   * deterministic signing via [`TxBuilder`] (no RNG, seed-driven keys),
//!   * a [`VeridagClient`] trait so the same application code runs against an
//!     in-process node, an HTTP endpoint, or a mock in tests.

#![forbid(unsafe_code)]

// --- Stable re-exports -----------------------------------------------------
pub use veridag_checkpoint::{Checkpoint, CheckpointError};
pub use veridag_codec::{Decode, Decoder, Encode, Encoder};
pub use veridag_crypto::Keypair;
pub use veridag_object_state::{Object, ObjectState};
pub use veridag_protocol_types::{
    Address, BatchId, ChainId, CheckpointId, Ed25519Signature, Epoch, Hash, ObjectId, ObjectRef,
    Ownership, ResourceBudget, TransactionId, ValidatorId, CURRENT_PROTOCOL_VERSION,
};
pub use veridag_transaction::{Operation, SignedTransaction, Transaction};

/// Re-exported signature type (alias for clarity at the SDK boundary).
pub use veridag_protocol_types::Ed25519Signature as Signature;

use veridag_crypto::hash;

/// Domain-separation tag for transaction signatures (must match core crates).
pub const TX_DOMAIN: &str = "VERIDAG_TX_V1";

/// A transfer/value-operation builder. Deterministic and RNG-free: the signing
/// key is seed-derived, so test fixtures and reproducible demos are stable.
pub struct TxBuilder {
    from: Keypair,
    chain: ChainId,
    nonce: u64,
    expiry_epoch: Epoch,
}

impl TxBuilder {
    /// Start building a transaction signed by `from`.
    pub fn new(from: Keypair) -> Self {
        Self {
            from,
            chain: 1,
            nonce: 0,
            expiry_epoch: u64::MAX,
        }
    }

    /// Set the chain id (defaults to 1, the reference chain).
    pub fn chain(mut self, chain: ChainId) -> Self {
        self.chain = chain;
        self
    }

    /// Set the sender nonce (object version / replay guard).
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }

    /// Set the expiry epoch (defaults to never-expire).
    pub fn expiry(mut self, epoch: Epoch) -> Self {
        self.expiry_epoch = epoch;
        self
    }

    /// Build a signed value transfer from the sender's default account to `to`.
    pub fn transfer(self, to: Address, amount: u64) -> SignedTransaction {
        let from_addr = self.from.address();
        let from_id = Object::derive_id(&from_addr, 0);
        let tx = Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: self.chain,
            sender: from_addr,
            nonce: self.nonce,
            expiry_epoch: self.expiry_epoch,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: Operation::TransferValue {
                from: ObjectRef {
                    id: from_id,
                    expected: self.nonce,
                },
                to,
                amount,
            },
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = self
            .from
            .sign(TX_DOMAIN, &veridag_codec::Encode::to_bytes(&tx));
        SignedTransaction { tx, signature: sig }
    }

    /// Build a signed transaction from an arbitrary [`Operation`].
    pub fn operation(self, op: Operation) -> SignedTransaction {
        let tx = Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: self.chain,
            sender: self.from.address(),
            nonce: self.nonce,
            expiry_epoch: self.expiry_epoch,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: op,
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = self
            .from
            .sign(TX_DOMAIN, &veridag_codec::Encode::to_bytes(&tx));
        SignedTransaction { tx, signature: sig }
    }
}

/// The stable client API every Veridag integration speaks.
///
/// Implementors: [`InProcessClient`] (wraps a full `Node`), an HTTP client
/// (feature `http`), and mocks in tests. Keeping this trait in the SDK crate
/// lets application code be transport-agnostic.
pub trait VeridagClient {
    /// Submit a signed transaction; returns its id once mempool-admitted.
    fn submit(&self, tx: &SignedTransaction) -> Result<TransactionId, ClientError>;
    /// Fetch the current committed state root (or `None` before first commit).
    fn state_root(&self) -> Option<Hash>;
    /// Fetch the latest finalized checkpoint, if any.
    fn latest_checkpoint(&self) -> Option<Checkpoint>;
    /// Fetch a balance object's value (big-endian u64) by owner address.
    fn balance_of(&self, owner: &Address) -> Result<u64, ClientError>;
    /// Fetch a raw object by id (serialized bytes), if it exists.
    fn get_object(&self, id: &ObjectId) -> Option<Vec<u8>>;
}

/// Client-side errors. These are transport/validation failures, distinct from
/// consensus-internal errors.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ClientError {
    #[error("rejected by mempool: {0}")]
    Rejected(String),
    #[error("object not found")]
    NotFound,
    #[error("transport error: {0}")]
    Transport(String),
}

/// Build a deterministic keypair from a single byte seed (0..=255).
pub fn seed_keypair(n: u8) -> Keypair {
    Keypair::from_seed(&[n; 32])
}

/// Clone a [`Keypair`] by re-deriving from its secret seed. `Keypair` is not
/// `Clone` (it wraps a sensitive signing key), so callers must be explicit.
pub fn clone_keypair(k: &Keypair) -> Keypair {
    Keypair::from_seed(&k.secret_seed())
}

/// Derive the canonical account object id for an address + slot.
pub fn account_id(owner: &Address, slot: u64) -> ObjectId {
    ObjectId(Object::derive_id(owner, slot).0)
}

/// Commitment helpers used by clients to reason about DAG/gossip content.
pub fn batch_commitment(txs: &[SignedTransaction]) -> BatchId {
    let mut buf = Vec::new();
    for t in txs {
        buf.extend_from_slice(&veridag_codec::Encode::to_bytes(t));
    }
    BatchId(hash("VERIDAG_BATCH_V1", &buf))
}

/// A minimal in-process client for tests and single-binary demos.
///
/// NOTE: this wraps a reference to a node-like state. In the alpha it is used
/// by the SDK test-suite to prove the client trait is exercisable end-to-end
/// without any network. Production deployments use an HTTP/gRPC implementor.
pub struct InProcessClient<'a> {
    state: &'a ObjectState,
    root: Option<Hash>,
    checkpoint: Option<Checkpoint>,
}

impl<'a> InProcessClient<'a> {
    pub fn new(state: &'a ObjectState, root: Option<Hash>, checkpoint: Option<Checkpoint>) -> Self {
        Self {
            state,
            root,
            checkpoint,
        }
    }
}

impl VeridagClient for InProcessClient<'_> {
    fn submit(&self, _tx: &SignedTransaction) -> Result<TransactionId, ClientError> {
        // In-process submission is a no-op for read-only state; the node binary
        // owns the mempool. We return the tx id for API completeness.
        Err(ClientError::Rejected(
            "in-process client is read-only; use a node-backed client".into(),
        ))
    }

    fn state_root(&self) -> Option<Hash> {
        self.root
    }

    fn latest_checkpoint(&self) -> Option<Checkpoint> {
        self.checkpoint.clone()
    }

    fn balance_of(&self, owner: &Address) -> Result<u64, ClientError> {
        let id = account_id(owner, 0);
        self.state.balance(&id).map_err(|_| ClientError::NotFound)
    }

    fn get_object(&self, id: &ObjectId) -> Option<Vec<u8>> {
        self.state.get(id).map(veridag_codec::Encode::to_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_protocol_types::object_type;

    #[test]
    fn tx_builder_signs_deterministically() {
        let alice = seed_keypair(100);
        let bob = seed_keypair(101);
        let stx1 = TxBuilder::new(clone_keypair(&alice)).transfer(bob.address(), 40);
        let stx2 = TxBuilder::new(clone_keypair(&alice)).transfer(bob.address(), 40);
        // Same inputs -> identical signed tx (RNG-free).
        assert_eq!(
            veridag_codec::Encode::to_bytes(&stx1),
            veridag_codec::Encode::to_bytes(&stx2)
        );
        // Signature verifies against alice's public key.
        assert!(stx1.verify_signature(&alice.public()).is_ok());
        // Different amount -> different tx.
        let stx3 = TxBuilder::new(clone_keypair(&alice)).transfer(bob.address(), 41);
        assert_ne!(
            veridag_codec::Encode::to_bytes(&stx1),
            veridag_codec::Encode::to_bytes(&stx3)
        );
    }

    #[test]
    fn in_process_client_reads_balance() {
        let mut state = ObjectState::new();
        let alice = seed_keypair(100);
        let bob = seed_keypair(101);
        state
            .create(Object::new(
                Object::derive_id(&alice.address(), 0),
                object_type::BALANCE,
                Ownership::Address(alice.address()),
                100u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
        state
            .create(Object::new(
                Object::derive_id(&bob.address(), 0),
                object_type::BALANCE,
                Ownership::Address(bob.address()),
                40u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
        let client = InProcessClient::new(&state, Some([7u8; 32]), None);
        assert_eq!(client.balance_of(&bob.address()).unwrap(), 40);
        assert_eq!(client.balance_of(&alice.address()).unwrap(), 100);
        assert_eq!(client.state_root(), Some([7u8; 32]));
    }

    #[test]
    fn batch_commitment_is_stable() {
        let alice = seed_keypair(100);
        let bob = seed_keypair(101);
        let a = TxBuilder::new(clone_keypair(&alice)).transfer(bob.address(), 10);
        let b = TxBuilder::new(clone_keypair(&alice)).transfer(bob.address(), 20);
        let id1 = batch_commitment(&[a.clone(), b.clone()]);
        let id2 = batch_commitment(&[a.clone(), b.clone()]);
        // Deterministic: identical order -> identical commitment.
        assert_eq!(id1, id2);
        let id3 = batch_commitment(&[b, a]);
        // Order-sensitive by design (canonical serialization order).
        assert_ne!(id1, id3);
    }
}
