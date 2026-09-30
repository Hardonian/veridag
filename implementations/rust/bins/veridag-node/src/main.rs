//! veridag-node: production validator node.
//!
//! Runs a single validator: a mempool collects client transactions, the node
//! proposes DAG vertices carrying batch commitments, BaselineDagBft commits
//! anchors, the parallel executor applies the committed ordering, and a
//! checkpoint is produced every CHECKPOINT_INTERVAL_WAVES committed waves.
//!
//! Modes:
//!   - `demo`   — in-process 4-validator consensus demonstration
//!   - `health` — ops-facing health probe with agreement assertion
//!   - `daemon` — full networked validator with QUIC mesh + HTTP/JSON RPC
//!
//! Production features:
//!   - Structured logging via `tracing` (RUST_LOG env filter, JSON output)
//!   - Prometheus metrics at `/v1/metrics`
//!   - Readiness probe at `/v1/ready`
//!   - Graceful shutdown on SIGTERM/Ctrl+C

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{info, warn};
use veridag_checkpoint::{
    dag_commitment, validator_set_commitment, Checkpoint, CHECKPOINT_INTERVAL_WAVES,
};

use veridag_codec::Decode;
use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};
use veridag_crypto::{address_of, Keypair};
use veridag_dag::{Dag, Vertex};
use veridag_execution::parallel::execute_parallel;
use veridag_execution::Executor;
use veridag_metrics::{Label, Metrics, Observation, PrometheusExporter};
use veridag_object_state::{Object, ObjectState};
use veridag_protocol_types::{
    object_type, Address, BatchId, ChainId, CheckpointId, Ed25519PublicKey, Epoch, ObjectId,
    ObjectRef, Ownership, ResourceBudget, Round, ValidatorId, VertexId, CURRENT_PROTOCOL_VERSION,
};
use veridag_storage::{CheckpointStore, DagStore, SledStore, StateStore};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

const CHAIN: ChainId = 1;
const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 1024 * 1024;
const HTTP_READ_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_RPC_RATE_LIMIT: u32 = 100;
const GOSSIP_TAG_VERTEX: u8 = 0;
const GOSSIP_TAG_TRANSACTION: u8 = 1;
const GOSSIP_TAG_CHECKPOINT: u8 = 2;
const GOSSIP_TAG_SYNC_REQUEST: u8 = 3;
const GOSSIP_TAG_SYNC_RESPONSE: u8 = 4;
const SYNC_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const SYNC_RESPONSE_INTERVAL: Duration = Duration::from_secs(1);
const MAX_SYNC_VERTICES_PER_REQUEST: usize = 4096;
const MAX_SYNC_ITEMS_PER_RESPONSE: usize = 8192;
const MAX_SYNC_RESPONSE_BYTES: usize = 256 * 1024;

fn encode_sync_request(round: Round) -> [u8; 8] {
    round.to_be_bytes()
}

fn decode_sync_request(payload: &[u8]) -> Option<Round> {
    let bytes: [u8; 8] = payload.try_into().ok()?;
    let round = Round::from_be_bytes(bytes);
    (round > 0).then_some(round)
}

fn sync_vertices(dag: &Dag, start_round: Round) -> Vec<Vertex> {
    let Some(frontier) = dag.round_vertices_max() else {
        return Vec::new();
    };
    if start_round > frontier {
        return Vec::new();
    }

    let mut vertices = Vec::new();
    for round in start_round..=frontier {
        for vertex_id in dag.round_vertices(round) {
            if let Some(vertex) = dag.get(vertex_id) {
                vertices.push(vertex.clone());
                if vertices.len() == MAX_SYNC_VERTICES_PER_REQUEST {
                    return vertices;
                }
            }
        }
    }
    vertices
}

fn encode_sync_response(
    vertices: &[Vertex],
    batches: &BTreeMap<BatchId, Vec<SignedTransaction>>,
) -> Vec<u8> {
    // Keep recovery frames comfortably below the transport ceiling. Large
    // near-limit QUIC streams are vulnerable to platform UDP/window limits,
    // especially through Docker Desktop's virtual network.
    let max_payload = MAX_SYNC_RESPONSE_BYTES.min(veridag_net::MAX_FRAME as usize - 1);
    let mut payload = vec![0u8; 4];
    let mut item_count = 0u32;
    let mut included_batches = BTreeSet::new();

    for vertex in vertices {
        let mut group = Vec::new();
        let mut complete = true;
        for batch_id in &vertex.batch_commitments {
            if included_batches.contains(batch_id) {
                continue;
            }
            let Some(transactions) = batches.get(batch_id) else {
                complete = false;
                break;
            };
            for transaction in transactions {
                group.push((
                    GOSSIP_TAG_TRANSACTION,
                    veridag_codec::Encode::to_bytes(transaction),
                ));
            }
        }
        if !complete {
            break;
        }
        group.push((GOSSIP_TAG_VERTEX, veridag_codec::Encode::to_bytes(vertex)));

        let group_size: usize = group.iter().map(|(_, bytes)| 5 + bytes.len()).sum();
        if payload.len() + group_size > max_payload
            || item_count as usize + group.len() > MAX_SYNC_ITEMS_PER_RESPONSE
        {
            break;
        }

        for batch_id in &vertex.batch_commitments {
            included_batches.insert(*batch_id);
        }
        for (tag, bytes) in group {
            payload.push(tag);
            payload.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            payload.extend_from_slice(&bytes);
            item_count += 1;
        }
    }

    payload[..4].copy_from_slice(&item_count.to_be_bytes());
    payload
}

fn decode_sync_response(payload: &[u8]) -> Option<Vec<(u8, Vec<u8>)>> {
    if payload.len() < 4 {
        return None;
    }
    let item_count = u32::from_be_bytes(payload[..4].try_into().ok()?) as usize;
    if item_count > MAX_SYNC_ITEMS_PER_RESPONSE {
        return None;
    }

    let mut cursor = 4usize;
    let mut items = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        let tag = *payload.get(cursor)?;
        if tag != GOSSIP_TAG_VERTEX && tag != GOSSIP_TAG_TRANSACTION {
            return None;
        }
        cursor += 1;
        let end_of_length = cursor.checked_add(4)?;
        let length =
            u32::from_be_bytes(payload.get(cursor..end_of_length)?.try_into().ok()?) as usize;
        cursor = end_of_length;
        let end = cursor.checked_add(length)?;
        items.push((tag, payload.get(cursor..end)?.to_vec()));
        cursor = end;
    }
    (cursor == payload.len()).then_some(items)
}

/// A minimal in-process mempool: signature-verified transactions awaiting
/// inclusion in a vertex batch.
#[derive(Default)]
struct Mempool {
    txs: Vec<SignedTransaction>,
    seen: BTreeSet<veridag_protocol_types::TransactionId>,
}

impl Mempool {
    /// Submit a transaction; the signature is verified before admission.
    fn submit(&mut self, stx: SignedTransaction, key_of_sender: &Ed25519PublicKey) -> bool {
        if stx.verify_signature(key_of_sender).is_err() {
            return false;
        }
        if !self.seen.insert(stx.id()) {
            return false; // duplicate
        }
        self.txs.push(stx);
        true
    }

    /// Drain up to `max` transactions for the next batch.
    fn drain(&mut self, max: usize) -> Vec<SignedTransaction> {
        let n = max.min(self.txs.len());
        self.txs.drain(..n).collect()
    }
}

/// A batch of transactions committed to by a vertex (id + resolved txs).
/// The id is the vertex-visible commitment; txs resolve it locally.
#[allow(dead_code)]
struct Batch {
    id: BatchId,
    txs: Vec<SignedTransaction>,
}

/// The validator node state.
struct Node {
    key: Keypair,
    id: ValidatorId,
    committee: StaticCommittee,
    keys_by_id: BTreeMap<ValidatorId, Ed25519PublicKey>,
    dag: Dag,
    mempool: Mempool,
    batches: BTreeMap<BatchId, Vec<SignedTransaction>>,
    state: ObjectState,
    executor: Executor,
    checkpoints: Vec<Checkpoint>,
    prev_checkpoint: CheckpointId,
    proposed: BTreeSet<Round>,
    nonce: u64,
    epoch: Epoch,
}

impl Node {
    fn new(
        key: Keypair,
        committee: StaticCommittee,
        keys_by_id: BTreeMap<ValidatorId, Ed25519PublicKey>,
        epoch: Epoch,
    ) -> Self {
        let id = ValidatorId(key.address());
        Self {
            key,
            id,
            committee,
            keys_by_id,
            dag: Dag::new(),
            mempool: Mempool::default(),
            batches: BTreeMap::new(),
            state: ObjectState::new(),
            executor: Executor::new(epoch),
            checkpoints: Vec::new(),
            prev_checkpoint: CheckpointId::ZERO,
            proposed: BTreeSet::new(),
            nonce: 0,
            epoch,
        }
    }

    fn validator_set(&self) -> BTreeSet<ValidatorId> {
        self.keys_by_id.keys().copied().collect()
    }

    /// Create a batch from the mempool and return its commitment.
    fn make_batch(&mut self, max: usize) -> Option<BatchId> {
        let txs = self.mempool.drain(max);
        if txs.is_empty() {
            return None;
        }
        let mut buf = Vec::new();
        for t in &txs {
            buf.extend_from_slice(&veridag_codec::Encode::to_bytes(t));
        }
        let id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &buf));
        self.batches.insert(id, txs);
        Some(id)
    }

    /// Propose a vertex for the next round if the frontier has a quorum.
    fn propose(&mut self) -> Option<Vertex> {
        let max_round = self.dag.round_vertices_max().unwrap_or(0);
        let next = max_round + 1;
        if self.proposed.contains(&next) {
            return None;
        }
        let parents: Vec<VertexId> = if next == 1 {
            Vec::new()
        } else {
            if !self.dag.quorum_reached(max_round, self.committee.quorum()) {
                return None;
            }
            self.dag.round_vertices(max_round).copied().collect()
        };
        let batch = self.make_batch(8);
        let batches: Vec<BatchId> = batch.into_iter().collect();
        self.nonce += 1;
        let v = Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            self.epoch,
            next,
            self.id,
            parents,
            batches,
            self.nonce.to_be_bytes().to_vec(),
            &self.key,
        )
        .ok()?;
        let is_val = self.validator_set();
        self.dag
            .add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                self.epoch,
                |x| is_val.contains(x),
                self.committee.quorum(),
                &[],
            )
            .ok()?;
        self.proposed.insert(next);
        Some(v)
    }

    /// Deliver a vertex (from a peer or self).
    fn deliver(&mut self, v: &Vertex) {
        let is_val = self.validator_set();
        let _ = self.dag.add(
            v.clone(),
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            self.epoch,
            |x| is_val.contains(x),
            self.committee.quorum(),
            &[],
        );
    }

    /// Resolve committed transactions in canonical order.
    fn committed_txs(&self) -> Vec<SignedTransaction> {
        let mw = highest_complete_wave(&self.dag);
        if mw == 0 {
            return Vec::new();
        }
        let seq = commit(&self.dag, &self.committee, mw);
        let mut out = Vec::new();
        for anchor in &seq.committed {
            for vid in &anchor.ordered {
                if let Some(v) = self.dag.get(vid) {
                    for b in &v.batch_commitments {
                        if let Some(txs) = self.batches.get(b) {
                            out.extend(txs.iter().cloned());
                        }
                    }
                }
            }
        }
        out
    }

    /// Execute committed transactions and produce a checkpoint when due.
    fn execute_committed(&mut self) -> Option<Checkpoint> {
        let mw = highest_complete_wave(&self.dag);
        if mw == 0 {
            return None;
        }
        let seq = commit(&self.dag, &self.committee, mw);
        if seq.committed.is_empty() {
            return None;
        }
        let anchor_ids: Vec<VertexId> = seq.committed.iter().map(|c| c.anchor).collect();
        let txs = self.committed_txs();
        if txs.is_empty() {
            return None;
        }
        let result = execute_parallel(&self.executor, &mut self.state, &txs);

        // Checkpoint every CHECKPOINT_INTERVAL_WAVES committed waves.
        let last_wave = seq.committed.last().map(|c| c.wave).unwrap_or(0);
        if !last_wave.is_multiple_of(veridag_checkpoint::CHECKPOINT_INTERVAL_WAVES) {
            return None;
        }
        let validators: Vec<ValidatorId> = self.keys_by_id.keys().copied().collect();
        let txids: Vec<_> = txs.iter().map(|t| t.id()).collect();
        let mut ckpt = Checkpoint::new(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            self.epoch,
            self.checkpoints.len() as u64 + 1,
            self.prev_checkpoint,
            result.state_root,
            veridag_execution::transaction_root(&txids),
            dag_commitment(&anchor_ids),
            validator_set_commitment(&validators),
        );
        // This node signs its own finality vote (quorum gathered across nodes
        // in a multi-process deployment; here we record the local vote).
        let vote = ckpt.sign_vote(&self.key);
        ckpt.add_vote(vote);
        self.prev_checkpoint = ckpt.id();
        self.checkpoints.push(ckpt.clone());
        Some(ckpt)
    }
}

#[derive(Parser)]
#[command(name = "veridag-node", about = "Veridag validator node v1.0.0")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a single-node in-process demo: genesis, a transfer, commit, checkpoint.
    Demo {
        /// Number of validators in the simulated committee.
        #[arg(long, default_value_t = 4)]
        validators: usize,
    },
    /// Report node health / live state for ops and monitors (go-live).
    Health {
        /// JSON output (one object per line, machine-parseable).
        #[arg(long)]
        json: bool,
    },
    /// Run as a networked validator daemon over QUIC.
    Daemon {
        /// Deterministic validator seed (1-4, development networks only).
        #[arg(
            long,
            conflicts_with = "key_file",
            required_unless_present = "key_file"
        )]
        seed: Option<u8>,
        /// File containing a 32-byte hex validator secret, or JSON with a
        /// `secret_seed` field. Required for non-development deployments.
        #[arg(long, conflicts_with = "seed")]
        key_file: Option<PathBuf>,
        /// Comma-separated Ed25519 committee public keys. Required with
        /// `--key-file`; development seeds derive the four-node dev committee.
        #[arg(long, value_delimiter = ',')]
        committee_pubkeys: Vec<String>,
        /// Comma-separated list of peer IP:PORT strings
        #[arg(long, value_delimiter = ',')]
        peers: Vec<String>,
        /// Bind address
        #[arg(long, default_value = "0.0.0.0:8000")]
        bind: String,
        /// HTTP/JSON RPC bind address (e.g. 0.0.0.0:8080)
        #[arg(long, default_value = "0.0.0.0:8080")]
        rpc: String,
        /// Persistent validator database directory. Defaults to
        /// `VERIDAG_DATA_DIR` or `./data`.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Create the deterministic Alice/Bob development balances when an
        /// empty database is opened.
        #[arg(long)]
        dev_genesis: bool,
        /// Permit unauthenticated transaction submission on a non-loopback
        /// RPC address. Development networks only.
        #[arg(long)]
        insecure_rpc: bool,
    },
}

struct DaemonConfig {
    dev_seed: Option<u8>,
    key_file: Option<PathBuf>,
    committee_pubkeys: Vec<String>,
    peers: Vec<String>,
    bind: String,
    rpc: String,
    data_dir: Option<PathBuf>,
    dev_genesis: bool,
    insecure_rpc: bool,
}

fn seed(n: u8) -> Keypair {
    Keypair::from_seed(&[n; 32])
}

fn parse_fixed_hex<const N: usize>(value: &str, label: &str) -> Result<[u8; N]> {
    let bytes = hex::decode(value.trim().trim_start_matches("0x"))
        .with_context(|| format!("{label} must be hexadecimal"))?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("{label} must be exactly {N} bytes"))
}

fn load_validator_key(path: &Path) -> Result<Keypair> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("read validator key metadata at {}", path.display()))?;
    if !metadata.is_file() {
        bail!(
            "validator key path is not a regular file: {}",
            path.display()
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            bail!(
                "validator key file {} is accessible by group or others; require mode 0600",
                path.display()
            );
        }
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("read validator key at {}", path.display()))?;
    let secret = if contents.trim_start().starts_with('{') {
        let value: serde_json::Value = serde_json::from_str(&contents)
            .with_context(|| format!("parse validator key JSON at {}", path.display()))?;
        value
            .get("secret_seed")
            .and_then(serde_json::Value::as_str)
            .context("validator key JSON must contain string field `secret_seed`")?
            .to_owned()
    } else {
        contents.trim().to_owned()
    };
    Ok(Keypair::from_seed(&parse_fixed_hex::<32>(
        &secret,
        "validator secret seed",
    )?))
}

fn parse_committee_pubkeys(values: &[String]) -> Result<BTreeMap<ValidatorId, Ed25519PublicKey>> {
    let validators: BTreeMap<_, _> = values
        .iter()
        .map(|value| {
            parse_fixed_hex::<32>(value, "committee public key")
                .map(|public_key| (ValidatorId(address_of(&public_key)), public_key))
        })
        .collect::<Result<_>>()?;
    if validators.len() < 4 {
        bail!("committee requires at least four distinct validator public keys");
    }
    Ok(validators)
}

fn unix_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn run_demo(n_validators: usize, quiet: bool) -> Result<Vec<Node>> {
    if !quiet {
        println!("veridag-node demo: {n_validators}-validator committee, in-process");
    }
    let keys: Vec<Keypair> = (1..=n_validators as u8).map(seed).collect();
    let validators: Vec<ValidatorId> = keys.iter().map(|k| ValidatorId(k.address())).collect();
    let keys_by_id: BTreeMap<ValidatorId, _> = keys
        .iter()
        .map(|k| (ValidatorId(k.address()), k.public()))
        .collect();
    let committee = StaticCommittee::new(validators.clone(), (n_validators - 1) / 3);

    // Genesis: fund alice and bob.
    let alice = seed(100);
    let bob = seed(101);
    let mut nodes: Vec<Node> = keys
        .iter()
        .map(|k| Node::new(k_clone(k), committee.clone(), keys_by_id.clone(), 0))
        .collect();
    for node in &mut nodes {
        node.state
            .create(Object::new(
                Object::derive_id(&alice.address(), 0),
                object_type::BALANCE,
                Ownership::Address(alice.address()),
                100u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
        node.state
            .create(Object::new(
                Object::derive_id(&bob.address(), 0),
                object_type::BALANCE,
                Ownership::Address(bob.address()),
                0u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
    }

    // Client tx: alice -> bob 40.
    let stx = signed_transfer(&alice, 0, bob.address(), 40);
    for node in &mut nodes {
        assert!(node.mempool.submit(stx.clone(), &alice.public()));
    }
    if !quiet {
        println!("submitted transfer alice->bob 40 to all mempools");
    }

    // Run rounds: each validator proposes, vertices broadcast to all. Run two
    // full waves plus vote rounds so wave 2 completes and a checkpoint fires.
    for round in 1..=(2 * WAVE + 3) {
        let mut produced = Vec::new();
        for node in &mut nodes {
            if let Some(v) = node.propose() {
                produced.push(v);
            }
        }
        for v in &produced {
            for node in &mut nodes {
                node.deliver(v);
            }
        }
        // Share batches so committed txs resolve everywhere.
        let all_batches: Vec<(BatchId, Vec<SignedTransaction>)> = nodes
            .iter()
            .flat_map(|n| n.batches.clone().into_iter())
            .collect();
        for node in &mut nodes {
            for (b, txs) in &all_batches {
                node.batches.entry(*b).or_insert_with(|| txs.clone());
            }
        }
        let committed = nodes[0].dag.round_vertices_max().unwrap_or(0);
        if !quiet {
            println!("round {round}: max round reached {committed}");
        }
    }

    // Execute committed and report checkpoints.
    let mut roots = Vec::new();
    for (i, node) in nodes.iter_mut().enumerate() {
        let ckpt = node.execute_committed();
        let root = node.state.state_root();
        roots.push(root);
        if !quiet {
            println!(
                "validator {i}: state_root=0x{} checkpoints={}",
                hex::encode(root),
                node.checkpoints.len()
            );
            if let Some(c) = ckpt {
                println!(
                    "  checkpoint seq={} id=0x{}",
                    c.sequence,
                    hex::encode(c.id().0)
                );
            }
        }
    }
    // Return the final node state for the caller (CLI demo prints verbose;
    // health probe prints a compact machine-readable snapshot and asserts).
    Ok(nodes)
}

fn run_health(json: bool) -> Result<()> {
    let n_validators = 4;
    let nodes = run_demo(n_validators, true)?;
    // Health probe asserts the pipeline actually reached agreement; if it did
    // not, the probe must fail so ops/alerts fire.
    let roots: Vec<_> = nodes.iter().map(|n| n.state.state_root()).collect();
    assert!(
        roots.iter().all(|r| *r == roots[0]),
        "health probe: validators did not agree on state root"
    );
    let last = &nodes[0];
    let mw = highest_complete_wave(&last.dag);
    let committed = last.committed_txs();
    let n_ckpts = last.checkpoints.len();
    let ckpt_ids: Vec<String> = last
        .checkpoints
        .iter()
        .map(|c| hex::encode(c.id().0))
        .collect();

    if json {
        let line = serde_json::json!({
            "binary": "veridag-node",
            "version": env!("CARGO_PKG_VERSION"),
            "protocol_version": CURRENT_PROTOCOL_VERSION,
            "chain_id": CHAIN,
            "n_validators": n_validators,
            "committee_n": last.committee.n(),
            "committee_quorum": last.committee.quorum(),
            "highest_complete_wave": mw,
            "max_round": last.dag.round_vertices_max().unwrap_or(0),
            "state_root": hex::encode(last.state.state_root()),
            "committed_tx_count": committed.len(),
            "checkpoint_count": n_ckpts,
            "checkpoint_ids": ckpt_ids,
            "proposal_nonce": last.nonce,
            "epoch": last.epoch,
            "keyset_fingerprint": hex::encode(validator_set_commitment(
                &last.validator_set().into_iter().collect::<Vec<_>>()
            )),
            "agreement": roots.iter().all(|r| *r == roots[0]),
        });
        println!("{line}");
    } else {
        println!("veridag-node health (demo-probed, {n_validators} validators)");
        println!("  version:        {}", env!("CARGO_PKG_VERSION"));
        println!("  protocol:       {CURRENT_PROTOCOL_VERSION}");
        println!("  chain_id:       {CHAIN}");
        println!(
            "  committee:      n={} quorum={}",
            last.committee.n(),
            last.committee.quorum()
        );
        println!(
            "  max round:      {}",
            last.dag.round_vertices_max().unwrap_or(0)
        );
        println!("  highest wave:   {mw}");
        println!(
            "  state root:     0x{}",
            hex::encode(last.state.state_root())
        );
        println!("  objects:        {}", last.state.iter().count());
        println!("  committed txs:  {}", committed.len());
        println!("  checkpoints:    {n_ckpts}");
        println!("  checkpoint_ids: {}", ckpt_ids.join(","));
        println!("  proposal nonce: {}", last.nonce);
        println!("  agreement:      {}", roots.iter().all(|r| *r == roots[0]));
    }
    Ok(())
}

fn k_clone(k: &Keypair) -> Keypair {
    Keypair::from_seed(&k.secret_seed())
}

fn signed_transfer(from: &Keypair, version: u64, to: Address, amount: u64) -> SignedTransaction {
    let from_id = Object::derive_id(&from.address(), 0);
    let tx = Transaction {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        chain_id: CHAIN,
        sender: from.address(),
        nonce: version,
        expiry_epoch: u64::MAX,
        declared_reads: vec![],
        declared_writes: vec![],
        capabilities: vec![],
        operation: Operation::TransferValue {
            from: ObjectRef {
                id: from_id,
                expected: version,
            },
            to,
            amount,
        },
        resource_budget: ResourceBudget::default(),
        metadata: vec![],
    };
    let sig = from.sign("VERIDAG_TX_V1", &veridag_codec::Encode::to_bytes(&tx));
    SignedTransaction { tx, signature: sig }
}

struct RpcContext {
    dag: Arc<RwLock<Dag>>,
    state: Arc<RwLock<ObjectState>>,
    checkpoints: Arc<RwLock<Vec<Checkpoint>>>,
    tx_sender: tokio::sync::mpsc::Sender<SignedTransaction>,
    committee: StaticCommittee,
    validator_id: ValidatorId,
    metrics: Arc<PrometheusExporter>,
    rpc_token: Option<String>,
    allowed_origin: String,
    rate_limiter: RpcRateLimiter,
    last_progress_ms: Arc<std::sync::atomic::AtomicU64>,
    persistent: bool,
}

#[derive(Debug)]
struct ClientRateWindow {
    started: Instant,
    requests: u32,
}

#[derive(Debug)]
struct RpcRateLimiter {
    requests_per_second: u32,
    clients: Mutex<BTreeMap<IpAddr, ClientRateWindow>>,
}

impl RpcRateLimiter {
    fn new(requests_per_second: u32) -> Self {
        Self {
            requests_per_second,
            clients: Mutex::new(BTreeMap::new()),
        }
    }

    fn allow(&self, client: IpAddr) -> bool {
        let now = Instant::now();
        let mut clients = self
            .clients
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if clients.len() > 10_000 {
            clients.retain(|_, window| now.duration_since(window.started) < Duration::from_secs(2));
        }
        let window = clients.entry(client).or_insert(ClientRateWindow {
            started: now,
            requests: 0,
        });
        if now.duration_since(window.started) >= Duration::from_secs(1) {
            window.started = now;
            window.requests = 0;
        }
        if window.requests >= self.requests_per_second {
            return false;
        }
        window.requests += 1;
        true
    }
}

async fn handle_http_request(
    ctx: &Arc<RpcContext>,
    method: &str,
    path: &str,
    body: &[u8],
) -> (u16, serde_json::Value) {
    if method == "GET" && (path == "/v1/health" || path == "/health") {
        let dag = ctx.dag.read().await;
        let state = ctx.state.read().await;
        let ckpts = ctx.checkpoints.read().await;
        let mw = highest_complete_wave(&dag);
        let max_r = dag.round_vertices_max().unwrap_or(0);
        let last_progress_ms = ctx
            .last_progress_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        let stalled_for_ms = unix_time_millis().saturating_sub(last_progress_ms);
        let progressing = stalled_for_ms < 30_000;
        let status = if progressing { "healthy" } else { "degraded" };
        return (
            if progressing { 200 } else { 503 },
            serde_json::json!({
                "status": status,
                "version": env!("CARGO_PKG_VERSION"),
                "protocol_version": CURRENT_PROTOCOL_VERSION,
                "chain_id": CHAIN,
                "validator_id": format!("0x{}", hex::encode(ctx.validator_id.0)),
                "committee_n": ctx.committee.n(),
                "committee_quorum": ctx.committee.quorum(),
                "max_round": max_r,
                "highest_wave": mw,
                "state_root": format!("0x{}", hex::encode(state.state_root())),
                "checkpoints_count": ckpts.len(),
                "last_progress_ms": last_progress_ms,
                "stalled_for_ms": stalled_for_ms,
                "persistent": ctx.persistent,
            }),
        );
    }
    if method == "GET" && path == "/v1/state/root" {
        let dag = ctx.dag.read().await;
        let state = ctx.state.read().await;
        let mw = highest_complete_wave(&dag);
        return (
            200,
            serde_json::json!({
                "state_root": format!("0x{}", hex::encode(state.state_root())),
                "highest_wave": mw,
            }),
        );
    }
    if method == "GET" && path == "/v1/checkpoints/latest" {
        let ckpts = ctx.checkpoints.read().await;
        if let Some(c) = ckpts.last() {
            return (
                200,
                serde_json::json!({
                    "id": format!("0x{}", hex::encode(c.id().0)),
                    "sequence": c.sequence,
                    "epoch": c.epoch,
                    "state_root": format!("0x{}", hex::encode(c.state_root)),
                    "transaction_root": format!("0x{}", hex::encode(c.transaction_root)),
                    "dag_commitment": format!("0x{}", hex::encode(c.dag_commitment)),
                    "validator_set_commitment": format!("0x{}", hex::encode(c.validator_set_commitment)),
                    "votes_count": c.finality_proof.votes.len(),
                }),
            );
        } else {
            return (
                404,
                serde_json::json!({ "error": "no checkpoints produced yet" }),
            );
        }
    }
    if method == "GET" && path.starts_with("/v1/state/account/") {
        let addr_str = path.trim_start_matches("/v1/state/account/");
        let addr_clean = addr_str.trim_start_matches("0x");
        if let Ok(bytes) = hex::decode(addr_clean) {
            if bytes.len() == 32 {
                let mut addr = [0u8; 32];
                addr.copy_from_slice(&bytes);
                let obj_id = ObjectId(Object::derive_id(&addr, 0).0);
                let state = ctx.state.read().await;
                let bal = state.balance(&obj_id).unwrap_or(0);
                let exists = state.get(&obj_id).is_some();
                return (
                    200,
                    serde_json::json!({
                        "address": format!("0x{}", hex::encode(addr)),
                        "object_id": format!("0x{}", hex::encode(obj_id.0)),
                        "balance": bal,
                        "exists": exists,
                    }),
                );
            }
        }
        return (
            400,
            serde_json::json!({ "error": "invalid 32-byte hex address" }),
        );
    }
    if method == "GET" && (path == "/v1/ready" || path == "/ready") {
        let dag = ctx.dag.read().await;
        let mw = highest_complete_wave(&dag);
        let stalled_for_ms = unix_time_millis().saturating_sub(
            ctx.last_progress_ms
                .load(std::sync::atomic::Ordering::Relaxed),
        );
        let ready = mw > 0 && stalled_for_ms < 30_000 && ctx.persistent;
        let code = if ready { 200 } else { 503 };
        return (
            code,
            serde_json::json!({
                "ready": ready,
                "highest_wave": mw,
                "validator_id": format!("0x{}", hex::encode(ctx.validator_id.0)),
                "stalled_for_ms": stalled_for_ms,
                "persistent": ctx.persistent,
            }),
        );
    }
    if method == "GET" && (path == "/v1/metrics" || path == "/metrics") {
        let rendered = ctx.metrics.render();
        // Return plain text for Prometheus scraping; wrap in JSON for uniformity
        // with other endpoints. Prometheus scrapers will strip the JSON envelope
        // via content negotiation or a relabel config.
        return (
            200,
            serde_json::json!({
                "content_type": "text/plain; version=0.0.4; charset=utf-8",
                "body": rendered,
            }),
        );
    }
    if method == "POST" && path == "/v1/tx/submit" {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) {
            let hex_str = val
                .get("raw_tx_hex")
                .or_else(|| val.get("tx_hex"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let clean_hex = hex_str.trim_start_matches("0x");
            let pubkey_bytes: Option<[u8; 32]> = val
                .get("public_key")
                .and_then(|v| v.as_str())
                .and_then(|s| hex::decode(s.trim_start_matches("0x")).ok())
                .and_then(|b| b.try_into().ok());

            if let Ok(tx_bytes) = hex::decode(clean_hex) {
                let mut d = veridag_codec::Decoder::new(&tx_bytes);
                if let Ok(stx) = SignedTransaction::decode(&mut d) {
                    if d.finish().is_ok() {
                        let verified = if let Some(pk) = pubkey_bytes {
                            veridag_crypto::address_of(&pk) == stx.tx.sender
                                && stx.verify_signature(&pk).is_ok()
                        } else {
                            stx.verify_signature(&stx.tx.sender).is_ok()
                        };

                        if verified {
                            let tx_id = stx.id();
                            let sender = stx.tx.sender;
                            return match ctx.tx_sender.try_send(stx) {
                                Ok(()) => (
                                    202,
                                    serde_json::json!({
                                        "status": "admitted",
                                        "tx_id": format!("0x{}", hex::encode(tx_id.0)),
                                        "sender": format!("0x{}", hex::encode(sender)),
                                    }),
                                ),
                                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => (
                                    429,
                                    serde_json::json!({ "error": "transaction admission queue is full" }),
                                ),
                                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => (
                                    503,
                                    serde_json::json!({ "error": "transaction admission is unavailable" }),
                                ),
                            };
                        } else {
                            return (
                                400,
                                serde_json::json!({ "error": "invalid transaction signature or sender address mismatch" }),
                            );
                        }
                    }
                }
            }
        }
        return (
            400,
            serde_json::json!({ "error": "malformed transaction payload, expected JSON with raw_tx_hex" }),
        );
    }

    (404, serde_json::json!({ "error": "endpoint not found" }))
}

async fn serve_http_connection(
    mut stream: tokio::net::TcpStream,
    ctx: Arc<RpcContext>,
    peer_ip: IpAddr,
) -> Result<()> {
    async fn respond(
        stream: &mut tokio::net::TcpStream,
        status_code: u16,
        content_type: &str,
        body: &str,
        allowed_origin: &str,
    ) -> Result<()> {
        let status_text = match status_code {
            200 => "OK",
            202 => "Accepted",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            413 => "Payload Too Large",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            503 => "Service Unavailable",
            _ => "Internal Server Error",
        };
        let response = format!(
            "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: {allowed_origin}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\nVary: Origin\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
        Ok(())
    }

    if !ctx.rate_limiter.allow(peer_ip) {
        return respond(
            &mut stream,
            429,
            "application/json",
            r#"{"error":"rate limit exceeded"}"#,
            &ctx.allowed_origin,
        )
        .await;
    }

    let mut buf = vec![0u8; MAX_HTTP_HEADER_BYTES];
    let mut total_read = 0;
    let mut header_end = None;

    while total_read < buf.len() {
        let n = tokio::time::timeout(HTTP_READ_TIMEOUT, stream.read(&mut buf[total_read..]))
            .await
            .context("HTTP header read timed out")??;
        if n == 0 {
            break;
        }
        total_read += n;
        if let Some(pos) = buf[..total_read].windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = Some(pos);
            break;
        }
    }

    let Some(header_pos) = header_end else {
        return respond(
            &mut stream,
            431,
            "application/json",
            r#"{"error":"request headers too large or incomplete"}"#,
            &ctx.allowed_origin,
        )
        .await;
    };

    let header_str = String::from_utf8_lossy(&buf[..header_pos]);
    let mut lines = header_str.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

    let mut content_length: usize = 0;
    let mut authorization = None;
    for line in lines {
        let lowercase = line.to_ascii_lowercase();
        if let Some(val) = lowercase.strip_prefix("content-length:") {
            if let Ok(len) = val.trim().parse::<usize>() {
                content_length = len;
            }
        }
        if lowercase.starts_with("authorization:") {
            authorization = line
                .split_once(':')
                .map(|(_, value)| value.trim().to_owned());
        }
    }

    if method == "OPTIONS" {
        return respond(&mut stream, 204, "text/plain", "", &ctx.allowed_origin).await;
    }

    if method == "POST" {
        if let Some(expected) = &ctx.rpc_token {
            let supplied = authorization
                .as_deref()
                .and_then(|value| value.strip_prefix("Bearer "));
            if supplied != Some(expected.as_str()) {
                return respond(
                    &mut stream,
                    401,
                    "application/json",
                    r#"{"error":"missing or invalid bearer token"}"#,
                    &ctx.allowed_origin,
                )
                .await;
            }
        }
    }

    if content_length > MAX_HTTP_BODY_BYTES {
        return respond(
            &mut stream,
            413,
            "application/json",
            r#"{"error":"request body exceeds 1 MiB limit"}"#,
            &ctx.allowed_origin,
        )
        .await;
    }

    let body_start = header_pos + 4;
    let already_read_body = total_read.saturating_sub(body_start);
    let mut body = Vec::with_capacity(content_length);
    if already_read_body > 0 {
        let to_take = already_read_body.min(content_length);
        body.extend_from_slice(&buf[body_start..body_start + to_take]);
    }

    while body.len() < content_length {
        let needed = content_length - body.len();
        let mut temp = vec![0u8; needed.min(8192)];
        let n = tokio::time::timeout(HTTP_READ_TIMEOUT, stream.read(&mut temp))
            .await
            .context("HTTP body read timed out")??;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }

    let (status_code, resp_json) = handle_http_request(&ctx, method, path, &body).await;
    let (content_type, body_string) = if path == "/v1/metrics" || path == "/metrics" {
        (
            "text/plain; version=0.0.4; charset=utf-8",
            resp_json
                .get("body")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        )
    } else {
        (
            "application/json",
            serde_json::to_string(&resp_json).unwrap_or_else(|_| "{}".to_owned()),
        )
    };

    respond(
        &mut stream,
        status_code,
        content_type,
        &body_string,
        &ctx.allowed_origin,
    )
    .await
}

async fn run_daemon(config: DaemonConfig) -> Result<()> {
    let DaemonConfig {
        dev_seed,
        key_file,
        committee_pubkeys,
        peers,
        bind,
        rpc,
        data_dir,
        dev_genesis,
        insecure_rpc,
    } = config;
    let key = match (dev_seed, key_file.as_deref()) {
        (Some(value), None) if (1..=4).contains(&value) => seed(value),
        (Some(_), None) => bail!("development seed must be between 1 and 4"),
        (None, Some(path)) => load_validator_key(path)?,
        _ => bail!("provide exactly one of --seed or --key-file"),
    };
    let committee_keys: BTreeMap<ValidatorId, Ed25519PublicKey> = if committee_pubkeys.is_empty() {
        if dev_seed.is_none() {
            bail!("--committee-pubkeys is required with --key-file");
        }
        (1..=4u8)
            .map(|value| {
                let key = seed(value);
                (ValidatorId(key.address()), key.public())
            })
            .collect()
    } else {
        parse_committee_pubkeys(&committee_pubkeys)?
    };
    let validators: BTreeSet<ValidatorId> = committee_keys.keys().copied().collect();
    let id = ValidatorId(key.address());
    if !validators.contains(&id) {
        bail!("validator key is not a member of the configured committee");
    }
    let committee = StaticCommittee::new(
        validators.iter().copied().collect(),
        (validators.len().saturating_sub(1)) / 3,
    );
    let is_val = |v: &ValidatorId| validators.contains(v);

    let data_dir = data_dir
        .or_else(|| std::env::var_os("VERIDAG_DATA_DIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("data"));
    let mut persistent_store = SledStore::open(&data_dir)
        .with_context(|| format!("open validator database at {}", data_dir.display()))?;

    let mut recovered_dag = Dag::new();
    let mut recovered_vertices = Vec::new();
    for vertex_id in persistent_store.iter_vertex_ids() {
        let bytes = persistent_store
            .get_vertex(&vertex_id)?
            .ok_or_else(|| anyhow::anyhow!("persisted vertex {vertex_id:?} disappeared"))?;
        let mut decoder = veridag_codec::Decoder::new(&bytes);
        let vertex = Vertex::decode(&mut decoder).context("decode persisted vertex")?;
        decoder
            .finish()
            .context("persisted vertex has trailing bytes")?;
        recovered_vertices.push(vertex);
    }
    recovered_vertices.sort_by_key(|vertex| (vertex.round, vertex.author));
    for vertex in &recovered_vertices {
        recovered_dag
            .add(
                vertex.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                0,
                is_val,
                committee.quorum(),
                &[],
            )
            .context("rebuild DAG from persistent store")?;
    }
    let mut proposed: BTreeSet<Round> = recovered_vertices
        .iter()
        .filter(|vertex| vertex.author == id)
        .map(|vertex| vertex.round)
        .collect();
    let dag = Arc::new(RwLock::new(recovered_dag));
    let mut batches: BTreeMap<BatchId, Vec<SignedTransaction>> = BTreeMap::new();

    let mut initial_state = ObjectState::new();
    let stored_objects = persistent_store.load_objects_checked()?;
    if stored_objects.is_empty() && dev_genesis {
        let alice = seed(100);
        let bob = seed(101);
        initial_state.create(Object::new(
            Object::derive_id(&alice.address(), 0),
            object_type::BALANCE,
            Ownership::Address(alice.address()),
            100u64.to_be_bytes().to_vec(),
            vec![],
        ))?;
        initial_state.create(Object::new(
            Object::derive_id(&bob.address(), 0),
            object_type::BALANCE,
            Ownership::Address(bob.address()),
            0u64.to_be_bytes().to_vec(),
            vec![],
        ))?;
        let objects: Vec<_> = initial_state
            .iter()
            .map(|(_, object)| object.clone())
            .collect();
        persistent_store.replace_all_objects(&objects)?;
        persistent_store.flush()?;
    } else {
        for object in stored_objects {
            initial_state
                .create(object)
                .context("load persisted object state")?;
        }
    }
    let state = Arc::new(RwLock::new(initial_state));
    let mut recovered_checkpoints = Vec::new();
    if let Some(latest_id) = persistent_store.latest() {
        let mut checkpoint_id = latest_id;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(checkpoint_id) {
                bail!("persisted checkpoint chain contains a cycle");
            }
            let bytes = persistent_store
                .get_checkpoint(&checkpoint_id)?
                .ok_or_else(|| {
                    anyhow::anyhow!("persisted checkpoint {checkpoint_id:?} is missing")
                })?;
            let mut decoder = veridag_codec::Decoder::new(&bytes);
            let checkpoint =
                Checkpoint::decode(&mut decoder).context("decode persisted checkpoint")?;
            decoder
                .finish()
                .context("persisted checkpoint has trailing bytes")?;
            if checkpoint.id() != checkpoint_id {
                bail!("persisted checkpoint id does not match its content");
            }
            let previous = checkpoint.previous_checkpoint;
            recovered_checkpoints.push(checkpoint);
            if previous == CheckpointId::ZERO {
                break;
            }
            checkpoint_id = previous;
        }
        recovered_checkpoints.reverse();
        for (index, checkpoint) in recovered_checkpoints.iter().enumerate() {
            let expected_sequence = index as u64 + 1;
            if checkpoint.sequence != expected_sequence {
                bail!("persisted checkpoint sequence is not contiguous");
            }
        }
    }
    let checkpoints = Arc::new(RwLock::new(recovered_checkpoints));
    let store = Arc::new(tokio::sync::Mutex::new(persistent_store));
    let (rpc_tx_sub, mut rpc_rx_sub) = tokio::sync::mpsc::channel::<SignedTransaction>(1024);

    let executor = Executor::new(0);

    let identity = veridag_net::Identity::from_keypair(&key)
        .context("create authenticated validator transport identity")?;

    // Resolve peers from hostnames (required for docker-compose)
    let mut peer_addrs = Vec::new();
    for p in &peers {
        let mut addrs = tokio::net::lookup_host(p)
            .await
            .with_context(|| format!("resolve configured peer {p}"))?;
        peer_addrs.push(
            addrs
                .next()
                .ok_or_else(|| anyhow::anyhow!("configured peer {p} resolved to no addresses"))?,
        );
    }

    let bind_addr = bind
        .parse()
        .with_context(|| format!("parse validator bind address {bind}"))?;
    let gossip = Arc::new(
        veridag_net::gossip::Gossip::bind(bind_addr, identity, validators.clone(), peer_addrs)
            .context("bind validator QUIC transport")?,
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<(u8, Vec<u8>)>(1024);
    let _recv = gossip.spawn_tagged_receiver(tx);

    // A clean restart can occur while the latest round is only partially
    // propagated. Re-advertise the two most recent persisted rounds so peers
    // can reconstruct a quorum and advance without waiting for new vertices
    // whose parents cannot yet be formed.
    if let Some(frontier) = dag.read().await.round_vertices_max() {
        let first_round = frontier.saturating_sub(1).max(1);
        for vertex in recovered_vertices
            .iter()
            .filter(|vertex| vertex.round >= first_round)
        {
            gossip.broadcast(vertex).await;
        }
    }

    let initial_sync_round = dag
        .read()
        .await
        .round_vertices_max()
        .unwrap_or(0)
        .saturating_add(1);
    gossip
        .broadcast_tagged(
            GOSSIP_TAG_SYNC_REQUEST,
            &encode_sync_request(initial_sync_round),
        )
        .await;

    info!(
        validator_id = %format!("0x{}", hex::encode(id.0)),
        bind = %bind,
        data_dir = %data_dir.display(),
        peers = peers.len(),
        "veridag-node daemon started"
    );

    let metrics = Arc::new(PrometheusExporter::new());
    let rpc_addr: SocketAddr = rpc
        .parse()
        .with_context(|| format!("parse RPC bind address {rpc}"))?;
    let rpc_token = std::env::var("VERIDAG_RPC_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty());
    if !rpc_addr.ip().is_loopback() && rpc_token.is_none() && !insecure_rpc {
        bail!(
            "refusing unauthenticated RPC on {rpc_addr}; set VERIDAG_RPC_TOKEN or pass --insecure-rpc for a development network"
        );
    }
    let allowed_origin = std::env::var("VERIDAG_RPC_ALLOWED_ORIGIN")
        .unwrap_or_else(|_| "http://localhost".to_owned());
    if allowed_origin == "*" && !insecure_rpc {
        bail!("wildcard RPC CORS requires --insecure-rpc");
    }
    let rpc_rate_limit = std::env::var("VERIDAG_RPC_RATE_LIMIT")
        .ok()
        .map(|value| value.parse::<u32>())
        .transpose()
        .context("VERIDAG_RPC_RATE_LIMIT must be a positive integer")?
        .unwrap_or(DEFAULT_RPC_RATE_LIMIT);
    if rpc_rate_limit == 0 {
        bail!("VERIDAG_RPC_RATE_LIMIT must be greater than zero");
    }
    let last_progress_ms = Arc::new(std::sync::atomic::AtomicU64::new(unix_time_millis()));

    // Spawn HTTP RPC server
    let rpc_ctx = Arc::new(RpcContext {
        dag: dag.clone(),
        state: state.clone(),
        checkpoints: checkpoints.clone(),
        tx_sender: rpc_tx_sub,
        committee: committee.clone(),
        validator_id: id,
        metrics: metrics.clone(),
        rpc_token,
        allowed_origin,
        rate_limiter: RpcRateLimiter::new(rpc_rate_limit),
        last_progress_ms: last_progress_ms.clone(),
        persistent: true,
    });

    let rpc_listener = TcpListener::bind(rpc_addr)
        .await
        .with_context(|| format!("bind RPC server at {rpc_addr}"))?;
    let rpc_task = tokio::spawn(async move {
        info!(addr = %rpc_addr, "RPC server listening");
        loop {
            match rpc_listener.accept().await {
                Ok((socket, peer)) => {
                    let ctx_clone = rpc_ctx.clone();
                    tokio::spawn(async move {
                        if let Err(error) =
                            serve_http_connection(socket, ctx_clone, peer.ip()).await
                        {
                            tracing::warn!(%error, client = %peer, "RPC connection failed");
                        }
                    });
                }
                Err(error) => {
                    tracing::warn!(%error, "RPC accept failed");
                }
            }
        }
    });

    let mut prev_mw = {
        let recovered = dag.read().await;
        highest_complete_wave(&recovered)
    };
    let mut executed_vertices = BTreeSet::new();
    let mut executed_anchors = BTreeSet::new();
    if prev_mw > 0 {
        let recovered = dag.read().await;
        for committed in commit(&recovered, &committee, prev_mw).committed {
            executed_anchors.insert(committed.anchor);
            executed_vertices.extend(committed.ordered);
        }
    }
    let mut pending_checkpoints: BTreeMap<CheckpointId, Checkpoint> = BTreeMap::new();
    let mut checkpoint_txids = Vec::new();
    let mut checkpoint_anchor_ids = Vec::new();
    let mut last_sync_request_at = Instant::now();
    let mut last_sync_response_at: Option<Instant> = None;
    loop {
        // Check for graceful shutdown signal.
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("received shutdown signal, draining and exiting");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
        }

        // Drain transactions submitted via RPC
        while let Ok(stx) = rpc_rx_sub.try_recv() {
            let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
            let batch_id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &tx_bytes));
            batches.entry(batch_id).or_insert_with(|| vec![stx]);
            gossip
                .broadcast_tagged(GOSSIP_TAG_TRANSACTION, &tx_bytes)
                .await;
        }

        let mut requested_sync_round: Option<Round> = None;
        let mut received_frames = VecDeque::new();
        while let Ok((tag, payload)) = rx.try_recv() {
            received_frames.push_back((tag, payload));
        }
        while let Some((tag, payload)) = received_frames.pop_front() {
            match tag {
                GOSSIP_TAG_VERTEX => {
                    let mut d = veridag_codec::Decoder::new(&payload);
                    if let Ok(v) = Vertex::decode(&mut d) {
                        if d.finish().is_ok() {
                            let vertex_id = v.id();
                            let vertex_bytes = veridag_codec::Encode::to_bytes(&v);
                            let mut d_write = dag.write().await;
                            let accepted = d_write.add(
                                v,
                                CURRENT_PROTOCOL_VERSION,
                                CHAIN,
                                0,
                                is_val,
                                committee.quorum(),
                                &[],
                            );
                            drop(d_write);
                            if accepted.is_ok() {
                                let mut persistent = store.lock().await;
                                persistent.put_vertex(vertex_id, &vertex_bytes)?;
                                persistent.flush()?;
                            }
                        }
                    }
                }
                GOSSIP_TAG_TRANSACTION => {
                    let mut d = veridag_codec::Decoder::new(&payload);
                    if let Ok(mstx) = SignedTransaction::decode(&mut d) {
                        if d.finish().is_ok() {
                            let mb = BatchId(veridag_crypto::hash(
                                "VERIDAG_BATCH_V1",
                                &veridag_codec::Encode::to_bytes(&mstx),
                            ));
                            batches.entry(mb).or_insert_with(|| vec![mstx]);
                        }
                    }
                }
                GOSSIP_TAG_CHECKPOINT => {
                    let mut decoder = veridag_codec::Decoder::new(&payload);
                    if let Ok(incoming) = Checkpoint::decode(&mut decoder) {
                        let structurally_valid = decoder.finish().is_ok()
                            && incoming.protocol_version == CURRENT_PROTOCOL_VERSION
                            && incoming.chain_id == CHAIN
                            && incoming.epoch == 0
                            && incoming.validator_set_commitment
                                == validator_set_commitment(
                                    &validators.iter().copied().collect::<Vec<_>>(),
                                )
                            && incoming
                                .verify_votes(|validator| committee_keys.get(validator).copied())
                                .is_ok();
                        if structurally_valid {
                            let checkpoint_id = incoming.id();
                            let pending =
                                pending_checkpoints.entry(checkpoint_id).or_insert_with(|| {
                                    let mut body = incoming.clone();
                                    body.finality_proof.votes.clear();
                                    body
                                });
                            let mut voters: BTreeSet<_> = pending
                                .finality_proof
                                .votes
                                .iter()
                                .map(|(validator, _)| *validator)
                                .collect();
                            for vote in incoming.finality_proof.votes {
                                if voters.insert(vote.0) {
                                    pending.add_vote(vote);
                                }
                            }
                        }
                    }
                }
                GOSSIP_TAG_SYNC_REQUEST => {
                    if let Some(round) = decode_sync_request(&payload) {
                        requested_sync_round =
                            Some(requested_sync_round.map_or(round, |current| current.min(round)));
                    }
                }
                GOSSIP_TAG_SYNC_RESPONSE => {
                    if let Some(items) = decode_sync_response(&payload) {
                        info!(
                            items = items.len(),
                            bytes = payload.len(),
                            "received DAG sync response"
                        );
                        received_frames.extend(items);
                    } else {
                        warn!(
                            bytes = payload.len(),
                            "discarded malformed DAG sync response"
                        );
                    }
                }
                _ => {}
            }
        }

        if let Some(start_round) = requested_sync_round {
            let may_respond = last_sync_response_at
                .is_none_or(|last_response| last_response.elapsed() >= SYNC_RESPONSE_INTERVAL);
            if may_respond {
                let vertices = {
                    let d_read = dag.read().await;
                    sync_vertices(&d_read, start_round)
                };
                let response = encode_sync_response(&vertices, &batches);
                let item_count =
                    u32::from_be_bytes(response[..4].try_into().expect("response count"));
                if item_count > 0 {
                    gossip
                        .broadcast_tagged(GOSSIP_TAG_SYNC_RESPONSE, &response)
                        .await;
                    info!(
                        start_round,
                        items = item_count,
                        bytes = response.len(),
                        "served DAG sync request"
                    );
                }
                last_sync_response_at = Some(Instant::now());
            }
        }

        let frontier = {
            let d_read = dag.read().await;
            d_read.round_vertices_max().unwrap_or(0)
        };

        for r in 1..=frontier + 1 {
            if proposed.contains(&r) {
                continue;
            }
            let (can, parents) = {
                let d_read = dag.read().await;
                let can = r == 1 || d_read.quorum_reached(r - 1, committee.quorum());
                let parents: Vec<VertexId> = if r == 1 {
                    Vec::new()
                } else {
                    d_read.round_vertices(r - 1).copied().collect()
                };
                (can, parents)
            };

            if !can {
                break;
            }

            let vbatches = batches.keys().copied().take(8).collect();

            if let Ok(v) = Vertex::new_signed(
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                0,
                r,
                id,
                parents,
                vbatches,
                Vec::new(),
                &key,
            ) {
                let mut d_write = dag.write().await;
                if d_write
                    .add(
                        v.clone(),
                        CURRENT_PROTOCOL_VERSION,
                        CHAIN,
                        0,
                        is_val,
                        committee.quorum(),
                        &[],
                    )
                    .is_ok()
                {
                    let vertex_id = v.id();
                    let vertex_bytes = veridag_codec::Encode::to_bytes(&v);
                    proposed.insert(r);
                    drop(d_write);
                    {
                        let mut persistent = store.lock().await;
                        persistent.put_vertex(vertex_id, &vertex_bytes)?;
                        persistent.flush()?;
                    }
                    gossip.broadcast(&v).await;
                    metrics.observe(Observation::Counter(Label("vertices_proposed"), 1));
                }
            }
        }

        let mw = {
            let d_read = dag.read().await;
            highest_complete_wave(&d_read)
        };

        if mw > prev_mw {
            let (txs, anchor_ids) = {
                let d_read = dag.read().await;
                let seq = commit(&d_read, &committee, mw);
                let mut txs = Vec::new();
                let mut anchor_ids = Vec::new();
                for a in &seq.committed {
                    if executed_anchors.insert(a.anchor) {
                        anchor_ids.push(a.anchor);
                    }
                    for vid in &a.ordered {
                        if executed_vertices.insert(*vid) {
                            let Some(v) = d_read.get(vid) else {
                                continue;
                            };
                            for b in &v.batch_commitments {
                                if let Some(bt) = batches.get(b) {
                                    txs.extend(bt.iter().cloned());
                                }
                            }
                        }
                    }
                }
                (txs, anchor_ids)
            };

            let state_root = if txs.is_empty() {
                state.read().await.state_root()
            } else {
                let mut s_write = state.write().await;
                let result = execute_parallel(&executor, &mut s_write, &txs);
                result.state_root
            };
            let objects: Vec<_> = state
                .read()
                .await
                .iter()
                .map(|(_, object)| object.clone())
                .collect();
            {
                let mut persistent = store.lock().await;
                persistent.replace_all_objects(&objects)?;
                persistent.flush()?;
            }
            last_progress_ms.store(unix_time_millis(), std::sync::atomic::Ordering::Relaxed);
            info!(
                wave = mw,
                state_root = %format!("0x{}", hex::encode(state_root)),
                txs = txs.len(),
                "wave committed"
            );
            metrics.observe(Observation::Counter(Label("waves_committed"), 1));
            metrics.observe(Observation::Counter(
                Label("txs_executed"),
                txs.len() as u64,
            ));
            checkpoint_txids.extend(txs.iter().map(SignedTransaction::id));
            checkpoint_anchor_ids.extend(anchor_ids);

            if mw % CHECKPOINT_INTERVAL_WAVES == 0 {
                let (sequence, previous_checkpoint) = {
                    let finalized = checkpoints.read().await;
                    (
                        finalized.len() as u64 + 1,
                        finalized
                            .last()
                            .map(Checkpoint::id)
                            .unwrap_or(CheckpointId::ZERO),
                    )
                };
                let validators_list: Vec<ValidatorId> = validators.iter().copied().collect();
                let mut checkpoint = Checkpoint::new(
                    CURRENT_PROTOCOL_VERSION,
                    CHAIN,
                    0,
                    sequence,
                    previous_checkpoint,
                    state_root,
                    veridag_execution::transaction_root(&checkpoint_txids),
                    dag_commitment(&checkpoint_anchor_ids),
                    validator_set_commitment(&validators_list),
                );
                checkpoint.add_vote(checkpoint.sign_vote(&key));
                let checkpoint_id = checkpoint.id();
                pending_checkpoints.insert(checkpoint_id, checkpoint.clone());
                gossip
                    .broadcast_tagged(
                        GOSSIP_TAG_CHECKPOINT,
                        &veridag_codec::Encode::to_bytes(&checkpoint),
                    )
                    .await;
            }
            prev_mw = mw;
        }

        // Sign matching checkpoint candidates and publish only checkpoints
        // that carry a cryptographically verified committee quorum.
        let current_root = state.read().await.state_root();
        let (next_sequence, previous_checkpoint) = {
            let finalized = checkpoints.read().await;
            (
                finalized.len() as u64 + 1,
                finalized
                    .last()
                    .map(Checkpoint::id)
                    .unwrap_or(CheckpointId::ZERO),
            )
        };
        let mut rebroadcast = Vec::new();
        for checkpoint in pending_checkpoints.values_mut() {
            let matches_local = checkpoint.sequence == next_sequence
                && checkpoint.previous_checkpoint == previous_checkpoint
                && checkpoint.state_root == current_root;
            let already_voted = checkpoint
                .finality_proof
                .votes
                .iter()
                .any(|(validator, _)| *validator == id);
            if matches_local && !already_voted {
                checkpoint.add_vote(checkpoint.sign_vote(&key));
                rebroadcast.push(veridag_codec::Encode::to_bytes(checkpoint));
            }
        }
        for bytes in rebroadcast {
            gossip.broadcast_tagged(GOSSIP_TAG_CHECKPOINT, &bytes).await;
        }

        let finalized_id = pending_checkpoints
            .iter()
            .find_map(|(checkpoint_id, checkpoint)| {
                let valid = checkpoint.sequence == next_sequence
                    && checkpoint.previous_checkpoint == previous_checkpoint
                    && checkpoint.state_root == current_root
                    && checkpoint
                        .verify_votes(|validator| committee_keys.get(validator).copied())
                        .is_ok()
                    && checkpoint
                        .verify_finality(
                            |validator| validators.contains(validator),
                            committee.quorum(),
                        )
                        .is_ok();
                valid.then_some(*checkpoint_id)
            });
        if let Some(checkpoint_id) = finalized_id {
            if let Some(checkpoint) = pending_checkpoints.remove(&checkpoint_id) {
                let checkpoint_bytes = veridag_codec::Encode::to_bytes(&checkpoint);
                {
                    let mut persistent = store.lock().await;
                    persistent.put_checkpoint(checkpoint_id, &checkpoint_bytes)?;
                    persistent.set_latest(checkpoint_id);
                    persistent.flush()?;
                }
                checkpoints.write().await.push(checkpoint);
                checkpoint_txids.clear();
                checkpoint_anchor_ids.clear();
                pending_checkpoints.retain(|_, candidate| candidate.sequence > next_sequence);
                metrics.observe(Observation::Counter(Label("checkpoints_produced"), 1));
            }
        }

        // Update gauges
        metrics.observe(Observation::Gauge(Label("highest_wave"), mw as i64));
        metrics.observe(Observation::Gauge(Label("max_round"), frontier as i64));

        let stalled_for_ms = unix_time_millis()
            .saturating_sub(last_progress_ms.load(std::sync::atomic::Ordering::Relaxed));
        if stalled_for_ms >= SYNC_RETRY_INTERVAL.as_millis() as u64
            && last_sync_request_at.elapsed() >= SYNC_RETRY_INTERVAL
        {
            let next_round = dag
                .read()
                .await
                .round_vertices_max()
                .unwrap_or(0)
                .saturating_add(1);
            warn!(
                next_round,
                stalled_for_ms, "requesting DAG catch-up from committee peers"
            );
            gossip
                .broadcast_tagged(GOSSIP_TAG_SYNC_REQUEST, &encode_sync_request(next_round))
                .await;
            last_sync_request_at = Instant::now();
        }
    }

    rpc_task.abort();
    {
        let persistent = store.lock().await;
        persistent.flush()?;
    }
    info!("shutdown complete");
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let use_json = std::env::var("VERIDAG_LOG_JSON").is_ok();

    if use_json {
        fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .init();
    } else {
        fmt().with_env_filter(filter).with_target(true).init();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Demo { validators } => {
            let nodes = run_demo(validators, false)?;
            let roots: Vec<_> = nodes.iter().map(|n| n.state.state_root()).collect();
            assert!(
                roots.iter().all(|r| *r == roots[0]),
                "demo: validators did not agree on state root"
            );
            let last = &nodes[0];
            println!("AGREEMENT OK: identical state root across {validators} validators");
            let bob = seed(101);
            let bob_id = ObjectId(Object::derive_id(&bob.address(), 0).0);
            let bal = last.state.balance(&bob_id).unwrap();
            println!("state_root=0x{}", hex::encode(last.state.state_root()));
            println!("checkpoints={}", last.checkpoints.len());
            for c in &last.checkpoints {
                println!(
                    "  checkpoint seq={} id=0x{}",
                    c.sequence,
                    hex::encode(c.id().0)
                );
            }
            println!("bob balance: {bal} (expected 40)");
            assert_eq!(bal, 40);
            Ok(())
        }
        Cmd::Health { json } => run_health(json),
        Cmd::Daemon {
            seed,
            key_file,
            committee_pubkeys,
            peers,
            bind,
            rpc,
            data_dir,
            dev_genesis,
            insecure_rpc,
        } => {
            run_daemon(DaemonConfig {
                dev_seed: seed,
                key_file,
                committee_pubkeys,
                peers,
                bind,
                rpc,
                data_dir,
                dev_genesis,
                insecure_rpc,
            })
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_request_codec_rejects_malformed_and_zero_rounds() {
        assert_eq!(decode_sync_request(&encode_sync_request(42)), Some(42));
        assert_eq!(decode_sync_request(&encode_sync_request(0)), None);
        assert_eq!(decode_sync_request(&[0; 7]), None);
        assert_eq!(decode_sync_request(&[0; 9]), None);
    }

    #[test]
    fn sync_response_codec_rejects_malformed_payloads() {
        assert_eq!(decode_sync_response(&[0, 0, 0, 0]), Some(Vec::new()));
        assert_eq!(decode_sync_response(&[0, 0, 0]), None);
        assert_eq!(decode_sync_response(&[0, 0, 0, 1, 99, 0, 0, 0, 0]), None);
        assert_eq!(
            decode_sync_response(&[0, 0, 0, 1, GOSSIP_TAG_VERTEX, 0, 0, 0, 2, 1]),
            None
        );
    }

    #[tokio::test]
    async fn test_rpc_endpoints_and_tcp_server() {
        let alice = Keypair::from_seed(&[100; 32]);
        let mut initial_state = ObjectState::new();
        initial_state
            .create(Object::new(
                Object::derive_id(&alice.address(), 0),
                object_type::BALANCE,
                Ownership::Address(alice.address()),
                100u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();

        let val_keys: Vec<Keypair> = (1..=4u8).map(|s| Keypair::from_seed(&[s; 32])).collect();
        let validators: Vec<ValidatorId> =
            val_keys.iter().map(|k| ValidatorId(k.address())).collect();
        let validator_id = validators[0];
        let committee = StaticCommittee::new(validators, 1);

        let (tx_sub, mut rx_sub) = tokio::sync::mpsc::channel(10);
        let rpc_ctx = Arc::new(RpcContext {
            dag: Arc::new(RwLock::new(Dag::new())),
            state: Arc::new(RwLock::new(initial_state)),
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            tx_sender: tx_sub,
            committee,
            validator_id,
            metrics: Arc::new(PrometheusExporter::new()),
            rpc_token: None,
            allowed_origin: "http://localhost".to_owned(),
            rate_limiter: RpcRateLimiter::new(100),
            last_progress_ms: Arc::new(std::sync::atomic::AtomicU64::new(unix_time_millis())),
            persistent: true,
        });

        // 1. Health endpoint
        let (code, health_json) = handle_http_request(&rpc_ctx, "GET", "/v1/health", &[]).await;
        assert_eq!(code, 200);
        assert_eq!(health_json["status"], "healthy");
        assert_eq!(health_json["chain_id"], 1);

        // 2. State root endpoint
        let (code, root_json) = handle_http_request(&rpc_ctx, "GET", "/v1/state/root", &[]).await;
        assert_eq!(code, 200);
        assert!(root_json["state_root"].as_str().unwrap().starts_with("0x"));

        // 3. Account balance endpoint
        let alice_addr_hex = hex::encode(alice.address());
        let (code, bal_json) = handle_http_request(
            &rpc_ctx,
            "GET",
            &format!("/v1/state/account/{}", alice_addr_hex),
            &[],
        )
        .await;
        assert_eq!(code, 200);
        assert_eq!(bal_json["balance"], 100);
        assert_eq!(bal_json["exists"], true);

        // 4. Transaction submission endpoint
        let bob = Keypair::from_seed(&[101; 32]);
        let stx = signed_transfer(&alice, 0, bob.address(), 25);
        let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
        let submit_body = serde_json::to_vec(&serde_json::json!({
            "raw_tx_hex": hex::encode(&tx_bytes),
            "public_key": hex::encode(alice.public())
        }))
        .unwrap();

        let (code, submit_json) =
            handle_http_request(&rpc_ctx, "POST", "/v1/tx/submit", &submit_body).await;
        assert_eq!(code, 202);
        assert_eq!(submit_json["status"], "admitted");
        assert!(rx_sub.recv().await.is_some());

        // 5. Live TCP integration test
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let ctx_srv = rpc_ctx.clone();

        tokio::spawn(async move {
            if let Ok((socket, peer)) = listener.accept().await {
                let _ = serve_http_connection(socket, ctx_srv, peer.ip()).await;
            }
        });

        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let req = b"GET /v1/health HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client.write_all(req).await.unwrap();

        let mut resp_buf = vec![0u8; 1024];
        let n = client.read(&mut resp_buf).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);
        assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
        assert!(resp_str.contains("\"status\":\"healthy\""));
    }
}
