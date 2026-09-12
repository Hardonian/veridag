//! Veridag advanced data availability — erasure-coded (Reed-Solomon) blob
//! dispersal and reconstruction over GF(2^8), supporting both 1D and 2D schemes.
//!
//! Goal: ensure a committed batch's bytes remain reconstructable even if up to
//! `m` of `n = k + m` dispersing nodes withhold their shares. Any `k` shares
//! suffice to rebuild the original blob, and reconstructed shares are
//! content-addressed (hash-bound) so a malicious share is detected.
//!
//! In Phase 16, 2D Reed-Solomon matrix dispersal is introduced:
//! - Data is partitioned into a `D_rows × D_cols` matrix.
//! - Extended into an `N_rows × N_cols` tensor grid with row and column parity.
//! - Independent row and column commitments yield a compact 2D DA root.
//! - Iterative alternating erasure decoding allows recovering data even from
//!   arbitrary scattered shard withholdings.
//! - Deterministic validator shard assignment for distributed replication.
//!
//! The implementation is a self-contained systematic Reed-Solomon code over
//! GF(2^8) with the standard 0x11d reduction polynomial. No external crypto or
//! linear-algebra dependencies — it is small, deterministic, and auditable.

#![forbid(unsafe_code)]
// Low-level GF(2^8) arithmetic and matrix loops are clearest with explicit
// indexing; these pedantic lints would only obscure the numerical code.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_memcpy)]

pub mod hw_accel;
pub use hw_accel::HwAccelEngine;

use std::collections::HashMap;
use veridag_crypto::hash;
use veridag_protocol_types::{Hash, ValidatorId};

#[cfg(feature = "metrics")]
use veridag_metrics::{Label, Observation};

#[cfg(feature = "metrics")]
mod metrics_handle {
    use std::sync::OnceLock;
    use veridag_metrics::Metrics;

    static BACKEND: OnceLock<&'static dyn Metrics> = OnceLock::new();

    pub fn set_backend(m: &'static dyn Metrics) {
        BACKEND.set(m).ok();
    }

    pub fn backend() -> Option<&'static dyn Metrics> {
        BACKEND.get().copied()
    }
}

/// Install the global metrics backend for the DA crate. Only available with
/// the `metrics` feature.
#[cfg(feature = "metrics")]
pub fn set_metrics_backend(m: &'static dyn veridag_metrics::Metrics) {
    metrics_handle::set_backend(m);
}

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
pub(crate) fn gf_mul(a: u8, b: u8) -> u8 {
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
fn vandermonde_row(share_index: u8, k: usize) -> Vec<u8> {
    (0..k)
        .map(|j| gf_pow(2, (share_index as u16) * (j as u16)))
        .collect()
}

// ---------------------------------------------------------------------------
// 1D Public DA API
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
    #[error("2D reconstruction failed: could not recover all data shards")]
    InsufficientShards2D,
}

/// Maximum blob size (kept small for edge/low-energy deployments; raise via config).
pub const MAX_BLOB: usize = 4 * 1024 * 1024;

/// Erasure-code `blob` into `n = k + m` shares. Returns the commitment + shares.
pub fn encode(config: DaConfig, blob: &[u8]) -> Result<Dispersal, DaError> {
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();

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
    #[cfg(feature = "metrics")]
    {
        if let Some(m) = metrics_handle::backend() {
            m.observe(Observation::Duration(
                Label("da_encode"),
                start.elapsed().as_nanos() as u64,
            ));
        }
    }
    Ok(Dispersal {
        content_hash,
        original_len: blob.len(),
        shares,
    })
}

/// Helper: recover data shard columns from any `k` valid shares.
fn reconstruct_data_shards(
    config: DaConfig,
    shard_len: usize,
    present: &[(usize, Vec<u8>)],
) -> Result<Vec<Vec<u8>>, DaError> {
    let k = config.data_shards;
    if present.len() < k {
        return Err(DaError::NotEnoughShares(present.len(), k));
    }
    let chosen: Vec<(usize, &[u8])> = present
        .iter()
        .take(k)
        .map(|(i, s)| (*i, s.as_slice()))
        .collect();

    let mut mat = vec![vec![0u8; k]; k];
    for r in 0..k {
        let coeffs = vandermonde_row(chosen[r].0 as u8, k);
        for c in 0..k {
            mat[r][c] = coeffs[c];
        }
    }
    let inv = invert_matrix(&mat)?;

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
    Ok(data_shards)
}

/// Reconstruct the original blob from any `k` of the `n` shares.
pub fn reconstruct(
    config: DaConfig,
    content_hash: &Hash,
    original_len: usize,
    present: &[(usize, Vec<u8>)],
) -> Result<Vec<u8>, DaError> {
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();

    let k = config.data_shards;
    if present.len() < k {
        return Err(DaError::NotEnoughShares(present.len(), k));
    }
    let shard_len = present[0].1.len();
    if present.iter().any(|(_, s)| s.len() != shard_len) {
        return Err(DaError::ShareLength);
    }

    let data_shards = reconstruct_data_shards(config, shard_len, present)?;

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
    #[cfg(feature = "metrics")]
    {
        if let Some(m) = metrics_handle::backend() {
            m.observe(Observation::Duration(
                Label("da_reconstruct"),
                start.elapsed().as_nanos() as u64,
            ));
        }
    }
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

// ---------------------------------------------------------------------------
// 2D Advanced DA API (Phase 16)
// ---------------------------------------------------------------------------

/// Configuration for 2D Reed-Solomon data availability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Da2DConfig {
    pub data_rows: usize,
    pub data_cols: usize,
    pub parity_rows: usize,
    pub parity_cols: usize,
}

impl Da2DConfig {
    pub fn new(data_rows: usize, data_cols: usize, parity_rows: usize, parity_cols: usize) -> Self {
        assert!(data_rows >= 1 && data_cols >= 1, "dimensions must be >= 1");
        assert!(
            parity_rows >= 1 && parity_cols >= 1,
            "parity dimensions must be >= 1"
        );
        assert!(
            data_rows + parity_rows <= 256 && data_cols + parity_cols <= 256,
            "exceeds GF(2^8)"
        );
        Self {
            data_rows,
            data_cols,
            parity_rows,
            parity_cols,
        }
    }

    pub fn total_rows(&self) -> usize {
        self.data_rows + self.parity_rows
    }

    pub fn total_cols(&self) -> usize {
        self.data_cols + self.parity_cols
    }

    pub fn total_shards(&self) -> usize {
        self.total_rows() * self.total_cols()
    }
}

/// A 2D dispersed blob with matrix shards, row commitments, column commitments, and 2D root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dispersal2D {
    pub content_hash: Hash,
    pub original_len: usize,
    /// 2D matrix of shards: `grid[row][col]` where row < total_rows and col < total_cols.
    pub grid: Vec<Vec<Vec<u8>>>,
    pub row_commitments: Vec<Hash>,
    pub col_commitments: Vec<Hash>,
    pub da_root: Hash,
}

/// Disperse a blob into a 2D Reed-Solomon matrix.
pub fn encode_2d(config: Da2DConfig, blob: &[u8]) -> Result<Dispersal2D, DaError> {
    if blob.len() > MAX_BLOB {
        return Err(DaError::TooLarge(blob.len(), MAX_BLOB));
    }
    let d_rows = config.data_rows;
    let d_cols = config.data_cols;
    let t_rows = config.total_rows();
    let t_cols = config.total_cols();

    let total_data_shards = d_rows * d_cols;
    let shard_len = blob.len().div_ceil(total_data_shards).max(1);
    let padded_len = shard_len * total_data_shards;
    let mut padded = blob.to_vec();
    padded.resize(padded_len, 0);

    // 1. Split raw data into D_rows x D_cols data chunks
    let mut data_chunks = vec![vec![vec![0u8; shard_len]; d_cols]; d_rows];
    for (idx, chunk) in padded.chunks_exact(shard_len).enumerate() {
        let r = idx / d_cols;
        let c = idx % d_cols;
        data_chunks[r][c] = chunk.to_vec();
    }

    // 2. Encode each data row: D_cols -> T_cols
    let mut row_encoded = vec![vec![vec![0u8; shard_len]; t_cols]; d_rows];
    for r in 0..d_rows {
        for c in 0..t_cols {
            let coeffs = vandermonde_row(c as u8, d_cols);
            let mut shard = vec![0u8; shard_len];
            for byte_idx in 0..shard_len {
                let mut acc = 0u8;
                for j in 0..d_cols {
                    acc ^= gf_mul(data_chunks[r][j][byte_idx], coeffs[j]);
                }
                shard[byte_idx] = acc;
            }
            row_encoded[r][c] = shard;
        }
    }

    // 3. Encode each column: D_rows -> T_rows
    let mut grid = vec![vec![vec![0u8; shard_len]; t_cols]; t_rows];
    for c in 0..t_cols {
        for r in 0..t_rows {
            let coeffs = vandermonde_row(r as u8, d_rows);
            let mut shard = vec![0u8; shard_len];
            for byte_idx in 0..shard_len {
                let mut acc = 0u8;
                for i in 0..d_rows {
                    acc ^= gf_mul(row_encoded[i][c][byte_idx], coeffs[i]);
                }
                shard[byte_idx] = acc;
            }
            grid[r][c] = shard;
        }
    }

    // 4. Row and column commitments
    let mut row_commitments = Vec::with_capacity(t_rows);
    for r in 0..t_rows {
        let mut buf = Vec::with_capacity(t_cols * shard_len);
        for c in 0..t_cols {
            buf.extend_from_slice(&grid[r][c]);
        }
        row_commitments.push(hash("VERIDAG_DA_ROW_V1", &buf));
    }

    let mut col_commitments = Vec::with_capacity(t_cols);
    for c in 0..t_cols {
        let mut buf = Vec::with_capacity(t_rows * shard_len);
        for r in 0..t_rows {
            buf.extend_from_slice(&grid[r][c]);
        }
        col_commitments.push(hash("VERIDAG_DA_COL_V1", &buf));
    }

    let mut root_buf = Vec::new();
    for rc in &row_commitments {
        root_buf.extend_from_slice(rc);
    }
    for cc in &col_commitments {
        root_buf.extend_from_slice(cc);
    }
    let da_root = hash("VERIDAG_DA_ROOT_2D_V1", &root_buf);
    let content_hash = hash("VERIDAG_DA_BLOB_V1", blob);

    Ok(Dispersal2D {
        content_hash,
        original_len: blob.len(),
        grid,
        row_commitments,
        col_commitments,
        da_root,
    })
}

/// Reconstruct blob from a sparse set of 2D shards using iterative row/column decoding.
pub fn reconstruct_2d(
    config: Da2DConfig,
    content_hash: &Hash,
    original_len: usize,
    present: &HashMap<(usize, usize), Vec<u8>>,
) -> Result<Vec<u8>, DaError> {
    let d_rows = config.data_rows;
    let d_cols = config.data_cols;
    let t_rows = config.total_rows();
    let t_cols = config.total_cols();

    if present.is_empty() {
        return Err(DaError::NotEnoughShares(0, d_rows * d_cols));
    }
    let shard_len = present.values().next().unwrap().len();

    let mut grid: Vec<Vec<Option<Vec<u8>>>> = vec![vec![None; t_cols]; t_rows];
    for (&(r, c), data) in present {
        if r < t_rows && c < t_cols && data.len() == shard_len {
            grid[r][c] = Some(data.clone());
        }
    }

    let row_cfg = DaConfig::new(d_cols, config.parity_cols);
    let col_cfg = DaConfig::new(d_rows, config.parity_rows);

    // Iterative alternating row/column recovery
    let mut progress = true;
    while progress {
        progress = false;

        // Try recovering rows
        for r in 0..t_rows {
            let present_in_row: Vec<(usize, Vec<u8>)> = (0..t_cols)
                .filter_map(|c| grid[r][c].as_ref().map(|s| (c, s.clone())))
                .collect();
            let missing_in_row: Vec<usize> =
                (0..t_cols).filter(|&c| grid[r][c].is_none()).collect();

            if !missing_in_row.is_empty() && present_in_row.len() >= d_cols {
                let data_shards = reconstruct_data_shards(row_cfg, shard_len, &present_in_row)?;
                for c in missing_in_row {
                    let coeffs = vandermonde_row(c as u8, d_cols);
                    let mut shard = vec![0u8; shard_len];
                    for byte_idx in 0..shard_len {
                        let mut acc = 0u8;
                        for j in 0..d_cols {
                            acc ^= gf_mul(data_shards[j][byte_idx], coeffs[j]);
                        }
                        shard[byte_idx] = acc;
                    }
                    grid[r][c] = Some(shard);
                    progress = true;
                }
            }
        }

        // Try recovering columns
        for c in 0..t_cols {
            let present_in_col: Vec<(usize, Vec<u8>)> = (0..t_rows)
                .filter_map(|r| grid[r][c].as_ref().map(|s| (r, s.clone())))
                .collect();
            let missing_in_col: Vec<usize> =
                (0..t_rows).filter(|&r| grid[r][c].is_none()).collect();

            if !missing_in_col.is_empty() && present_in_col.len() >= d_rows {
                let data_shards = reconstruct_data_shards(col_cfg, shard_len, &present_in_col)?;
                for r in missing_in_col {
                    let coeffs = vandermonde_row(r as u8, d_rows);
                    let mut shard = vec![0u8; shard_len];
                    for byte_idx in 0..shard_len {
                        let mut acc = 0u8;
                        for j in 0..d_rows {
                            acc ^= gf_mul(data_shards[j][byte_idx], coeffs[j]);
                        }
                        shard[byte_idx] = acc;
                    }
                    grid[r][c] = Some(shard);
                    progress = true;
                }
            }
        }
    }

    // Check if we have at least D_cols complete columns (each having >= D_rows cells)
    // to invert the column transform and recover row_encoded[0..D_rows]
    let mut row_encoded = vec![vec![vec![0u8; shard_len]; t_cols]; d_rows];
    for c in 0..t_cols {
        let present_in_col: Vec<(usize, Vec<u8>)> = (0..t_rows)
            .filter_map(|r| grid[r][c].as_ref().map(|s| (r, s.clone())))
            .collect();
        if present_in_col.len() < d_rows {
            return Err(DaError::InsufficientShards2D);
        }
        let col_data = reconstruct_data_shards(col_cfg, shard_len, &present_in_col)?;
        for r in 0..d_rows {
            row_encoded[r][c] = col_data[r].clone();
        }
    }

    // Now invert each row to recover data_chunks[r][0..D_cols]
    let mut raw_chunks = vec![vec![vec![0u8; shard_len]; d_cols]; d_rows];
    for r in 0..d_rows {
        let present_in_row: Vec<(usize, Vec<u8>)> = (0..t_cols)
            .map(|c| (c, row_encoded[r][c].clone()))
            .collect();
        let row_data = reconstruct_data_shards(row_cfg, shard_len, &present_in_row)?;
        for c in 0..d_cols {
            raw_chunks[r][c] = row_data[c].clone();
        }
    }

    // Concatenate raw chunks
    let mut blob = Vec::with_capacity(d_rows * d_cols * shard_len);
    for r in 0..d_rows {
        for c in 0..d_cols {
            blob.extend_from_slice(&raw_chunks[r][c]);
        }
    }

    if blob.len() < original_len {
        return Err(DaError::HashMismatch);
    }
    blob.truncate(original_len);

    let got = hash("VERIDAG_DA_BLOB_V1", &blob);
    if &got != content_hash {
        return Err(DaError::HashMismatch);
    }

    Ok(blob)
}

/// Deterministic shard assignment for validator replication in 2D DA networks.
pub struct ValidatorReplicationScheme;

impl ValidatorReplicationScheme {
    /// Deterministically assign matrix coordinate `(row, col)` to a validator.
    pub fn assign_shard(row: usize, col: usize, validators: &[ValidatorId]) -> ValidatorId {
        assert!(!validators.is_empty(), "validator set cannot be empty");
        let idx = (row * 31 + col * 17) % validators.len();
        validators[idx]
    }

    /// Retrieve all shard coordinates assigned to a specific validator.
    pub fn assigned_shards(
        validator: &ValidatorId,
        validators: &[ValidatorId],
        config: Da2DConfig,
    ) -> Vec<(usize, usize)> {
        let mut shards = Vec::new();
        for r in 0..config.total_rows() {
            for c in 0..config.total_cols() {
                if &Self::assign_shard(r, c, validators) == validator {
                    shards.push((r, c));
                }
            }
        }
        shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf_basics() {
        assert_eq!(gf_mul(5, 1), 5);
        assert_eq!(gf_mul(5, 0), 0);
        assert_eq!(gf_div(gf_mul(7, 9), 9), 7);
        assert_eq!(gf_div(gf_mul(0x53, 0xca), 0xca), 0x53);
    }

    #[test]
    fn roundtrip_full_shares() {
        let cfg = DaConfig::new(4, 2);
        let blob = b"veridag advanced data availability layer - reconstruct me".to_vec();
        let d = encode(cfg, &blob).unwrap();
        assert_eq!(d.shares.len(), 6);
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
        let present: Vec<(usize, Vec<u8>)> = (4..8).map(|i| (i, d.shares[i].clone())).collect();
        let out = reconstruct(cfg, &d.content_hash, d.original_len, &present).unwrap();
        assert_eq!(out, blob);
    }

    #[test]
    fn reconstruct_with_arbitrary_k_subset() {
        let cfg = DaConfig::new(5, 3);
        let blob = b"edge deployment resilience test payload ".repeat(37);
        let d = encode(cfg, &blob).unwrap();
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

    #[test]
    fn da_2d_roundtrip_and_commitments() {
        let cfg = Da2DConfig::new(2, 2, 2, 2); // 2x2 data -> 4x4 matrix (16 shards)
        let blob = b"Institutional 2D DA tensor erasure coding test payload".repeat(8);
        let d2 = encode_2d(cfg, &blob).expect("2D encode should succeed");

        assert_eq!(d2.grid.len(), 4);
        assert_eq!(d2.grid[0].len(), 4);
        assert_eq!(d2.row_commitments.len(), 4);
        assert_eq!(d2.col_commitments.len(), 4);

        // Reconstruct from all data shards
        let mut present = HashMap::new();
        for r in 0..2 {
            for c in 0..2 {
                present.insert((r, c), d2.grid[r][c].clone());
            }
        }
        let recovered = reconstruct_2d(cfg, &d2.content_hash, d2.original_len, &present)
            .expect("reconstruct 2D from data shards should succeed");
        assert_eq!(recovered, blob);
    }

    #[test]
    fn da_2d_reconstruction_with_scattered_erasures() {
        let cfg = Da2DConfig::new(2, 2, 2, 2); // 4x4 matrix
        let blob = b"Scattered erasure test for 2D Reed-Solomon alternating decoding".repeat(5);
        let d2 = encode_2d(cfg, &blob).unwrap();

        // Introduce scattered dropouts: drop data shard (0,0) and (1,1) but provide enough parity
        let mut present = HashMap::new();
        // Row 0 has (0, 1), (0, 2) -> 2 shards (>= D_cols=2), so (0,0) can be recovered!
        present.insert((0, 1), d2.grid[0][1].clone());
        present.insert((0, 2), d2.grid[0][2].clone());

        // Row 1 has (1, 0), (1, 3) -> 2 shards (>= D_cols=2), so (1,1) can be recovered!
        present.insert((1, 0), d2.grid[1][0].clone());
        present.insert((1, 3), d2.grid[1][3].clone());

        let recovered = reconstruct_2d(cfg, &d2.content_hash, d2.original_len, &present)
            .expect("iterative alternating recovery should succeed");
        assert_eq!(recovered, blob);
    }

    #[test]
    fn validator_replication_assignment() {
        let v1 = ValidatorId([1u8; 32]);
        let v2 = ValidatorId([2u8; 32]);
        let v3 = ValidatorId([3u8; 32]);
        let validators = vec![v1, v2, v3];

        let cfg = Da2DConfig::new(2, 2, 2, 2);
        let shards_v1 = ValidatorReplicationScheme::assigned_shards(&v1, &validators, cfg);
        let shards_v2 = ValidatorReplicationScheme::assigned_shards(&v2, &validators, cfg);
        let shards_v3 = ValidatorReplicationScheme::assigned_shards(&v3, &validators, cfg);

        // Every shard is assigned to exactly one validator
        assert_eq!(
            shards_v1.len() + shards_v2.len() + shards_v3.len(),
            cfg.total_shards()
        );
    }
}
