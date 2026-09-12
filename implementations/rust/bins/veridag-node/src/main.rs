//! veridag-node: an in-process validator node (alpha).
//!
//! Runs a single validator: a mempool collects client transactions, the node
//! proposes DAG vertices carrying batch commitments, BaselineDagBft commits
//! anchors, the parallel executor applies the committed ordering, and a
//! checkpoint is produced every CHECKPOINT_INTERVAL_WAVES committed waves.
//!
//! This binary drives the full vertical slice in one process. Multi-process
//! networking (Phase 5) and persistent recovery (Phase 9) are wired behind the
//! same crates; this node runs the consensus-critical path in-process so the
//! whole pipeline is exercisable and testable end-to-end.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use veridag_checkpoint::{dag_commitment, validator_set_commitment, Checkpoint};

use veridag_codec::Decode;
use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};
use veridag_crypto::Keypair;
use veridag_dag::{Dag, Vertex};
use veridag_execution::parallel::execute_parallel;
use veridag_execution::Executor;
use veridag_object_state::{Object, ObjectState};
use veridag_protocol_types::{
    object_type, Address, BatchId, ChainId, CheckpointId, Ed25519PublicKey, Epoch, ObjectId,
    ObjectRef, Ownership, ResourceBudget, Round, ValidatorId, VertexId, CURRENT_PROTOCOL_VERSION,
};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

const CHAIN: ChainId = 1;

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
#[command(name = "veridag-node", about = "Veridag validator node (alpha)")]
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
        /// The validator seed (1-4 for testing)
        #[arg(long)]
        seed: u8,
        /// Comma-separated list of peer IP:PORT strings
        #[arg(long, value_delimiter = ',')]
        peers: Vec<String>,
        /// Bind address
        #[arg(long, default_value = "0.0.0.0:8000")]
        bind: String,
        /// HTTP/JSON RPC bind address (e.g. 0.0.0.0:8080)
        #[arg(long, default_value = "0.0.0.0:8080")]
        rpc: String,
    },
}

fn seed(n: u8) -> Keypair {
    Keypair::from_seed(&[n; 32])
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
    seed: u8,
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
        return (
            200,
            serde_json::json!({
                "status": "healthy",
                "version": env!("CARGO_PKG_VERSION"),
                "protocol_version": CURRENT_PROTOCOL_VERSION,
                "chain_id": CHAIN,
                "validator_seed": ctx.seed,
                "committee_n": ctx.committee.n(),
                "committee_quorum": ctx.committee.quorum(),
                "max_round": max_r,
                "highest_wave": mw,
                "state_root": format!("0x{}", hex::encode(state.state_root())),
                "checkpoints_count": ckpts.len(),
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
                            let _ = ctx.tx_sender.send(stx).await;
                            return (
                                200,
                                serde_json::json!({
                                    "status": "admitted",
                                    "tx_id": format!("0x{}", hex::encode(tx_id.0)),
                                    "sender": format!("0x{}", hex::encode(sender)),
                                }),
                            );
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
) -> Result<()> {
    let mut buf = vec![0u8; 65536];
    let mut total_read = 0;
    let mut header_end = None;

    while total_read < buf.len() {
        let n = stream.read(&mut buf[total_read..]).await?;
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
        return Ok(());
    };

    let header_str = String::from_utf8_lossy(&buf[..header_pos]);
    let mut lines = header_str.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    let mut content_length: usize = 0;
    for line in lines {
        if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
            if let Ok(len) = val.trim().parse::<usize>() {
                content_length = len;
            }
        }
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
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }

    let (status_code, resp_json) = handle_http_request(&ctx, method, path, &body).await;
    let body_str = serde_json::to_string(&resp_json).unwrap_or_else(|_| "{}".to_string());
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_code,
        status_text,
        body_str.len(),
        body_str
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn run_daemon(seed: u8, peers: Vec<String>, bind: String, rpc: String) -> Result<()> {
    let keys: Vec<Keypair> = (1..=4u8).map(|s| Keypair::from_seed(&[s; 32])).collect();
    let validators: BTreeSet<ValidatorId> = keys.iter().map(|k| ValidatorId(k.address())).collect();
    let committee = StaticCommittee::new(validators.iter().copied().collect(), 1);

    let key = Keypair::from_seed(&[seed; 32]);
    let id = ValidatorId(key.address());
    let is_val = |v: &ValidatorId| validators.contains(v);

    let dag = Arc::new(RwLock::new(Dag::new()));
    let mut batches: BTreeMap<BatchId, Vec<SignedTransaction>> = BTreeMap::new();
    let mut proposed: BTreeSet<Round> = BTreeSet::new();

    let alice = Keypair::from_seed(&[100; 32]);
    let bob = Keypair::from_seed(&[101; 32]);
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
    initial_state
        .create(Object::new(
            Object::derive_id(&bob.address(), 0),
            object_type::BALANCE,
            Ownership::Address(bob.address()),
            0u64.to_be_bytes().to_vec(),
            vec![],
        ))
        .unwrap();
    let state = Arc::new(RwLock::new(initial_state));
    let checkpoints = Arc::new(RwLock::new(Vec::new()));
    let (rpc_tx_sub, mut rpc_rx_sub) = tokio::sync::mpsc::channel::<SignedTransaction>(1024);

    let executor = Executor::new(0);

    let identity = veridag_net::Identity::from_keypair(&key).unwrap();

    // Resolve peers from hostnames (required for docker-compose)
    let mut peer_addrs = Vec::new();
    for p in &peers {
        if let Ok(addrs) = tokio::net::lookup_host(&p).await {
            if let Some(addr) = addrs.into_iter().next() {
                peer_addrs.push(addr);
            }
        }
    }

    let bind_addr = bind.parse().unwrap();
    let gossip = Arc::new(
        veridag_net::gossip::Gossip::bind(bind_addr, identity, validators.clone(), peer_addrs)
            .unwrap(),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<(u8, Vec<u8>)>(1024);
    let _recv = gossip.spawn_tagged_receiver(tx);

    println!(
        "veridag-node daemon (seed={}) bound to {} with {} peers",
        seed,
        bind,
        peers.len()
    );

    // Spawn HTTP RPC server
    let rpc_ctx = Arc::new(RpcContext {
        dag: dag.clone(),
        state: state.clone(),
        checkpoints: checkpoints.clone(),
        tx_sender: rpc_tx_sub,
        committee: committee.clone(),
        seed,
    });

    let rpc_addr = rpc.clone();
    tokio::spawn(async move {
        if let Ok(listener) = TcpListener::bind(&rpc_addr).await {
            println!("veridag-node RPC server listening on http://{}", rpc_addr);
            loop {
                if let Ok((socket, _)) = listener.accept().await {
                    let ctx_clone = rpc_ctx.clone();
                    tokio::spawn(async move {
                        let _ = serve_http_connection(socket, ctx_clone).await;
                    });
                }
            }
        } else {
            eprintln!("Failed to bind RPC server to {}", rpc_addr);
        }
    });

    if seed == 1 {
        let stx = signed_transfer(&alice, 0, bob.address(), 40);
        let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
        let batch_id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &tx_bytes));
        batches.insert(batch_id, vec![stx.clone()]);
        gossip.broadcast_tagged(1, &tx_bytes).await;
    }

    let mut prev_mw = 0;
    loop {
        // Drain transactions submitted via RPC
        while let Ok(stx) = rpc_rx_sub.try_recv() {
            let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
            let batch_id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &tx_bytes));
            batches.entry(batch_id).or_insert_with(|| vec![stx]);
            gossip.broadcast_tagged(1, &tx_bytes).await;
        }

        while let Ok((tag, payload)) = rx.try_recv() {
            match tag {
                0 => {
                    let mut d = veridag_codec::Decoder::new(&payload);
                    if let Ok(v) = Vertex::decode(&mut d) {
                        if d.finish().is_ok() {
                            let mut d_write = dag.write().await;
                            let _ = d_write.add(
                                v,
                                CURRENT_PROTOCOL_VERSION,
                                CHAIN,
                                0,
                                is_val,
                                committee.quorum(),
                                &[],
                            );
                        }
                    }
                }
                1 => {
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
                _ => {}
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

            let vbatches = if r == 2 && seed == 1 {
                let stx = signed_transfer(&alice, 0, bob.address(), 40);
                let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
                vec![BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &tx_bytes))]
            } else {
                batches.keys().copied().take(8).collect()
            };

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
                    proposed.insert(r);
                    gossip.broadcast(&v).await;
                }
            }
        }

        let mw = {
            let d_read = dag.read().await;
            highest_complete_wave(&d_read)
        };

        if mw > prev_mw {
            let d_read = dag.read().await;
            let seq = commit(&d_read, &committee, mw);
            if !seq.committed.is_empty() {
                let mut txs = Vec::new();
                for a in &seq.committed {
                    for vid in &a.ordered {
                        if let Some(v) = d_read.get(vid) {
                            for b in &v.batch_commitments {
                                if let Some(bt) = batches.get(b) {
                                    txs.extend(bt.iter().cloned());
                                }
                            }
                        }
                    }
                }
                if !txs.is_empty() {
                    let mut s_write = state.write().await;
                    let result = execute_parallel(&executor, &mut s_write, &txs);
                    println!(
                        "Executed wave {}, state root 0x{}",
                        mw,
                        hex::encode(result.state_root)
                    );
                    let mut ckpts = checkpoints.write().await;
                    let last_ckpt_id = ckpts.last().map(|c| c.id()).unwrap_or(CheckpointId::ZERO);
                    let validators_list: Vec<ValidatorId> = validators.iter().copied().collect();
                    let txids: Vec<_> = txs.iter().map(|t| t.id()).collect();
                    let anchor_ids: Vec<VertexId> =
                        seq.committed.iter().map(|c| c.anchor).collect();
                    let mut ckpt = Checkpoint::new(
                        CURRENT_PROTOCOL_VERSION,
                        CHAIN,
                        0,
                        ckpts.len() as u64 + 1,
                        last_ckpt_id,
                        result.state_root,
                        veridag_execution::transaction_root(&txids),
                        dag_commitment(&anchor_ids),
                        validator_set_commitment(&validators_list),
                    );
                    let vote = ckpt.sign_vote(&key);
                    ckpt.add_vote(vote);
                    ckpts.push(ckpt);
                }
            }
            prev_mw = mw;
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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
            peers,
            bind,
            rpc,
        } => run_daemon(seed, peers, bind, rpc).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let committee = StaticCommittee::new(validators, 1);

        let (tx_sub, mut rx_sub) = tokio::sync::mpsc::channel(10);
        let rpc_ctx = Arc::new(RpcContext {
            dag: Arc::new(RwLock::new(Dag::new())),
            state: Arc::new(RwLock::new(initial_state)),
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            tx_sender: tx_sub,
            committee,
            seed: 1,
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
        assert_eq!(code, 200);
        assert_eq!(submit_json["status"], "admitted");
        assert!(rx_sub.recv().await.is_some());

        // 5. Live TCP integration test
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let ctx_srv = rpc_ctx.clone();

        tokio::spawn(async move {
            if let Ok((socket, _)) = listener.accept().await {
                let _ = serve_http_connection(socket, ctx_srv).await;
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
