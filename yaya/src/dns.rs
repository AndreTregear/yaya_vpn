use anyhow::Result;
use tracing::info;

use crate::config::Config;

/// Start the mesh-internal DNS resolver for .yaya domain
pub async fn start(config: &Config) -> Result<()> {
    info!("Starting .yaya internal DNS resolver");

    // The DNS resolver maps <nodename>.yaya → mesh IP
    // It runs on the mesh interface (100.64.0.x:53)

    // For the PoC, we write entries to a hosts-style file
    // and optionally configure systemd-resolved

    let hosts_path = config.config_dir().join("hosts");
    update_hosts_file(config, &hosts_path)?;

    // Try to configure systemd-resolved for .yaya domain
    if let Err(e) = configure_systemd_resolved(config) {
        info!("systemd-resolved not available: {e}. Using hosts file at {}", hosts_path.display());
        info!("Add to /etc/hosts or configure your DNS resolver manually.");
    }

    Ok(())
}

fn update_hosts_file(config: &Config, path: &std::path::Path) -> Result<()> {
    let mut content = String::from("# Yaya mesh DNS entries (auto-generated)\n");

    // Add self
    if let Some(ref ip) = config.mesh.ip {
        content.push_str(&format!(
            "{ip} self.yaya\n"
        ));
    }

    // Add peers
    for peer in &config.peers {
        if let Some(ref ip) = peer.mesh_ip {
            content.push_str(&format!(
                "{ip} {}.yaya\n",
                peer.name
            ));
        }
    }

    std::fs::write(path, &content)?;
    Ok(())
}

fn configure_systemd_resolved(config: &Config) -> Result<()> {
    let interface = config.interface_name();

    // Set DNS for the WireGuard interface to resolve .yaya
    let status = std::process::Command::new("resolvectl")
        .args(["domain", interface, "~yaya"])
        .status()?;

    if !status.success() {
        anyhow::bail!("resolvectl failed");
    }

    Ok(())
}
