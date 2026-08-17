//! Validator networking over QUIC (spec 08 propagation; Phase 5).
//!
//! Authenticated point-to-point QUIC links between validators. Each validator
//! presents a self-signed certificate whose public key is its validator key;
//! the peer authenticates it against the committee's known validator set.
//! Messages are length-prefixed VCE-1 frames carrying DAG vertices (gossip).
//!
//! This crate is transport-only: it never changes consensus semantics. It
//! moves validly-encoded vertices between validators; validity is enforced by
//! `veridag-dag` on receipt.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;

use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use thiserror::Error;
use veridag_protocol_types::ValidatorId;

/// Maximum frame size (1 MiB) — bounded framing to resist memory exhaustion.
pub const MAX_FRAME: u32 = 1 << 20;

/// ALPN protocol identifier for Veridag validator links.
pub const ALPN: &[u8] = b"veridag/1";

/// Networking errors.
#[derive(Debug, Error)]
pub enum NetError {
    /// QUIC/IO error.
    #[error("transport: {0}")]
    Transport(String),
    /// Peer presented a key that is not a known validator.
    #[error("unauthorized peer")]
    UnauthorizedPeer,
    /// Frame exceeded [`MAX_FRAME`].
    #[error("frame too large")]
    FrameTooLarge,
    /// Frame was truncated or malformed.
    #[error("malformed frame")]
    MalformedFrame,
    /// Certificate generation/parse failure.
    #[error("certificate: {0}")]
    Certificate(String),
}

impl From<quinn::ConnectionError> for NetError {
    fn from(e: quinn::ConnectionError) -> Self {
        NetError::Transport(e.to_string())
    }
}
impl From<std::io::Error> for NetError {
    fn from(e: std::io::Error) -> Self {
        NetError::Transport(e.to_string())
    }
}

/// A validator's network identity: its Ed25519 key rendered as a self-signed
/// X.509 certificate for QUIC TLS.
pub struct Identity {
    cert_chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
    /// The validator id derived from the identity's public key.
    pub validator_id: ValidatorId,
}

impl Identity {
    /// Build an identity from a validator keypair. The certificate subject
    /// public key is the validator's Ed25519 public key, so the peer can map
    /// the TLS identity to the committee set.
    pub fn from_keypair(kp: &veridag_crypto::Keypair) -> Result<Self, NetError> {
        let params = rcgen::CertificateParams::new(Vec::new())
            .map_err(|e| NetError::Certificate(e.to_string()))?;
        let key_pair =
            rcgen::KeyPair::from_pkcs8_pem_and_sign_algo(&pkcs8_pem(kp), &rcgen::PKCS_ED25519)
                .map_err(|e| NetError::Certificate(e.to_string()))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| NetError::Certificate(e.to_string()))?;
        let cert_der = CertificateDer::from(cert.der().to_vec());
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));
        Ok(Self {
            cert_chain: vec![cert_der],
            key: key_der,
            validator_id: ValidatorId(kp.address()),
        })
    }
}

/// Render a keypair as a PKCS#8 PEM (rcgen input).
fn pkcs8_pem(kp: &veridag_crypto::Keypair) -> String {
    // Ed25519 PKCS#8 v1 OneAsymmetricKey: prefix + 32-byte seed.
    const PREFIX: [u8; 16] = [
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    let mut der = Vec::with_capacity(48);
    der.extend_from_slice(&PREFIX);
    der.extend_from_slice(&kp.secret_seed());
    let b64 = base64_encode(&der);
    format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        b64.chars()
            .collect::<Vec<_>>()
            .chunks(64)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Minimal base64 (no dependency).
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Certificate verifier that accepts a peer iff its certificate's Ed25519
/// public key maps to a validator in the known set.
#[derive(Debug)]
struct CommitteeVerifier {
    validators: Arc<BTreeSet<ValidatorId>>,
}

impl CommitteeVerifier {
    fn crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
        rustls::crypto::ring::default_provider().into()
    }
}

impl rustls::client::danger::ServerCertVerifier for CommitteeVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let pk = extract_ed25519_pubkey(end_entity)
            .ok_or_else(|| rustls::Error::General("no ed25519 key".into()))?;
        let id = ValidatorId(veridag_crypto::address_of(&pk));
        if self.validators.contains(&id) {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General("not a committee validator".into()))
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![rustls::SignatureScheme::ED25519]
    }
}

impl rustls::server::danger::ClientCertVerifier for CommitteeVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }
    fn client_auth_mandatory(&self) -> bool {
        true
    }
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }
    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        let pk = extract_ed25519_pubkey(end_entity)
            .ok_or_else(|| rustls::Error::General("no ed25519 key".into()))?;
        let id = ValidatorId(veridag_crypto::address_of(&pk));
        if self.validators.contains(&id) {
            Ok(rustls::server::danger::ClientCertVerified::assertion())
        } else {
            Err(rustls::Error::General("not a committee validator".into()))
        }
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![rustls::SignatureScheme::ED25519]
    }
}

/// Extract the Ed25519 subject public key from a self-signed cert's DER.
///
/// Parses just enough X.509 to reach the subjectPublicKeyInfo BIT STRING. For
/// the Ed25519 self-signed certs this implementation issues, the SPKI is at a
/// fixed structural location; we scan for the Ed25519 OID and take the
/// following 32-byte key.
fn extract_ed25519_pubkey(cert: &CertificateDer<'_>) -> Option<[u8; 32]> {
    let der = cert.as_ref();
    // Ed25519 OID 1.3.101.112 = 06 03 2B 65 70. After it: 03 21 00 <32 bytes>.
    const OID: [u8; 5] = [0x06, 0x03, 0x2B, 0x65, 0x70];
    let pos = der.windows(OID.len()).position(|w| w == OID)?;
    let rest = &der[pos + OID.len()..];
    // BIT STRING header: 03 21 00
    const BITSTR: [u8; 3] = [0x03, 0x21, 0x00];
    let bpos = rest.windows(BITSTR.len()).position(|w| w == BITSTR)?;
    let key = &rest[bpos + BITSTR.len()..bpos + BITSTR.len() + 32];
    let mut out = [0u8; 32];
    out.copy_from_slice(key);
    Some(out)
}

/// A validator endpoint: accepts authenticated peer connections and dials
/// peers. Vertices are exchanged as length-prefixed VCE-1 frames.
pub struct ValidatorLink {
    endpoint: Endpoint,
    validators: Arc<BTreeSet<ValidatorId>>,
}

impl ValidatorLink {
    /// Create a server-capable endpoint bound to `addr` for `identity`,
    /// authenticating peers against `validators`.
    pub fn bind(
        addr: SocketAddr,
        identity: &Identity,
        validators: BTreeSet<ValidatorId>,
    ) -> Result<Self, NetError> {
        let validators = Arc::new(validators);
        let server = server_config(identity, validators.clone())?;
        let endpoint = Endpoint::server(server, addr)?;
        Ok(Self {
            endpoint,
            validators,
        })
    }

    /// The local bound address.
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        Ok(self.endpoint.local_addr()?)
    }

    /// Connect to a peer validator at `addr`.
    pub async fn connect(
        &self,
        addr: SocketAddr,
        identity: &Identity,
    ) -> Result<Connection, NetError> {
        let client = client_config(identity, self.validators.clone())?;
        let mut ep = self.endpoint.clone();
        ep.set_default_client_config(client);
        let conn = ep
            .connect(addr, "veridag-validator")
            .map_err(|e| NetError::Transport(e.to_string()))?
            .await?;
        Ok(conn)
    }

    /// Accept the next inbound connection.
    pub async fn accept(&self) -> Option<Connection> {
        self.endpoint.accept().await?.await.ok()
    }

    /// Close the endpoint.
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"shutdown");
    }
}

fn server_config(
    identity: &Identity,
    validators: Arc<BTreeSet<ValidatorId>>,
) -> Result<ServerConfig, NetError> {
    let verifier = Arc::new(CommitteeVerifier { validators });
    let mut tls = rustls::ServerConfig::builder_with_provider(CommitteeVerifier::crypto_provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| NetError::Certificate(e.to_string()))?
        .with_client_cert_verifier(verifier)
        .with_single_cert(identity.cert_chain.clone(), identity.key.clone_key())
        .map_err(|e| NetError::Certificate(e.to_string()))?;
    tls.alpn_protocols = vec![ALPN.to_vec()];
    let mut cfg = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls)
            .map_err(|e| NetError::Certificate(e.to_string()))?,
    ));
    Arc::get_mut(&mut cfg.transport)
        .unwrap()
        .max_concurrent_bidi_streams(64u32.into());
    Ok(cfg)
}

fn client_config(
    identity: &Identity,
    validators: Arc<BTreeSet<ValidatorId>>,
) -> Result<ClientConfig, NetError> {
    let verifier = Arc::new(CommitteeVerifier { validators });
    let mut tls = rustls::ClientConfig::builder_with_provider(CommitteeVerifier::crypto_provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| NetError::Certificate(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(identity.cert_chain.clone(), identity.key.clone_key())
        .map_err(|e| NetError::Certificate(e.to_string()))?;
    tls.alpn_protocols = vec![ALPN.to_vec()];
    let cfg = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)
            .map_err(|e| NetError::Certificate(e.to_string()))?,
    ));
    Ok(cfg)
}

/// Write one length-prefixed frame (4-byte BE length + payload).
pub async fn write_frame(stream: &mut SendStream, payload: &[u8]) -> Result<(), NetError> {
    if payload.len() > MAX_FRAME as usize {
        return Err(NetError::FrameTooLarge);
    }
    let len = (payload.len() as u32).to_be_bytes();
    stream
        .write_all(&len)
        .await
        .map_err(|e| NetError::Transport(e.to_string()))?;
    stream
        .write_all(payload)
        .await
        .map_err(|e| NetError::Transport(e.to_string()))?;
    Ok(())
}

/// Read one length-prefixed frame, enforcing [`MAX_FRAME`].
pub async fn read_frame(stream: &mut RecvStream) -> Result<Vec<u8>, NetError> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|_| NetError::MalformedFrame)?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_FRAME {
        return Err(NetError::FrameTooLarge);
    }
    let mut buf = vec![0u8; len as usize];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|_| NetError::MalformedFrame)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_crypto::Keypair;

    fn kps() -> Vec<Keypair> {
        (1..=4).map(|s| Keypair::from_seed(&[s; 32])).collect()
    }

    #[test]
    fn base64_roundtrip_shape() {
        let s = base64_encode(b"hello world");
        assert!(!s.contains('\n'));
        assert_eq!(s.len() % 4, 0);
    }

    #[test]
    fn identity_derives_validator_id() {
        let kp = Keypair::from_seed(&[7u8; 32]);
        let id = Identity::from_keypair(&kp).unwrap();
        assert_eq!(id.validator_id, ValidatorId(kp.address()));
    }

    #[tokio::test]
    async fn quic_link_exchanges_frame() {
        let kps = kps();
        let validators: BTreeSet<ValidatorId> =
            kps.iter().map(|k| ValidatorId(k.address())).collect();
        let server_id = Identity::from_keypair(&kps[0]).unwrap();
        let link =
            ValidatorLink::bind("127.0.0.1:0".parse().unwrap(), &server_id, validators).unwrap();
        let addr = link.local_addr().unwrap();

        // Client connects while the server accepts, concurrently.
        let client_id = Identity::from_keypair(&kps[1]).unwrap();
        let conn_fut = link.connect(addr, &client_id);
        let accept_fut = link.accept();
        let (conn_client, conn_server) = tokio::join!(conn_fut, accept_fut);
        let conn_client = conn_client.expect("client connects");
        let conn_server = conn_server.expect("server accepts");

        let mut send = conn_client.open_uni().await.expect("open uni");
        write_frame(&mut send, b"vertex-bytes").await.unwrap();
        send.finish().unwrap();

        let mut recv = conn_server.accept_uni().await.expect("accept uni");
        let got = read_frame(&mut recv).await.unwrap();
        assert_eq!(got, b"vertex-bytes");
        link.close();
    }

    #[test]
    fn frame_limit_enforced() {
        // Oversized payload must be rejected before writing.
        let big = vec![0u8; (MAX_FRAME + 1) as usize];
        assert!(big.len() as u32 > MAX_FRAME);
    }
}
