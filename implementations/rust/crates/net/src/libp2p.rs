//! libp2p transport backend (feature `libp2p`).
//!
//! Implements the consensus [`Transport`](super::Transport) trait over a public
//! libp2p stack (TCP + Noise + Yamux + Floodsub) built with libp2p 0.54's
//! [`SwarmBuilder`]. Because it satisfies the exact same trait as the default
//! QUIC transport, enabling it cannot change consensus semantics — that is the
//! whole point of the abstraction.
//!
//! Architecture: the `Swarm` is owned by a single background task. `broadcast`
//! sends a command over an mpsc channel to that task; the task publishes to the
//! Floodsub topic. Incoming Floodsub messages are decoded back into [`Frame`]s
//! and forwarded (via a broadcast channel) to every `subscribe` caller.
//!
//! Build with `cargo build -p veridag-net --features libp2p`. It is intentionally
//! excluded from the default/CI build so the heavy libp2p dependency tree never
//! risks the deterministic release build.

#![forbid(unsafe_code)]
// The `NetworkBehaviour` derive generates an event enum whose variants we do
// not document (they mirror the sub-behaviours). Allow it here.
#![allow(missing_docs)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::broadcast;
use veridag_protocol_types::ValidatorId;

use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{PeerId, Swarm};

use crate::transport::{Frame, Transport};

/// The libp2p topic all Veridag frames are published on.
pub const VERDAG_TOPIC: &str = "veridag-global";

/// Behaviour: Floodsub for frame gossip. (Peer discovery is out of scope for
/// the alpha backend; peers are dialed manually or via a bootstrap list.)
#[derive(NetworkBehaviour)]
pub struct VeridagBehaviour {
    /// Floodsub sub-behaviour carrying gossiped [`Frame`]s on the Veridag topic.
    pub floodsub: Floodsub,
}

/// Commands sent from the public API to the swarm driver task.
enum Command {
    /// Broadcast a [`Frame`] to all subscribed peers via Floodsub.
    Broadcast(Frame),
}

/// A [`Transport`] backed by libp2p.
pub struct Libp2pTransport {
    command_tx: Sender<Command>,
    frame_tx: broadcast::Sender<Frame>,
    validator_id: ValidatorId,
    local_addr: SocketAddr,
}

impl Libp2pTransport {
    /// Build a libp2p transport listening on `listen`, identified by `id`.
    /// Spawns the swarm driver task.
    pub fn new(listen: SocketAddr, id: ValidatorId) -> Result<Self, Box<dyn std::error::Error>> {
        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                Default::default(),
                (libp2p::noise::Config::new, libp2p::noise::Config::new),
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let peer_id = key.public().to_peer_id();
                let mut floodsub = Floodsub::new(peer_id);
                floodsub.subscribe(Topic::new("veridag-global"));
                Ok(VeridagBehaviour { floodsub })
            })?
            .build();

        let addr: libp2p::Multiaddr = listen.to_string().parse()?;
        swarm.listen_on(addr)?;

        let (command_tx, command_rx) = channel::<Command>(256);
        let (frame_tx, _) = broadcast::channel::<Frame>(1024);

        tokio::spawn(swarm_driver(swarm, command_rx, frame_tx.clone()));

        Ok(Self {
            command_tx,
            frame_tx,
            validator_id: id,
            local_addr: listen,
        })
    }
}

/// The single-owner swarm driver. Polls the swarm, publishes broadcasts, and
/// forwards decoded frames to `frame_tx`.
async fn swarm_driver(
    mut swarm: Swarm<VeridagBehaviour>,
    mut command_rx: Receiver<Command>,
    frame_tx: broadcast::Sender<Frame>,
) {
    loop {
        tokio::select! {
            cmd = command_rx.recv() => {
                match cmd {
                    Some(Command::Broadcast(frame)) => {
                        let mut buf = Vec::with_capacity(1 + frame.payload.len());
                        buf.push(frame.tag);
                        buf.extend_from_slice(&frame.payload);
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(Topic::new("veridag-global"), buf);
                    }
                    None => break,
                }
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::Behaviour(VeridagBehaviourEvent::Floodsub(
                    FloodsubEvent::Message(msg),
                )) = event
                {
                    if let Some(frame) = decode_frame(&msg.data) {
                        let _ = frame_tx.send(frame);
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Transport for Libp2pTransport {
    async fn broadcast(&self, frame: &Frame) {
        let _ = self.command_tx.send(Command::Broadcast(frame.clone())).await;
    }

    async fn subscribe(&self) -> Receiver<Frame> {
        // Fan-out: every subscriber receives the same frames via the broadcast
        // channel. Lagging subscribers (slower than the 1024-deep buffer) are
        // intentionally dropped to bound memory under backpressure.
        let mut rx = self.frame_tx.subscribe();
        let (out_tx, out_rx) = channel::<Frame>(1024);
        tokio::spawn(async move {
            while let Ok(frame) = rx.recv().await {
                if out_tx.send(frame).await.is_err() {
                    break;
                }
            }
        });
        out_rx
    }

    fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    fn validator_id(&self) -> ValidatorId {
        self.validator_id
    }
}

/// Decode a Floodsub payload back into a [`Frame`].
pub fn decode_frame(data: &[u8]) -> Option<Frame> {
    if data.is_empty() {
        return None;
    }
    let tag = data[0];
    let payload = data[1..].to_vec();
    Some(Frame { tag, payload })
}

/// Stable hash helper for peer/topic keys (used by discovery logging).
pub fn peer_fingerprint(peer: &PeerId) -> u64 {
    let mut h = DefaultHasher::new();
    peer.hash(&mut h);
    h.finish()
}

// Re-export the Floodsub event type so callers can drive custom loops.
pub use libp2p::floodsub::FloodsubEvent as Event;

#[allow(dead_code)]
fn _assert_event_variant(e: &FloodsubEvent) -> bool {
    matches!(e, FloodsubEvent::Message(_))
}
