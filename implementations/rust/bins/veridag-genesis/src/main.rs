//! veridag-genesis: deterministic genesis generation, inspection, verification.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use veridag_codec::Encoder;
use veridag_crypto::hash;
use veridag_protocol_types::Ed25519PublicKey;

/// Genesis input (deterministic; identical input -> identical commitment).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Genesis {
    protocol_version: u64,
    chain_id: u64,
    validators: Vec<ValidatorEntry>,
    epoch_length_checkpoints: u64,
    max_tx_bytes: u32,
    max_batch_size: u32,
}

/// A genesis validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidatorEntry {
    /// Hex-encoded Ed25519 public key.
    pubkey: String,
    /// Uniform weight in v0.1.
    weight: u32,
}

fn encode_genesis(g: &Genesis) -> Result<Vec<u8>> {
    if g.validators.is_empty() {
        bail!("genesis must contain at least one validator");
    }
    if g.epoch_length_checkpoints == 0 {
        bail!("epoch_length_checkpoints must be greater than zero");
    }
    if g.max_tx_bytes == 0 || g.max_batch_size == 0 {
        bail!("transaction and batch limits must be greater than zero");
    }

    let validator_count = u32::try_from(g.validators.len())
        .context("validator count exceeds the canonical u32 limit")?;
    let mut public_keys = BTreeSet::new();
    let mut parsed_validators = Vec::with_capacity(g.validators.len());
    for (index, validator) in g.validators.iter().enumerate() {
        if validator.weight == 0 {
            bail!("validator {index} has zero weight");
        }
        let public_key = parse_pubkey(&validator.pubkey)
            .with_context(|| format!("invalid validator public key at index {index}"))?;
        if !public_keys.insert(public_key) {
            bail!("duplicate validator public key at index {index}");
        }
        parsed_validators.push((public_key, validator.weight));
    }

    let mut encoder = Encoder::new();
    encoder.u64(g.protocol_version);
    encoder.u64(g.chain_id);
    encoder.u32(validator_count);
    for (public_key, weight) in parsed_validators {
        encoder.fixed(&public_key);
        encoder.u32(weight);
    }
    encoder.u64(g.epoch_length_checkpoints);
    encoder.u32(g.max_tx_bytes);
    encoder.u32(g.max_batch_size);
    Ok(encoder.into_bytes())
}

fn genesis_commitment(g: &Genesis) -> Result<[u8; 32]> {
    Ok(hash("VERIDAG_GENESIS_V1", &encode_genesis(g)?))
}

fn parse_pubkey(s: &str) -> Result<Ed25519PublicKey> {
    let bytes = hex::decode(s.trim_start_matches("0x")).context("public key is not valid hex")?;
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        anyhow::anyhow!("public key must be 32 bytes, got {}", bytes.len())
    })
}

#[derive(Parser)]
#[command(name = "veridag-genesis", about = "Veridag genesis tool")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a genesis JSON from parameters.
    Generate {
        #[arg(long)]
        chain_id: u64,
        #[arg(long, value_delimiter = ',')]
        validators: Vec<String>,
        #[arg(long, default_value_t = 100)]
        epoch_length_checkpoints: u64,
        #[arg(long, default_value_t = 1_048_576)]
        max_tx_bytes: u32,
        #[arg(long, default_value_t = 500_000)]
        max_batch_size: u32,
    },
    /// Inspect a genesis JSON file.
    Inspect {
        #[arg(long)]
        file: String,
    },
    /// Verify a genesis JSON file's commitment is reproducible.
    Verify {
        #[arg(long)]
        file: String,
        #[arg(long)]
        expect: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Generate {
            chain_id,
            validators,
            epoch_length_checkpoints,
            max_tx_bytes,
            max_batch_size,
        } => {
            let entries: Vec<ValidatorEntry> = validators
                .iter()
                .map(|s| {
                    parse_pubkey(s)?;
                    Ok(ValidatorEntry {
                        pubkey: s.clone(),
                        weight: 1,
                    })
                })
                .collect::<Result<_>>()?;
            let g = Genesis {
                protocol_version: 1,
                chain_id,
                validators: entries,
                epoch_length_checkpoints,
                max_tx_bytes,
                max_batch_size,
            };
            let commitment = genesis_commitment(&g)?;
            println!("{}", serde_json::to_string_pretty(&g)?);
            eprintln!("genesis_commitment: 0x{}", hex::encode(commitment));
        }
        Cmd::Inspect { file } => {
            let data = std::fs::read_to_string(&file)
                .with_context(|| format!("failed to read genesis file '{file}'"))?;
            let g: Genesis = serde_json::from_str(&data)
                .with_context(|| format!("failed to parse genesis file '{file}'"))?;
            let commitment = genesis_commitment(&g)?;
            println!("protocol_version: {}", g.protocol_version);
            println!("chain_id: {}", g.chain_id);
            println!("validators: {}", g.validators.len());
            let n = g.validators.len();
            let f = (n.saturating_sub(1)) / 3;
            println!("max Byzantine tolerated (f): {f} (n={n}, needs n>=3f+1)");
            println!("genesis_commitment: 0x{}", hex::encode(commitment));
        }
        Cmd::Verify { file, expect } => {
            let data = std::fs::read_to_string(&file)
                .with_context(|| format!("failed to read genesis file '{file}'"))?;
            let g: Genesis = serde_json::from_str(&data)
                .with_context(|| format!("failed to parse genesis file '{file}'"))?;
            let c = genesis_commitment(&g)?;
            let got = format!("0x{}", hex::encode(c));
            println!("genesis_commitment: {got}");
            if let Some(exp) = expect {
                if exp == got {
                    println!("VERIFY: OK (matches expected)");
                } else {
                    println!("VERIFY: FAIL (expected {exp})");
                    std::process::exit(1);
                }
            } else {
                println!("VERIFY: OK (commitment reproducible)");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn genesis(pubkeys: Vec<String>) -> Genesis {
        Genesis {
            protocol_version: 1,
            chain_id: 1,
            validators: pubkeys
                .into_iter()
                .map(|pubkey| ValidatorEntry { pubkey, weight: 1 })
                .collect(),
            epoch_length_checkpoints: 100,
            max_tx_bytes: 1_048_576,
            max_batch_size: 500_000,
        }
    }

    #[test]
    fn invalid_public_keys_return_errors_instead_of_panicking() {
        let malformed = genesis(vec!["not-hex".into()]);
        assert!(genesis_commitment(&malformed).is_err());

        let wrong_length = genesis(vec!["00".repeat(31)]);
        assert!(genesis_commitment(&wrong_length).is_err());
    }

    #[test]
    fn duplicate_validators_are_rejected() {
        let public_key = "01".repeat(32);
        assert!(genesis_commitment(&genesis(vec![public_key.clone(), public_key])).is_err());
    }

    #[test]
    fn commitment_is_deterministic() {
        let valid = genesis(vec!["01".repeat(32), "02".repeat(32)]);
        assert_eq!(
            genesis_commitment(&valid).unwrap(),
            genesis_commitment(&valid).unwrap()
        );
    }
}
