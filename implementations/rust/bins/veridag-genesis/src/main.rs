//! veridag-genesis: deterministic genesis generation, inspection, verification.

#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use veridag_codec::{Encode, Encoder};
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

impl Encode for Genesis {
    fn encode(&self, e: &mut Encoder) {
        e.u64(self.protocol_version);
        e.u64(self.chain_id);
        e.u32(self.validators.len() as u32);
        for v in &self.validators {
            let pk_bytes = hex::decode(v.pubkey.trim_start_matches("0x"))
                .expect("validator pubkey must be hex");
            e.fixed(&pk_bytes);
            e.u32(v.weight);
        }
        e.u64(self.epoch_length_checkpoints);
        e.u32(self.max_tx_bytes);
        e.u32(self.max_batch_size);
    }
}

fn genesis_commitment(g: &Genesis) -> [u8; 32] {
    hash("VERIDAG_GENESIS_V1", &g.to_bytes())
}

fn parse_pubkey(s: &str) -> Ed25519PublicKey {
    let b = hex::decode(s.trim_start_matches("0x")).expect("invalid hex pubkey");
    let mut a = [0u8; 32];
    a.copy_from_slice(&b);
    a
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

fn main() {
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
                    let _ = parse_pubkey(s); // validate
                    ValidatorEntry {
                        pubkey: s.clone(),
                        weight: 1,
                    }
                })
                .collect();
            let g = Genesis {
                protocol_version: 1,
                chain_id,
                validators: entries,
                epoch_length_checkpoints,
                max_tx_bytes,
                max_batch_size,
            };
            let commitment = genesis_commitment(&g);
            println!("{}", serde_json::to_string_pretty(&g).unwrap());
            eprintln!("genesis_commitment: 0x{}", hex::encode(commitment));
        }
        Cmd::Inspect { file } => {
            let data = std::fs::read_to_string(&file).expect("read genesis file");
            let g: Genesis = serde_json::from_str(&data).expect("parse genesis");
            println!("protocol_version: {}", g.protocol_version);
            println!("chain_id: {}", g.chain_id);
            println!("validators: {}", g.validators.len());
            let n = g.validators.len();
            let f = (n.saturating_sub(1)) / 3;
            println!("max Byzantine tolerated (f): {f} (n={n}, needs n>=3f+1)");
            println!(
                "genesis_commitment: 0x{}",
                hex::encode(genesis_commitment(&g))
            );
        }
        Cmd::Verify { file, expect } => {
            let data = std::fs::read_to_string(&file).expect("read genesis file");
            let g: Genesis = serde_json::from_str(&data).expect("parse genesis");
            let c = genesis_commitment(&g);
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
}
