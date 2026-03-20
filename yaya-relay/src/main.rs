use anyhow::Result;
use quinn::{Endpoint, ServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use std::collections::HashMap;

type PeerId = String;
type PeerMap = Arc<RwLock<HashMap<PeerId, quinn::Connection>>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("yaya_relay=info".parse()?))
        .init();

    let listen: SocketAddr = std::env::var("YAYA_RELAY_LISTEN")
        .unwrap_or_else(|_| "0.0.0.0:4433".into())
        .parse()?;

    let (server_config, _cert_der) = generate_self_signed_config()?;
    let endpoint = Endpoint::server(server_config, listen)?;

    info!(listen = %listen, "Yaya relay server listening");

    let peers: PeerMap = Arc::new(RwLock::new(HashMap::new()));

    while let Some(incoming) = endpoint.accept().await {
        let peers = peers.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(incoming, peers).await {
                warn!(error = %e, "Connection error");
            }
        });
    }

    Ok(())
}

async fn handle_connection(incoming: quinn::Incoming, peers: PeerMap) -> Result<()> {
    let conn = incoming.await?;
    let remote = conn.remote_address();
    info!(remote = %remote, "New relay connection");

    // Read the peer's identity from the first bi-directional stream
    let (mut send, mut recv) = conn.accept_bi().await?;

    let mut id_buf = [0u8; 256];
    let n = recv
        .read(&mut id_buf)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Empty peer ID"))?;
    let peer_id = String::from_utf8_lossy(&id_buf[..n]).to_string();

    info!(peer_id = %peer_id, "Peer registered for relay");

    // Store the connection
    peers.write().await.insert(peer_id.clone(), conn.clone());

    // Relay loop: accept streams and forward to target peer
    loop {
        match conn.accept_bi().await {
            Ok((mut incoming_send, mut incoming_recv)) => {
                // Read target peer ID
                let mut target_buf = [0u8; 256];
                let n = match incoming_recv.read(&mut target_buf).await? {
                    Some(n) => n,
                    None => continue,
                };
                let target_id = String::from_utf8_lossy(&target_buf[..n]).to_string();

                let peers_read = peers.read().await;
                if let Some(target_conn) = peers_read.get(&target_id) {
                    // Open a stream to the target and relay data
                    let (mut target_send, mut target_recv) = target_conn.open_bi().await?;

                    // Relay bidirectionally
                    tokio::spawn(async move {
                        let _ = tokio::io::copy(&mut incoming_recv, &mut target_send).await;
                    });
                    tokio::spawn(async move {
                        let _ = tokio::io::copy(&mut target_recv, &mut incoming_send).await;
                    });
                } else {
                    warn!(target = %target_id, "Target peer not connected to relay");
                }
            }
            Err(e) => {
                info!(peer_id = %peer_id, error = %e, "Peer disconnected");
                peers.write().await.remove(&peer_id);
                break;
            }
        }
    }

    Ok(())
}

fn generate_self_signed_config() -> Result<(ServerConfig, Vec<u8>)> {
    let cert = rcgen::generate_simple_self_signed(vec!["relay.yaya.sh".into()])?;
    let cert_der = cert.cert.der().to_vec();
    let key_der = cert.key_pair.serialize_der();

    let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der.clone())];
    let key = rustls::pki_types::PrivateKeyDer::try_from(key_der)
        .map_err(|e| anyhow::anyhow!("Invalid private key: {e}"))?;

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)?;

    server_crypto.alpn_protocols = vec![b"yaya-relay/1".to_vec()];

    let server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
    ));

    Ok((server_config, cert_der))
}
