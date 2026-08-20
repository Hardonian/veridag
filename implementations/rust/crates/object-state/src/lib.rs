//! Object-centric state (spec 06-object-model) with BMH-1 commitments.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use thiserror::Error;
use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder, MAX_BYTES};
use veridag_crypto::hash;
use veridag_protocol_types::{
    object_type, Address, Hash, ObjectId, ObjectRef, ObjectVersion, Ownership,
};

/// An object in persistent state.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Object {
    /// Unique id.
    pub id: ObjectId,
    /// Version; starts 0, +1 per mutation.
    pub version: ObjectVersion,
    /// Object type (built-in or application-defined).
    pub object_type: u32,
    /// Ownership mode.
    pub owner: Ownership,
    /// Committed payload hash.
    pub payload_commit: Hash,
    /// Payload bytes.
    pub payload: Vec<u8>,
    /// Opaque metadata (<= 256 bytes).
    pub metadata: Vec<u8>,
}

/// State errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StateError {
    /// Object not found.
    #[error("object not found")]
    NotFound,
    /// Expected version did not match current version.
    #[error("version conflict")]
    VersionConflict,
    /// Operation not permitted on this ownership mode.
    #[error("immutable object")]
    Immutable,
    /// Duplicate object id.
    #[error("duplicate object id")]
    Duplicate,
}

impl Object {
    /// Create a new object at version 0, computing its payload commitment.
    pub fn new(
        id: ObjectId,
        object_type: u32,
        owner: Ownership,
        payload: Vec<u8>,
        metadata: Vec<u8>,
    ) -> Self {
        let payload_commit = hash("VERIDAG_OBJECT_PAYLOAD_V1", &payload);
        Self {
            id,
            version: 0,
            object_type,
            owner,
            payload_commit,
            payload,
            metadata,
        }
    }

    /// Derive a deterministic ObjectId for a creator+nonce (spec 02/06).
    pub fn derive_id(creator: &Address, nonce: u64) -> ObjectId {
        let mut buf = Vec::with_capacity(40);
        buf.extend_from_slice(creator);
        buf.extend_from_slice(&nonce.to_be_bytes());
        ObjectId(hash("VERIDAG_OBJECT_ID_V1", &buf))
    }
}

/// The object state: a BTreeMap for canonical (sorted) iteration.
#[derive(Clone, Default, Debug)]
pub struct ObjectState {
    objects: BTreeMap<ObjectId, Object>,
}

impl ObjectState {
    /// Create an empty state.
    pub fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
        }
    }

    /// Get an object.
    pub fn get(&self, id: &ObjectId) -> Option<&Object> {
        self.objects.get(id)
    }

    /// Insert a new object; fails on duplicate id.
    pub fn create(&mut self, obj: Object) -> Result<(), StateError> {
        if self.objects.contains_key(&obj.id) {
            return Err(StateError::Duplicate);
        }
        self.objects.insert(obj.id, obj);
        Ok(())
    }

    /// Unconditionally insert or replace an object, taking the object's own
    /// version and payload-commit as authoritative.
    ///
    /// This is the deterministic merge primitive for speculative parallel
    /// execution: a transaction that executed on a snapshot has already
    /// advanced the object's version and recomputed its payload commitment, so
    /// merging its write-set must store the object verbatim, not re-drive
    /// `mutate` (which would double-hash and bump the version again). It is
    /// only correct for objects whose write domain was exclusive during the
    /// speculative window (the scheduler guarantees this); using it for shared
    /// objects would break version discipline.
    pub fn upsert_verbatim(&mut self, obj: Object) {
        self.objects.insert(obj.id, obj);
    }

    /// Check that an ObjectRef's expected version matches the current version.
    pub fn check_version(&self, r: &ObjectRef) -> Result<(), StateError> {
        let obj = self.objects.get(&r.id).ok_or(StateError::NotFound)?;
        if obj.version != r.expected {
            return Err(StateError::VersionConflict);
        }
        Ok(())
    }

    /// Mutate an object, enforcing version discipline and immutability.
    pub fn mutate(
        &mut self,
        r: &ObjectRef,
        f: impl FnOnce(&mut Object),
    ) -> Result<ObjectVersion, StateError> {
        let obj = self.objects.get_mut(&r.id).ok_or(StateError::NotFound)?;
        if obj.owner == Ownership::Immutable {
            return Err(StateError::Immutable);
        }
        if obj.version != r.expected {
            return Err(StateError::VersionConflict);
        }
        f(obj);
        obj.version += 1;
        obj.payload_commit = hash("VERIDAG_OBJECT_PAYLOAD_V1", &obj.payload);
        Ok(obj.version)
    }

    /// Delete an object (not Immutable), enforcing version discipline.
    pub fn delete(&mut self, r: &ObjectRef) -> Result<(), StateError> {
        {
            let obj = self.objects.get(&r.id).ok_or(StateError::NotFound)?;
            if obj.owner == Ownership::Immutable {
                return Err(StateError::Immutable);
            }
            if obj.version != r.expected {
                return Err(StateError::VersionConflict);
            }
        }
        self.objects.remove(&r.id);
        Ok(())
    }

    /// Iterate objects in canonical (ObjectId) order.
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectId, &Object)> {
        self.objects.iter()
    }

    /// Compute the BMH-1 state root.
    pub fn state_root(&self) -> Hash {
        let leaves: Vec<(ObjectId, Hash)> = self
            .objects
            .iter()
            .map(|(id, obj)| (*id, veridag_merkle::leaf_hash(id, &obj.to_bytes())))
            .collect();
        veridag_merkle::root(&leaves)
    }

    /// Read the u64 value of a Balance object.
    pub fn balance(&self, id: &ObjectId) -> Result<u64, StateError> {
        let obj = self.objects.get(id).ok_or(StateError::NotFound)?;
        if obj.object_type != object_type::BALANCE || obj.payload.len() != 8 {
            return Err(StateError::NotFound);
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(&obj.payload);
        Ok(u64::from_be_bytes(a))
    }
}

// --- VCE-1 canonical encoding for Object -------------------------------------
// (Ownership/ObjectRef/ResourceBudget encodings live in veridag-protocol-types;
//  reuse them to keep one canonical definition.)

impl Encode for Object {
    fn encode(&self, e: &mut Encoder) {
        e.fixed(self.id.as_bytes());
        e.u64(self.version);
        e.u32(self.object_type);
        self.owner.encode(e);
        e.fixed(&self.payload_commit);
        e.bytes(&self.payload);
        e.bytes(&self.metadata);
    }
}

impl Decode for Object {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let id = ObjectId(d.fixed::<32>()?);
        let version = d.u64()?;
        let object_type = d.u32()?;
        let owner = Ownership::decode(d)?;
        let payload_commit = d.fixed::<32>()?;
        let payload = d.bytes(MAX_BYTES)?.to_vec();
        let metadata = d.bytes(256)?.to_vec();
        Ok(Self {
            id,
            version,
            object_type,
            owner,
            payload_commit,
            payload,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_protocol_types::object_type;

    fn addr(b: u8) -> Address {
        [b; 32]
    }

    fn make_balance(creator: &Address, nonce: u64, amount: u64) -> Object {
        Object::new(
            Object::derive_id(creator, nonce),
            object_type::BALANCE,
            Ownership::Address(*creator),
            amount.to_be_bytes().to_vec(),
            vec![],
        )
    }

    #[test]
    fn create_and_read_balance() {
        let mut s = ObjectState::new();
        let a = addr(1);
        let obj = make_balance(&a, 0, 100);
        let id = obj.id;
        s.create(obj).unwrap();
        assert_eq!(s.balance(&id).unwrap(), 100);
    }

    #[test]
    fn version_conflict_rejected() {
        let mut s = ObjectState::new();
        let a = addr(1);
        let obj = make_balance(&a, 0, 100);
        let id = obj.id;
        s.create(obj).unwrap();
        // expected version 5 but actual is 0
        let r = ObjectRef { id, expected: 5 };
        assert_eq!(
            s.mutate(&r, |o| o.payload = 1u64.to_be_bytes().to_vec()),
            Err(StateError::VersionConflict)
        );
    }

    #[test]
    fn mutation_bumps_version() {
        let mut s = ObjectState::new();
        let a = addr(1);
        let obj = make_balance(&a, 0, 100);
        let id = obj.id;
        s.create(obj).unwrap();
        let r = ObjectRef { id, expected: 0 };
        let v = s
            .mutate(&r, |o| o.payload = 50u64.to_be_bytes().to_vec())
            .unwrap();
        assert_eq!(v, 1);
        assert_eq!(s.balance(&id).unwrap(), 50);
    }

    #[test]
    fn immutable_cannot_be_mutated() {
        let mut s = ObjectState::new();
        let a = addr(1);
        let mut obj = make_balance(&a, 0, 100);
        obj.owner = Ownership::Immutable;
        let id = obj.id;
        s.create(obj).unwrap();
        let r = ObjectRef { id, expected: 0 };
        assert_eq!(
            s.mutate(&r, |o| o.payload = 1u64.to_be_bytes().to_vec()),
            Err(StateError::Immutable)
        );
    }

    #[test]
    fn state_root_deterministic_and_changes() {
        let mut s = ObjectState::new();
        let a = addr(1);
        let b = addr(2);
        let r0 = s.state_root();
        s.create(make_balance(&a, 0, 100)).unwrap();
        s.create(make_balance(&b, 0, 25)).unwrap();
        let r1 = s.state_root();
        assert_ne!(r0, r1);
        // determinism: rebuild identical state -> identical root
        let mut s2 = ObjectState::new();
        s2.create(make_balance(&a, 0, 100)).unwrap();
        s2.create(make_balance(&b, 0, 25)).unwrap();
        assert_eq!(r1, s2.state_root());
    }

    #[test]
    fn canonical_encoding_roundtrip() {
        let a = addr(3);
        let obj = make_balance(&a, 7, 12345);
        let bytes = obj.to_bytes();
        let mut d = Decoder::new(&bytes);
        let out = Object::decode(&mut d).unwrap();
        d.finish().unwrap();
        assert_eq!(obj, out);
    }
}
