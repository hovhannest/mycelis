# Mycelia — Technology Landscape & Competitive Survey

**Status:** Living draft — Last updated: 2026-07-22  
**Research pass:** Internet verification of stack, competitors, and market (see also [market-and-research.md](market-and-research.md))

This document surveys the technologies referenced in the Mycelia PRD, and the broader landscape of comparable and competing projects. It exists to inform architecture decisions, sharpen positioning, and identify prior art we can reuse.

**TL;DR:** Mycelia sits in a mostly empty niche: a *higher-level organization, authorization, and service-discovery layer* on top of an existing cryptographic mesh substrate (Reticulum). Nearly all comparable projects solve either the substrate (Reticulum, Yggdrasil, cjdns, libp2p), the naming (Handshake, ENS, GNS), the identity (SSB, DIDs), or the apps (Briar, Nomad Network) — but almost none combine *domains + communities + scoped service discovery + policy* as a distinct fabric. The closest conceptual analogs are **libp2p** (comprehensive P2P framework) and **Veilid** (privacy-app framework), and the closest ecosystem overlap is **Nomad Network / Sideband / MeshChat** (the existing Reticulum application layer).

---

## Table of Contents

1. [Foundational Technologies](#1-foundational-technologies)
2. [Transports & Local Discovery](#2-transports--local-discovery)
3. [Competitor & Prior-Art Landscape](#3-competitor--prior-art-landscape)
4. [Comparison Matrix](#4-comparison-matrix)
5. [Where Mycelia Fits (Gap Analysis)](#5-where-mycelia-fits-gap-analysis)
6. [Risks, Open Questions & Recommendations](#6-risks-open-questions--recommendations)
7. [Sources](#7-sources)
8. [Design Discussion & Decisions (2026-07-22)](#8-design-discussion--decisions-2026-07-22)

---

## 1. Foundational Technologies

### Reticulum Network Stack (RNS) — the substrate Mycelia builds on

Reticulum is a cryptography-based networking stack for local and wide-area networks over readily available hardware, designed to keep working under adverse conditions (very high latency, extremely low bandwidth). Created by Mark Qvist; the protocol was dedicated to the public domain in 2016; reference implementation **RNS 1.0.0** released July 2025 (manual tracked ~1.2.x–1.4.x into 2026).

Key properties directly relevant to Mycelia:

- **Cryptographic identity = address.** Every destination is a cryptographic identity; the address is derived from the public key. Identities are self-sovereign, portable, and persistent.
- **Encryption is mandatory.** X25519 ECDH + Ed25519 signatures; AES-256-CBC with HMAC-SHA256 (Fernet-style tokens); ephemeral keys + forward secrecy by default. Unencrypted packets are invalid.
- **No source addresses.** Packets carry no origin information — reinforces `Reachable ≠ Visible ≠ Authorized`.
- **Transport-agnostic.** LoRa (RNode), packet radio (AX.25/KISS), TCP/IP, UDP/IP, Ethernet, serial, I2P tunnels, custom stdio interfaces. Store-and-forward and unforgeable delivery acknowledgements.
- **Companion protocols:**
  - **LXMF** — delay/disruption-tolerant messaging with propagation nodes; propagation stamps / PoW for abuse resistance; paper/QR encoding possible; designed for ~300-baud links.
  - **LXST** — real-time voice/signal transport (used by Sideband voice features).

**Ecosystem caveat (2025–2026):** The Reticulum *License* added anti-AI-training and “no purposeful harm” clauses (April 2025), which blocks packaging in Debian/F-Droid/many distros. Mark Qvist reduced public engagement after late 2025; community forks (notably **RetiNet**, AGPL, RNS 1.0-compatible) and multiple implementations are organizing independently. The *protocol* remains public domain. Mycelia must track which implementation(s) to depend on and how licensing affects distribution. See [market-and-research.md](market-and-research.md).

**Implication for Mycelia:** Reticulum already provides identity, routing, path discovery, transport multiplexing, and store-and-forward. Mycelia must **not** duplicate these. The open space is domains, communities, authorization, scoped visibility, service registry, and policy. Interop discipline: anything not using Reticulum’s cryptographic primitives “is not Reticulum.”

### libp2p Kademlia DHT — the proposed global bootstrap/discovery provider

The PRD names libp2p’s Kademlia DHT as Provider 1 for bootstrap, initial peer discovery, relay discovery, and network entry — a decentralized phone book, **not** used for application traffic.

How it works (design-relevant):

- Maps peer IDs and content keys into a 256-bit keyspace (SHA-256); k-buckets by XOR distance (typically k=20); lookups in **O(log n)** via iterative parallel `FIND_NODE`.
- **Bootstrap nodes are critical infrastructure:** a node that cannot reach any bootstrap peer cannot join the DHT. libp2p/IPFS ship default bootstrap lists (e.g. `bootstrap.libp2p.io` DNSADDR seeds). This is a soft centralization — mitigated in Mycelia by peer cache + PEX + local/RNS providers.
- Dual-identity resolved by demotion: libp2p PeerID = disposable plumbing; Reticulum identity = source of truth.

---

## 2. Transports & Local Discovery

Inherited from Reticulum, plus local-discovery providers named in the PRD:

- **LoRa / RNode** — primary off-grid transport (license-exempt ISM bands, e.g. 902–928 MHz NA).
- **Packet radio (AX.25/KISS), free-space optical, ad-hoc WiFi, serial** — first-class Reticulum interfaces.
- **TCP/UDP over IP** — Internet / private-network bridge; widely used.
- **I2P** — anonymous transport interface for Reticulum.
- **mDNS / DNS-SD (RFC 6762 / 6763)** — Zeroconf local service discovery. libp2p uses `_p2p._udp.local`; private-network isolation via fingerprint suffix is a reusable pattern.
- **BLE advertisements / WiFi multicast / local broadcast** — nearby-peer detection (same role Briar uses for offline sync).

**Takeaway:** reuse Zeroconf/DNS-SD patterns for local discovery; the novel part is *scoping* advertisements by domain/community/visibility.

---

## 3. Competitor & Prior-Art Landscape

### 3.1 Reticulum-native application layer (closest ecosystem overlap)

- **Nomad Network (NomadNet)** — terminal encrypted suite on Reticulum + LXMF: messaging, file sharing, page server/browser (Micron markup), node directory, auth. Client, page/file server, or LXMF propagation node. Works over ~300-baud links.
- **Sideband** — GUI LXMF/LXST client (Android, iOS, Linux, macOS, Windows): messaging, files, voice, telemetry, plugins, page browser.
- **MeshChat / MeshChatX** — full-featured LXMF clients; MeshChatX fork adds broader LXST/tooling; browses Nomad Network pages.

**Positioning:** Nomad Network already hosts pages/services and a node directory, but **not** domains, authorization-bearing communities, or a policy-scoped service registry. Mycelia’s wedge is the structured fabric. **Interop, don’t compete:** register Nomad pages and LXMF endpoints as Mycelia services; target Sideband/MeshChat as clients.

### 3.2 Encrypted overlay / mesh routing

- **Yggdrasil** — E2E-encrypted IPv6 overlay; address from Curve25519 public key hash; spanning-tree + greedy routing; Kademlia-like DHT for coordinates. Userspace Go router; auto-peers via link-local IPv6 multicast. Substrate competitor to Reticulum, not to Mycelia’s fabric layer.
- **cjdns** — intellectual predecessor (key → IPv6, DHT path finder). Largely superseded by Yggdrasil in practice.

### 3.3 Overlay / mesh VPNs

What people use today for “private mesh” — IP-centric, usually with a control plane:

| Project | Model | Control plane | Notes (2026) |
|---|---|---|---|
| **Tailscale** | WireGuard mesh | Vendor-hosted; SSO | Market UX leader; ~5M+ active users cited; proprietary control plane |
| **Headscale** | Tailscale-compatible | Self-hosted | Official clients + your coordination server |
| **NetBird** | WireGuard mesh | Cloud or self-host | OSS platform + UI + OIDC; Series A Jan 2026 (~USD 10M) |
| **Nebula** (Slack) | Noise + Curve25519 | Self-hosted CA + Lighthouse | Certs carry overlay IP + **groups** + expiry — closest authz analog |
| **ZeroTier** | L2 virtual Ethernet | SaaS (self-host possible) | Broadcast/multicast; Hamachi-class use cases |
| **Twingate / Cloudflare Zero Trust** | ZTNA | Vendor | Enterprise per-app access, not mesh fabric |

**Differentiator:** these assume IP and (usually) a coordinator. Mycelia targets no mandatory central infra + heterogeneous transports (LoRa, radio, offline).

### 3.4 P2P application frameworks

- **Veilid** (Cult of the Dead Cow; Rust + Flutter) — “IPFS + Tor but faster”; private routing (safety + private routes); Kademlia-style DHT with schemas/multi-writer; **keyed networks** ≈ Mycelia domains; any node can bootstrap. Flagship: VeilidChat. UDP/TCP/WebSocket; Linux/macOS/Windows/Android/iOS/WASM.
- **GNUnet** — research stack with DHT, privacy-first networking, and the **GNU Name System** (best naming prior art). Earlier drafts mislabeled this space as “Ceptr”; Ceptr is a separate Holochain-adjacent project — **GNUnet/GNS** is the relevant analog.
- **IPFS / libp2p** — content addressing + Kad-DHT + mDNS + bootstrap. Reference for discovery mechanics, not fabric competitor.
- **Secure Scuttlebutt (SSB)** — offline-first gossip; Ed25519 feeds; follow-graph trust; Secret Handshake; petnames. Prior art for communities + anti-spam.
- **Hypercore / Holepunch** — append-log P2P data (adjacent).

### 3.5 Decentralized identity

- **W3C DIDs v1.1** — Candidate Recommendation Snapshot (W3C invited implementations 2026); DID → DID document with keys and **service endpoints**.
- **Verifiable Credentials Data Model v2.0** — tamper-evident signed claims; natural fit for domain/community membership attestations.

Mycelia does **not** need a DID method to function (RNS identities suffice), but VC-style attestations and optional `did:reticulum` / `did:mycelia` remain interoperability options.

### 3.6 Decentralized naming / domains

Mycelia “domains” ≠ DNS. Prior art:

- **GNU Name System (GNS)** — [RFC 9498](https://www.rfc-editor.org/rfc/rfc9498.html). Each user administers their own root; zones = public keys; signed records; DHT publication with **query privacy / blinded keys**. Best philosophical match for self-sovereign domains.
- **Handshake (HNS)** — blockchain root-zone bidding; limited resolver adoption.
- **ENS** — Ethereum naming; Web3/wallet-centric.
- **Namecoin** — historical `.bit`.

**Recommendation:** document terminology collision; prefer petnames over global naming; treat blockchain roots as optional gateways only.

### 3.7 Privacy / anonymity networks

- **Tor** — onion routing; directory authorities; strong for clearnet + `.onion`; vulnerable to global traffic correlation.
- **I2P** — garlic routing; DHT peer selection (no directory authority); optimized for in-network services; Reticulum can run over I2P.
- **Nym** — mixnet (packet mixing, cover traffic, timing obfuscation; ~5 hops in anonymous mode) — stronger against global passive adversaries, higher latency.

Mycelia must ensure discovery/registry does not leak the social graph Reticulum/I2P work to hide.

### 3.8 Off-grid / LoRa mesh

- **Meshtastic** — managed flooding; huge community (~40k+ GitHub stars / 80k+ Reddit cited early 2026); AES-256-CTR channels + PKC DMs (firmware 2.5+); hop cap ~7; congestion at scale. Going mainstream with cheap (~$30–50) nodes.
- **MeshCore** — flood-then-learn source routing via dedicated Repeaters/Room Servers; up to ~64 hops; quieter at scale; weaker payload crypto (AES-128-ECB + short MAC frequently criticized); smaller community (launched ~2025).

**Relevance:** transport/app peers sharing hardware ethos with RNode users; weaker E2E story than Reticulum — a positioning asset. Recruit from these communities; bridge via Reticulum LoRa interfaces. Protocols are mutually incompatible.

### 3.9 Offline / censorship-resistant messaging apps

- **Briar** — Tor when online; Bluetooth/Wi‑Fi sync offline; contact/forum model. App competitor, not fabric.

---

## 4. Comparison Matrix

Legend: ✓ = yes/native, ~ = partial/optional, ✗ = no

| Project | Layer | No mandatory central infra | Self-sovereign identity | E2E crypto default | Non-IP / off-grid | Scoped service discovery | Domains/communities/policy | Language |
|---|---|---|---|---|---|---|---|---|
| **Mycelia** (proposed) | Fabric | ✓ (goal) | ✓ (via RNS) | ✓ (via RNS) | ✓ (via RNS) | ✓ (core) | ✓ (core) | TBD |
| **Reticulum** | Substrate | ✓ | ✓ | ✓ | ✓ | ~ (announces) | ✗ | Python |
| **Nomad / Sideband / MeshChat** | App on RNS | ✓ | ✓ | ✓ | ✓ | ~ (pages/nodes) | ✗ | Python / etc. |
| **Yggdrasil** | IPv6 overlay | ✓ | ✓ (key→IPv6) | ✓ | ✗ (over IP) | ✗ | ✗ | Go |
| **cjdns** | IP mesh | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | C |
| **Veilid** | App framework | ✓ | ✓ | ✓ | ✗ | ~ (DHT) | ~ (keyed nets) | Rust |
| **GNUnet / GNS** | Stack + naming | ✓ | ✓ | ~ | ~ | ~ | ~ (zones) | C |
| **IPFS / libp2p** | Storage + P2P | ~ (bootstrap) | ✓ (PeerID) | ~ | ✗ | ~ (mDNS/DHT) | ✗ | Go/JS/Rust |
| **SSB** | Social protocol | ~ (pubs) | ✓ | ✓ (SHS) | ~ (LAN) | ✗ | ~ (follow graph) | JS |
| **Nebula** | Overlay VPN | ~ (CA/lighthouse) | ~ (certs) | ✓ | ✗ | ✗ | ~ (cert groups) | Go |
| **Tailscale** | Overlay VPN | ✗ | ~ (SSO) | ✓ (WG) | ✗ | ✗ | ~ (ACLs) | Go |
| **NetBird / Headscale** | Overlay VPN | ~ (self-host) | ~ (OIDC) | ✓ | ✗ | ✗ | ~ (policies) | Go |
| **HNS / ENS** | Naming | ✗/~ (chain) | ✓ | ~ | ✗ | ✗ | ~ (names) | various |
| **Briar** | Messaging | ✓ | ✓ | ✓ | ~ (BT/WiFi) | ~ | ✗ | Java |
| **Meshtastic / MeshCore** | LoRa app | ✓ | ~ | ~ | ✓ (LoRa) | ✗ | ~ (channels) | C++ |
| **Tor / I2P / Nym** | Anonymity | ~/✓/~ | ~ | ✓ | ✗ | ✗ | ✗ | C/Java/Rust |

*Point of the matrix: the rightmost two columns are nearly empty elsewhere. That is the Mycelia wedge.*

---

## 5. Where Mycelia Fits (Gap Analysis)

**The empty niche.** Market is well-served at every layer *except* Mycelia’s:

| Layer | Served by |
|---|---|
| Substrate | Reticulum, Yggdrasil, cjdns, libp2p |
| Naming | GNS, Handshake, ENS |
| Identity | RNS identities, DIDs, SSB feeds |
| Apps | Nomad Network, Sideband, Briar, VeilidChat, Meshtastic |
| **Org / authz / scoped service fabric** | **largely unserved** |

**Positioning line:** *the DNS + service mesh + org directory of the decentralized mesh world* — domains (ownership), communities (participation), discoverable authorized services — without a central authority.

**Nearest neighbors to borrow from:**

1. Nomad Network / Sideband / MeshChat — same substrate; interop
2. Veilid — keyed networks, private routing, schema’d DHT
3. GNS / GNUnet — self-owned roots, blinded lookups
4. SSB — follow-graph trust / anti-spam
5. Nebula — cert-embedded group membership
6. W3C VCs — signed membership attestations

---

## 6. Risks, Open Questions & Recommendations

1. **Dual discovery / dual identity.** Keep libp2p as optional IP-only disposable locator; do **not** build a private empty Mycelia DHT. RNS identity remains authoritative.
2. **Bootstrap soft-centralization.** Pluggable redundant providers + peer cache + PEX; document degradation paths.
3. **Terminology:** Mycelia domain ≠ DNS domain; use petnames for UX naming.
4. **Social-graph leakage** at discovery/registry — enforce GNS-style query privacy / blinded lookups cryptographically.
5. **Authorization format** — prototype signed, revocable, offline-verifiable attestations (VC-inspired or Nebula-style).
6. **Ecosystem interop** — build on RNS/LXMF; register Nomad/LXMF as services; remain a Reticulum citizen.
7. **Language / deps / MCU** — **Resolved** in [tech-stack.md](tech-stack.md): Rust + tiered profiles; FreeTAKTeam `reticulum-rs` primary; libp2p full-only.
8. **Reticulum License / packaging** — mitigated by preferring EPL-2.0 FreeTAKTeam crates; Python `rns` / lelloman rns-rs (Reticulum License) remain interop/fallback only.
9. **Maintainer continuity** — RNS public engagement reduced; plan for community implementations and protocol-stable APIs (interop gates in tech-stack §8).

---

## 7. Sources

Verified / primary (2026-07-22 research pass):

**Foundational**

- https://reticulum.network
- https://reticulum.network/manual/whatis.html
- https://github.com/markqvist/Reticulum
- https://github.com/markqvist/Reticulum/releases/tag/1.0.0
- https://reticulum.network/manual/software.html
- https://github.com/markqvist/LXMF
- https://github.com/markqvist/nomadnet
- https://unsigned.io/sideband
- https://github.com/liamcottle/reticulum-meshchat
- https://libp2p.io/docs/dht/
- https://specs.ipfs.tech/routing/kad-dht/
- https://github.com/libp2p/specs/blob/master/discovery/mdns.md
- RFC 6762 / RFC 6763

**Overlays & VPNs**

- https://yggdrasil-network.org/
- https://www.youngju.dev/blog/culture/2026-05-16-overlay-vpn-mesh-networking-2026-tailscale-headscale-zerotier-nebula-wireguard-netbird-deep-dive.en
- https://www.infralovers.com/blog/2026-05-27-netbird-vs-tailscale-vs-headscale/

**Naming & identity**

- https://veilid.com/
- https://veilid.com/how-it-works/private-routing/
- https://www.rfc-editor.org/rfc/rfc9498.html (GNS)
- https://docs.gnunet.org/latest/users/gns.html
- https://www.w3.org/TR/did-1.1/
- https://www.w3.org/TR/vc-data-model-2.0/
- https://ssbc.github.io/scuttlebutt-protocol-guide/

**Privacy**

- https://nym.com/blog/nymvpn-v-vpns-v-tor-v-i2p-v-dvpns
- https://i2p.net/en/docs/overview/comparison/
- https://briarproject.org/

**Off-grid**

- https://d-central.tech/meshtastic-vs-meshcore/
- https://meshtastic.org/
- https://www.offgrid.technology/index.php/2026/05/04/meshtastic-is-going-mainstream-and-theres-a-48-device-that-proves-it/

**Market**

- https://www.thebusinessresearchcompany.com/report/wireless-mesh-network-global-market-report
- See [market-and-research.md](market-and-research.md)

---

## 8. Design Discussion & Decisions (2026-07-22)

Product constraints used as filters:

- **C1 — Zero-config Internet bootstrap.** Internet connectivity alone should join the network; no user-supplied bootstrap lists required.
- **C2 — Modularity for restricted/hostile networks.** Adapt by swapping modules, not rewriting.

### 8.1 Bootstrap reality

A Kademlia DHT cannot bootstrap from nothing. Constraint 1 reframed as: **no single load-bearing anchor.** Separate:

- **(a) Cold-start anchors** — unavoidable; mitigate with diversity + redundancy + peer cache + PEX.
- **(b) Scalable global discovery** — what Kad-DHT is good at after first contact.

### 8.2 Decision: libp2p as disposable locator; do NOT build our own DHT

- Do not stand up a private Mycelia-only DHT (recreates fixed anchors; misses libp2p NAT tooling).
- libp2p DHT is IP-only — correct role for C1.
- PeerID = plumbing; RNS identity = truth.
- Provider order: Peer Cache/PEX → Local → RNS Announces → libp2p DHT → Static.
- Prefer piggybacking a **public** DHT with a Mycelia namespace over operating private seeds.

### 8.3 Nomad Network & “why build Mycelia?”

Shared goals with Nomad/Sideband/LXMF: no central infra, key identity, E2E privacy, offline resilience.

**Missing (Mycelia wedge):** structured domains, authorization-bearing communities, policy-scoped service registry, selective visibility at mesh scale.

Analogy: Reticulum ≈ TCP/IP; Nomad/Sideband ≈ apps; **Mycelia ≈ directory + DNS + service mesh**.

**Decision:** build on and interoperate — RNS + LXMF control plane; index Nomad/LXMF as services; MVP = domain/community/service directory and access-policy layer over existing Reticulum services.

### 8.4 Promoted into the PRD vs kept as research

| Suggestion | In PRD | Driven by |
|---|---|---|
| Honest zero-config bootstrap | ✓ | C1 |
| Peer Cache + PEX; layered providers | ✓ | C1 |
| libp2p as disposable Internet locator | ✓ | C1 |
| Modular / swappable stack | ✓ | C2 |
| Censorship / pluggable transports | ✓ | C2 |
| Discovery privacy (blinded lookups) | ✓ | C2 |
| Capability-based offline authz | ✓ | C2 |
| Sybil/spam resistance | ✓ | open discovery |
| Distributed signed expiring registry | ✓ | C1/C2 |
| Ecosystem interop (RNS/LXMF/Nomad) | ✓ | reuse |

Kept for later (see market research / open questions):

- Group key management / revocation (VCs, MLS/RFC 9420)
- Registry CRDT choice
- Petname UX for domains
- Token format bake-off beyond MVP compact attestations (VC v2, Biscuit, macaroons)
- Whether fallback to lelloman `rns-rs` is ever required (tech-stack §4.3)
