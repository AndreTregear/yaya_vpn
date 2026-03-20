use anyhow::Result;
use std::process::Command;
use tracing::info;

use crate::config::Config;

/// Expose a local port through an exit node's public IP
pub async fn expose_port(config: &Config, local_port: u16) -> Result<()> {
    // Find an exit node to expose through
    let exit_peer = config
        .peers
        .iter()
        .find(|p| p.is_exit)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No exit node available. Ask a peer to run `yaya exit serve`, \
                 or specify one with --via"
            )
        })?;

    let peer_mesh_ip = exit_peer
        .mesh_ip
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Exit peer has no mesh IP assigned"))?;

    // Allocate a public port on the exit node
    // For now, use the same port number (in production, this would be negotiated)
    let public_port = local_port;

    info!(
        local_port = local_port,
        exit_node = %exit_peer.name,
        "Requesting port exposure"
    );

    // Set up DNAT on the exit node via a control message over the mesh
    // For the PoC, we'll configure it locally if we ARE the exit node,
    // or instruct the user to run the setup on the exit node
    setup_exposure_rules(config, local_port, public_port, peer_mesh_ip)?;

    println!(
        "Exposing localhost:{local_port} via exit node \"{}\" ({})",
        exit_peer.name, exit_peer.endpoint
    );
    println!(
        "Public URL: http://{}:{public_port}",
        exit_peer.endpoint.split(':').next().unwrap_or("unknown")
    );
    println!("Press Ctrl+C to stop.");

    // Keep running until interrupted
    tokio::signal::ctrl_c().await?;

    // Cleanup
    cleanup_exposure_rules(local_port, public_port)?;
    println!("\nExposure stopped.");

    Ok(())
}

/// Set up DNAT and forwarding rules on the exit node
fn setup_exposure_rules(
    config: &Config,
    local_port: u16,
    public_port: u16,
    target_mesh_ip: &str,
) -> Result<()> {
    let interface = config.interface_name();

    // DNAT: incoming traffic on public_port → mesh IP target
    Command::new("iptables")
        .args([
            "-t", "nat",
            "-A", "PREROUTING",
            "-p", "tcp",
            "--dport", &public_port.to_string(),
            "-j", "DNAT",
            "--to-destination", &format!("{target_mesh_ip}:{local_port}"),
            "-m", "comment", "--comment", &format!("yaya-expose-{local_port}"),
        ])
        .status()?;

    // Allow forwarded traffic
    Command::new("iptables")
        .args([
            "-A", "FORWARD",
            "-p", "tcp",
            "-d", target_mesh_ip,
            "--dport", &local_port.to_string(),
            "-j", "ACCEPT",
            "-m", "comment", "--comment", &format!("yaya-expose-{local_port}"),
        ])
        .status()?;

    Ok(())
}

/// Remove exposure DNAT rules
fn cleanup_exposure_rules(local_port: u16, public_port: u16) -> Result<()> {
    let comment = format!("yaya-expose-{local_port}");

    // Remove rules matching our comment
    for (table, chain) in &[("nat", "PREROUTING"), ("filter", "FORWARD")] {
        loop {
            let output = Command::new("iptables")
                .args(["-t", table, "-L", chain, "--line-numbers", "-n"])
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let line = stdout.lines().find(|l| l.contains(&comment));

            if let Some(line) = line {
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
