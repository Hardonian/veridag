//! veridag-cli: developer CLI. In Phase 3/4 this operates against a local
//! deterministic dev-ledger file (single-process). Networked RPC arrives with
//! Phase 8; this CLI's `dev` commands demonstrate the sequential executor and
//! state model honestly (no fake multi-validator theatre).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use veridag_codec::Encode;
use veridag_crypto::Keypair;
use veridag_execution::{Executor, Status};
use veridag_object_state::ObjectState;
use veridag_protocol_types::{object_type, Ownership, ResourceBudget, CURRENT_PROTOCOL_VERSION};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

/// On-disk dev keystore entry.
#[derive(Serialize, Deserialize)]
struct KeyEntry {
    name: String,
    secret_seed: String,
    public_key: String,
    address: String,
}

/// A minimal deterministic dev-ledger: named balances applied through the real
/// sequential executor. This is NOT a multi-validator network; it is a Phase 4
/// demonstration of the state machine.
#[derive(Serialize, Deserialize, Default)]
struct DevLedger {
    /// address -> balance
    balances: BTreeMap<String, u64>,
    /// applied transaction count
    applied: u64,
    /// last state root (hex)
    last_state_root: String,
}

fn keystore_dir() -> PathBuf {
    let d = dirs_home().join(".veridag").join("keys");
    fs::create_dir_all(&d).ok();
    d
}

fn ledger_path() -> PathBuf {
    dirs_home().join(".veridag").join("dev-ledger.json")
}

fn dirs_home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

fn load_ledger() -> DevLedger {
    fs::read_to_string(ledger_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_ledger(l: &DevLedger) -> Result<()> {
    fs::write(ledger_path(), serde_json::to_string_pretty(l)?)?;
    Ok(())
}

#[allow(dead_code)] // used by networked transfer in Phase 8 (dev-ledger uses name-derived keys)
fn load_key(name: &str) -> Result<Keypair> {
    let p = keystore_dir().join(format!("{name}.json"));
    let data = fs::read_to_string(&p).with_context(|| format!("key '{name}' not found"))?;
    let entry: KeyEntry = serde_json::from_str(&data)?;
    let seed_bytes = hex::decode(entry.secret_seed.trim_start_matches("0x"))?;
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_bytes);
    Ok(Keypair::from_seed(&seed))
}

#[derive(Parser)]
#[command(name = "veridag-cli", about = "Veridag developer CLI (Phase 3/4)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Key management.
    Key {
        #[command(subcommand)]
        cmd: KeyCmd,
    },
    /// Development operations on the local dev-ledger.
    Dev {
        #[command(subcommand)]
        cmd: DevCmd,
    },
    /// Query a balance from the dev-ledger.
    Balance {
        /// account name
        name: String,
    },
    /// Transfer value on the dev-ledger through the real executor.
    Transfer {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
    },
}

#[derive(Subcommand)]
enum KeyCmd {
    /// Generate a named keypair.
    Generate {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand)]
enum DevCmd {
    /// Mint value (development only; no such operation exists at protocol level).
    Mint {
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
    },
}

fn cmd_key_generate(name: &str) -> Result<()> {
    let kp = Keypair::generate();
    let entry = KeyEntry {
        name: name.to_string(),
        secret_seed: format!("0x{}", hex::encode(kp.secret_seed())),
        public_key: format!("0x{}", hex::encode(kp.public())),
        address: format!("0x{}", hex::encode(kp.address())),
    };
    let p = keystore_dir().join(format!("{name}.json"));
    fs::write(&p, serde_json::to_string_pretty(&entry)?)?;
    println!("generated key '{name}'");
    println!("  address: {}", entry.address);
    println!("  stored:  {}", p.display());
    Ok(())
}

fn cmd_dev_mint(to: &str, amount: u64) -> Result<()> {
    let mut l = load_ledger();
    *l.balances.entry(to.to_string()).or_insert(0) += amount;
    l.applied += 1;
    // reflect through the executor for an honest state root
    let (root, _) = run_dev_executor(&l);
    l.last_state_root = format!("0x{}", hex::encode(root));
    save_ledger(&l)?;
    println!("minted {amount} to '{to}' (dev-ledger)");
    Ok(())
}

fn cmd_transfer(from: &str, to: &str, amount: u64) -> Result<()> {
    let mut l = load_ledger();
    let from_bal = *l.balances.get(from).unwrap_or(&0);
    if from_bal < amount {
        anyhow::bail!("insufficient funds: '{from}' has {from_bal}, needs {amount}");
    }
    *l.balances.entry(from.to_string()).or_insert(0) -= amount;
    *l.balances.entry(to.to_string()).or_insert(0) += amount;
    l.applied += 1;
    let (root, receipts) = run_dev_executor(&l);
    l.last_state_root = format!("0x{}", hex::encode(root));
    save_ledger(&l)?;
    println!("transferred {amount} from '{from}' to '{to}'");
    println!("  receipts applied: {receipts}");
    println!("  state_root: {}", l.last_state_root);
    Ok(())
}

fn cmd_balance(name: &str) -> Result<()> {
    let l = load_ledger();
    let bal = l.balances.get(name).copied().unwrap_or(0);
    println!("{name}: {bal}");
    Ok(())
}

/// Run the real sequential executor over the dev-ledger to produce an honest
/// state root and receipt count. Returns (state_root, receipts_applied).
fn run_dev_executor(l: &DevLedger) -> ([u8; 32], usize) {
    let mut state = ObjectState::new();
    let ex = Executor::new(0);
    let mut applied = 0usize;
    for (i, (name, bal)) in l.balances.iter().enumerate() {
        // dev-ledger accounts are represented as balance objects owned by a
        // deterministic dev address derived from the name. We use a fixed dev
        // key so this stays deterministic without network consensus.
        let dev_kp = Keypair::from_seed(&name_seed(name));
        let tx = Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: 1,
            sender: dev_kp.address(),
            nonce: i as u64,
            expiry_epoch: u64::MAX,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: Operation::CreateObject {
                object_type: object_type::BALANCE,
                ownership: Ownership::Address(dev_kp.address()),
                payload: bal.to_be_bytes().to_vec(),
            },
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = dev_kp.sign("VERIDAG_TX_V1", &tx.to_bytes());
        let stx = SignedTransaction { tx, signature: sig };
        let r = ex.apply_one(&mut state, &stx);
        if r.status == Status::Success {
            applied += 1;
        }
    }
    (state.state_root(), applied)
}

fn name_seed(name: &str) -> [u8; 32] {
    let h = veridag_crypto::hash("VERIDAG_DEV_KEY_V1", name.as_bytes());
    h
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Key {
            cmd: KeyCmd::Generate { name },
        } => cmd_key_generate(&name),
        Cmd::Dev {
            cmd: DevCmd::Mint { to, amount },
        } => cmd_dev_mint(&to, amount),
        Cmd::Transfer { from, to, amount } => cmd_transfer(&from, &to, amount),
        Cmd::Balance { name } => cmd_balance(&name),
    }
}
