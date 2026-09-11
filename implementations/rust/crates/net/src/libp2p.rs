//! libp2p transport backend (feature `libp2p`).
//!
//! Implements the consensus [`Transport`](super::Transport) trait over a public
//! libp2p stack (TCP + Noise + Yamux + Floodsub) built with libp2p 0.56's
//! [`SwarmBuilder`]. Because it satisfies the exact same trait as the default
//! QUIC transport, enabling it cannot change consensus semantics — that is the
//! whole point of the abstraction.
//!
//! Consensus safety invariant:
//! Core validator BFT consensus (vertices, batch commitments, checkpoint votes)
//! executes on direct, domain-separated authenticated QUIC TLS 1.3 channels.
//! The libp2p transport provides the public peer-to-peer plane for selective
//! transaction gossip, peer discovery, and light client relaying. Even if the
//! public libp2p swarm experiences network partitioning or Sybil flooding,
//! consensus safety and DAG finality remain mathematically uncompromised.
//!
//! Build with `cargo build -p veridag-net --features libp2p`. It is intentionally
//! excluded from the default/CI build so the heavy libp2p dependency tree never
//! risks the deterministic release build.

#![forbid(unsafe_code)]
// The `NetworkBehaviour` derive generates an event enum whose variants we do
// not document (they mirror the sub-behaviours). Allow it here.
#![allow(missing_docs)]

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use veridag_protocol_types::ValidatorId;

use libp2p::floodsub::{Behaviour as Floodsub, Event as FloodsubEvent, Topic};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{PeerId, Swarm};

use crate::transport::{Frame, Transport};

/// The libp2p topic all Veridag frames are published on.
pub const VERDAG_TOPIC: &str = "veridag-global";

/// Discovery and admission policy for the public P2P plane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoveryPolicy {
    /// Open public gossip plane: accepts frames from any discovered peer.
    Open,
    /// Selective discovery: only admits frames from authenticated peers in the allowlist.
    Allowlist(HashSet<PeerId>),
}

impl DiscoveryPolicy {
    /// Check whether a peer is permitted under this policy.
    pub fn is_allowed(&self, peer: &PeerId) -> bool {
        match self {
            DiscoveryPolicy::Open => true,
            DiscoveryPolicy::Allowlist(allowed) => allowed.contains(peer),
        }
    }
}

/// Behaviour: Floodsub for frame gossip.
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

/// A [`Transport`] backed by libp2p with selective discovery support.
pub struct Libp2pTransport {
    command_tx: Sender<Command>,
    frame_tx: broadcast::Sender<Frame>,
    validator_id: ValidatorId,
    local_addr: SocketAddr,
    policy: DiscoveryPolicy,
}

impl Libp2pTransport {
    /// Build a libp2p transport listening on `listen`, identified by `id`, with default Open policy.
    pub fn new(listen: SocketAddr, id: ValidatorId) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_policy(listen, id, DiscoveryPolicy::Open)
    }

    /// Build a libp2p transport with a selective [`DiscoveryPolicy`].
    pub fn with_policy(
        listen: SocketAddr,
        id: ValidatorId,
        policy: DiscoveryPolicy,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
                floodsub.subscribe(Topic::new(VERDAG_TOPIC));
                Ok(VeridagBehaviour { floodsub })
            })?
            .build();

        let addr: libp2p::Multiaddr = listen.to_string().parse()?;
        swarm.listen_on(addr)?;

        let (command_tx, command_rx) = channel::<Command>(256);
        let (frame_tx, _) = broadcast::channel::<Frame>(1024);

        let driver_policy = policy.clone();
        tokio::spawn(swarm_driver(swarm, command_rx, frame_tx.clone(), driver_policy));

        Ok(Self {
            command_tx,
            frame_tx,
            validator_id: id,
            local_addr: listen,
            policy,
        })
    }

    /// The active discovery policy of this transport.
    pub fn policy(&self) -> &DiscoveryPolicy {
        &self.policy
    }
}

/// The single-owner swarm driver. Polls the swarm, publishes broadcasts, and
/// forwards decoded frames to `frame_tx`, enforcing selective discovery policy.
async fn swarm_driver(
    mut swarm: Swarm<VeridagBehaviour>,
    mut command_rx: Receiver<Command>,
    frame_tx: broadcast::Sender<Frame>,
    policy: DiscoveryPolicy,
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
                            .publish(Topic::new(VERDAG_TOPIC), buf);
                    }
                    None => break,
                }
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::Behaviour(VeridagBehaviourEvent::Floodsub(
                    FloodsubEvent::Message(msg),
                )) = event
                {
                    // Enforce selective discovery filtering: drop messages from unapproved peers
                    if !policy.is_allowed(&msg.source) {
                        continue;
                    }
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
        let _ = self
            .command_tx
            .send(Command::Broadcast(frame.clone()))
            .await;
    }

    async fn subscribe(&self) -> Receiver<Frame> {
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
pub use libp2p::floodsub::Event;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_valid_frame() {
        let payload = vec![1, 2, 3, 4];
        let mut raw = vec![0x42];
        raw.extend_from_slice(&payload);
        let frame = decode_frame(&raw).expect("decode should succeed");
        assert_eq!(frame.tag, 0x42);
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn decode_empty_frame_fails() {
        assert!(decode_frame(&[]).is_none());
    }

    #[test]
    fn discovery_policy_filtering() {
        let peer1 = PeerId::random();
        let peer2 = PeerId::random();

        let open = DiscoveryPolicy::Open;
        assert!(open.is_allowed(&peer1));
        assert!(open.is_allowed(&peer2));

        let mut allowed = HashSet::new();
        allowed.insert(peer1);
        let selective = DiscoveryPolicy::Allowlist(allowed);

        assert!(selective.is_allowed(&peer1));
        assert!(!selective.is_allowed(&peer2));
    }

    #[test]
    fn peer_fingerprint_deterministic() {
        let peer = PeerId::random();
        assert_eq!(peer_fingerprint(&peer), peer_fingerprint(&peer));
    }
}
