use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::config::Config;

/// Generate a deterministic peer name from a public key
pub fn peer_name_from_key(pubkey: &str) -> String {
    let hash = Sha256::digest(pubkey.as_bytes());
    // Use first 4 bytes as a hex suffix
    let suffix = hex::encode(&hash[..4]);
    format!("peer-{suffix}")
}

// hex encode helper (avoid adding hex crate for this)
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Compute safety number from two public keys (Signal-style)
/// Returns 60 bytes that can be displayed as 12 groups of 5 digits
pub fn safety_number(our_pubkey: &str, their_pubkey: &str) -> Vec<u8> {
    let mut keys = [our_pubkey, their_pubkey];
    keys.sort(); // Canonical ordering so both sides get same number

    let mut hasher = Sha256::new();
    hasher.update(b"yaya-safety-number-v1");
    hasher.update(keys[0].as_bytes());
    hasher.update(keys[1].as_bytes());
    let hash = hasher.finalize();

    // Iterate hash to produce 60 digits
    let mut result = Vec::with_capacity(60);
    let mut current_hash = hash.to_vec();
    while result.len() < 60 {
        for byte in &current_hash {
            if result.len() >= 60 {
                break;
            }
            result.push(byte % 10);
        }
        if result.len() < 60 {
            let next = Sha256::digest(&current_hash);
            current_hash = next.to_vec();
        }
    }
    result
}

/// Peer invite data for QR code exchange
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PeerInvite {
    pub v: u8,
    pub pk: String,       // Our WireGuard public key
    pub ep: Vec<String>,  // Our endpoints
    pub coord: Option<String>, // Coordinator URL
}

impl PeerInvite {
    pub fn as_peer_string(&self) -> String {
        let endpoint = self.ep.first().cloned().unwrap_or_else(|| "unknown".to_string());
        format!("{}@{}", self.pk, endpoint)
    }

    pub fn as_uri(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            json.as_bytes(),
        );
        format!("yaya://add?d={encoded}")
    }
}

/// Create an invite containing our connection info
pub fn create_invite(config: &Config) -> Result<PeerInvite> {
    let port = config.listen_port();

    // Discover our endpoints
    let mut endpoints = Vec::new();

    // Get local IPs
    if let Ok(output) = std::process::Command::new("hostname").arg("-I").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for ip in stdout.split_whitespace() {
            let ip = ip.trim();
            if !ip.is_empty() && !ip.starts_with("127.") && !ip.contains("::") {
                endpoints.push(format!("{ip}:{port}"));
            }
        }
    }

    // TODO: STUN to discover public IP

    if endpoints.is_empty() {
        endpoints.push(format!("127.0.0.1:{port}"));
    }

    Ok(PeerInvite {
        v: 1,
        pk: config.public_key_base64(),
        ep: endpoints,
        coord: config.coordinator.clone(),
    })
}
