//! Veridag advanced data availability — erasure-coded (Reed-Solomon) blob
//! dispersal and reconstruction over GF(2^8).
//!
//! Goal: ensure a committed batch's bytes remain reconstructable even if up to
//! `m` of `n = k + m` dispersing nodes withhold their shares. Any `k` shares
//! suffice to rebuild the original blob, and reconstructed shares are
//! content-addressed (hash-bound) so a malicious share is detected.
//!
//! The implementation is a self-contained systematic Reed-Solomon code over
//! GF(2^8) with the standard 0x11d reduction polynomial. No external crypto or
//! linear-algebra dependencies — it is small, deterministic, and auditable.

#![forbid(unsafe_code)]
// Low-level GF(2^8) arithmetic and matrix loops are clearest with explicit
// indexing; these pedantic lints would only obscure the numerical code.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_memcpy)]

use veridag_crypto::hash;
use veridag_protocol_types::Hash;

// ---------------------------------------------------------------------------
// GF(2^8) with reduction polynomial 0x11d (the AES/Reed-Solomon standard).
// ---------------------------------------------------------------------------

/// Multiply a GF(2^8) element by the generator 2 (i.e. by x).
#[inline]
fn gf_mul2(v: u8) -> u8 {
    let mut r = v << 1;
    if v & 0x80 != 0 {
        r ^= 0x1d; // low byte of the 0x11d reduction polynomial
    }
    r
}

fn build_log_tables() -> ([u8; 256], [u8; 256]) {
    let mut log = [0u8; 256];
    let mut exp = [0u8; 256];
    let mut x: u8 = 1;
    for i in 0..255u16 {
        exp[i as usize] = x;
        log[x as usize] = i as u8;
        x = gf_mul2(x);
    }
    // exp[255] == exp[0] == 1 (generator has order 255).
    exp[255] = exp[0];
    (log, exp)
}

static GF_TABLES: std::sync::OnceLock<([u8; 256], [u8; 256])> = std::sync::OnceLock::new();

fn tables() -> &'static ([u8; 256], [u8; 256]) {
    GF_TABLES.get_or_init(build_log_tables)
}

#[inline]
fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let (log, exp) = tables();
    let s = log[a as usize] as u16 + log[b as usize] as u16;
    exp[(s % 255) as usize]
}

#[inline]
fn gf_div(a: u8, b: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    let (log, exp) = tables();
    let s = (log[a as usize] as i16 - log[b as usize] as i16).rem_euclid(255) as u16;
    exp[s as usize]
}

/// GF(2^8) exponentiation: `2^exp` via the log/exp tables.
#[inline]
fn gf_pow(mut base: u8, mut exp: u16) -> u8 {
    let mut result: u8 = 1;
    while exp > 0 {
        if exp & 1 != 0 {
            result = gf_mul(result, base);
        }
        base = gf_mul(base, base);
        exp >>= 1;
    }
    result
}

/// The `share_index`-th encoding row of the systematic Vandermonde matrix:
/// `V[share_index][j] = 2^(share_index * j)` for `j in 0..k`.
///
/// Data shards use `share_index in 0..k` (identity columns), parity shards use
/// `share_index in k..n`. Any `k` rows form an invertible `k×k` submatrix, which
/// is what makes reconstruction from any `k` of `n` shares possible.
fn vandermonde_row(share_index: u8, k: usize) -> Vec<u8> {
    (0..k)
        .map(|j| gf_pow(2, (share_index as u16) * (j as u16)))
        .collect()
}

// ---------------------------------------------------------------------------
// Public DA API
// ---------------------------------------------------------------------------

/// DA configuration: `k` data shards, `m` parity shards (total `n = k + m`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaConfig {
    pub data_shards: usize,
    pub parity_shards: usize,
}

impl DaConfig {
    pub fn new(data_shards: usize, parity_shards: usize) -> Self {
        assert!(data_shards >= 1, "need at least one data shard");
        assert!(parity_shards >= 1, "need at least one parity shard");
        assert!(
            data_shards + parity_shards <= 256,
            "too many shards for GF(2^8)"
        );
        Self {
            data_shards,
            parity_shards,
        }
    }

    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }
}

/// A dispersed blob: the content hash plus `n` shares. Shares are independent
/// byte vectors of equal length; the original blob is padded to a multiple of
/// `data_shards`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dispersal {
    /// Content hash of the original blob (the availability commitment).
    pub content_hash: Hash,
    /// Original (unpadded) blob length, so reconstruction can strip padding.
    pub original_len: usize,
    /// All `n` shares (data shards first, then parity shards).
    pub shares: Vec<Vec<u8>>,
}

/// DA errors.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum DaError {
    #[error("blob too large: {0} bytes (max {1})")]
    TooLarge(usize, usize),
    #[error("not enough shares to reconstruct: have {0}, need {1}")]
    NotEnoughShares(usize, usize),
    #[error("share length mismatch")]
    ShareLength,
    #[error("reconstructed content hash does not match the commitment")]
    HashMismatch,
}

/// Maximum blob size (kept small for edge/low-energy deployments; raise via config).
pub const MAX_BLOB: usize = 4 * 1024 * 1024;

/// Erasure-code `blob` into `n = k + m` shares. Returns the commitment + shares.
pub fn encode(config: DaConfig, blob: &[u8]) -> Result<Dispersal, DaError> {
    if blob.len() > MAX_BLOB {
        return Err(DaError::TooLarge(blob.len(), MAX_BLOB));
    }
    let k = config.data_shards;

    // Pad to a multiple of k so each data shard has equal length.
    let shard_len = blob.len().div_ceil(k).max(1);
    let padded_len = shard_len * k;
    let mut padded = blob.to_vec();
    padded.resize(padded_len, 0);

    // Build k data shards (column-major: shard j holds byte i*k + j).
    let mut data_shards: Vec<Vec<u8>> = vec![vec![0u8; shard_len]; k];
    for (off, &byte) in padded.iter().enumerate() {
        let shard = off % k;
        let row = off / k;
        data_shards[shard][row] = byte;
    }

    // Encode ALL n shares through the Vandermonde matrix V[s][j] = 2^(s*j):
    //   shares[s][row] = Σ_j data_shards[j][row] * V[s][j]
    // This keeps encode and reconstruct consistent (any k rows invert).
    let n = config.total_shards();
    let mut shares = Vec::with_capacity(n);
    for s in 0..n {
        let coeffs = vandermonde_row(s as u8, k);
        let mut shard = vec![0u8; shard_len];
        for row in 0..shard_len {
            let mut acc: u8 = 0;
            for j in 0..k {
                acc ^= gf_mul(data_shards[j][row], coeffs[j]);
            }
            shard[row] = acc;
        }
        shares.push(shard);
    }

    let content_hash = hash("VERIDAG_DA_BLOB_V1", blob);
    Ok(Dispersal {
        content_hash,
        original_len: blob.len(),
        shares,
    })
}

/// Reconstruct the original blob from any `k` of the `n` shares.
///
/// `present` is a list of `(share_index, share_bytes)` for the shares the
/// caller actually has. Requires `>= k` entries.
pub fn reconstruct(
    config: DaConfig,
    content_hash: &Hash,
    original_len: usize,
    present: &[(usize, Vec<u8>)],
) -> Result<Vec<u8>, DaError> {
    let k = config.data_shards;
    if present.len() < k {
        return Err(DaError::NotEnoughShares(present.len(), k));
    }
    let shard_len = present[0].1.len();
    if present.iter().any(|(_, s)| s.len() != shard_len) {
        return Err(DaError::ShareLength);
    }

    // Use the first k present shares to solve the linear system over GF(2^8).
    let chosen: Vec<(usize, &[u8])> = present
        .iter()
        .take(k)
        .map(|(i, s)| (*i, s.as_slice()))
        .collect();

    // Build the k×k interpolation matrix A where A[r][c] = basis(coeff_of_share c at point r).
    // We need to invert the Vandermonde-like evaluation so we can recover data shards.
    let mut mat = vec![vec![0u8; k]; k];
    for r in 0..k {
        let coeffs = vandermonde_row(chosen[r].0 as u8, k);
        for c in 0..k {
            mat[r][c] = coeffs[c];
        }
    }
    let inv = invert_matrix(&mat)?;

    // Recover each data shard column: data[c] = Σ_r inv[c][r] * present_share[r].
    let mut data_shards: Vec<Vec<u8>> = vec![vec![0u8; shard_len]; k];
    for c in 0..k {
        for row in 0..shard_len {
            let mut acc: u8 = 0;
            for r in 0..k {
                acc ^= gf_mul(inv[c][r], chosen[r].1[row]);
            }
            data_shards[c][row] = acc;
        }
    }

    // Reassemble the blob column-major.
    let mut blob = Vec::with_capacity(shard_len * k);
    for row in 0..shard_len {
        for c in 0..k {
            blob.push(data_shards[c][row]);
        }
    }

    let got = hash("VERIDAG_DA_BLOB_V1", &blob[..original_len]);
    if &got != content_hash {
        return Err(DaError::HashMismatch);
    }
    blob.truncate(original_len);
    Ok(blob)
}

/// Invert an `n×n` matrix over GF(2^8) via Gaussian elimination.
fn invert_matrix(m: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, DaError> {
    let n = m.len();
    let mut a = m.to_vec();
    let mut inv = vec![vec![0u8; n]; n];
    for i in 0..n {
        inv[i][i] = 1;
    }
    for col in 0..n {
        // Find pivot.
        let mut pivot = None;
        for r in col..n {
            if a[r][col] != 0 {
                pivot = Some(r);
                break;
            }
        }
        let pr = pivot.ok_or(DaError::ShareLength)?;
        a.swap(col, pr);
        inv.swap(col, pr);
        let pv = a[col][col];
        let inv_pv = gf_div(1, pv);
        for c in 0..n {
            a[col][c] = gf_mul(a[col][c], inv_pv);
            inv[col][c] = gf_mul(inv[col][c], inv_pv);
        }
        for r in 0..n {
            if r != col && a[r][col] != 0 {
                let factor = a[r][col];
                for c in 0..n {
                    a[r][c] ^= gf_mul(factor, a[col][c]);
                    inv[r][c] ^= gf_mul(factor, inv[col][c]);
                }
            }
        }
    }
    Ok(inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf_basics() {
        // x * 1 == x, x * 0 == 0
        assert_eq!(gf_mul(5, 1), 5);
        assert_eq!(gf_mul(5, 0), 0);
        // division is the inverse of multiplication
        assert_eq!(gf_div(gf_mul(7, 9), 9), 7);
        assert_eq!(gf_div(gf_mul(0x53, 0xca), 0xca), 0x53);
    }

    #[test]
    fn roundtrip_full_shares() {
        let cfg = DaConfig::new(4, 2);
        let blob = b"veridag advanced data availability layer - reconstruct me".to_vec();
        let d = encode(cfg, &blob).unwrap();
        assert_eq!(d.shares.len(), 6);
        // Reconstruct from all data shares.
        let present: Vec<(usize, Vec<u8>)> = (0..4).map(|i| (i, d.shares[i].clone())).collect();
        let out = reconstruct(cfg, &d.content_hash, d.original_len, &present).unwrap();
        assert_eq!(out, blob);
    }

    #[test]
    fn reconstruct_from_parity_only() {
        let cfg = DaConfig::new(4, 4);
        let blob = (0..1000u16)
            .map(|i| (i as u8).wrapping_mul(7))
            .collect::<Vec<u8>>();
        let d = encode(cfg, &blob).unwrap();
        // Drop ALL data shards; use only the 4 parity shares.
        let present: Vec<(usize, Vec<u8>)> = (4..8).map(|i| (i, d.shares[i].clone())).collect();
        let out = reconstruct(cfg, &d.content_hash, d.original_len, &present).unwrap();
        assert_eq!(out, blob, "must reconstruct from parity-only shares");
    }

    #[test]
    fn reconstruct_with_arbitrary_k_subset() {
        let cfg = DaConfig::new(5, 3);
        let blob = b"edge deployment resilience test payload ".repeat(37);
        let d = encode(cfg, &blob).unwrap();
        // Pick a scrambled subset of 5 shares (indices 0,2,4,5,7).
        let idx = [0usize, 2, 4, 5, 7];
        let present: Vec<(usize, Vec<u8>)> =
            idx.iter().map(|&i| (i, d.shares[i].clone())).collect();
        let out = reconstruct(cfg, &d.content_hash, d.original_len, &present).unwrap();
        assert_eq!(out, blob);
    }

    #[test]
    fn tampered_share_detected() {
        let cfg = DaConfig::new(4, 2);
        let blob = b"integrity checked availability".to_vec();
        let d = encode(cfg, &blob).unwrap();
        let mut present: Vec<(usize, Vec<u8>)> = (0..4).map(|i| (i, d.shares[i].clone())).collect();
        // Flip a byte in one share.
        present[1].1[0] ^= 0xff;
        let res = reconstruct(cfg, &d.content_hash, d.original_len, &present);
        assert!(matches!(res, Err(DaError::HashMismatch)));
    }

    #[test]
    fn too_few_shares_errors() {
        let cfg = DaConfig::new(4, 2);
        let blob = b"need k shares".to_vec();
        let d = encode(cfg, &blob).unwrap();
        let present: Vec<(usize, Vec<u8>)> = (0..3).map(|i| (i, d.shares[i].clone())).collect();
        assert!(matches!(
            reconstruct(cfg, &d.content_hash, d.original_len, &present),
            Err(DaError::NotEnoughShares(3, 4))
        ));
    }
}
