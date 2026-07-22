# Mycelia — Market Context & Technology Research Notes

**Research date:** 2026-07-22  
**Scope:** Internet research on markets, competitors, and every major protocol/library named in the Mycelia PRD and landscape survey.  
**Companion docs:** [PRD.md](PRD.md), [landscape-survey.md](landscape-survey.md)

---

## 1. Market context

### 1.1 Where demand is coming from

Mycelia sits at the intersection of four demand waves:

| Wave | What buyers/users want | Typical products today |
|---|---|---|
| **Mesh / ZTNA for teams** | Easy private connectivity without site-to-site VPN pain | Tailscale, NetBird, Headscale, Nebula, ZeroTier |
| **Consumer off-grid LoRa** | Local messaging without cell/Internet | Meshtastic, MeshCore, RNode + Reticulum apps |
| **Censorship / metadata privacy** | Survive hostile networks and traffic analysis | Tor, I2P, Nym, Briar, pluggable transports |
| **Self-sovereign identity & naming** | Own keys and names without CA/IdP | DIDs/VCs, GNS, SSB, Handshake/ENS |

Commercial money concentrates in the **first** wave (WireGuard mesh + control plane). Grassroots energy and hardware volume concentrate in the **second**. Mycelia’s thesis is that wave 2–4 users eventually need an **organization fabric** (domains, communities, scoped services) that wave-1 products only solve with centralized coordinators and IP assumptions.

### 1.2 Market sizing (directional, not Mycelia TAM)

Published figures are coarse and mostly describe adjacent markets:

- **Wireless mesh networking (broad industrial/Wi‑Fi mesh):** ~USD 9.95B (2025) → ~USD 11.25B (2026) → ~USD 18.7B by ~2030–2035 at ~13% CAGR ([The Business Research Company](https://www.thebusinessresearchcompany.com/report/wireless-mesh-network-global-market-report)).
- **LoRa / LoRaWAN IoT:** often cited near ~USD 15B by 2026 with high double-digit CAGRs in long-range forecasts — industrial IoT dominated, not hobby mesh.
- **Consumer off-grid LoRa mesh:** treated by analysts as a **blue-ocean / high-growth niche** rather than a measured category. Signal: Meshtastic mainstreaming (tens of thousands of GitHub stars, ~80k+ Reddit members cited early 2026; nodes ~USD 30–50; city meshes in many US metros).
- **Mesh VPN / ZTNA:** Tailscale as de-facto UX standard (multi-million active users cited); NetBird raised ~USD 10M Series A (Jan 2026); Headscale remains the sovereignty fork of Tailscale’s control plane; enterprise spend also flows to Twingate / Cloudflare Zero Trust.

**Implication for Mycelia:** do not compete as “another Tailscale.” Compete as the **governance/discovery layer** for people who outgrow flat LoRa chats and Reticulum key piles — and who refuse SaaS coordinators.

### 1.3 Adoption segments Mycelia can address

1. **Reticulum power users** — already run Nomad Network / Sideband / MeshChat; want domains, invites, service catalogs.
2. **Meshtastic / MeshCore communities** — hit crypto and org limits; may migrate upward via RNode bridges.
3. **Homelab / friend-group “private internet”** — today on Tailscale/Headscale; want offline + radio without giving up org boundaries.
4. **Disaster / civic resilience** — intermittent backhaul; store-and-forward; local autonomy.
5. **Hostile-network operators** — need pluggable transports and discovery that does not leak membership graphs.

---

## 2. Technology deep-dives (project stack)

### 2.1 Reticulum Network Stack (RNS)

| Item | Finding |
|---|---|
| Role in Mycelia | Substrate: identity, routing, path finding, transports, store-and-forward |
| Protocol status | Public domain (2016); defined by reference impl + manual |
| Reference impl | Python `rns`; **1.0.0** released 2025-07-14; docs ~1.2–1.4 into 2026 |
| Crypto | Identity = 512-bit EC keyset; X25519 + Ed25519; AES-256-CBC + HMAC-SHA256; FS by default; **no source addresses** |
| Interfaces | Ethernet, Wi‑Fi, LoRa/RNode, KISS/AX.25, serial, TCP, UDP, I2P, stdio/custom |
| Performance envelope | Designed from ~hundreds of bit/s upward; usable on Pi Zero; userland Python |
| Utilities | `rnsd`, `rnstatus`, `rnpath`, `rnprobe`, `rncp`, `rnx`, `rnid`, `rngit`, … |

#### Reticulum License & ecosystem risk

April 2025 license additions: ban on use in systems intended to harm humans, and ban on use in AI/ML training datasets. Practical effects reported by community (FOSDEM 2026 materials):

- Hard to package in Debian, F-Droid, Alpine main, many grant pipelines.
- Mark Qvist reduced public engagement after late 2025; development continues with less public interaction.
- Community responses: **RetiNet** (AGPL, RNS 1.0-compatible fork) and other implementations organizing independently.

**Mycelia action items:**

- Treat the **protocol** as stable public-domain substrate.
- **Decided:** implement against FreeTAKTeam Rust crates (EPL-2.0); see [tech-stack.md](tech-stack.md). Python `rns` / RetiNet for interop tests only.
- Document redistribution constraints if the lelloman / Reticulum License fallback is ever activated.

### 2.2 LXMF / LXST

| Item | Finding |
|---|---|
| LXMF | Delay-tolerant encrypted messaging over Reticulum; propagation nodes sync messages; stamps/PoW for spam resistance |
| Role in Mycelia | Preferred **control-plane** transport for invites, attestations, policy updates, service announcements |
| Clients | Sideband, Nomad Network, MeshChat / MeshChatX |
| LXST | Real-time voice/signal; relevant for Sideband voice — not core to Mycelia MVP |

### 2.3 libp2p Kademlia DHT

| Item | Finding |
|---|---|
| Role in Mycelia | Optional **Internet locator** only (phone book → RNS identity/reachability) |
| Mechanics | XOR distance, k-buckets (~k=20), iterative `FIND_NODE`, O(log n) |
| Bootstrap | Requires reachable bootstrap peers (e.g. `bootstrap.libp2p.io` DNSADDR seeds). Docs: “Bootstrap nodes are critical infrastructure.” |
| NAT tooling value | AutoNAT, hole punching, QUIC, Circuit Relay v2 — reasons **not** to reinvent a private DHT |
| Constraint | IP-only; useless over LoRa/radio — correct as Internet-plane module |

**Decision (locked in PRD):** piggyback public DHT with Mycelia namespace; PeerID disposable; peer cache + PEX reduce repeat dependency on seeds.

### 2.4 Local discovery (mDNS / BLE / multicast)

- **RFC 6762 / 6763** (mDNS / DNS-SD) remain the standard LAN zero-config pattern.
- libp2p mDNS (`_p2p._udp.local`, multiaddrs in TXT) is a direct template; consider fingerprint-scoped service tags for domain isolation.
- BLE advertisements and Wi‑Fi multicast cover “same room / same park” joins without Internet.

### 2.5 Identity & authorization libraries (candidates)

| Tech | Use for Mycelia | Notes |
|---|---|---|
| **W3C DID 1.1** | Optional interop method | CR Snapshot 2026; service endpoints map to registry idea |
| **VC Data Model 2.0** | Membership attestations | Offline-verifiable signed claims; enterprise traction (e.g. EUDI wallets) |
| **Nebula-style group claims** | Compact membership in certs | Proven at fleet scale; CA-ish — adapt without central CA |
| **Biscuit / macaroons** | Attenuated capabilities | Strong fit for delegatable tokens |
| **MLS (RFC 9420)** | Group key / membership epochs | Later: revocation & group crypto |
| **SSB follow-graph** | Trust / anti-spam | Not a wire dependency — pattern for Sybil resistance |

### 2.6 Privacy transports (optional modules)

| Network | Model | Fit |
|---|---|---|
| **Tor** | 3-hop onion; directory authorities | Pluggable egress / censorship resistance; `.onion` ecosystem |
| **I2P** | Garlic; DHT peers; in-network services | Already a Reticulum interface option |
| **Nym** | Mixnet + cover traffic | Strongest metadata story; higher latency — gateway/optional path |

Discovery/registry design must not undo these layers by leaking membership graphs.

---

## 3. Competitor research summaries (2026)

### 3.1 Mesh VPN camp (substitute for “private mesh” among IP users)

- **Tailscale:** WireGuard + hosted coordination; MagicDNS, SSH, Funnel/Serve; seat pricing pressure in 2026 pushes some teams to Headscale/NetBird.
- **Headscale:** self-hosted Tailscale control plane; keep official clients.
- **NetBird:** full OSS stack + cloud; EU positioning; dual-stack IPv6 (v0.71, May 2026).
- **Nebula:** cert PKI with **groups** — best authz analog in this camp.
- **ZeroTier:** L2 virtual Ethernet; different abstraction (broadcast domains).

**Mycelia vs this camp:** no mandatory coordinator; works offline and over non-IP; domains/communities are cryptographic, not ACL rows on a SaaS.

### 3.2 Reticulum app camp (ecosystem partners)

- **Nomad Network:** pages, files, messaging, node directory — flat, not multi-tenant org fabric.
- **Sideband / MeshChat:** user clients Mycelia should integrate with, not replace.

### 3.3 P2P frameworks

- **Veilid:** closest “build private apps” framework; keyed networks ≈ domains; IP transports only; own DHT/routing — parallel universe, not a dependency.
- **GNUnet + GNS (RFC 9498):** closest naming/privacy philosophy (self-owned roots, blinded queries).

### 3.4 LoRa consumer mesh

- **Meshtastic:** flooding mesh; large community; stronger channel crypto than MeshCore; weak org/scoping story (channels).
- **MeshCore:** structured repeaters; scale/airtime advantages; crypto frequently criticized (AES-128-ECB + short MAC).

Both are **recruitment pools** and **hardware-adjacent**; neither provides Mycelia’s fabric. Reticulum’s mandatory E2E identity crypto is a clear upgrade path narrative.

### 3.5 Naming / identity markets

- Blockchain naming (ENS/Handshake) conflicts with “no tokens / no mandatory infra” spirit → optional gateways only.
- GNS/petnames align with Mycelia domains.

---

## 4. Positioning statement (research-backed)

**Mycelia is not a VPN, not a LoRa messenger, and not a second Reticulum.**  
It is the **organization, authorization, and scoped-discovery fabric** that turns a cryptographic mesh into governable domains and communities with a privacy-preserving service registry — starting as an index and policy layer over existing Reticulum/LXMF services, with pluggable Internet bootstrap via libp2p and graceful degradation to fully offline operation.

---

## 5. Recommended next research / engineering spikes

1. **G1/G2 from tech-stack:** build `mycelia-core` + FreeTAKTeam embedded crates for one MCU target; announce/link against Python `rns`.
2. **Locator spike:** Mycelia-namespaced libp2p DHT record → RNS destination (full profile only).
3. **Attestation spike:** compact Ed25519 membership tokens sized for LoRa.
4. **Dep budget:** `cargo tree` / `cargo deny` per profile (no libp2p/tokio on leaf).
5. **UX spike:** petname UI for domains (avoid calling them “DNS”).

Normative stack: [tech-stack.md](tech-stack.md).

---

## 6. Changelog

| Date | Change |
|---|---|
| 2026-07-22 | Initial research pass; incorporated into project docs; retired standalone `mycelia-prd.md` / `mycelia-landscape-survey.md` drafts |
