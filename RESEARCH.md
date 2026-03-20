# Yaya: Post-Quantum Sovereign Mesh VPN — Technical Report

> "You can't surveil what you can't access."

---

## Executive Summary

Yaya is a post-quantum WireGuard mesh VPN for individuals — not enterprises. It combines
WireGuard's proven tunnel performance with Rosenpass post-quantum key exchange, wrapped in
a no-account, self-hostable architecture bootstrapped via `curl yaya.sh | bash`.

**Key technical decisions:**

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | **Rust** | Mullvad's GotaTun proves Rust beats Go for packet processing (0.40% → 0.01% crash rate). boringtun/GotaTun ecosystem is mature. |
| WireGuard impl | **GotaTun** (Mullvad's boringtun fork) | Production-proven on millions of devices, MPL-2.0, active maintenance, DAITA support |
| PQ layer | **Rosenpass** (Classic McEliece + ML-KEM hybrid) | Only production-grade PQ wrapper for WireGuard; refreshes PSK every 2 min |
| Coordinator | **Custom minimal binary** (Rust) | Headscale is Go + Tailscale-protocol-coupled; too heavy. Build ~2000 LOC coordinator. |
| NAT traversal | **STUN + custom relay** | No TURN dependency; lightweight relay over QUIC like NetBird |
| License | **AGPL-3.0** | Prevents surveillance companies from forking without sharing modifications |
| Binary signing | **minisign** | Ed25519, dead simple, no infrastructure dependency (unlike cosign/Sigstore) |

**Biggest risks:**
1. Rosenpass maturity — v0.2.2, uses liboqs (not production-hardened)
2. Classic McEliece key sizes (~1MB public keys) may cause UX friction on constrained links
3. Userspace WireGuard requires CAP_NET_ADMIN — truly rootless operation is not possible
4. Cover traffic (DAITA-style) has real bandwidth cost — must be opt-in
5. Coordinator is a bootstrap SPOF even if data plane is fully p2p

---

## Protocol Stack Diagram

```
┌─────────────────────────────────────────────────────┐
│                   Application                        │
│            (any TCP/UDP service)                     │
├─────────────────────────────────────────────────────┤
│              .yaya Internal DNS                      │
│         (mesh-internal CoreDNS instance)             │
├─────────────────────────────────────────────────────┤
│            yaya expose / yaya exit                   │
│     (reverse proxy, exit routing, multi-hop)         │
├─────────────────────────────────────────────────────┤
│              WireGuard Tunnel                        │
│    (GotaTun — Rust userspace, Noise_IK handshake)    │
│    ChaCha20-Poly1305 symmetric, Curve25519 DH       │
│    Session rekey every ~120s, key zeroed at ~540s     │
├─────────────────────────────────────────────────────┤
│            Rosenpass PQ Layer                         │
│  Classic McEliece 460896 + ML-KEM-768 (hybrid)       │
│  → outputs PSK → injected into WireGuard PSK slot    │
│  Refreshes every 2 minutes                           │
├─────────────────────────────────────────────────────┤
│           NAT Traversal (ICE/STUN)                   │
│     Direct P2P preferred → QUIC relay fallback       │
├─────────────────────────────────────────────────────┤
│              UDP Transport                           │
│         (single UDP port, default 51820)             │
└─────────────────────────────────────────────────────┘
```

---

## Section 1: Core Mesh Protocol

### WireGuard Implementations Compared

| Implementation | Language | Kernel? | Platforms | Privilege | Performance | Status |
|---------------|----------|---------|-----------|-----------|-------------|--------|
| Linux kernel module | C | Yes | Linux only | root / CAP_NET_ADMIN | Best (~1Gbps+) | Reference impl |
| wireguard-go | Go | No | All | CAP_NET_ADMIN + /dev/net/tun | ~60-70% of kernel | De facto userspace standard |
| boringtun | Rust | No | Linux, macOS, iOS, Android | CAP_NET_ADMIN | Near-kernel on x86_64 | Cloudflare, stale maintenance |
| **GotaTun** | Rust | No | Linux, macOS, iOS, Android, Windows | CAP_NET_ADMIN | Near-kernel, zero-copy | **Mullvad fork of boringtun, active** |
| defguard_wireguard_rs | Rust | Both | Linux, FreeBSD, macOS, Windows | Varies | Unified API | v0.7.8, active |

**Recommendation: GotaTun as primary transport.**
- Fork of boringtun by Mullvad (the most privacy-focused VPN company)
- MPL-2.0 licensed (compatible with AGPL)
- Proven: crash rate dropped from 0.40% to 0.01% replacing wireguard-go on Android
- Supports DAITA (traffic analysis defense) — directly relevant to Yaya's anti-surveillance mission
- Zero-copy memory strategies, safe multi-threading
- v0.4.0 released Feb 2026

**Alternative: defguard_wireguard_rs** for the unified kernel+userspace API abstraction.
Could be used as the management layer that wraps GotaTun for userspace and kernel WireGuard
where available (Linux). Worth evaluating whether to depend on it or vendor GotaTun directly.

### Rosenpass — Post-Quantum Layer

**What it is:** A Rust tool that performs post-quantum key exchange and feeds the resulting
symmetric key into WireGuard's PSK (Pre-Shared Key) slot. This is a hybrid approach:
even if PQ crypto is broken, WireGuard's classical Noise_IK handshake remains intact.

**Cryptographic details:**
- **KEMs used:** Classic McEliece 460896 + CRYSTALS-Kyber (ML-KEM-768)
- **Why both:** Classic McEliece is code-based (40+ years of cryptanalysis, very conservative).
  ML-KEM is lattice-based (NIST standardized as FIPS 203, smaller keys, faster).
  Using both provides defense-in-depth: an attacker must break *both*.
- **Underlying library:** liboqs (Open Quantum Safe)
- **Key rotation:** Every 2 minutes, Rosenpass derives a fresh PSK and injects it
- **Protocol:** Based on a Noise-like handshake, formally verified with ProVerif
- **Current version:** v0.2.2 (June 2024)
- **License:** Apache-2.0 + MIT dual

**NIST PQC Status (as of 2026):**
- ML-KEM (Kyber): **FIPS 203 — standardized** (Aug 2024)
- Classic McEliece: **Round 4 candidate** — not yet standardized but deployed (Rosenpass, Mullvad)

**Open questions:**
- Classic McEliece public keys are ~1MB. How does this affect initial handshake on slow links?
- liboqs explicitly states it is "not production grade" — is this acceptable?
- Should Yaya vendor Rosenpass or run it as a sidecar process?
  - Sidecar (current Rosenpass design): simpler integration, process isolation
  - Vendored library: tighter integration, single binary, but more maintenance

**Recommendation:** Run Rosenpass as a sidecar process initially (v0.1), evaluate vendoring
the key exchange logic into the Yaya binary for v1.0. The PSK injection mechanism means
Rosenpass is cleanly separable.

### Exact Cryptographic Stack for a Yaya Tunnel

```
Handshake:
  1. Rosenpass performs PQ key exchange:
     - Classic McEliece 460896 KEM encapsulation
     - ML-KEM-768 KEM encapsulation
     - Derives 256-bit PSK from both
  2. WireGuard Noise_IK handshake:
     - Curve25519 ECDH (classical)
     - PSK from Rosenpass mixed in
     - HKDF-BLAKE2s key derivation
  3. Result: session key for symmetric encryption

Symmetric transport:
  - ChaCha20-Poly1305 AEAD
  - 64-bit nonce counter
  - Rekey after ~120 seconds or 2^64 - 2^4 - 1 messages

Key zeroing:
  - Ephemeral keys zeroed after REJECT_AFTER_TIME * 3 (~540 seconds)
  - Rosenpass PSK refreshed every 120 seconds
```

### NetBird Architecture — What to Copy, What to Discard

**NetBird components:**
1. **Management Server** — Go, central coordinator: auth, topology, IP assignment, ACLs, user mgmt
2. **Signal Server** — Go, lightweight message relay for ICE candidate exchange
3. **Relay Server** — QUIC + WebSocket TURN, fallback when P2P fails
4. **Client Agent** — Go, runs on each node

**What to copy:**
- Signal/Relay separation — Signal is stateless message forwarding, Relay carries data
- ICE candidate exchange pattern (encrypted end-to-end, Signal can't read payloads)
- QUIC-first relay with WebSocket fallback
- Progressive connection: Direct P2P → QUIC relay → WebSocket relay
- IP allocation from 100.64.0.0/10 (CGNAT range, avoids conflicts)

**What to discard:**
- SSO/OIDC authentication (enterprise feature)
- Granular access controls / ACL policies (enterprise feature)
- Management server's full user/group/policy model
- Dashboard UI
- NetBird's dependency on Coturn (heavy TURN server)

---

## Section 2: Coordinator / Control Plane

### Headscale Analysis

**Architecture:** Go binary, implements Tailscale's coordination protocol, SQLite database,
manages nodes for a single tailnet. Works with official Tailscale clients.

**Why NOT fork Headscale:**
1. Written in Go — language mismatch (Yaya is Rust)
2. Implements Tailscale's proprietary coordination protocol — tight coupling
3. Depends on DERP (Tailscale's relay protocol) — another proprietary layer
4. Designed for Tailscale clients — Yaya has its own client
5. Schema assumes Tailscale concepts (users, ACLs, namespaces)

**What to learn from Headscale:**
- Single binary with SQLite is the right deployment model
- CLI-only administration works for technical users
- Pre-auth keys for non-interactive node registration

### Minimal Yaya Coordinator Design

The coordinator exists to bootstrap peer discovery. Once peers have each other's
endpoints + public keys, the data plane is fully P2P.

**What the coordinator MUST do:**
1. **Peer registration** — accept node public key + endpoint, assign mesh IP
2. **Peer list distribution** — tell each node about every other node in its mesh
3. **Relay coordination** — provide relay server addresses for NAT-blocked peers
4. **Key rotation notification** — propagate when a peer changes its WireGuard public key

**What the coordinator MUST NOT do:**
- Authentication beyond keypair verification (no accounts, no SSO)
- Traffic inspection or relay (data never touches coordinator)
- Policy enforcement (no ACLs — this is a trust-based personal mesh)
- Persistent session state (stateless beyond the peer registry)

**Design:**
```
yaya-coordinator binary (~2000 LOC Rust)
├── HTTP/HTTPS API (axum or warp)
│   ├── POST /peers          — register new peer
│   ├── GET  /peers          — list peers in mesh
│   ├── DELETE /peers/:id    — remove peer
│   └── WS   /sync          — real-time peer updates
├── Storage: SQLite (via rusqlite)
│   └── peers(pubkey, mesh_ip, endpoints[], last_seen, metadata)
├── Relay registry
│   └── List of available relay servers
└── Config: TOML file, <50 lines
```

**Can coordinator-less (pure P2P) work?**
No, not for initial peer discovery. You need at least one rendezvous point. However:
- The coordinator can be run by any peer (self-host on the same box)
- Multiple coordinators can exist (no single point of failure)
- Once peers discover each other, coordinator is optional for ongoing operation
- DHT-based discovery (like libp2p Kademlia) is possible but adds massive complexity
  for a network of 3-50 nodes. Overkill.

### libp2p Assessment

**Verdict: Too heavy.** libp2p is designed for thousands-of-nodes P2P networks (IPFS, Filecoin).
Yaya's target is 3-50 nodes per mesh. libp2p would add:
- ~15+ crate dependencies
- Kademlia DHT (unnecessary for small meshes)
- mDNS, relay, hole-punching (redundant with WireGuard's own mechanisms)
- Protocol negotiation layer (Yaya has exactly one protocol)

**What to cherry-pick from libp2p:** Nothing. Use STUN directly (`stun-rs` crate) and
build a minimal QUIC relay (~500 LOC).

### NAT Traversal

**WireGuard's NAT problem:** WireGuard uses static UDP endpoints. Behind NAT, the peer's
public endpoint changes. Solutions:

1. **STUN** — Discover own public IP:port. Use `stun-rs` crate (implements RFC 8489).
2. **Persistent keepalive** — WireGuard's built-in 25-second keepalive maintains NAT mappings.
3. **QUIC relay fallback** — When direct P2P fails (symmetric NAT, strict firewalls),
   relay traffic through a QUIC tunnel via a public relay node.
4. **ICE-like candidate gathering** — Try all local IPs, STUN-discovered IPs, and relay
   addresses. Pick the best working path.

**Carrier-grade NAT (CGNAT):** The hardest case. Two peers both behind CGNAT cannot directly
connect via STUN alone. The relay is mandatory here. NetBird handles this with their
QUIC relay — Yaya should do the same.

**Rust crates:**
- `stun-rs` — STUN client/server, supports ICE and TURN extensions
- `quinn` — QUIC implementation in Rust (for relay transport)
- `webrtc-rs` — Full WebRTC stack (too heavy, but `webrtc-ice` subcrate may be useful)

---

## Section 3: Exit Node Architecture

### How Tailscale Exit Nodes Work

Tailscale exit nodes use Linux policy routing:

1. **Mark packets** — iptables/nftables marks forwarded packets with fwmark
2. **Policy routing** — `ip rule` entries route marked packets through the WireGuard interface
3. **NAT/masquerade** — iptables MASQUERADE rule on the exit node's public interface
4. **IP forwarding** — `net.ipv4.ip_forward = 1` on the exit node

```
# Simplified exit node setup:
ip rule add fwmark 0x1 table 52
ip route add default via <exit_gw> table 52
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
sysctl net.ipv4.ip_forward=1
```

### `yaya exit use --rotate 5m` Design

**What must happen:**

1. **Node A** (the user) sends command to rotate exit every 5 minutes
2. The Yaya daemon maintains a list of willing exit nodes from the mesh
3. Every 5 minutes:
   a. Select next exit node (round-robin or random from available exits)
   b. Update WireGuard's `AllowedIPs = 0.0.0.0/0` to point to new exit peer
   c. Update routing table: `ip route replace default` via new WireGuard peer
   d. Tear down old exit peer's default route
   e. Brief connectivity gap (~100-500ms) during switchover

**At the kernel/routing level on the client:**
```bash
# Remove old default route through exit-A
ip route del default dev wg0 via 100.64.0.2

# Add new default route through exit-B
ip route add default dev wg0 via 100.64.0.3

# Update WireGuard peer config
wg set wg0 peer <exit-B-pubkey> allowed-ips 0.0.0.0/0,::/0
```

**At the exit node:**
```bash
# Already configured (static for exit nodes):
iptables -t nat -A POSTROUTING -s 100.64.0.0/10 -o eth0 -j MASQUERADE
sysctl net.ipv4.ip_forward=1
```

**Decision: who picks the exit?**
The client decides. The client has the full peer list and knows which nodes have
advertised themselves as exit nodes. The coordinator just flags which peers are
willing exits.

### Traffic Shaping for Timing Attack Resistance

This is Yaya's most ambitious anti-surveillance feature. Research findings:

| Technique | Mechanism | Overhead | Feasibility |
|-----------|-----------|----------|-------------|
| **Packet padding** | Pad all packets to MTU (1420 bytes for WG) | ~15-40% bandwidth increase (avg packet is ~800 bytes) | Feasible, low CPU cost |
| **Cover traffic** | Constant bitrate dummy packets when idle | 50-500 Kbps always-on bandwidth | Feasible but burns mobile data |
| **Jitter injection** | Random 0-50ms delay per packet | 25ms average added latency | Feasible, tolerable for most apps |
| **DAITA-style** | Probabilistic state machines for dummy injection | ~50% dummy packets (v2 reduced from v1) | Best approach — Mullvad proved it works |

**Recommendation:** Implement DAITA-style defense using the **Maybenot** framework
(open-source, Rust, funded by Mullvad). This is the same framework GotaTun uses.
Make it opt-in: `yaya config set traffic-shaping on`.

**Multi-hop routing:**
Two approaches:
1. **Sequential forwarding** — Node A → Node B → Node C → Internet. Each hop is a
   separate WireGuard tunnel. Simple but each hop sees the next hop's plaintext IP.
2. **Onion-style** — Node A encrypts for C inside encryption for B. B peels one layer.
   More private but requires custom encapsulation logic.

**Recommendation for v0.1:** Sequential forwarding (2-3 hops). It's much simpler and
still provides significant traffic analysis resistance. Onion routing can be v1.0.

**Open questions:**
- What's the maximum practical hop count before latency becomes unusable?
  (Estimate: 3 hops × ~30ms = ~90ms added, acceptable for most uses)
- Should hop selection be random or deterministic?
- How do we prevent circular routing?

---

## Section 4: Trust Model & Peer Authentication

### Design: No Accounts, Pure Keypair Trust

Yaya has no central authority, no accounts, no passwords. Trust is established by
direct key exchange between individuals — like exchanging phone numbers, but for
network identities.

### `yaya peer add` Flow — End to End

```
INITIATOR (Alice)                         RESPONDER (Bob)
─────────────────                         ────────────────
1. yaya peer add --invite
   → Generates:
     - Ephemeral X25519 keypair (for ECDH)
     - 6-word mnemonic (BIP39 subset) as
       human-readable verification code
     - QR code containing:
       {
         "v": 1,
         "pk": <Alice WG pubkey, 32 bytes>,
         "epk": <ephemeral pubkey, 32 bytes>,
         "ep": <Alice's endpoints[]>,
         "coord": <coordinator URL>,
         "code": <HMAC-SHA256 of pk, first 6 words>
       }
     - QR also available as yaya://add?... URI

2. Alice shows QR to Bob                 3. Bob scans QR:
   (in person, or via trusted channel)       yaya peer add --scan
                                             → Decodes QR
                                             → Verifies HMAC matches pubkey
                                             → Generates own ephemeral X25519 key

                                          4. Bob contacts Alice's endpoint
                                             (or coordinator) with:
                                             {
                                               "pk": <Bob WG pubkey>,
                                               "epk": <Bob ephemeral pubkey>,
                                               "ep": <Bob's endpoints[]>
                                             }

5. ECDH: Alice's ephemeral × Bob's ephemeral
   → Shared secret (verifies physical proximity
     or trusted channel was used)

6. Both sides:
   - Add peer's WG pubkey to WireGuard config
   - Set AllowedIPs for the peer's mesh IP
   - Rosenpass begins PQ key exchange
   - WireGuard handshake completes
   - Tunnel UP in ~500ms

7. Verification (Signal-style):
   - Safety number = SHA256(sort(Alice_pk, Bob_pk))
     displayed as 12 groups of 5 digits
   - Users can verify out-of-band any time:
     yaya peer verify <name>
```

**Key verification (adapted from Signal):**
- Safety number is derived from both peers' long-term WireGuard public keys
- Displayed as 60-digit number (12 groups of 5 digits) or QR code
- Changes if either peer regenerates their identity key
- User is warned on key change: "Bob's identity has changed. Verify in person."

**What happens when a peer is removed:**
1. `yaya peer remove <name>`
2. WireGuard peer entry deleted (pubkey removed from allowed peers)
3. Rosenpass stops PSK rotation for that peer
4. Coordinator notified → removes peer from mesh peer list → all other peers remove it
5. Old session keys are zeroed immediately (WireGuard's REJECT_AFTER_TIME)
6. Removed peer can no longer establish new handshakes (pubkey not in any peer's config)

**Threat model — state actor trying to join:**
- Must physically obtain QR code or intercept the exact trusted channel
- QR code contains ephemeral key — single-use, expires in 5 minutes
- No way to "request" addition — must be invited by an existing peer
- Each peer independently decides who to add — no central directory to compromise
- Key verification (safety numbers) detects MITM during pairing
- **Friction:** requires out-of-band communication and intentional action from both parties

**WireGuard key rotation:**
- Session keys rotate every ~120 seconds (REKEY_AFTER_TIME)
- Rosenpass PSK rotates every 120 seconds (independent cycle)
- Long-term identity keys: user-controlled, rotate via `yaya key rotate`
  (triggers re-pairing with all peers)

---

## Section 5: Application Layer & Inbound Routing

### `yaya expose <port>` — End to End Design

This is the "sovereign ngrok" primitive. A node inside the mesh exposes a local port
via an exit node's public IP.

**Command:** `yaya expose 8080`

**What happens on the HOST NODE (running the service):**
1. Yaya opens a control channel to the selected exit node over the WG tunnel
2. Requests: "allocate a public port for my local port 8080"
3. Exit node responds with: `<exit-public-ip>:<allocated-port>`
4. Host node displays: `Service exposed at https://203.0.113.5:34567`
5. All traffic arriving at exit node on that port is tunneled back to host's localhost:8080

**What happens on the EXIT NODE:**
```
Internet → 203.0.113.5:34567
         → iptables DNAT to WireGuard peer (100.64.0.x)
         → WG tunnel carries packet to host node
         → Host node delivers to localhost:8080
         → Response travels reverse path
```

Routing table on exit node:
```bash
# For each exposed service:
iptables -t nat -A PREROUTING -p tcp --dport 34567 -j DNAT --to-destination 100.64.0.5:8080
iptables -A FORWARD -p tcp -d 100.64.0.5 --dport 8080 -j ACCEPT
```

**What the user sees:**
```
$ yaya expose 8080
Exposing localhost:8080 via exit node "relay-nl" (203.0.113.5)
Public URL: http://203.0.113.5:34567
Press Ctrl+C to stop.
```

**For HTTP services, optional TLS termination:**
```
$ yaya expose 8080 --tls --domain myapp.example.com
Exposing localhost:8080 via exit node "relay-nl"
Public URL: https://myapp.example.com
TLS certificate: auto-provisioned via Let's Encrypt (ACME)
```

**Latency characteristics:**
- Direct WireGuard tunnel: ~1-5ms added latency per hop
- Webhook delivery: HTTP request → exit node → WG tunnel → host → response → WG → exit → HTTP response
- Total added latency: ~5-20ms for single hop, ~20-60ms for multi-hop
- This is acceptable for webhooks, APIs, web apps. Not suitable for real-time gaming.

### Internal DNS: `.yaya` Domain

**Recommendation: Mesh-internal DNS server, not mDNS or hosts file.**

Why not mDNS: mDNS is link-local (same broadcast domain). Mesh nodes are on different networks.
Why not hosts file: Can't update dynamically when peers join/leave.

**Design:**
- Every Yaya node runs a lightweight DNS resolver (trust-dns / hickory-dns crate)
- Listens on `100.64.0.1:53` (the node's own mesh interface)
- Resolves `<nodename>.yaya` → mesh IP by querying the coordinator's peer list
- System resolver configured to use `.yaya` suffix via `/etc/resolv.conf` or `systemd-resolved`
- Queries for non-.yaya domains pass through to upstream DNS

```
$ ping laptop.yaya
PING laptop.yaya (100.64.0.3): 56 data bytes
64 bytes from 100.64.0.3: time=12.3ms
```

### Port Forwarding vs Reverse Proxy vs SOCKS5

| Approach | Use case | Complexity | Recommended? |
|----------|----------|------------|-------------|
| Port forwarding (DNAT) | Raw TCP/UDP services | Low | **Yes — default** |
| Reverse proxy (HTTP) | Web services, TLS termination | Medium | Yes, opt-in for HTTP |
| SOCKS5 | Browser-level proxying | Low | Yes, for `yaya proxy` command |

**Recommendation:** Port forwarding (DNAT) as the default for `yaya expose`.
HTTP reverse proxy as an optional mode (`yaya expose 8080 --http`).
SOCKS5 as a separate command (`yaya proxy`) for browser routing.

---

## Section 6: The Installer (`yaya.sh`)

### Installer Flow

```bash
curl -fsSL yaya.sh | bash
```

**Steps (target: ~200 lines of bash):**

1. **Detect platform:** `uname -s` (Linux/Darwin), `uname -m` (x86_64/aarch64/armv7l)
2. **Check dependencies:** `wg` (wireguard-tools), warn if missing
3. **Download binary:** `curl -fsSL https://releases.yaya.sh/v{VERSION}/yaya-{os}-{arch}.tar.gz`
4. **Download signature:** `curl -fsSL https://releases.yaya.sh/v{VERSION}/yaya-{os}-{arch}.tar.gz.minisig`
5. **Verify signature:** `minisign -Vm yaya-{os}-{arch}.tar.gz -P <embedded-pubkey>`
   - If minisign not installed: download minisign binary first, or verify SHA256 as fallback
6. **Install binary:** `sudo install -m 755 yaya /usr/local/bin/yaya`
7. **Generate identity:** `yaya init` (creates ~/.config/yaya/ with WireGuard keypair)
8. **Print next steps:** "Run `yaya peer add` to join a mesh"

**300 lines is realistic.** rustup's installer is ~400 lines but handles much more
(toolchain management, shell profile modification). Yaya's installer is simpler:
download, verify, install, init.

### Binary Signing: minisign

**Why minisign over alternatives:**
- **vs GPG:** GPG is bloated, complex key management, bad UX. minisign is a single binary.
- **vs cosign/Sigstore:** Requires Sigstore infrastructure (Fulcio CA, Rekor transparency log).
  Good for enterprise supply chain, overkill for a single project. Also adds a dependency on
  Google's infrastructure — antithetical to sovereignty.
- **minisign:** Ed25519, 2 files (secret key, public key), verification is one command.
  Created by Frank Denis (libsodium author). Used by Zig, WinGet, others.
  The public key is ~60 characters — can be embedded directly in the installer script.

### Cross-Compilation Matrix

| Target | Rust triple | Build tool | Notes |
|--------|-------------|------------|-------|
| linux/amd64 | x86_64-unknown-linux-gnu | Native or cross | Primary target |
| linux/arm64 | aarch64-unknown-linux-gnu | cross-rs | RPi 4, cloud ARM |
| linux/armv7 | armv7-unknown-linux-gnueabihf | cross-rs | RPi 3, older ARM |
| darwin/amd64 | x86_64-apple-darwin | macOS runner | Requires macOS |
| darwin/arm64 | aarch64-apple-darwin | macOS runner | Apple Silicon |

**Build pipeline:** GitHub Actions with:
- `cross-rs` for Linux ARM targets (Docker-based cross compilation)
- macOS GitHub-hosted runners for Darwin targets
- `cargo-zigbuild` as alternative (Zig as linker, handles musl easily)

### Privilege Requirements

**Honest assessment: root (or CAP_NET_ADMIN) is required.**

Userspace WireGuard (GotaTun) still needs:
- `/dev/net/tun` access (TUN device creation)
- `CAP_NET_ADMIN` capability (network interface configuration)
- Route table modification (policy routing for exit nodes)

**Mitigation:**
- The Yaya daemon runs as a systemd service with only `CAP_NET_ADMIN` and `CAP_NET_RAW`
  (not full root)
- The `yaya` CLI talks to the daemon via Unix socket (unprivileged)
- Installation requires sudo; runtime does not require interactive root

### Auto-Update

**Design: User-controlled, never silent.**

```
$ yaya update check
New version available: v0.3.0 (current: v0.2.1)
Run `yaya update` to install.

$ yaya update
Downloading yaya v0.3.0 for linux/amd64...
Verifying signature... OK
Replacing /usr/local/bin/yaya...
Restarting yaya daemon...
Updated to v0.3.0.
```

- **No auto-update by default** — sovereign computing means user decides
- `yaya update check` queries releases.yaya.sh for latest version
- `yaya update` downloads, verifies (minisign), replaces binary, restarts daemon
- Opt-in auto-check: `yaya config set update-check daily` (prints notice, never auto-installs)
- Rust crate: `self_update` (or custom — it's just HTTP + file replacement)

---

## Section 7: Repository Architecture

### GitHub Organization: `yaya-sh/`

```
yaya-sh/
├── yaya              # Main node binary (Rust) — AGPL-3.0
│   ├── src/
│   │   ├── main.rs
│   │   ├── tunnel/        # WireGuard/GotaTun integration
│   │   ├── rosenpass/     # PQ key exchange sidecar management
│   │   ├── mesh/          # Peer management, coordinator client
│   │   ├── nat/           # STUN, relay client
│   │   ├── expose/        # Service exposure (DNAT, reverse proxy)
│   │   ├── exit/          # Exit node routing
│   │   ├── dns/           # .yaya internal DNS
│   │   └── cli/           # CLI commands
│   ├── Cargo.toml
│   └── Cargo.lock
│
├── yaya-coordinator  # Coordinator server (Rust) — AGPL-3.0
│   ├── src/
│   │   ├── main.rs
│   │   ├── api.rs         # HTTP/WS API
│   │   ├── store.rs       # SQLite peer storage
│   │   └── relay.rs       # Relay registry
│   └── Cargo.toml
│
├── yaya-relay        # QUIC relay server (Rust) — AGPL-3.0
│   └── src/
│
├── yaya.sh           # Installer script (bash) — MIT
│   └── install.sh
│
├── docs              # Documentation (mdBook) — CC-BY-4.0
│   └── src/
│
└── .github/          # CI/CD, release automation
```

**Recommendation: Polyrepo** (separate repos for node, coordinator, relay, installer, docs)

Rationale:
- Different release cadences (node releases often, coordinator rarely)
- Different languages (Rust binary vs bash script vs markdown docs)
- Cleaner issue tracking per component
- Installer can be versioned independently
- Coordinator can be optional / self-hosted separately

However, `yaya` and `yaya-coordinator` could share a Cargo workspace in a monorepo
if the team is small (1-3 people). Switch to polyrepo when the team grows.

**For v0.1: monorepo Cargo workspace** with:
```
yaya-project/
├── Cargo.toml          # workspace
├── yaya/               # node binary
├── yaya-coordinator/   # coordinator
├── yaya-relay/         # relay
├── install.sh          # installer
└── docs/
```

### License: AGPL-3.0

**Why AGPL over alternatives:**

| License | Prevents proprietary fork? | Network use = distribution? | Yaya fit |
|---------|---------------------------|----------------------------|----------|
| MIT | No | No | Too permissive |
| Apache-2.0 | No (but patent protection) | No | Too permissive |
| GPL-3.0 | Yes for distributed binaries | No — SaaS loophole | Close but insufficient |
| **AGPL-3.0** | **Yes** | **Yes** | **Best fit** |

AGPL ensures: if anyone runs a modified Yaya as a service (e.g., a VPN provider
using Yaya's code), they must release their modifications. This directly prevents
a surveillance company from taking Yaya, adding backdoors, and offering it as a
"privacy" service.

**Exception:** The installer (`yaya.sh`) should be MIT — it's trivial code and
MIT maximizes adoption (other projects can reference the installer pattern).

---

## Section 8: Competitive Landscape

### Tailscale
| Aspect | Tailscale | Yaya |
|--------|-----------|------|
| Coordinator | Proprietary cloud (or Headscale) | Self-hosted, minimal |
| Auth | Google/Microsoft/GitHub SSO | No accounts, keypair only |
| PQ crypto | None | Rosenpass (Classic McEliece + ML-KEM) |
| Exit nodes | Yes | Yes, with rotation + traffic shaping |
| Pricing | Free tier + paid | Free forever (self-hosted) |
| Target | Teams, enterprises | Individuals, activists, developers |
| Code | Partially open (client only) | Fully open (AGPL) |

**Honest assessment:** Tailscale has better UX, more features, more polish. Yaya's edge
is sovereignty (no account required, self-hosted everything) and post-quantum security.

### NetBird
| Aspect | NetBird | Yaya |
|--------|---------|------|
| Architecture | Management + Signal + Relay | Coordinator + Relay (simpler) |
| Auth | SSO/OIDC required | No accounts |
| PQ crypto | Rosenpass integration (recent) | Rosenpass from day 1 |
| Target | Teams, enterprises | Individuals |
| Language | Go | Rust |
| Self-host | Yes (but complex: 4+ services) | Yes (1-2 binaries) |

**Honest assessment:** NetBird is the closest competitor architecturally. Yaya differentiates
on simplicity (no SSO, no ACLs) and sovereignty (no account required at all).

### Innernet (Tonari)
- Written in Rust, WireGuard-based, open source
- Hub-and-spoke model (not full mesh)
- No PQ crypto, no exit nodes, no service exposure
- Still "experimental," no independent security audit
- **Relevant prior art** for Rust + WireGuard integration patterns

### Nebula (Defined Networking)
- Not WireGuard (custom Noise protocol implementation)
- Certificate-based auth (closest to Yaya's model)
- Lighthouses for peer discovery (similar to coordinator)
- AWS-style security groups (enterprise-focused)
- No PQ crypto
- **Relevant prior art** for certificate-based mesh authentication

### Tor
- Different threat model: anonymity vs. privacy
- Much higher latency (3+ hops through volunteer relays, ~200-500ms)
- Not a VPN — doesn't carry arbitrary traffic well
- Yaya is NOT an anonymity tool — it's a private network
- **Positioning:** "Tor protects your identity. Yaya protects your infrastructure."

### Does PQ + no-account + self-host exist today?
**No.** This is Yaya's unique position:
- Rosenpass exists but is a protocol, not a mesh
- NetBird added Rosenpass but requires SSO accounts
- No project combines all three: post-quantum + no-account + self-hosted mesh

---

## Implementation Roadmap

### v0.1 — Proof of Concept (Weeks 1-3)
- Two nodes establish a WireGuard tunnel via GotaTun
- Rosenpass sidecar performs PQ key exchange
- Manual peer configuration (no coordinator yet)
- Basic `yaya init`, `yaya peer add <pubkey@endpoint>`, `yaya status`
- Linux only

### v0.2 — Mesh Foundation (Weeks 4-8)
- Coordinator server (peer registration, peer list sync)
- QR code peer pairing (`yaya peer add --invite`)
- NAT traversal (STUN + relay)
- Exit node support (`yaya exit serve`, `yaya exit use`)
- `.yaya` internal DNS
- `curl yaya.sh | bash` installer with minisign verification
- Linux + macOS

### v0.3 — Sovereign Services (Weeks 9-14)
- `yaya expose <port>` (service exposure via exit node)
- Exit rotation (`yaya exit use --rotate 5m`)
- Multi-hop routing (2-3 sequential hops)
- Auto-update mechanism
- ARM support (RPi)

### v1.0 — Production (Weeks 15-24)
- Traffic shaping (DAITA/Maybenot integration)
- Safety number key verification
- Multiple coordinator support (redundancy)
- Security audit
- Documentation site

---

## Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | **Rosenpass immaturity** — liboqs not production-grade, v0.2.2 | High | High | Hybrid security: WG remains secure even if PQ layer fails. Monitor liboqs maturity. |
| 2 | **Classic McEliece key size** — ~1MB public keys slow initial handshake | Medium | Medium | Keys exchanged once per peer pair, cached locally. Acceptable on broadband, problematic on satellite. |
| 3 | **CAP_NET_ADMIN requirement** — cannot run truly rootless | High | Medium | Use systemd capabilities (not full root). CLI is unprivileged. |
| 4 | **Single coordinator SPOF** — if coordinator down, new peers can't join | Medium | Medium | Allow multiple coordinators. Data plane works without coordinator once established. |
| 5 | **Adoption chicken-and-egg** — mesh is useless with 1 node | High | High | Focus messaging on concrete use cases (expose local service, route through exit). Provide hosted relay/coordinator for zero-friction start. |

---

## First Sprint — 10-Day PoC

**Goal:** Two nodes on different networks, one acting as exit node, one local HTTP service
exposed through the exit, all traffic PQ-encrypted.

### Day 1-2: Project Scaffolding
```
cargo init yaya --name yaya
cargo init yaya-coordinator --name yaya-coordinator
# Add workspace Cargo.toml
```

**Dependencies (Cargo.toml):**
```toml
[dependencies]
# WireGuard
gotatun = { git = "https://github.com/mullvad/gotatun" }  # or vendored boringtun
defguard_wireguard_rs = "0.7"  # WireGuard interface management

# Networking
tokio = { version = "1", features = ["full"] }
quinn = "0.11"          # QUIC (for relay)
stun-rs = "0.1"         # STUN client

# CLI
clap = { version = "4", features = ["derive"] }

# Crypto
x25519-dalek = "2"      # Key generation
base64 = "0.22"
rand = "0.8"

# DNS
hickory-server = "0.24"  # Internal DNS

# Config
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

### Day 3-4: WireGuard Tunnel
- Create TUN interface using `defguard_wireguard_rs`
- Generate Curve25519 keypair
- Establish WireGuard tunnel between two nodes
- Verify encrypted ping works
- **Files:** `yaya/src/tunnel/mod.rs`, `yaya/src/tunnel/interface.rs`

### Day 5-6: Rosenpass Integration
- Download/compile Rosenpass binary
- Launch as sidecar process from Yaya
- Configure Rosenpass to inject PSK into WireGuard interface
- Verify PSK rotation every 2 minutes in WireGuard status
- **Files:** `yaya/src/rosenpass/sidecar.rs`

### Day 7-8: Exit Node + Service Exposure
- Implement exit node: ip forwarding, masquerade, policy routing
- Implement `yaya expose`: DNAT rule on exit node, tunnel traffic back
- Test: HTTP server on Node A, exposed via Node B's public IP
- **Files:** `yaya/src/exit/mod.rs`, `yaya/src/expose/mod.rs`

### Day 9-10: CLI + Installer
- Implement CLI commands: `init`, `peer add`, `status`, `exit serve`, `exit use`, `expose`
- Write `install.sh` (~200 lines)
- Generate minisign keys, sign first binary
- End-to-end test: `curl yaya.sh | bash` → `yaya init` → peer add → tunnel up → expose service
- **Files:** `yaya/src/cli/mod.rs`, `install.sh`

### Verification Commands
```bash
# On Node A (service host):
yaya init
yaya peer add <Node-B-pubkey>@<Node-B-ip>:51820
python3 -m http.server 8080 &
yaya expose 8080

# On Node B (exit node):
yaya init
yaya peer add <Node-A-pubkey>@<Node-A-ip>:51820
yaya exit serve

# From the internet:
curl http://<Node-B-public-ip>:<exposed-port>/
# Should see Node A's directory listing

# Verify PQ:
wg show wg0  # Should show PSK: (set)
yaya status  # Should show "Rosenpass: active, last PSK rotation: 45s ago"
```

---

## Challenging the Assumptions

### Rust vs Go?
**Verdict: Rust is correct.** Mullvad's GotaTun proves Rust outperforms Go for packet
processing (0.01% vs 0.40% crash rate, better latency). The WireGuard Rust ecosystem
(GotaTun, defguard_wireguard_rs, Rosenpass) is now mature enough. Go's advantage is
faster development velocity, but for a security-critical binary that processes every
packet, Rust's safety guarantees and performance are worth the slower development.

### Rosenpass vs ML-KEM only?
**Verdict: Rosenpass is correct.** It's the only production PQ wrapper for WireGuard.
Building our own ML-KEM integration would require reimplementing the PSK injection
mechanism, the key rotation logic, and the formal verification. Rosenpass has done this.
The hybrid (Classic McEliece + ML-KEM) approach is more conservative than ML-KEM alone.

### Minimal coordinator vs coordinator-less?
**Verdict: Minimal coordinator is correct.** Pure P2P discovery (DHT) adds massive
complexity for networks of 3-50 nodes. A coordinator is ~2000 LOC and can run on the
same box as a node. The data plane is still fully P2P — the coordinator is only needed
for initial peer discovery.

### No root?
**Verdict: Not practical.** WireGuard (even userspace) requires CAP_NET_ADMIN for TUN
device creation and route manipulation. The mitigation is: daemon runs with minimal
Linux capabilities (not full root), CLI is unprivileged and talks to daemon via socket.

### Installer under 300 lines?
**Verdict: Achievable at ~200 lines.** The installer only needs to: detect OS/arch,
download binary + signature, verify signature, install to /usr/local/bin, run `yaya init`.
rustup is ~400 lines but does much more (toolchain management, PATH modification).
Yaya's installer is simpler.
