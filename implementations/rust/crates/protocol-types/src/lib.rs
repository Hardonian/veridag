//! Canonical protocol types for Veridag (Level 3 realization of Level 1 spec).
//!
//! These types mirror `protocol/specification/02-identifiers.md`,
//! `05-transactions.md`, `06-object-model.md`, `07-capabilities.md`,
//! `08-dag.md`, and `13-checkpoints.md`. The Rust in-memory layout is NOT the
//! protocol; the protocol byte form is defined by VCE-1 (see `veridag-codec`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Cryptographic hash output (32 bytes).
pub type Hash = [u8; 32];

/// Address derived from an Ed25519 public key.
pub type Address = [u8; 32];

/// Ed25519 public key.
pub type Ed25519PublicKey = [u8; 32];

/// Ed25519 signature.
pub type Ed25519Signature = [u8; 64];

macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name(pub Hash);

        impl $name {
            /// All-zero id (useful as a sentinel in tests).
            pub const ZERO: Self = Self([0u8; 32]);

            /// Borrow the raw bytes.
            pub fn as_bytes(&self) -> &Hash {
                &self.0
            }
        }

        impl From<Hash> for $name {
            fn from(h: Hash) -> Self {
                Self(h)
            }
        }
    };
}

id_newtype!(/// Unique identifier of a transaction.
TransactionId);
id_newtype!(/// Unique identifier of an object.
ObjectId);
id_newtype!(/// Unique identifier of a capability.
CapabilityId);
id_newtype!(/// Unique identifier of a DAG vertex.
VertexId);
id_newtype!(/// Unique identifier of a checkpoint.
CheckpointId);
id_newtype!(/// Unique identifier of a transaction batch.
BatchId);
id_newtype!(/// Unique identifier of a validator.
ValidatorId);
id_newtype!(/// Unique identifier of an application.
ApplicationId);

/// Protocol version (current = 1).
pub type ProtocolVersion = u64;

/// Chain identifier fixed at genesis.
pub type ChainId = u64;

/// Epoch number.
pub type Epoch = u64;

/// DAG round number.
pub type Round = u64;

/// Checkpoint sequence number.
pub type CheckpointSequence = u64;

/// Object version; starts at 0, +1 per successful mutation.
pub type ObjectVersion = u64;

/// The active protocol version for this build.
pub const CURRENT_PROTOCOL_VERSION: ProtocolVersion = 1;

/// Ownership mode of an object (spec 06).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ownership {
    /// Controlled by a single address's key.
    Address(Address),
    /// Mutable by any transaction carrying a valid capability.
    Shared,
    /// Never mutated or deleted after creation.
    Immutable,
    /// Controlled by protocol-native logic only.
    System,
    /// Mutations require the named capability.
    Capability(CapabilityId),
    /// Mutations only via that application's execution.
    Application(ApplicationId),
}

/// A reference to an object at an expected version (anti-replay / conflicts).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectRef {
    /// The object being referenced.
    pub id: ObjectId,
    /// The version the transaction expects to consume.
    pub expected: ObjectVersion,
}

/// Abstract resource budget (no token price). Spec 37.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ResourceBudget {
    /// Compute units.
    pub compute: u64,
    /// Memory units.
    pub memory: u64,
    /// Storage units.
    pub storage: u64,
    /// Bandwidth units.
    pub bandwidth: u64,
}

/// Built-in object types (spec 06, 19).
pub mod object_type {
    /// Account object (nonce, capability refs).
    pub const ACCOUNT: u32 = 0;
    /// Balance object (payload = u64be amount).
    pub const BALANCE: u32 = 1;
    /// Capability object.
    pub const CAPABILITY: u32 = 2;
    /// Stablecoin account object (payload = VCE-1 StablecoinAccountPayload). Spec 19.
    pub const STABLECOIN: u32 = 3;
    /// Proof-of-Reserves attestation object. Spec 19.
    pub const RESERVE_ATTESTATION: u32 = 4;
}

// --- VCE-1 canonical encodings for protocol types ---------------------------
//
// These are the single canonical home for VCE-1 encodings of protocol-level
// types. Implementations elsewhere must reuse these, not re-derive them.

use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder};

impl Encode for Ownership {
    fn encode(&self, e: &mut Encoder) {
        match self {
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
}

impl Decode for Ownership {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
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
}

impl Encode for ObjectRef {
    fn encode(&self, e: &mut Encoder) {
        e.fixed(self.id.as_bytes());
        e.u64(self.expected);
    }
}

impl Decode for ObjectRef {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(ObjectRef {
            id: ObjectId(d.fixed::<32>()?),
            expected: d.u64()?,
        })
    }
}

impl Encode for ResourceBudget {
    fn encode(&self, e: &mut Encoder) {
        e.u64(self.compute);
        e.u64(self.memory);
        e.u64(self.storage);
        e.u64(self.bandwidth);
    }
}

impl Decode for ResourceBudget {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(ResourceBudget {
            compute: d.u64()?,
            memory: d.u64()?,
            storage: d.u64()?,
            bandwidth: d.u64()?,
        })
    }
}

/// Ethereum 20-byte address representation (spec 20).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct EthAddress(pub [u8; 20]);

impl EthAddress {
    /// All-zero address.
    pub const ZERO: Self = Self([0u8; 20]);

    /// Borrow the 20 raw bytes.
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl From<[u8; 20]> for EthAddress {
    fn from(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }
}

impl Encode for EthAddress {
    fn encode(&self, e: &mut Encoder) {
        e.fixed(&self.0);
    }
}

impl Decode for EthAddress {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self(d.fixed::<20>()?))
    }
}

/// Ethereum 32-byte transaction / block / log hash (spec 20).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct EthTxHash(pub [u8; 32]);

impl EthTxHash {
    /// All-zero hash.
    pub const ZERO: Self = Self([0u8; 32]);

    /// Borrow the 32 raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for EthTxHash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl Encode for EthTxHash {
    fn encode(&self, e: &mut Encoder) {
        e.fixed(&self.0);
    }
}

impl Decode for EthTxHash {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self(d.fixed::<32>()?))
    }
}
