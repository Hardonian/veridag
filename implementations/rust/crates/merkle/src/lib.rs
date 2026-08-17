//! BMH-1: Binary Merkle Hash-map commitment (spec 12-state).
//!
//! Deterministic authenticated commitment to a set of `(ObjectId -> Object)`
//! pairs. Leaves sorted by ObjectId bytewise; canonical encoding throughout.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;
use veridag_crypto::hash;
use veridag_protocol_types::{Hash, ObjectId};

/// Errors from proof verification.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MerkleError {
    /// Proof did not recompute to the expected root.
    #[error("invalid proof")]
    InvalidProof,
    /// Proof encoding was malformed.
    #[error("malformed proof")]
    MalformedProof,
}

/// A BMH-1 inclusion proof: sibling hashes from leaf to root, with a bit
/// indicating whether the proven node is the left (false) or right (true) child.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InclusionProof {
    /// Sibling hashes, leaf-first.
    pub siblings: Vec<Hash>,
    /// For each level, true if the proven node is the right child.
    pub right: Vec<bool>,
}

/// Compute the leaf hash of an object given its canonical encoding.
pub fn leaf_hash(object_bytes: &[u8]) -> Hash {
    hash("VERIDAG_BMH_LEAF_V1", object_bytes)
}

fn node_hash(l: &Hash, r: &Hash) -> Hash {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(l);
    buf[32..].copy_from_slice(r);
    hash("VERIDAG_BMH_NODE_V1", &buf)
}

/// The canonical empty-tree root.
pub fn empty_root() -> Hash {
    let mut buf = [0u8; 2];
    hash("VERIDAG_BMH_NODE_V1", &{
        buf[0] = 0;
        buf[1] = 0;
        buf
    })
}

/// Compute the BMH-1 root of a set of `(ObjectId, object_bytes)` leaves.
///
/// Leaves are sorted by ObjectId bytewise. Duplicate ids are rejected by the
/// caller before calling this function.
pub fn root(leaves: &[(ObjectId, Hash)]) -> Hash {
    if leaves.is_empty() {
        return empty_root();
    }
    let mut sorted: Vec<(ObjectId, Hash)> = leaves.to_vec();
    sorted.sort_by_key(|(id, _)| *id);
    let mut level: Vec<Hash> = sorted.into_iter().map(|(_, h)| h).collect();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            if i + 1 < level.len() {
                next.push(node_hash(&level[i], &level[i + 1]));
            } else {
                // odd leaf promoted unchanged
                next.push(level[i]);
            }
            i += 2;
        }
        level = next;
    }
    level[0]
}

/// Generate an inclusion proof for the object at `index` in the sorted leaf list.
pub fn prove(leaves: &[(ObjectId, Hash)], index: usize) -> Option<InclusionProof> {
    if leaves.is_empty() || index >= leaves.len() {
        return None;
    }
    let mut sorted: Vec<(ObjectId, Hash)> = leaves.to_vec();
    sorted.sort_by_key(|(id, _)| *id);
    let mut level: Vec<Hash> = sorted.iter().map(|(_, h)| *h).collect();
    let mut idx = index;
    let mut siblings = Vec::new();
    let mut right = Vec::new();
    while level.len() > 1 {
        let sibling_idx = if idx.is_multiple_of(2) {
            idx + 1
        } else {
            idx - 1
        };
        // If there is no sibling (odd node promoted), record the node's own hash
        // as sibling with a flag; verification handles promotion.
        if sibling_idx < level.len() {
            siblings.push(level[sibling_idx]);
            right.push(!idx.is_multiple_of(2));
        } else {
            siblings.push(level[idx]);
            right.push(false); // promoted: treat as left, sibling is self
        }
        idx /= 2;
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            if i + 1 < level.len() {
                next.push(node_hash(&level[i], &level[i + 1]));
            } else {
                next.push(level[i]);
            }
            i += 2;
        }
        level = next;
    }
    Some(InclusionProof { siblings, right })
}

/// Verify an inclusion proof for `leaf` against `expected_root`.
pub fn verify(
    leaf: &Hash,
    proof: &InclusionProof,
    expected_root: &Hash,
) -> Result<(), MerkleError> {
    if proof.siblings.len() != proof.right.len() {
        return Err(MerkleError::MalformedProof);
    }
    let mut acc = *leaf;
    for (sib, is_right) in proof.siblings.iter().zip(proof.right.iter()) {
        acc = if *is_right {
            node_hash(sib, &acc)
        } else if sib == &acc {
            // promotion level: node carried up unchanged
            acc
        } else {
            node_hash(&acc, sib)
        };
    }
    if &acc == expected_root {
        Ok(())
    } else {
        Err(MerkleError::InvalidProof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(b: u8) -> ObjectId {
        ObjectId([b; 32])
    }
    fn h(b: u8) -> Hash {
        hash("VERIDAG_BMH_LEAF_V1", &[b])
    }

    #[test]
    fn empty_tree_root_is_deterministic() {
        assert_eq!(root(&[]), empty_root());
    }

    #[test]
    fn root_is_order_independent_for_input() {
        let l1 = vec![(id(1), h(1)), (id(2), h(2))];
        let l2 = vec![(id(2), h(2)), (id(1), h(1))];
        assert_eq!(root(&l1), root(&l2), "root sorts leaves canonically");
    }

    #[test]
    fn root_changes_with_leaves() {
        let a = root(&[(id(1), h(1))]);
        let b = root(&[(id(1), h(9))]);
        assert_ne!(a, b);
    }

    #[test]
    fn inclusion_proof_verifies() {
        let leaves: Vec<(ObjectId, Hash)> = (0u8..7).map(|b| (id(b), h(b))).collect();
        let r = root(&leaves);
        for i in 0..leaves.len() {
            let proof = prove(&leaves, i).unwrap();
            let mut sorted = leaves.clone();
            sorted.sort_by_key(|(id, _)| *id);
            assert!(
                verify(&sorted[i].1, &proof, &r).is_ok(),
                "proof {i} must verify"
            );
        }
    }

    #[test]
    fn bad_proof_rejected() {
        let leaves: Vec<(ObjectId, Hash)> = (0u8..4).map(|b| (id(b), h(b))).collect();
        let r = root(&leaves);
        let proof = prove(&leaves, 0).unwrap();
        assert_eq!(verify(&h(99), &proof, &r), Err(MerkleError::InvalidProof));
    }
}
