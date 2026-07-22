# Mycelia PRD — Decentralized Domain, Community & Service Fabric Built on Reticulum

**Version:** 1.1  
**Status:** Draft  
**Project Name:** Mycelia  
**Last updated:** 2026-07-22

---

## Vision

Mycelia is a decentralized domain, community, and service fabric built on top of the Reticulum Network Stack.

Mycelia enables people, devices, organizations, and communities to securely communicate and share services across decentralized infrastructure without dependence on centralized authorities.

**Reticulum provides:**

- Identity
- Routing
- Path discovery
- Transport abstraction
- Mesh networking

**Mycelia provides:**

- Domains
- Communities
- Service discovery
- Authorization
- Scoped visibility
- Policy enforcement

Together they form a resilient communication ecosystem capable of operating:

- locally
- globally
- online
- offline
- across heterogeneous transports
- without dependence on central infrastructure

*Tagline: "A living network without a center."*

---

## Core Principles

### No Mandatory Central Infrastructure

Mycelia must not require:

- Central servers
- Central cloud services
- Central identity providers
- Certificate authorities
- Central relay operators
- Central service registries

### Automatic Network Participation

Any newly installed node connected to the Internet must be capable of discovering the Mycelia network automatically.

The user should not need to:

- manually configure peers
- manually configure routing
- manually define network topology
- manually supply bootstrap nodes

The network should naturally discover and connect itself.

### Bootstrap Without a Single Point of Dependency

Pure zero-knowledge cold start is not physically possible: a node joining a network must make first contact through *some* reachable participant.

Mycelia does not pretend to remove this need. Instead it removes any single, load-bearing dependency by:

- using many diverse, redundant entry points instead of one
- caching previously seen peers and reconnecting to them first
- exchanging known peers with others (peer exchange / PEX)
- degrading gracefully across multiple discovery providers

The goal is not "no anchors." The goal is that discovery survives the loss of any single anchor, operator, domain, or address. After the first successful join, a node should be able to rejoin from its own memory without contacting any fixed provider.

### Local Autonomy

The network must remain functional when disconnected from Internet discovery systems. Existing peers must continue communicating even if discovery infrastructure becomes unavailable.

### Self-Sovereign Identity

Every node owns its identity. Identities:

- are self-generated
- require no registration
- require no authority
- are portable
- are persistent

### Privacy By Default

- Reachability does not imply visibility.
- Visibility does not imply discoverability.
- Discoverability does not imply authorization.

### Modular and Adaptable

The network must run in hostile and restricted environments, where specific protocols, frequencies, regions, or technologies may be blocked or dangerous to use.

Every externally observable part of the stack must be replaceable:

- transports (swap radios, IP, or tunnels)
- obfuscation / pluggable-transport layers (to defeat protocol filtering)
- discovery providers
- gateways

No component may hard-depend on any single protocol, network, region, or provider. Adapting to a new environment must mean swapping a module, not rewriting the system.

### Censorship and Surveillance Resistance

Mycelia must be able to operate where it is unwelcome.

- Traffic should be able to blend in with, or tunnel through, permitted channels.
- Discovery must not expose who belongs to which domain or community.
- Loss of any provider, region, or transport must not disable the network.

---

## Architecture Overview

```
┌──────────────────────────────────────────────┐
│                 Applications                  │
├──────────────────────────────────────────────┤
│                Mycelia API                     │
├──────────────────────────────────────────────┤
│ Domains | Communities | Service Registry |     │
│ Authorization | Discovery Routines             │
├──────────────────────────────────────────────┤
│         Discovery Providers (pluggable)        │
│  Peer Cache/PEX | Local Discovery |            │
│  Reticulum Announces | libp2p DHT (Internet    │
│  Locator) | Static Peers                       │
├──────────────────────────────────────────────┤
│           Reticulum Network Stack              │
│  Identity | Routing | Path Finding |           │
│  Transport Multiplexing                        │
├──────────────────────────────────────────────┤
│      Pluggable Transports / Obfuscation        │
├──────────────────────────────────────────────┤
│      LoRa | WiFi | TCP | BLE | Serial | etc.   │
└──────────────────────────────────────────────┘
```

---

## Responsibilities

### Reticulum Responsibilities

Reticulum serves as the foundational networking substrate.

Responsible for:

- node identities
- routing
- path discovery
- forwarder operation
- transport abstraction
- mesh formation
- store-and-forward support

Mycelia should not duplicate these capabilities.

### Mycelia Responsibilities

Mycelia provides higher-level organization and visibility controls.

Responsible for:

- domains
- communities
- service discovery
- service advertisement
- authorization
- access control
- scoped visibility
- gateway management

---

## Identity

Each Mycelia node maps to a Reticulum identity.

Example:

```yaml
Mycelia Node:
  Reticulum Identity: <hash>
  Mycelia Metadata:
    Domain: home
    Communities:
      - friends
```

Reticulum identities remain authoritative.

---

## Domains

**Purpose:** domains provide ownership and trust boundaries.

A domain represents a cryptographically controlled group of devices.

Example:

```yaml
Domain: net-home
Members:
  - phone
  - laptop
  - nas
  - home-server
```

Domains define:

- visibility
- authorization
- policy
- service scope

Domains do not define routing.

> **Terminology:** A Mycelia domain is **not** a DNS domain. It is a cryptographic ownership boundary. If human-readable names are needed later, prefer petname-style local naming (cf. GNU Name System, SSB) over a global namespace.

---

## Communities

Domains represent ownership. Communities represent participation.

Example:

```yaml
Domain: home
Communities:
  - friends
  - biking
  - gaming
```

A node may belong to many communities simultaneously.

---

## Service Discovery

Services are first-class entities.

Examples:

```yaml
Service:
  name: printer
---
Service:
  name: matrix
---
Service:
  name: files
---
Service:
  name: camera
```

Services can be advertised with visibility controls.

### Visibility Levels

**Public**

```yaml
visibility: public
```

Visible to everyone.

**Community**

```yaml
visibility: community
```

Visible only to authorized community members.

**Domain**

```yaml
visibility: domain
```

Visible only to domain members.

**Invitation**

```yaml
visibility: invitation
```

Visible only to explicitly authorized identities.

**Hidden**

```yaml
visibility: hidden
```

Not discoverable. Accessible only through referrals or direct identifiers.

---

## Discovery Architecture

### Purpose

Discovery exists exclusively to locate participants. Discovery is not required for message delivery once participants know each other.

### Design Principle

```
Discovery ≠ Routing
```

Reticulum handles routing. Discovery locates peers.

### Discovery Providers

**Provider 1 — libp2p Kademlia DHT (Internet Locator)**

Purpose:

- Bootstrap over the public Internet
- Initial peer discovery
- Relay discovery
- Network entry

The DHT is not used for application traffic. The DHT is not used for ongoing communications.

The DHT functions as a decentralized phone book. It is an IP-only, Internet-plane provider. It acts as a "disposable locator": its only job is to resolve to a peer's authoritative Reticulum identity and reachability. The libp2p peer identity is transport plumbing; the Reticulum identity remains the single source of truth. This avoids maintaining competing identity systems.

Because a DHT cannot bootstrap itself from nothing, Mycelia relies on a large, diverse anchor set — and prefers reusing existing public infrastructure — so that no single bootstrap node is load-bearing. Standing up a private, empty Mycelia-only DHT is discouraged, because it would recreate the very fixed-anchor dependency this design tries to avoid.

This provider is optional and pluggable. In environments where it is blocked, Mycelia must still function through the other providers.

**Provider 2 — Reticulum Announces**

Purpose:

- local visibility
- mesh awareness
- decentralized discovery

**Provider 3 — Local Discovery**

Provides:

- BLE advertisements
- WiFi multicast
- local broadcast
- mDNS

Used for nearby peer detection.

**Provider 4 — Static Peers**

Optional, manually configured peers. Provides zero-dependency bootstrapping.

**Provider 5 — Peer Cache and Peer Exchange**

Once a node has joined, it remembers the peers it has seen and reconnects to them first on subsequent starts. Nodes also exchange lists of known peers (PEX), so knowledge of the network spreads and heals without contacting any fixed provider.

After the first successful join, bootstrap providers become a fallback rather than a requirement.

### Discovery Is Layered and Pluggable

Discovery providers are tried in order of least dependency, and the network keeps working as long as any one of them succeeds:

```
Peer Cache / PEX     (no external dependency)
Local Discovery       (nearby radio neighbors)
Reticulum Announces   (existing mesh topology)
libp2p DHT            (public Internet locator)
Static Peers          (manual fallback)
```

Every provider is a module that can be added, removed, or replaced to suit the environment. A restricted network may disable the DHT and rely on local discovery plus obfuscated static peers; an off-grid mesh may use only local discovery and announces.

### Discovery Privacy

Discovery must not leak the social graph that Reticulum works to hide.

- Records are encrypted to their intended audience.
- Community and domain lookups use blinded keys, so stored records cannot be enumerated or correlated without authorization.
- Answering a query must not reveal domain or community membership to unauthorized parties.

Reachability, visibility, and discoverability remain independent.

### Automatic Network Join

**Requirement:** a new node connecting to the Internet should discover the network automatically.

**Process:**

```
Start Node
  → Peer Cache / PEX
  → Local Discovery
  → Reticulum Announces
  → libp2p DHT
  → Static Peers Found
  → Reticulum Paths Established
  → Node Joins Network
```

The node stops at the first provider that succeeds. Providers that are blocked or unavailable are skipped without failing the join.

---

## Service Registry

The Mycelia Service Registry records service metadata.

Example:

```yaml
Service:
  name: photos
  owner: <identity>
  visibility: domain
```

The registry stores:

- service name
- visibility
- permissions
- capabilities
- node location metadata

Actual traffic flows through Reticulum.

The registry is distributed and partition-tolerant. Records are eventually consistent, carry expiry, and are signed by their owner. Records are encrypted to their audience, so visibility is enforced cryptographically rather than by convention.

A domain may host its own registry records instead of publishing them to any shared index.

---

## Authorization Model

Authorization is capability-based and does not depend on any central authority.

- Access is granted by signed, revocable, delegatable capability tokens (attestations), not by lookups against a central registry.
- Domain and community membership is expressed as signed attestations that can be verified offline.
- Tokens can be attenuated and delegated, and remain verifiable during network partitions.

Because identities are free to generate, discovery and membership must resist Sybil and spam abuse — for example through web-of-trust / follow-graph trust and proof-of-work rate limiting on writes.

---

## Gateways

Gateways connect services with external systems.

Examples:

```
HTTP
SOCKS
LXMF
SIP
```

Gateways are optional. Gateways are the boundary where Mycelia meets non-native systems, so each gateway carries its own trust model. Gateways may also double as bridges, letting communities choose to help the network survive on restricted networks.

---

## Privacy Model

### Core Rule

```
Reachable ≠ Visible ≠ Authorized
```

A node may route packets for another node without knowing its:

- services
- domains
- communities
- ownership

Discovery and the service registry must uphold the same rule: locating a peer must not reveal what it hosts, who owns it, or which domains or communities it belongs to.

### Domain Isolation Example

```yaml
Domain: net-home
Members: phone, laptop, nas, home-server
```

User belongs to: `home`  
User sees: `phone, laptop, nas, home-server`  
…and not the remaining network.

---

## Security Requirements

**Mandatory:**

- End-to-end encryption
- Self-sovereign identity
- Mutual authentication
- Permission-controlled discovery
- Cryptographic domain membership
- Secure service advertisement
- Discovery privacy (no social-graph leakage)
- Capability-based, offline-verifiable authorization
- Sybil and spam resistance
- Censorship resistance via pluggable transports and obfuscation

**Mycelia must never require:**

- certificate authorities
- identity providers
- centralized trust anchors

---

## Software Architecture

Single executable:

```bash
mycelisd
```

Examples:

```bash
mycelisd start
mycelisd status
mycelisd domains list
mycelisd communities list
```

### Internal Components

```
mycelisd
├── Domain Manager
├── Community Manager
├── Service Registry
├── Discovery Manager
│   ├── Peer Cache / PEX
│   └── Pluggable Providers
├── Transport Manager (pluggable + obfuscation)
├── Authorization Manager
├── Gateway Manager
├── Policy Engine
└── API Layer
```

---

## Ecosystem and Interoperability

Mycelia builds on the existing Reticulum ecosystem instead of replacing it.

- Reticulum provides identity, routing, path discovery, transport multiplexing, and store-and-forward.
- LXMF is used as the delay-tolerant control-plane transport for invitations, membership attestations, policy updates, and service announcements.
- Existing Reticulum services (for example Nomad Network pages and LXMF endpoints) can be registered and governed as Mycelia services.

Mycelia is positioned as the organization, authorization, and scoped-discovery layer above these tools — not as another messaging application.

---

## PRD Scope

**Uses Reticulum for:**

- identity
- routing
- mesh networking
- path learning
- transport abstraction

**Uses layered, pluggable discovery:**

- Peer Cache / PEX
- Local Discovery
- Reticulum Announces
- libp2p Kademlia DHT (Internet Locator)
- Static Peers

The libp2p DHT is used for Internet bootstrap and global peer discovery, as a disposable locator that resolves to Reticulum identities. The network must still join and operate when any single provider is unavailable.

---

## Mycelia Features (Summary)

- Domains
- Communities
- Service Registry & Visibility Controls
- Authorization (capability-based)
- Gateways
- Pluggable discovery and transports
- Peer cache and peer exchange

---

## Open Questions (tracked, not blocking the PRD)

- Exact attestation format details (compact Ed25519 blobs are MVP per [tech-stack.md](tech-stack.md); VC/Biscuit/MLS later)
- Registry consistency / CRDT mechanics
- Group key management and membership revocation (e.g. MLS / RFC 9420)

**Resolved (see [tech-stack.md](tech-stack.md)):**

- Implementation language: **Rust**
- Deployment model: **tiered profiles** (`leaf` / `node` / `full`) for MCU + desktop
- Substrate: **FreeTAKTeam `reticulum-rs` family (EPL-2.0)**; lelloman `rns-rs` only as documented fallback
- libp2p: **full profile only**, feature-gated Internet locator

---

## Vision Statement

Mycelia is a decentralized domain, community, and service fabric built on top of the Reticulum Network Stack.

Reticulum provides resilient decentralized connectivity across diverse transports.

Mycelia provides cryptographic domains, communities, scoped discovery, authorization, and service discovery, allowing people, devices, and organizations to securely collaborate without dependence on centralized infrastructure.

Together they form a living communication ecosystem that can grow organically, operate offline or online, and connect communities across local meshes, global Internet connectivity, and future transport technologies.

*"A living network without a center."*
