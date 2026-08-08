<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Yaya — The Self-Financing Compute Mesh</title>
  <meta name="description" content="Yaya is a post-quantum sovereign mesh that lets you download, share, and buy research compute. Computing reasoning for everyone. Free to join.">
  <style>
    *,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
    :root{
      --bg:#0a0a0f;
      --surface:#12121a;
      --border:#1e1e2e;
      --text:#e0e0e8;
      --muted:#8888a0;
      --accent:#7c6ff7;
      --accent-glow:#7c6ff740;
      --green:#34d399;
      --gold:#f5b942;
      --gold-dim:#f5b94220;
    }
    html{scroll-behavior:smooth}
    body{
      font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
      background:var(--bg);
      color:var(--text);
      line-height:1.6;
      -webkit-font-smoothing:antialiased;
    }

    /* nav */
    nav{
      position:fixed;top:0;left:0;right:0;z-index:100;
      background:var(--bg)ee;
      backdrop-filter:blur(12px);
      border-bottom:1px solid var(--border);
      padding:0 2rem;
      display:flex;align-items:center;justify-content:space-between;
      height:60px;
    }
    nav .logo{font-weight:700;font-size:1.25rem;letter-spacing:-.02em}
    nav .logo span{color:var(--accent)}
    nav ul{list-style:none;display:flex;gap:2rem}
    nav a{color:var(--muted);text-decoration:none;font-size:.9rem;transition:color .2s}
    nav a:hover{color:var(--text)}
    nav .nav-cta{
      color:var(--gold);border:1px solid var(--gold);
      padding:.3rem .9rem;border-radius:6px;font-weight:600;
      transition:all .2s;
    }
    nav .nav-cta:hover{background:var(--gold);color:var(--bg)}

    /* hero */
    .hero{
      min-height:100vh;
      display:flex;flex-direction:column;align-items:center;justify-content:center;
      text-align:center;
      padding:6rem 1.5rem 4rem;
      position:relative;
      overflow:hidden;
    }
    .hero::before{
      content:'';position:absolute;
      width:700px;height:700px;
      background:radial-gradient(circle,var(--accent-glow) 0%,transparent 70%);
      top:5%;left:50%;transform:translateX(-50%);
      pointer-events:none;
    }
    .hero::after{
      content:'';position:absolute;
      width:500px;height:500px;
      background:radial-gradient(circle,var(--gold-dim) 0%,transparent 70%);
      bottom:10%;right:10%;
      pointer-events:none;
    }
    .hero .tagline{
      color:var(--green);
      font-family:"Courier New",monospace;
      font-size:.95rem;
      letter-spacing:.05em;
      margin-bottom:1.5rem;
    }
    .hero h1{
      font-size:clamp(2.5rem,6vw,4.5rem);
      font-weight:800;
      letter-spacing:-.03em;
      line-height:1.1;
      max-width:800px;
      margin-bottom:1.5rem;
    }
    .hero h1 em{font-style:normal;color:var(--accent)}
    .hero h1 .gold{color:var(--gold)}
    .hero p.subtitle{
      color:var(--muted);
      font-size:1.15rem;
      max-width:600px;
      margin-bottom:2.5rem;
    }

    /* install box */
    .install-box{
      background:var(--surface);
      border:1px solid var(--border);
      border-radius:12px;
      padding:1rem 1.5rem;
      display:flex;align-items:center;gap:1rem;
      font-family:"Courier New",monospace;
      font-size:1rem;
      position:relative;
      margin-bottom:1rem;
    }
    .install-box code{color:var(--green);flex:1;text-align:left;user-select:all}
    .install-box .copy-btn{
      background:none;border:1px solid var(--border);
      color:var(--muted);
      border-radius:6px;padding:.4rem .75rem;
      cursor:pointer;font-size:.8rem;
      transition:all .2s;
    }
    .install-box .copy-btn:hover{border-color:var(--accent);color:var(--text)}
    .install-hint{color:var(--muted);font-size:.8rem}

    /* sections */
    section{padding:5rem 2rem;max-width:1000px;margin:0 auto}
    section h2{
      font-size:2rem;font-weight:700;letter-spacing:-.02em;
      margin-bottom:.5rem;
    }
    section .section-sub{color:var(--muted);margin-bottom:3rem;font-size:1.05rem}

    /* big value prop */
    .value-props{
      display:grid;
      grid-template-columns:1fr 1fr 1fr;
      gap:0;
      margin-bottom:4rem;
      border:1px solid var(--border);
      border-radius:12px;
      overflow:hidden;
    }
    .value-prop{
      padding:2.5rem 2rem;
      text-align:center;
      border-right:1px solid var(--border);
    }
    .value-prop:last-child{border-right:none}
    .value-prop .big{
      font-size:2.5rem;font-weight:800;
      line-height:1;margin-bottom:.5rem;
    }
    .value-prop .big.purple{color:var(--accent)}
    .value-prop .big.green{color:var(--green)}
    .value-prop .big.gold{color:var(--gold)}
    .value-prop .label{color:var(--muted);font-size:.9rem}

    /* feature grid */
    .features{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:1.5rem}
    .feature{
      background:var(--surface);
      border:1px solid var(--border);
      border-radius:12px;
      padding:1.75rem;
      transition:border-color .3s;
    }
    .feature:hover{border-color:var(--accent)}
    .feature .icon{font-size:1.5rem;margin-bottom:.75rem;display:block}
    .feature h3{font-size:1.1rem;margin-bottom:.5rem;font-weight:600}
    .feature p{color:var(--muted);font-size:.9rem;line-height:1.5}
    .feature.highlight{border-color:var(--gold)}
    .feature.highlight:hover{border-color:var(--gold);box-shadow:0 0 30px var(--gold-dim)}

    /* how it works */
    .steps{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:1.5rem}
    .step{text-align:center;padding:1.5rem}
    .step .num{
      display:inline-flex;align-items:center;justify-content:center;
      width:40px;height:40px;border-radius:50%;
      background:var(--accent);color:#fff;
      font-weight:700;font-size:1rem;margin-bottom:1rem;
    }
    .step h3{font-size:1rem;margin-bottom:.5rem}
    .step p{color:var(--muted);font-size:.85rem}

    /* compute section */
    .compute-section{
      background:var(--surface);
      border:1px solid var(--border);
      border-radius:16px;
      padding:3rem;
      margin-top:2rem;
    }
    .compute-section h3{font-size:1.5rem;margin-bottom:1rem;font-weight:700}
    .compute-section p{color:var(--muted);font-size:1rem;margin-bottom:1.5rem;max-width:600px}
    .compute-flow{
      display:grid;
      grid-template-columns:1fr auto 1fr auto 1fr;
      align-items:center;
      gap:1rem;
      margin-top:2rem;
    }
    .compute-node{
      background:var(--bg);
      border:1px solid var(--border);
      border-radius:10px;
      padding:1.25rem;
      text-align:center;
    }
    .compute-node .node-title{font-weight:600;font-size:.95rem;margin-bottom:.25rem}
    .compute-node .node-desc{color:var(--muted);font-size:.8rem}
    .compute-arrow{color:var(--accent);font-size:1.5rem;text-align:center}

    /* stack diagram */
    .stack{
      background:var(--surface);
      border:1px solid var(--border);
      border-radius:12px;
      padding:2rem;
      overflow-x:auto;
    }
    .stack pre{
      font-family:"Courier New",monospace;
      font-size:.85rem;
      color:var(--green);
      line-height:1.6;
      margin:0;
    }

    /* comparison table */
    .comparison{overflow-x:auto}
    .comparison table{
      width:100%;border-collapse:collapse;
      font-size:.9rem;
    }
    .comparison th,.comparison td{
      padding:.75rem 1rem;
      text-align:left;
      border-bottom:1px solid var(--border);
    }
    .comparison th{color:var(--muted);font-weight:500;font-size:.8rem;text-transform:uppercase;letter-spacing:.05em}
    .comparison td:first-child{font-weight:600}
    .comparison .yes{color:var(--green)}
    .comparison .no{color:#f87171}

    /* cta */
    .cta{text-align:center;padding:6rem 2rem}
    .cta h2{font-size:clamp(2rem,5vw,3rem);margin-bottom:1rem;font-weight:800}
    .cta p{color:var(--muted);margin-bottom:2.5rem;font-size:1.1rem;max-width:500px;margin-left:auto;margin-right:auto}
    .cta-buttons{display:flex;gap:1rem;justify-content:center;flex-wrap:wrap}
    .cta .btn{
      display:inline-block;
      padding:.9rem 2rem;border-radius:8px;
      text-decoration:none;font-weight:600;font-size:1rem;
      transition:all .2s;
    }
    .cta .btn-primary{background:var(--accent);color:#fff}
    .cta .btn-primary:hover{opacity:.85}
    .cta .btn-gold{background:var(--gold);color:var(--bg)}
    .cta .btn-gold:hover{opacity:.85}

    /* footer */
    footer{
      border-top:1px solid var(--border);
      padding:2rem;text-align:center;
      color:var(--muted);font-size:.8rem;
    }
    footer a{color:var(--accent);text-decoration:none}

    @media(max-width:768px){
      .value-props{grid-template-columns:1fr}
      .value-prop{border-right:none;border-bottom:1px solid var(--border)}
      .value-prop:last-child{border-bottom:none}
      .compute-flow{grid-template-columns:1fr;text-align:center}
      .compute-arrow{transform:rotate(90deg)}
    }
    @media(max-width:600px){
      nav ul{display:none}
      .install-box{flex-direction:column;text-align:center}
    }
  </style>
</head>
<body>

<nav>
  <div class="logo"><span>yaya</span>.sh</div>
  <ul>
    <li><a href="#network">Network</a></li>
    <li><a href="#compute">Compute</a></li>
    <li><a href="#how">How it works</a></li>
    <li><a href="#stack">Stack</a></li>
    <li><a href="https://docs.yaya.sh">Docs</a></li>
    <li><a href="https://github.com/AndreTregear/yaya_vpn">GitHub</a></li>
    <li><a href="#compute" class="nav-cta">Buy Compute</a></li>
  </ul>
</nav>

<!-- Hero -->
<div class="hero">
  <div class="tagline">Computing reasoning — now available for everyone.</div>
  <h1>Download. Connect.<br><em>Reason.</em><br>The mesh <span class="gold">pays for itself.</span></h1>
  <p class="subtitle">
    Yaya is a self-financing post-quantum mesh network. Join for free, share your spare compute,
    buy research-grade reasoning power — all over a sovereign, encrypted mesh. No middlemen.
  </p>
  <div class="install-box">
    <code>curl -fsSL yaya.sh | bash</code>
    <button class="copy-btn" onclick="navigator.clipboard.writeText('curl -fsSL yaya.sh | bash');this.textContent='Copied!';setTimeout(()=>this.textContent='Copy',1500)">Copy</button>
  </div>
  <div class="install-hint">Free to join. Linux &amp; macOS &mdash; x86_64, ARM64, ARMv7</div>
</div>

<!-- Value Props -->
<section>
  <div class="value-props">
    <div class="value-prop">
      <div class="big green">Free</div>
      <div class="label">To join the mesh</div>
    </div>
    <div class="value-prop">
      <div class="big purple">Post-Quantum</div>
      <div class="label">WireGuard + Rosenpass encryption</div>
    </div>
    <div class="value-prop">
      <div class="big gold">Self-Financing</div>
      <div class="label">The network sustains itself</div>
    </div>
  </div>
</section>

<!-- Network Features -->
<section id="network">
  <h2>A sovereign mesh, not just a VPN.</h2>
  <p class="section-sub">Post-quantum encrypted. Peer-to-peer. No accounts. No cloud dependency.</p>
  <div class="features">
    <div class="feature">
      <span class="icon">&#128272;</span>
      <h3>Post-Quantum Encryption</h3>
      <p>Rosenpass hybrid key exchange (Classic McEliece + ML-KEM-768) refreshes your WireGuard PSK every 2 minutes. Harvest-now-decrypt-later attacks fail.</p>
    </div>
    <div class="feature">
      <span class="icon">&#128376;</span>
      <h3>True P2P Mesh</h3>
      <p>Devices connect directly via WireGuard tunnels. The coordinator only bootstraps — your data never touches a third party.</p>
    </div>
    <div class="feature">
      <span class="icon">&#9940;</span>
      <h3>No Accounts, No Tracking</h3>
      <p>Identity is a key pair on your machine. No email, no OAuth, no telemetry. Your mesh is sovereign.</p>
    </div>
    <div class="feature">
      <span class="icon">&#127760;</span>
      <h3>NAT Traversal Built In</h3>
      <p>STUN hole punching for direct connections. QUIC relay fallback when needed. Works behind any firewall.</p>
    </div>
    <div class="feature">
      <span class="icon">&#128259;</span>
      <h3>Expose &amp; Exit Nodes</h3>
      <p>Publish services with <code>yaya expose</code>. Route traffic through any peer with <code>yaya exit</code>. Full control.</p>
    </div>
    <div class="feature">
      <span class="icon">&#128268;</span>
      <h3>Mesh-Internal DNS</h3>
      <p>Every peer gets a <code>.yaya</code> hostname. Access devices by name. DNS never leaks outside the mesh.</p>
    </div>
  </div>
</section>

<!-- Research Compute -->
<section id="compute">
  <h2>Buy &amp; sell research compute.</h2>
  <p class="section-sub">The mesh is the marketplace. Every node can be a provider.</p>
  <div class="features">
    <div class="feature highlight">
      <span class="icon">&#129504;</span>
      <h3>Reasoning on Demand</h3>
      <p>Access distributed computing power for research, inference, and analysis. Buy exactly the compute you need from peers on the mesh.</p>
    </div>
    <div class="feature highlight">
      <span class="icon">&#128200;</span>
      <h3>Sell Your Spare Cycles</h3>
      <p>Have idle compute? Share it with the mesh and earn. Your machine works while you sleep. The network finances itself.</p>
    </div>
    <div class="feature highlight">
      <span class="icon">&#127891;</span>
      <h3>Built for Researchers</h3>
      <p>Universities, labs, independent researchers — access compute without procurement bureaucracy or cloud vendor lock-in.</p>
    </div>
  </div>

  <div class="compute-section">
    <h3>How the compute mesh works</h3>
    <p>Every Yaya node is both a consumer and a potential provider. The mesh routes compute jobs over the same encrypted tunnels as your data — no separate infrastructure needed.</p>
    <div class="compute-flow">
      <div class="compute-node">
        <div class="node-title">You</div>
        <div class="node-desc">Submit a reasoning job</div>
      </div>
      <div class="compute-arrow">&rarr;</div>
      <div class="compute-node">
        <div class="node-title">Yaya Mesh</div>
        <div class="node-desc">Encrypted P2P routing</div>
      </div>
      <div class="compute-arrow">&rarr;</div>
      <div class="compute-node">
        <div class="node-title">Compute Peers</div>
        <div class="node-desc">Execute &amp; return results</div>
      </div>
    </div>
  </div>
</section>

<!-- How it works -->
<section id="how">
  <h2>Up and running in 60 seconds.</h2>
  <p class="section-sub">No config files. No sign-ups. No credit card.</p>
  <div class="steps">
    <div class="step">
      <div class="num">1</div>
      <h3>Download</h3>
      <p><code>curl -fsSL yaya.sh | bash</code><br>Signed binary, verified install.</p>
    </div>
    <div class="step">
      <div class="num">2</div>
      <h3>Join the mesh</h3>
      <p><code>yaya peer add --invite</code><br>Connect to peers. Encrypted automatically.</p>
    </div>
    <div class="step">
      <div class="num">3</div>
      <h3>Compute</h3>
      <p><code>yaya compute buy</code><br>Buy research compute from the mesh, or share yours.</p>
    </div>
    <div class="step">
      <div class="num">4</div>
      <h3>Earn</h3>
      <p><code>yaya compute sell</code><br>The network sustains itself. Everyone benefits.</p>
    </div>
  </div>
</section>

<!-- Stack -->
<section id="stack">
  <h2>The stack.</h2>
  <p class="section-sub">Every layer built for speed, safety, and sovereignty.</p>
  <div class="stack">
    <pre>
┌──────────────────────────────────────────────────────┐
│             Compute Marketplace                       │
│    (buy/sell reasoning · job routing · settlements)   │
├──────────────────────────────────────────────────────┤
│               Application Layer                       │
│          (any TCP/UDP service on your mesh)           │
├──────────────────────────────────────────────────────┤
│            .yaya Internal DNS                         │
│       (mesh-only name resolution, zero leaks)         │
├──────────────────────────────────────────────────────┤
│          yaya expose  /  yaya exit                    │
│    (reverse proxy · exit routing · multi-hop)         │
├──────────────────────────────────────────────────────┤
│            WireGuard Tunnel (GotaTun)                 │
│     ChaCha20-Poly1305 · Curve25519 · Noise_IK        │
│      Rekey ~120s · Key zeroed at ~540s                │
├──────────────────────────────────────────────────────┤
│          Rosenpass Post-Quantum Layer                  │
│    Classic McEliece 460896 + ML-KEM-768 hybrid        │
│     → PSK injected into WireGuard every 2 min         │
├──────────────────────────────────────────────────────┤
│         NAT Traversal  (STUN + QUIC relay)            │
│       Direct P2P preferred → relay fallback           │
├──────────────────────────────────────────────────────┤
│                 UDP Transport                          │
└──────────────────────────────────────────────────────┘</pre>
  </div>
</section>

<!-- Comparison -->
<section id="compare">
  <h2>How Yaya compares.</h2>
  <p class="section-sub">The only mesh that's both a private network and a compute marketplace.</p>
  <div class="comparison">
    <table>
      <thead>
        <tr>
          <th></th>
          <th>Yaya</th>
          <th>Tailscale</th>
          <th>Netbird</th>
          <th>Cloud Compute</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>Post-quantum encryption</td>
          <td class="yes">&#10003;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
        </tr>
        <tr>
          <td>No account required</td>
          <td class="yes">&#10003;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
        </tr>
        <tr>
          <td>Buy/sell compute</td>
          <td class="yes">&#10003;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
          <td>Sell only</td>
        </tr>
        <tr>
          <td>Self-financing</td>
          <td class="yes">&#10003;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
        </tr>
        <tr>
          <td>True P2P mesh</td>
          <td class="yes">&#10003;</td>
          <td class="yes">&#10003;</td>
          <td class="yes">&#10003;</td>
          <td class="no">&#10007;</td>
        </tr>
        <tr>
          <td>One-command install</td>
          <td class="yes">&#10003;</td>
          <td class="yes">&#10003;</td>
          <td class="no">&#10007;</td>
          <td class="no">&#10007;</td>
        </tr>
        <tr>
          <td>No vendor lock-in</td>
          <td class="yes">&#10003;</td>
          <td class="no">Partial</td>
          <td class="yes">&#10003;</td>
          <td class="no">&#10007;</td>
        </tr>
        <tr>
          <td>License</td>
          <td>AGPL-3.0</td>
          <td>BSD-3</td>
          <td>BSD-3</td>
          <td>Proprietary</td>
        </tr>
      </tbody>
    </table>
  </div>
</section>

<!-- CTA -->
<div class="cta">
  <h2>Computing reasoning.<br>For everyone. For free.</h2>
  <p>Join the mesh. Share compute. Buy research power. The network pays for itself.</p>
  <div class="cta-buttons">
    <a href="https://docs.yaya.sh" class="btn btn-primary">Read the docs</a>
    <a href="#compute" class="btn btn-gold">Buy Compute</a>
  </div>
</div>

<footer>
  <p>
    Yaya is <a href="https://github.com/AndreTregear/yaya_vpn">open source</a> under AGPL-3.0.
    No tracking. No cookies. No surveillance.
    Built by researchers, for researchers.
  </p>
</footer>

</body>
</html>
