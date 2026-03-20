use anyhow::Result;
use tracing::{info, warn};

use crate::config::Config;

pub struct TunnelStatus {
    pub mesh_ip: String,
    pub peer_count: usize,
    pub rx_bytes: String,
    pub tx_bytes: String,
}

pub struct TunnelManager {
    interface_name: String,
    listen_port: u16,
    private_key: [u8; 32],
}

impl TunnelManager {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            interface_name: config.interface_name().to_string(),
            listen_port: config.listen_port(),
            private_key: config.private_key_bytes()?,
        })
    }

    /// Create the WireGuard interface if it doesn't exist
    pub fn ensure_interface(&self) -> Result<()> {
        info!(interface = %self.interface_name, port = self.listen_port, "Ensuring WireGuard interface");

        // Use defguard_wireguard_rs to create interface
        // For now, fall back to wg-quick style setup via system commands
        if self.interface_exists()? {
            info!("Interface {} already exists", self.interface_name);
            return Ok(());
        }

        self.create_interface()?;
        Ok(())
    }

    fn interface_exists(&self) -> Result<bool> {
        let output = std::process::Command::new("ip")
            .args(["link", "show", &self.interface_name])
            .output()?;
        Ok(output.status.success())
    }

    fn create_interface(&self) -> Result<()> {
        info!("Creating WireGuard interface: {}", self.interface_name);

        // Create the WireGuard interface using ip link
        let status = std::process::Command::new("ip")
            .args(["link", "add", &self.interface_name, "type", "wireguard"])
            .status()?;

        if !status.success() {
            // Fall back to userspace (boringtun/gotatun) TUN device
            warn!("Kernel WireGuard unavailable, attempting userspace setup");
            return self.create_userspace_interface();
        }

        // Set private key via wg
        let key_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &self.private_key,
        );

        let mut child = std::process::Command::new("wg")
            .args(["set", &self.interface_name, "private-key", "/dev/stdin",
                   "listen-port", &self.listen_port.to_string()])
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            stdin.write_all(key_b64.as_bytes())?;
        }
        child.wait()?;

        // Bring interface up
        std::process::Command::new("ip")
            .args(["link", "set", &self.interface_name, "up"])
            .status()?;

        info!("WireGuard interface {} created and up", self.interface_name);
        Ok(())
    }

    fn create_userspace_interface(&self) -> Result<()> {
        // Create TUN device and run GotaTun/boringtun in userspace
        // This is the fallback for systems without kernel WireGuard
        info!("Setting up userspace WireGuard via boringtun-cli");

        let key_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &self.private_key,
        );

        // Try boringtun-cli (or gotatun-cli)
        let status = std::process::Command::new("boringtun-cli")
            .args([
                &self.interface_name,
                "--disable-drop-privileges", "root",
            ])
            .env("WG_SUDO", "1")
            .status();

        match status {
            Ok(s) if s.success() => {
                // Now configure it with wg
                let mut child = std::process::Command::new("wg")
                    .args(["set", &self.interface_name, "private-key", "/dev/stdin",
                           "listen-port", &self.listen_port.to_string()])
                    .stdin(std::process::Stdio::piped())
                    .spawn()?;

                if let Some(ref mut stdin) = child.stdin {
                    use std::io::Write;
                    stdin.write_all(key_b64.as_bytes())?;
                }
                child.wait()?;

                std::process::Command::new("ip")
                    .args(["link", "set", &self.interface_name, "up"])
                    .status()?;

                Ok(())
            }
            _ => {
                anyhow::bail!(
                    "Cannot create WireGuard interface. Install wireguard kernel module \
                     or boringtun-cli. See: https://github.com/mullvad/gotatun"
                );
            }
        }
    }

    /// Add a WireGuard peer
    pub fn add_peer(&self, pubkey: &str, endpoint: &str, mesh_ip: &str) -> Result<()> {
        info!(pubkey = %pubkey, endpoint = %endpoint, mesh_ip = %mesh_ip, "Adding WireGuard peer");

        // Add peer via wg set
        std::process::Command::new("wg")
            .args([
                "set", &self.interface_name,
                "peer", pubkey,
                "endpoint", endpoint,
                "allowed-ips", &format!("{mesh_ip}/32"),
                "persistent-keepalive", "25",
            ])
            .status()?;

        // Add route
        std::process::Command::new("ip")
            .args([
                "route", "add", &format!("{mesh_ip}/32"),
                "dev", &self.interface_name,
            ])
            .status()?;

        Ok(())
    }

    /// Remove a WireGuard peer
    pub fn remove_peer(&self, pubkey: &str) -> Result<()> {
        info!(pubkey = %pubkey, "Removing WireGuard peer");

        std::process::Command::new("wg")
            .args(["set", &self.interface_name, "peer", pubkey, "remove"])
            .status()?;

        Ok(())
    }

    /// Set peer as exit node (AllowedIPs = 0.0.0.0/0)
    pub fn set_exit_peer(&self, pubkey: &str, endpoint: &str) -> Result<()> {
        info!(pubkey = %pubkey, "Setting peer as exit node");

        std::process::Command::new("wg")
            .args([
                "set", &self.interface_name,
                "peer", pubkey,
                "endpoint", endpoint,
                "allowed-ips", "0.0.0.0/0,::/0",
                "persistent-keepalive", "25",
            ])
            .status()?;

        // Add default route via WireGuard
        std::process::Command::new("ip")
            .args(["route", "add", "0.0.0.0/1", "dev", &self.interface_name])
            .status()?;
        std::process::Command::new("ip")
            .args(["route", "add", "128.0.0.0/1", "dev", &self.interface_name])
            .status()?;

        Ok(())
    }

    /// Remove exit routing
    pub fn clear_exit_route(&self) -> Result<()> {
        std::process::Command::new("ip")
            .args(["route", "del", "0.0.0.0/1", "dev", &self.interface_name])
            .status()?;
        std::process::Command::new("ip")
            .args(["route", "del", "128.0.0.0/1", "dev", &self.interface_name])
            .status()?;
        Ok(())
    }

    /// Get interface status
    pub fn status(&self) -> Result<TunnelStatus> {
        let output = std::process::Command::new("wg")
            .args(["show", &self.interface_name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("WireGuard interface not active");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        let peer_count = stdout.matches("peer:").count();
        let mut rx_total = 0u64;
        let mut tx_total = 0u64;

        for line in stdout.lines() {
            let line = line.trim();
            if let Some(transfer) = line.strip_prefix("transfer:") {
                // Parse "X.XX KiB received, Y.YY KiB sent"
                let parts: Vec<&str> = transfer.split(',').collect();
                if parts.len() == 2 {
                    rx_total += parse_transfer_bytes(parts[0]);
                    tx_total += parse_transfer_bytes(parts[1]);
                }
            }
        }

        Ok(TunnelStatus {
            mesh_ip: "100.64.0.1".to_string(), // TODO: read from config
            peer_count,
            rx_bytes: format_bytes(rx_total),
            tx_bytes: format_bytes(tx_total),
        })
    }

    pub fn interface_name(&self) -> &str {
        &self.interface_name
    }
}

fn parse_transfer_bytes(s: &str) -> u64 {
    let s = s.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() >= 2 {
        let val: f64 = parts[0].parse().unwrap_or(0.0);
        let unit = parts[1].to_lowercase();
        match unit.as_str() {
            "b" => val as u64,
            "kib" => (val * 1024.0) as u64,
            "mib" => (val * 1024.0 * 1024.0) as u64,
            "gib" => (val * 1024.0 * 1024.0 * 1024.0) as u64,
            _ => val as u64,
        }
    } else {
        0
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
