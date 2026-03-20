use anyhow::Result;
use std::process::Command;
use tracing::info;

use crate::config::Config;
use crate::tunnel::TunnelManager;

/// Configure this node as an exit node
pub async fn serve(config: &Config) -> Result<()> {
    info!("Configuring exit node...");

    // Enable IP forwarding
    Command::new("sysctl")
        .args(["-w", "net.ipv4.ip_forward=1"])
        .status()?;

    // Add masquerade rule for mesh traffic
    // Use the mesh subnet 100.64.0.0/10
    Command::new("iptables")
        .args([
            "-t", "nat",
            "-A", "POSTROUTING",
            "-s", "100.64.0.0/10",
            "-o", default_interface()?.as_str(),
            "-j", "MASQUERADE",
            "-m", "comment", "--comment", "yaya-exit",
        ])
        .status()?;

    // Accept forwarded traffic
    Command::new("iptables")
        .args([
            "-A", "FORWARD",
            "-i", config.interface_name(),
            "-j", "ACCEPT",
            "-m", "comment", "--comment", "yaya-exit",
        ])
        .status()?;

    Command::new("iptables")
        .args([
            "-A", "FORWARD",
            "-o", config.interface_name(),
            "-m", "state", "--state", "RELATED,ESTABLISHED",
            "-j", "ACCEPT",
            "-m", "comment", "--comment", "yaya-exit",
        ])
        .status()?;

    info!("Exit node active");
    Ok(())
}

/// Route traffic through an exit node
pub async fn use_exit(
    config: &Config,
    node: Option<&str>,
    rotate_secs: Option<u64>,
) -> Result<()> {
    let tunnel = TunnelManager::new(config)?;

    // Find exit nodes
    let exit_peers: Vec<_> = config.peers.iter().filter(|p| p.is_exit).collect();

    if exit_peers.is_empty() {
        if let Some(name) = node {
            // Use specified peer as exit even if not flagged
            let peer = config.get_peer(name)?;
            tunnel.set_exit_peer(&peer.public_key, &peer.endpoint)?;
            println!("Routing all traffic through {name}");
        } else {
            anyhow::bail!(
                "No exit nodes available. Ask a peer to run `yaya exit serve`, \
                 or specify a peer: `yaya exit use <peer-name>`"
            );
        }
        return Ok(());
    }

    match rotate_secs {
        Some(interval) => {
            println!("Rotating exit node every {}s", interval);
            let mut idx = 0;
            loop {
                let peer = &exit_peers[idx % exit_peers.len()];
                tunnel.clear_exit_route().ok(); // Ignore if no route exists yet
                tunnel.set_exit_peer(&peer.public_key, &peer.endpoint)?;
                println!("Now routing through: {} ({})", peer.name, peer.endpoint);

                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                idx += 1;
            }
        }
        None => {
            let peer = if let Some(name) = node {
                exit_peers
                    .iter()
                    .find(|p| p.name == name)
                    .ok_or_else(|| anyhow::anyhow!("Exit node not found: {name}"))?
            } else {
                exit_peers.first().unwrap()
            };
            tunnel.set_exit_peer(&peer.public_key, &peer.endpoint)?;
            println!("Routing all traffic through: {} ({})", peer.name, peer.endpoint);
            Ok(())
        }
    }
}

/// Stop exit node functionality
pub async fn stop(config: &Config) -> Result<()> {
    // Remove iptables rules
    cleanup_iptables()?;

    // Remove exit routing
    let tunnel = TunnelManager::new(config)?;
    tunnel.clear_exit_route().ok();

    // Disable IP forwarding
    Command::new("sysctl")
        .args(["-w", "net.ipv4.ip_forward=0"])
        .status()?;

    Ok(())
}

fn cleanup_iptables() -> Result<()> {
    // Remove all yaya-exit rules
    // This is a simplified cleanup — production would track rule numbers
    let tables = ["nat", "filter"];
    let chains = [("nat", "POSTROUTING"), ("filter", "FORWARD")];

    for (table, chain) in &chains {
        loop {
            let output = Command::new("iptables")
                .args([
                    "-t", table, "-L", chain, "--line-numbers", "-n",
                ])
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let yaya_line = stdout.lines().find(|l| l.contains("yaya-exit"));

            if let Some(line) = yaya_line {
                if let Some(num) = line.split_whitespace().next() {
                    Command::new("iptables")
                        .args(["-t", table, "-D", chain, num])
                        .status()?;
                    continue;
                }
            }
            break;
        }
    }
    Ok(())
}

/// Detect the default network interface
fn default_interface() -> Result<String> {
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse "default via X.X.X.X dev eth0 ..."
    for part in stdout.split_whitespace().collect::<Vec<_>>().windows(2) {
        if part[0] == "dev" {
            return Ok(part[1].to_string());
        }
    }

    anyhow::bail!("Could not detect default network interface")
}
