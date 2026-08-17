//! Vertex gossip over authenticated QUIC links (Phase 5 propagation).
//!
//! A [`Gossip`] binds a [`ValidatorLink`], dials known peers, and exchanges
//! VCE-1-encoded DAG vertices as bounded frames. Receiving is validity-
//! agnostic: frames are decoded and handed to the caller, which inserts them
//! into its `veridag-dag` (the DAG enforces all validity rules). This keeps
//! consensus semantics out of the transport.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::mpsc;
use veridag_codec::{Decode, Decoder, Encode};
use veridag_dag::Vertex;
use veridag_protocol_types::ValidatorId;

use crate::{read_frame, write_frame, Identity, NetError, ValidatorLink};

/// A gossip endpoint for one validator.
pub struct Gossip {
    link: Arc<ValidatorLink>,
    identity: Arc<Identity>,
    peers: Vec<SocketAddr>,
}

impl Gossip {
    /// Bind a gossip endpoint for `identity` on `bind`, authenticating against
    /// the committee `validators`, dialing the given `peers`.
    pub fn bind(
        bind: SocketAddr,
        identity: Identity,
        validators: BTreeSet<ValidatorId>,
        peers: Vec<SocketAddr>,
    ) -> Result<Self, NetError> {
        let link = ValidatorLink::bind(bind, &identity, validators)?;
        Ok(Self {
            link: Arc::new(link),
            identity: Arc::new(identity),
            peers,
        })
    }

    /// The local bound address.
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.link.local_addr()
    }

    /// Consume this gossip endpoint and return one with the given peer list,
    /// keeping the already-bound socket. Useful when addresses must be learned
    /// before the full peer set is known.
    pub fn with_peers(self, peers: Vec<SocketAddr>) -> Self {
        Self {
            link: self.link,
            identity: self.identity,
            peers,
        }
    }

    /// Broadcast a tagged message to all peers (best-effort; a down peer is
    /// skipped). Opens a fresh connection per peer per broadcast. The send is
    /// awaited so the frame is flushed before the stream finishes; finishing
    /// immediately after `write_all` can drop the data. The connection stays
    /// open until the peer closes it after reading.
    pub async fn broadcast_tagged(&self, tag: u8, payload: &[u8]) {
        let mut frame_payload = Vec::with_capacity(1 + payload.len());
        frame_payload.push(tag);
        frame_payload.extend_from_slice(payload);
        let mut sends = Vec::with_capacity(self.peers.len());
        for peer in &self.peers {
            let link = self.link.clone();
            let identity = self.identity.clone();
            let bytes = frame_payload.clone();
            let peer = *peer;
            sends.push(tokio::spawn(async move {
                if let Ok(conn) = link.connect(peer, &identity).await {
                    if let Ok(mut send) = conn.open_uni().await {
                        if write_frame(&mut send, &bytes).await.is_ok() {
                            let _ = send.finish();
                        }
                    }
                    // Keep the connection alive until the peer closes it after
                    // reading, ensuring the frame is delivered before teardown.
                    let _ = conn.closed().await;
                }
            }));
        }
        for s in sends {
            let _ = s.await;
        }
    }

    /// Broadcast a vertex (tag 0) to all peers.
    pub async fn broadcast(&self, v: &Vertex) {
        self.broadcast_tagged(0, &v.to_bytes()).await;
    }

    /// Spawn an accept loop that forwards every received tagged message to
    /// `tx` as `(tag, payload)`. Returns the join handle.
    pub fn spawn_tagged_receiver(
        self: &Arc<Self>,
        tx: mpsc::Sender<(u8, Vec<u8>)>,
    ) -> tokio::task::JoinHandle<()> {
        let this = self.clone();
        tokio::spawn(async move {
            loop {
                let Some(conn) = this.link.accept().await else {
                    break;
                };
                let tx = tx.clone();
                tokio::spawn(async move {
                    // One broadcast = one connection = one uni stream = one
                    // frame. Read it, forward it, then close so the sender's
                    // `closed()` resolves and both tasks terminate cleanly.
                    if let Ok(mut recv) = conn.accept_uni().await {
                        if let Ok(frame) = read_frame(&mut recv).await {
                            if !frame.is_empty() {
                                let tag = frame[0];
                                let payload = frame[1..].to_vec();
                                let _ = tx.send((tag, payload)).await;
                            }
                        }
                    }
                    conn.close(0u32.into(), b"read-done");
                });
            }
        })
    }

    /// Spawn an accept loop that forwards every received vertex to `tx`.
    /// Returns the join handle; drop `tx` to stop.
    pub fn spawn_receiver(
        self: &Arc<Self>,
        tx: mpsc::Sender<Vertex>,
    ) -> tokio::task::JoinHandle<()> {
        let (tag_tx, mut tag_rx) = mpsc::channel::<(u8, Vec<u8>)>(1024);
        let handle = self.spawn_tagged_receiver(tag_tx);
        tokio::spawn(async move {
            while let Some((0, payload)) = tag_rx.recv().await {
                let mut d = Decoder::new(&payload);
                if let Ok(v) = Vertex::decode(&mut d) {
                    if d.finish().is_ok() && tx.send(v).await.is_err() {
                        break;
                    }
                }
            }
        });
        handle
    }
}
