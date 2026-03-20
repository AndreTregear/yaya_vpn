use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

const MESH_SUBNET: u32 = 0x64400000; // 100.64.0.0
const MESH_PREFIX: u8 = 10; // /10

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    config_dir: PathBuf,
    pub identity: Identity,
    #[serde(default)]
    pub mesh: MeshConfig,
    #[serde(default)]
    pub peers: Vec<PeerConfig>,
    #[serde(default)]
    pub coordinator: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Identity {
    pub private_key: String, // base64-encoded Curve25519 private key
    pub public_key: String,  // base64-encoded Curve25519 public key
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MeshConfig {
    pub ip: Option<String>,
    pub interface: Option<String>,
    pub listen_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub name: String,
    pub public_key: String,
    pub endpoint: String,
    pub mesh_ip: Option<String>,
    #[serde(default)]
    pub is_exit: bool,
}

impl Config {
    pub fn default_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("yaya")
    }

    pub fn init() -> Result<Self> {
        let config_dir = Self::default_config_dir();
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config dir: {}", config_dir.display()))?;

        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            return Self::load();
        }

        // Generate Curve25519 keypair
        let private = x25519_dalek::StaticSecret::random_from_rng(rand::thread_rng());
        let public = x25519_dalek::PublicKey::from(&private);

        let identity = Identity {
            private_key: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                private.as_bytes(),
            ),
            public_key: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                public.as_bytes(),
            ),
        };

        let config = Config {
            config_dir: config_dir.clone(),
            identity,
            mesh: MeshConfig {
                ip: None,
                interface: Some("wg-yaya".to_string()),
                listen_port: Some(51820),
            },
            peers: vec![],
            coordinator: None,
        };

        let toml_str = toml::to_string_pretty(&config)?;
        fs::write(&config_path, &toml_str)?;

        // Set restrictive permissions on config (contains private key)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let config_dir = Self::default_config_dir();
        let config_path = config_dir.join("config.toml");

        if !config_path.exists() {
            anyhow::bail!(
                "Yaya not initialized. Run `yaya init` first.\n  Expected config at: {}",
                config_path.display()
            );
        }

        let content = fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&content)?;
        config.config_dir = config_dir;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = self.config_dir.join("config.toml");
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(&config_path, &toml_str)?;
        Ok(())
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn public_key_base64(&self) -> String {
        self.identity.public_key.clone()
    }

    pub fn private_key_bytes(&self) -> Result<[u8; 32]> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.identity.private_key)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }

    pub fn interface_name(&self) -> &str {
        self.mesh.interface.as_deref().unwrap_or("wg-yaya")
    }

    pub fn listen_port(&self) -> u16 {
        self.mesh.listen_port.unwrap_or(51820)
    }

    pub fn add_peer(&self, name: &str, pubkey: &str, endpoint: &str) -> Result<()> {
        let mut config = Self::load()?;
        if config.peers.iter().any(|p| p.public_key == pubkey) {
            anyhow::bail!("Peer with this public key already exists");
        }
        config.peers.push(PeerConfig {
            name: name.to_string(),
            public_key: pubkey.to_string(),
            endpoint: endpoint.to_string(),
            mesh_ip: None,
            is_exit: false,
        });
        config.save()
    }

    pub fn remove_peer(&self, name_or_key: &str) -> Result<()> {
        let mut config = Self::load()?;
        let len_before = config.peers.len();
        config.peers.retain(|p| p.name != name_or_key && p.public_key != name_or_key);
        if config.peers.len() == len_before {
            anyhow::bail!("Peer not found: {name_or_key}");
        }
        config.save()
    }

    pub fn list_peers(&self) -> Result<Vec<PeerConfig>> {
        Ok(self.peers.clone())
    }

    pub fn get_peer(&self, name_or_key: &str) -> Result<PeerConfig> {
        self.peers
            .iter()
            .find(|p| p.name == name_or_key || p.public_key == name_or_key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Peer not found: {name_or_key}"))
    }

    pub fn allocate_mesh_ip(&self) -> Result<String> {
        // Simple allocation: hash the public key to get an IP in 100.64.0.0/10
        let existing: Vec<u32> = self
            .peers
            .iter()
            .filter_map(|p| {
                p.mesh_ip.as_ref().and_then(|ip| {
                    ip.parse::<Ipv4Addr>().ok().map(|a| u32::from(a))
                })
            })
            .collect();

        // Start from 100.64.0.2 (100.64.0.1 is reserved for self)
        for offset in 2u32..((1u32 << (32 - MESH_PREFIX)) - 1) {
            let candidate = MESH_SUBNET + offset;
            if !existing.contains(&candidate) {
                let ip = Ipv4Addr::from(candidate);
                return Ok(ip.to_string());
            }
        }
        anyhow::bail!("No available mesh IPs")
    }

    pub fn rosenpass_dir(&self) -> PathBuf {
        self.config_dir.join("rosenpass")
    }
}
