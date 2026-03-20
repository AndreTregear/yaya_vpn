use anyhow::Result;
use tracing::info;

/// STUN client for NAT traversal — discover our public IP:port
pub async fn discover_public_endpoint(listen_port: u16) -> Result<Option<String>> {
    info!("Discovering public endpoint via STUN...");

    // Use well-known STUN servers
    let stun_servers = [
        "stun.l.google.com:19302",
        "stun1.l.google.com:19302",
        "stun.cloudflare.com:3478",
    ];

    for server in &stun_servers {
        match stun_query(server, listen_port).await {
            Ok(endpoint) => {
                info!(endpoint = %endpoint, "Public endpoint discovered");
                return Ok(Some(endpoint));
            }
            Err(e) => {
                tracing::debug!(server = %server, error = %e, "STUN query failed");
            }
        }
    }

    info!("Could not discover public endpoint via STUN");
    Ok(None)
}

async fn stun_query(server: &str, local_port: u16) -> Result<String> {
    use tokio::net::UdpSocket;

    let socket = UdpSocket::bind(format!("0.0.0.0:{local_port}")).await?;
    socket.connect(server).await?;

    // Build a minimal STUN Binding Request
    // RFC 5389: Type=0x0001, Magic Cookie=0x2112A442, Transaction ID=random
    let mut request = vec![
        0x00, 0x01, // Type: Binding Request
        0x00, 0x00, // Length: 0 (no attributes)
        0x21, 0x12, 0xA4, 0x42, // Magic Cookie
    ];
    // Transaction ID: 12 random bytes
    let txn_id: [u8; 12] = rand::random();
    request.extend_from_slice(&txn_id);

    socket.send(&request).await?;

    let mut buf = [0u8; 256];
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        socket.recv(&mut buf),
    )
    .await??;

    // Parse STUN Binding Response
    // Look for XOR-MAPPED-ADDRESS attribute (type 0x0020)
    parse_stun_response(&buf[..timeout])
}

fn parse_stun_response(data: &[u8]) -> Result<String> {
    if data.len() < 20 {
        anyhow::bail!("STUN response too short");
    }

    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != 0x0101 {
        // Not a Binding Success Response
        anyhow::bail!("Unexpected STUN message type: {msg_type:#06x}");
    }

    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let magic = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    let mut offset = 20; // Skip header
    let end = std::cmp::min(20 + msg_len, data.len());

    while offset + 4 <= end {
        let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;

        if attr_type == 0x0020 && attr_len >= 8 {
            // XOR-MAPPED-ADDRESS
            let family = data[offset + 1];
            let xor_port = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) ^ 0x2112;

            if family == 0x01 {
                // IPv4
                let ip = [
                    data[offset + 4] ^ 0x21,
                    data[offset + 5] ^ 0x12,
                    data[offset + 6] ^ 0xA4,
                    data[offset + 7] ^ 0x42,
                ];
                return Ok(format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], xor_port));
            }
        }

        // Align to 4 bytes
        offset += (attr_len + 3) & !3;
    }

    anyhow::bail!("No XOR-MAPPED-ADDRESS in STUN response")
}
