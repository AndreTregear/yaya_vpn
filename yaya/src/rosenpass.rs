use anyhow::Result;
use std::fs;
use std::process::{Child, Command};
use tracing::{info, warn};

use crate::config::Config;

static mut ROSENPASS_PROCESS: Option<Child> = None;

pub struct RosenpassStatus {
    pub last_rotation_secs: u64,
}

/// Ensure Rosenpass sidecar is running
pub fn ensure_running(config: &Config) -> Result<()> {
    let rp_dir = config.rosenpass_dir();
    fs::create_dir_all(&rp_dir)?;

    // Check if rosenpass binary exists
    if !is_installed() {
        warn!(
            "Rosenpass not installed. PQ key exchange disabled.\n\
             Install: https://rosenpass.eu or `cargo install rosenpass`"
        );
        return Ok(());
    }

    // Generate Rosenpass keys if they don't exist
    let sk_path = rp_dir.join("sk");
    let pk_path = rp_dir.join("pk");

    if !sk_path.exists() {
        info!("Generating Rosenpass keypair...");
        let status = Command::new("rosenpass")
            .args(["gen-keys", "--secret-key", sk_path.to_str().unwrap(),
                   "--public-key", pk_path.to_str().unwrap()])
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to generate Rosenpass keys");
        }
    }

    // Build Rosenpass config
    let rp_config = build_config(config)?;
    let config_path = rp_dir.join("config.toml");
    fs::write(&config_path, &rp_config)?;

    // Launch Rosenpass
    info!("Starting Rosenpass PQ key exchange sidecar...");
    let child = Command::new("rosenpass")
        .args(["exchange-config", config_path.to_str().unwrap()])
        .spawn()?;

    unsafe {
        ROSENPASS_PROCESS = Some(child);
    }

    info!("Rosenpass active — PSK rotation every 2 minutes");
    Ok(())
}

/// Stop Rosenpass sidecar
pub fn stop() -> Result<()> {
    unsafe {
        if let Some(ref mut child) = ROSENPASS_PROCESS {
            info!("Stopping Rosenpass sidecar...");
            child.kill().ok();
            child.wait().ok();
            ROSENPASS_PROCESS = None;
        }
    }
    Ok(())
}

/// Check Rosenpass status
pub fn status() -> Result<RosenpassStatus> {
    unsafe {
        if let Some(ref mut child) = ROSENPASS_PROCESS {
            match child.try_wait()? {
                None => {
                    // Still running
                    Ok(RosenpassStatus {
                        last_rotation_secs: 0, // TODO: track actual rotation time
                    })
                }
                Some(_) => {
                    anyhow::bail!("Rosenpass process exited");
                }
            }
        } else {
            anyhow::bail!("Rosenpass not running");
        }
    }
}

fn is_installed() -> bool {
    Command::new("rosenpass")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn build_config(config: &Config) -> Result<String> {
    let rp_dir = config.rosenpass_dir();
    let interface = config.interface_name();

    let mut toml = format!(
        r#"secret_key = "{sk}"
public_key = "{pk}"
listen = ["0.0.0.0:9999"]

"#,
        sk = rp_dir.join("sk").display(),
        pk = rp_dir.join("pk").display(),
    );

    // Add each peer
    for peer in &config.peers {
        let peer_pk_path = rp_dir.join(format!("peer-{}.pk", &peer.name));

        toml.push_str(&format!(
            r#"[[peers]]
public_key = "{peer_pk}"
endpoint = "{endpoint}"
wg = {{ interface = "{interface}", peer = "{wg_peer}" }}

"#,
            peer_pk = peer_pk_path.display(),
            endpoint = peer.endpoint.replace(":51820", ":9999"),
            interface = interface,
            wg_peer = peer.public_key,
        ));
    }

    Ok(toml)
}
