use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::tunnel::TunnelManager;

#[derive(Parser)]
#[command(name = "yaya", about = "Post-quantum sovereign mesh VPN")]
#[command(version, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize Yaya identity and configuration
    Init,

    /// Manage mesh peers
    Peer {
        #[command(subcommand)]
        action: PeerAction,
    },

    /// Show mesh status
    Status,

    /// Exit node operations
    Exit {
        #[command(subcommand)]
        action: ExitAction,
    },

    /// Expose a local port via an exit node
    Expose {
        /// Local port to expose
        port: u16,

        /// Use HTTP reverse proxy mode
        #[arg(long)]
        http: bool,

        /// Domain for TLS termination
        #[arg(long)]
        domain: Option<String>,

        /// Specific exit node to use
        #[arg(long)]
        via: Option<String>,
    },

    /// Start a SOCKS5 proxy for browser routing
    Proxy {
        /// Local bind port
        #[arg(short, long, default_value = "1080")]
        port: u16,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Check for and apply updates
    Update {
        #[command(subcommand)]
        action: Option<UpdateAction>,
    },

    /// Rotate identity keys
    #[command(name = "key")]
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },

    /// Run the Yaya daemon
    Daemon,
}

#[derive(Subcommand)]
pub enum PeerAction {
    /// Add a peer by pubkey@endpoint
    Add {
        /// Peer in format pubkey@host:port
        peer: Option<String>,

        /// Generate an invite QR code
        #[arg(long)]
        invite: bool,

        /// Scan an invite QR code
        #[arg(long)]
        scan: bool,
    },

    /// Remove a peer
    Remove {
        /// Peer name or public key
        peer: String,
    },

    /// List all peers
    List,

    /// Verify a peer's safety number
    Verify {
        /// Peer name or public key
        peer: String,
    },
}

#[derive(Subcommand)]
pub enum ExitAction {
    /// Advertise this node as an exit node
    Serve,

    /// Route traffic through an exit node
    Use {
        /// Specific exit node name
        node: Option<String>,

        /// Rotate exit node at interval (e.g., 5m, 1h)
        #[arg(long)]
        rotate: Option<String>,
    },

    /// Stop using/serving exit node
    Stop,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a configuration value
    Set {
        key: String,
        value: String,
    },
    /// Get a configuration value
    Get {
        key: String,
    },
}

#[derive(Subcommand)]
pub enum UpdateAction {
    /// Check for available updates
    Check,
}

#[derive(Subcommand)]
pub enum KeyAction {
    /// Rotate identity keys (re-pairing required)
    Rotate,
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init => cmd_init().await,
        Command::Peer { action } => cmd_peer(action).await,
        Command::Status => cmd_status().await,
        Command::Exit { action } => cmd_exit(action).await,
        Command::Expose { port, http, domain, via } => cmd_expose(port, http, domain, via).await,
        Command::Daemon => cmd_daemon().await,
        _ => {
            tracing::warn!("Command not yet implemented");
            Ok(())
        }
    }
}

async fn cmd_init() -> Result<()> {
    let config = Config::init()?;
    let pubkey = config.public_key_base64();

    println!("Yaya initialized.");
    println!("  Config:    {}", config.config_dir().display());
    println!("  Public key: {pubkey}");
    println!("  Mesh IP:    (assigned on first peer connection)");
    println!();
    println!("Next: run `yaya peer add <pubkey@host:port>` to join a mesh.");
    Ok(())
}

async fn cmd_peer(action: PeerAction) -> Result<()> {
    let config = Config::load()?;

    match action {
        PeerAction::Add { peer, invite, scan } => {
            if invite {
                return cmd_peer_invite(&config).await;
            }
            if scan {
                println!("QR scanning not yet implemented. Use `yaya peer add <pubkey@host:port>`.");
                return Ok(());
            }
            let peer_str = peer.ok_or_else(|| anyhow::anyhow!(
                "Usage: yaya peer add <pubkey@host:port> or yaya peer add --invite"
            ))?;
            cmd_peer_add(&config, &peer_str).await
        }
        PeerAction::Remove { peer } => {
            config.remove_peer(&peer)?;
            println!("Peer {peer} removed.");
            Ok(())
        }
        PeerAction::List => {
            let peers = config.list_peers()?;
            if peers.is_empty() {
                println!("No peers configured. Run `yaya peer add` to get started.");
            } else {
                println!("{:<20} {:<45} {:<20}", "NAME", "PUBLIC KEY", "ENDPOINT");
                for p in &peers {
                    println!("{:<20} {:<45} {:<20}", p.name, p.public_key, p.endpoint);
                }
            }
            Ok(())
        }
        PeerAction::Verify { peer } => {
            let peer_info = config.get_peer(&peer)?;
            let safety_number = crate::mesh::safety_number(
                &config.public_key_base64(),
                &peer_info.public_key,
            );
            println!("Safety number for {peer}:");
            println!();
            // Display as 12 groups of 5 digits
            for (i, chunk) in safety_number.chunks(5).enumerate() {
                let s: String = chunk.iter().map(|b| format!("{}", b % 10)).collect();
                print!("{s}");
                if (i + 1) % 4 == 0 {
                    println!();
                } else {
                    print!(" ");
                }
            }
            println!();
            println!("Compare this number with your peer in person or via a trusted channel.");
            Ok(())
        }
    }
}

async fn cmd_peer_invite(config: &Config) -> Result<()> {
    let invite = crate::mesh::create_invite(config)?;
    println!("Share this invite with your peer (expires in 5 minutes):\n");
    println!("  yaya peer add {}", invite.as_peer_string());
    println!();
    // Display QR code in terminal
    if let Err(e) = qr2term::print_qr(&invite.as_uri()) {
        tracing::warn!("Could not display QR code: {e}");
        println!("URI: {}", invite.as_uri());
    }
    Ok(())
}

async fn cmd_peer_add(config: &Config, peer_str: &str) -> Result<()> {
    let (pubkey, endpoint) = parse_peer_string(peer_str)?;

    let name = crate::mesh::peer_name_from_key(&pubkey);

    config.add_peer(&name, &pubkey, &endpoint)?;

    // Set up WireGuard peer
    let tunnel = TunnelManager::new(config)?;
    let mesh_ip = config.allocate_mesh_ip()?;
    tunnel.add_peer(&pubkey, &endpoint, &mesh_ip)?;

    println!("Peer added: {name}");
    println!("  Public key: {pubkey}");
    println!("  Endpoint:   {endpoint}");
    println!("  Mesh IP:    {mesh_ip}");

    // Start Rosenpass for this peer
    crate::rosenpass::ensure_running(config)?;

    println!("Tunnel UP. Rosenpass PQ key exchange active.");
    Ok(())
}

fn parse_peer_string(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.splitn(2, '@').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid peer format. Expected: <pubkey>@<host:port>");
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

async fn cmd_status() -> Result<()> {
    let config = Config::load()?;
    let pubkey = config.public_key_base64();

    println!("Yaya v{}", env!("CARGO_PKG_VERSION"));
    println!("  Public key:  {pubkey}");
    println!("  Interface:   wg-yaya");

    // Show WireGuard status
    let tunnel = TunnelManager::new(&config)?;
    match tunnel.status() {
        Ok(status) => {
            println!("  Mesh IP:     {}", status.mesh_ip);
            println!("  Peers:       {}", status.peer_count);
            println!("  Transfer:    {} rx / {} tx", status.rx_bytes, status.tx_bytes);
        }
        Err(_) => {
            println!("  Status:      not connected");
        }
    }

    // Show Rosenpass status
    match crate::rosenpass::status() {
        Ok(rp_status) => {
            println!("  Rosenpass:   active, last PSK rotation: {}s ago", rp_status.last_rotation_secs);
        }
        Err(_) => {
            println!("  Rosenpass:   inactive");
        }
    }

    Ok(())
}

async fn cmd_exit(action: ExitAction) -> Result<()> {
    let config = Config::load()?;

    match action {
        ExitAction::Serve => {
            println!("Configuring this node as an exit node...");
            crate::exit::serve(&config).await?;
            println!("Exit node active. Other peers can route through this node.");
        }
        ExitAction::Use { node, rotate } => {
            let rotate_secs = rotate.as_deref().map(parse_duration).transpose()?;
            crate::exit::use_exit(&config, node.as_deref(), rotate_secs).await?;
        }
        ExitAction::Stop => {
            crate::exit::stop(&config).await?;
            println!("Exit node stopped.");
        }
    }
    Ok(())
}

async fn cmd_expose(port: u16, _http: bool, _domain: Option<String>, _via: Option<String>) -> Result<()> {
    let config = Config::load()?;

    println!("Exposing localhost:{port} via exit node...");
    crate::expose::expose_port(&config, port).await?;
    Ok(())
}

async fn cmd_daemon() -> Result<()> {
    let config = Config::load()?;
    tracing::info!("Starting Yaya daemon");

    // Initialize tunnel
    let tunnel = TunnelManager::new(&config)?;
    tunnel.ensure_interface()?;

    // Start Rosenpass sidecar
    crate::rosenpass::ensure_running(&config)?;

    // Start internal DNS
    crate::dns::start(&config).await?;

    // Block forever
    tracing::info!("Yaya daemon running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down...");
    crate::rosenpass::stop()?;
    Ok(())
}

fn parse_duration(s: &str) -> Result<u64> {
    let s = s.trim();
    if let Some(mins) = s.strip_suffix('m') {
        Ok(mins.parse::<u64>()? * 60)
    } else if let Some(hours) = s.strip_suffix('h') {
        Ok(hours.parse::<u64>()? * 3600)
    } else if let Some(secs) = s.strip_suffix('s') {
        Ok(secs.parse::<u64>()?)
    } else {
        s.parse::<u64>().map_err(Into::into)
    }
}
