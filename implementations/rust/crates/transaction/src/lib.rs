//! Transaction model and validation (spec 05-transactions).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;
use veridag_capabilities::Capability;
use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder, MAX_SEQ};
use veridag_crypto::{hash, verify};
use veridag_protocol_types::{
    Address, ApplicationId, CapabilityId, ChainId, Ed25519Signature, Epoch, ObjectId, ObjectRef,
    Ownership, ProtocolVersion, ResourceBudget, TransactionId,
};

/// A transaction operation (variant index normative, spec 05).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Operation {
    /// Create a new object.
    CreateObject {
        /// object type
        object_type: u32,
        /// ownership mode
        ownership: Ownership,
        /// initial payload
        payload: Vec<u8>,
    },
    /// Update an object's payload.
    UpdateObject {
        /// target object
        object: ObjectRef,
        /// new payload
        new_payload: Vec<u8>,
    },
    /// Delete an object.
    DeleteObject {
        /// target object
        object: ObjectRef,
    },
    /// Transfer object ownership.
    TransferObject {
        /// target object
        object: ObjectRef,
        /// new owner
        new_owner: Ownership,
    },
    /// Transfer native balance value.
    TransferValue {
        /// sender's Balance object
        from: ObjectRef,
        /// recipient address
        to: Address,
        /// amount
        amount: u64,
    },
    /// Grant a capability.
    GrantCapability {
        /// the capability to grant
        capability: Box<Capability>,
    },
    /// Revoke a capability.
    RevokeCapability {
        /// capability to revoke
        capability_id: CapabilityId,
    },
    /// Invoke an application (post-v0.1 runtime).
    InvokeApplication {
        /// application id
        app: ApplicationId,
        /// input bytes
        input: Vec<u8>,
    },
}

/// A transaction (unsigned body).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Transaction {
    /// Protocol version.
    pub protocol_version: ProtocolVersion,
    /// Chain id.
    pub chain_id: ChainId,
    /// Sender address.
    pub sender: Address,
    /// Anti-replay nonce.
    pub nonce: u64,
    /// Expiry epoch.
    pub expiry_epoch: Epoch,
    /// Declared reads.
    pub declared_reads: Vec<ObjectRef>,
    /// Declared writes.
    pub declared_writes: Vec<ObjectRef>,
    /// Capabilities carried.
    pub capabilities: Vec<CapabilityId>,
    /// Operation.
    pub operation: Operation,
    /// Resource budget.
    pub resource_budget: ResourceBudget,
    /// Opaque metadata (<= 256 bytes).
    pub metadata: Vec<u8>,
}

/// A signed transaction (consensus-visible form).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SignedTransaction {
    /// The transaction body.
    pub tx: Transaction,
    /// Ed25519 signature over VERIDAG_TX_V1.
    pub signature: Ed25519Signature,
}

/// Transaction validation errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TxError {
    /// Codec error.
    #[error("codec: {0}")]
    Codec(#[from] DecodeError),
    /// Wrong protocol version.
    #[error("protocol version mismatch")]
    ProtocolVersion,
    /// Wrong chain id.
    #[error("chain id mismatch")]
    ChainId,
    /// Transaction expired.
    #[error("expired")]
    Expired,
    /// Bad signature.
    #[error("invalid signature")]
    InvalidSignature,
    /// Duplicate write object.
    #[error("duplicate write object")]
    DuplicateWrite,
    /// Too many declared dependencies.
    #[error("too many dependencies")]
    TooManyDeps,
}

impl SignedTransaction {
    /// Compute the canonical TransactionId.
    pub fn id(&self) -> TransactionId {
        TransactionId(hash("VERIDAG_TX_V1", &self.to_bytes()))
    }

    /// Verify the signature against the sender's key.
    ///
    /// `sender_pubkey` is the Ed25519 public key whose derived address equals
    /// `tx.sender`.
    pub fn verify_signature(
        &self,
        sender_pubkey: &veridag_protocol_types::Ed25519PublicKey,
    ) -> Result<(), TxError> {
        verify(
            sender_pubkey,
            "VERIDAG_TX_V1",
            &self.tx.to_bytes(),
            &self.signature,
        )
        .map_err(|_| TxError::InvalidSignature)
    }

    /// Cheap structural validation (before signature/state): version, chain,
    /// expiry, duplicate writes, dependency limits.
    pub fn check_structural(
        &self,
        active_protocol: ProtocolVersion,
        chain_id: ChainId,
        current_epoch: Epoch,
    ) -> Result<(), TxError> {
        if self.tx.protocol_version != active_protocol {
            return Err(TxError::ProtocolVersion);
        }
        if self.tx.chain_id != chain_id {
            return Err(TxError::ChainId);
        }
        if current_epoch > self.tx.expiry_epoch {
            return Err(TxError::Expired);
        }
        if self.tx.declared_reads.len() > MAX_SEQ || self.tx.declared_writes.len() > MAX_SEQ {
            return Err(TxError::TooManyDeps);
        }
        // no duplicate write object ids
        let mut seen = std::collections::BTreeSet::new();
        for w in &self.tx.declared_writes {
            if !seen.insert(w.id) {
                return Err(TxError::DuplicateWrite);
            }
        }
        Ok(())
    }
}

// --- VCE-1 encoding ---------------------------------------------------------

fn encode_ownership(e: &mut Encoder, o: &Ownership) {
    // Reuse the canonical Ownership encoding from object-state via a local copy
    // to avoid a circular dependency. Field order matches spec 06.
    match o {
        Ownership::Address(a) => {
            e.u8(0);
            e.fixed(a);
        }
        Ownership::Shared => e.u8(1),
        Ownership::Immutable => e.u8(2),
        Ownership::System => e.u8(3),
        Ownership::Capability(c) => {
            e.u8(4);
            e.fixed(c.as_bytes());
        }
        Ownership::Application(a) => {
            e.u8(5);
            e.fixed(a.as_bytes());
        }
    }
}

fn decode_ownership(d: &mut Decoder<'_>) -> Result<Ownership, DecodeError> {
    match d.u8()? {
        0 => Ok(Ownership::Address(d.fixed::<32>()?)),
        1 => Ok(Ownership::Shared),
        2 => Ok(Ownership::Immutable),
        3 => Ok(Ownership::System),
        4 => Ok(Ownership::Capability(CapabilityId(d.fixed::<32>()?))),
        5 => Ok(Ownership::Application(ApplicationId(d.fixed::<32>()?))),
        v => Err(DecodeError::UnknownVariant(v)),
    }
}

fn encode_objref(e: &mut Encoder, r: &ObjectRef) {
    e.fixed(r.id.as_bytes());
    e.u64(r.expected);
}

fn decode_objref(d: &mut Decoder<'_>) -> Result<ObjectRef, DecodeError> {
    Ok(ObjectRef {
        id: ObjectId(d.fixed::<32>()?),
        expected: d.u64()?,
    })
}

fn encode_budget(e: &mut Encoder, b: &ResourceBudget) {
    e.u64(b.compute);
    e.u64(b.memory);
    e.u64(b.storage);
    e.u64(b.bandwidth);
}

fn decode_budget(d: &mut Decoder<'_>) -> Result<ResourceBudget, DecodeError> {
    Ok(ResourceBudget {
        compute: d.u64()?,
        memory: d.u64()?,
        storage: d.u64()?,
        bandwidth: d.u64()?,
    })
}

impl Encode for Transaction {
    fn encode(&self, e: &mut Encoder) {
        e.u64(self.protocol_version);
        e.u64(self.chain_id);
        e.fixed(&self.sender);
        e.u64(self.nonce);
        e.u64(self.expiry_epoch);
        e.seq(&self.declared_reads, encode_objref);
        e.seq(&self.declared_writes, encode_objref);
        e.seq(&self.capabilities, |e, c| e.fixed(c.as_bytes()));
        // operation
        match &self.operation {
            Operation::CreateObject {
                object_type,
                ownership,
                payload,
            } => {
                e.u8(0);
                e.u32(*object_type);
                encode_ownership(e, ownership);
                e.bytes(payload);
            }
            Operation::UpdateObject {
                object,
                new_payload,
            } => {
                e.u8(1);
                encode_objref(e, object);
                e.bytes(new_payload);
            }
            Operation::DeleteObject { object } => {
                e.u8(2);
                encode_objref(e, object);
            }
            Operation::TransferObject { object, new_owner } => {
                e.u8(3);
                encode_objref(e, object);
                encode_ownership(e, new_owner);
            }
            Operation::TransferValue { from, to, amount } => {
                e.u8(4);
                encode_objref(e, from);
                e.fixed(to);
                e.u64(*amount);
            }
            Operation::GrantCapability { capability } => {
                e.u8(5);
                capability.encode(e);
            }
            Operation::RevokeCapability { capability_id } => {
                e.u8(6);
                e.fixed(capability_id.as_bytes());
            }
            Operation::InvokeApplication { app, input } => {
                e.u8(7);
                e.fixed(app.as_bytes());
                e.bytes(input);
            }
        }
        encode_budget(e, &self.resource_budget);
        e.bytes(&self.metadata);
    }
}

impl Decode for Transaction {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let protocol_version = d.u64()?;
        let chain_id = d.u64()?;
        let sender: Address = d.fixed::<32>()?;
        let nonce = d.u64()?;
        let expiry_epoch = d.u64()?;
        let declared_reads = d.seq(MAX_SEQ, decode_objref)?;
        let declared_writes = d.seq(MAX_SEQ, decode_objref)?;
        let capabilities = d.seq(MAX_SEQ, |dd| Ok(CapabilityId(dd.fixed::<32>()?)))?;
        let operation = match d.u8()? {
            0 => Operation::CreateObject {
                object_type: d.u32()?,
                ownership: decode_ownership(d)?,
                payload: d.bytes(veridag_codec::MAX_BYTES)?.to_vec(),
            },
            1 => Operation::UpdateObject {
                object: decode_objref(d)?,
                new_payload: d.bytes(veridag_codec::MAX_BYTES)?.to_vec(),
            },
            2 => Operation::DeleteObject {
                object: decode_objref(d)?,
            },
            3 => Operation::TransferObject {
                object: decode_objref(d)?,
                new_owner: decode_ownership(d)?,
            },
            4 => Operation::TransferValue {
                from: decode_objref(d)?,
                to: d.fixed::<32>()?,
                amount: d.u64()?,
            },
            5 => Operation::GrantCapability {
                capability: Box::new(Capability::decode(d)?),
            },
            6 => Operation::RevokeCapability {
                capability_id: CapabilityId(d.fixed::<32>()?),
            },
            7 => Operation::InvokeApplication {
                app: ApplicationId(d.fixed::<32>()?),
                input: d.bytes(veridag_codec::MAX_BYTES)?.to_vec(),
            },
            v => return Err(DecodeError::UnknownVariant(v)),
        };
        let resource_budget = decode_budget(d)?;
        let metadata = d.bytes(256)?.to_vec();
        Ok(Self {
            protocol_version,
            chain_id,
            sender,
            nonce,
            expiry_epoch,
            declared_reads,
            declared_writes,
            capabilities,
            operation,
            resource_budget,
            metadata,
        })
    }
}

impl Encode for SignedTransaction {
    fn encode(&self, e: &mut Encoder) {
        self.tx.encode(e);
        e.fixed(&self.signature);
    }
}

impl Decode for SignedTransaction {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let tx = Transaction::decode(d)?;
        let signature = d.fixed::<64>()?;
        Ok(Self { tx, signature })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_crypto::Keypair;
    use veridag_protocol_types::{object_type, CURRENT_PROTOCOL_VERSION};

    fn mk_tx(sender: Address, nonce: u64, op: Operation) -> Transaction {
        Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: 1,
            sender,
            nonce,
            expiry_epoch: 100,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: op,
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        }
    }

    #[test]
    fn signed_tx_roundtrip_and_id_stable() {
        let kp = Keypair::generate();
        let tx = mk_tx(
            kp.address(),
            0,
            Operation::CreateObject {
                object_type: object_type::BALANCE,
                ownership: Ownership::Address(kp.address()),
                payload: 100u64.to_be_bytes().to_vec(),
            },
        );
        let sig = kp.sign("VERIDAG_TX_V1", &tx.to_bytes());
        let stx = SignedTransaction { tx, signature: sig };
        let bytes = stx.to_bytes();
        let mut d = Decoder::new(&bytes);
        let out = SignedTransaction::decode(&mut d).unwrap();
        d.finish().unwrap();
        assert_eq!(stx, out);
        assert_eq!(stx.id(), out.id());
    }

    #[test]
    fn signature_verifies_and_wrong_key_fails() {
        let kp = Keypair::generate();
        let tx = mk_tx(
            kp.address(),
            0,
            Operation::DeleteObject {
                object: ObjectRef {
                    id: ObjectId::ZERO,
                    expected: 0,
                },
            },
        );
        let sig = kp.sign("VERIDAG_TX_V1", &tx.to_bytes());
        let stx = SignedTransaction { tx, signature: sig };
        assert!(stx.verify_signature(&kp.public()).is_ok());
        let other = Keypair::generate();
        assert_eq!(
            stx.verify_signature(&other.public()),
            Err(TxError::InvalidSignature)
        );
    }

    #[test]
    fn structural_checks() {
        let kp = Keypair::generate();
        let mut tx = mk_tx(
            kp.address(),
            0,
            Operation::DeleteObject {
                object: ObjectRef {
                    id: ObjectId::ZERO,
                    expected: 0,
                },
            },
        );
        let sig = kp.sign("VERIDAG_TX_V1", &tx.to_bytes());
        let stx = SignedTransaction {
            tx: tx.clone(),
            signature: sig,
        };
        assert!(stx.check_structural(CURRENT_PROTOCOL_VERSION, 1, 0).is_ok());
        // wrong protocol
        assert_eq!(
            stx.check_structural(999, 1, 0),
            Err(TxError::ProtocolVersion)
        );
        // wrong chain
        assert_eq!(
            stx.check_structural(CURRENT_PROTOCOL_VERSION, 2, 0),
            Err(TxError::ChainId)
        );
        // expired
        assert_eq!(
            stx.check_structural(CURRENT_PROTOCOL_VERSION, 1, 101),
            Err(TxError::Expired)
        );
        // duplicate writes
        tx.declared_writes = vec![
            ObjectRef {
                id: ObjectId::ZERO,
                expected: 0,
            },
            ObjectRef {
                id: ObjectId::ZERO,
                expected: 0,
            },
        ];
        let sig2 = kp.sign("VERIDAG_TX_V1", &tx.to_bytes());
        let stx2 = SignedTransaction {
            tx,
            signature: sig2,
        };
        assert_eq!(
            stx2.check_structural(CURRENT_PROTOCOL_VERSION, 1, 0),
            Err(TxError::DuplicateWrite)
        );
    }

    #[test]
    fn unknown_variant_rejected() {
        // operation variant 99 must be rejected
        let kp = Keypair::generate();
        let tx = mk_tx(
            kp.address(),
            0,
            Operation::DeleteObject {
                object: ObjectRef {
                    id: ObjectId::ZERO,
                    expected: 0,
                },
            },
        );
        let mut bytes = tx.to_bytes();
        // find the operation tag: it follows the capabilities seq (empty = 4 bytes)
        // Instead of hunting offsets, just decode a corrupted buffer:
        let _ = &mut bytes;
        let bad = {
            let mut e = Encoder::new();
            e.u64(1);
            e.u64(1);
            e.fixed(&[1u8; 32]);
            e.u64(0);
            e.u64(100);
            e.u32(0); // reads
            e.u32(0); // writes
            e.u32(0); // caps
            e.u8(99); // invalid op variant
            e.into_bytes()
        };
        let mut d = Decoder::new(&bad);
        assert!(matches!(
            Transaction::decode(&mut d),
            Err(DecodeError::UnknownVariant(99))
        ));
    }

    #[test]
    fn version_field_type_is_u64() {
        let r = ObjectRef {
            id: ObjectId::ZERO,
            expected: u64::MAX,
        };
        let mut e = Encoder::new();
        encode_objref(&mut e, &r);
        let bytes = e.into_bytes();
        let mut d = Decoder::new(&bytes);
        let out = decode_objref(&mut d).unwrap();
        assert_eq!(r, out);
    }
}
