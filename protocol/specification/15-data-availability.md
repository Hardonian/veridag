# 15 — Data Availability (DA)

Status: NORMATIVE  
Version: 1.0  
Scope: 2D Reed-Solomon Erasure Coding, Chunk-Unrolled Hardware Acceleration, and Validator Replication Schemes

The protocol separates data availability from consensus. A vertex commits to batches by `BatchId`; the DA layer guarantees that referenced batch bytes are fully recoverable before those batches can be causally ordered and executed (08 §7).

## 1. 2D Reed-Solomon Tensor Erasure Coding

To protect against adversarial data withholding and network packet drop:
1. **Tensor Matrix Construction**: Batch payload bytes are partitioned into an $M \times N$ matrix of data chunks.
2. **2D Parity Expansion**: Systematic Reed-Solomon erasure coding extends the matrix with $P_{\text{row}}$ row parity chunks and $P_{\text{col}}$ column parity chunks.
3. **Iterative Row/Column Reconstruction**: Under scattered chunk loss, the reconstruction engine (`reconstruct_2d`) alternates between horizontal and vertical Reed-Solomon decoding until the full batch payload is reconstituted.

## 2. Independent Commitments & Merkle Proofs

- Every row and column produces an independent cryptographic Merkle commitment.
- Light clients and sampling nodes verify individual chunks against row/column commitments without downloading the entire matrix.

## 3. Validator Replication Scheme (`ValidatorReplicationScheme`)

- **Deterministic Assignment**: Parity and data chunks are deterministically assigned across the active validator set based on `ValidatorId` hashing.
- **Sovereignty**: No external third-party DA chain (e.g. Celestia/EigenDA) is required. The consensus committee autonomously replicates and certifies availability.

## 4. Hardware Acceleration (`veridag-da::hw_accel`)

- Batch Galois Field $\text{GF}(2^8)$ multiplication and SIMD chunk XOR operations are accelerated with chunk-unrolling under strict `#![forbid(unsafe_code)]`.

