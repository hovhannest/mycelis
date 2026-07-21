# Mycelis PRD
## Decentralized Transport-Agnostic Communication Fabric

Version: 1.0  
Status: Draft  
Project Name: Mycelis

---

# Vision

Mycelis is a decentralized, transport-agnostic communication fabric that enables devices, people, communities, and organizations to communicate securely without dependence on centralized infrastructure.

Mycelis combines:

- Self-sovereign identity
- Cryptographic trust domains
- Scoped discovery
- Distributed routing
- Emergent relays
- Pluggable transports
- Decentralized peer discovery

into a unified networking platform.

The network behaves like a living mycelial system:

- self-growing
- self-healing
- distributed
- resilient
- adaptive

Every node can contribute connectivity.

Every node can route.

Every node can relay.

No node is required.

No central authority exists.

---

# Tagline

> A living network without a center.

---

# Core Principles

## 1. No Mandatory Infrastructure

Mycelis must not require:

- Central servers
- Central cloud providers
- Central identity providers
- Central certificate authorities
- Central domains
- Central relay operators

---

## 2. Transport Independence

The network should operate across any communication medium capable of transporting bytes.

Examples:

- Bluetooth LE
- Bluetooth Classic
- Wi-Fi
- Ethernet
- LoRa
- Serial
- TCP
- QUIC
- Tor
- I2P
- Future transports

Applications should not depend on the underlying transport.

---

## 3. Self-Sovereign Identity

Each node generates its own cryptographic identity.

Identities:

- are self-owned
- require no registration
- require no authority
- are globally unique

---

## 4. Emergent Infrastructure

Infrastructure should emerge organically.

Nodes dynamically become:

- relays
- gateways
- discovery participants
- service providers

based on capability and policy.

---

## 5. Privacy by Default

Mycelis must not assume:

```text
Routing Visibility
=
Discovery Visibility
=
Service Visibility
```

These concerns remain separate.

---

# Project Goals

## Functional Goals

### Device Connectivity

Allow devices to communicate across arbitrary transport combinations.

Example:

```text
Phone
  |
Bluetooth
  |
Laptop
  |
QUIC
  |
Relay
  |
LoRa
  |
Remote Device
```

---

### Global Reachability

Nodes should be discoverable from anywhere when connectivity exists.

---

### Local Autonomy

The network must remain useful even when disconnected from the Internet.

---

### Domain-Based Isolation

Users must be able to operate private networks while using public infrastructure.

---

### City-Scale Deployment

The architecture should support:

- households
- friend groups
- neighborhoods
- cities
- nations
- global deployments

without architectural changes.

---

# High-Level Architecture

```text
+---------------------------------------------------+
|                   Applications                    |
+---------------------------------------------------+
|                  Platform API                     |
+---------------------------------------------------+
|                 Mycelis Core                      |
|                                                   |
|  Identity | Domains | Discovery | Routing         |
+---------------------------------------------------+
|             Discovery Providers                   |
+---------------------------------------------------+
|                Transport Layer                    |
+---------------------------------------------------+
| BLE | WiFi | TCP | QUIC | Tor | I2P | LoRa | ...  |
+---------------------------------------------------+
```

---

# Core Architecture

## Identity Layer

Every node possesses:

```yaml
Node ID:
  cryptographic identity
```

Properties:

- globally unique
- self-generated
- portable
- persistent

Identity survives:

- IP changes
- location changes
- transport changes

---

# Domains

## Purpose

Domains provide cryptographic ownership boundaries.

Domains replace traditional IP subnet concepts.

---

## Example

```text
Domain: hvh-home

Members:
├── Phone
├── Laptop
├── NAS
└── Home Server
```

Friend:

```text
Domain: alice-home

Members:
├── Laptop
└── Raspberry Pi
```

---

## Domain Properties

Domains define:

- visibility
- trust
- authorization
- service discovery scope
- policy

Domains do not define routing.

---

# Communities

Domains represent ownership.

Communities represent participation.

---

## Example

```yaml
Phone:
  Domains:
    - hvh-home

  Communities:
    - friends
    - gaming
    - hiking
```

---

## Community Examples

```text
Friends Group
Gaming Group
Engineering Team
Neighborhood Mesh
City Network
```

A node may join multiple communities.

---

# Discovery Model

## Design Principle

A fundamental principle of Mycelis:

```text
Routing Scope
≠
Discovery Scope
```

---

## Discovery Requirements

Discovery must:

- be decentralized
- be replaceable
- support global scale
- support privacy
- support authorization

---

## Discovery Levels

### Level 0 — Public

```yaml
Visibility: public
```

Visible to everyone.

Examples:

- public relays
- public services
- community resources

---

### Level 1 — Domain

```yaml
Visibility: domain
```

Visible only to domain members.

---

### Level 2 — Invite

```yaml
Visibility: invite
```

Visible only to explicitly authorized identities.

---

### Level 3 — Hidden

```yaml
Visibility: hidden
```

Not discoverable.

Accessible only via:

- direct identity
- invitation
- out-of-band referral

---

# Discovery Providers

## Default Provider

### libp2p DHT

Responsibilities:

- peer lookup
- peer advertisement
- decentralized discovery
- bootstrap assistance

The DHT is used only for discovery.

Application traffic must not depend on the DHT.

---

## Future Discovery Providers

Potential plugins:

- Nostr-based discovery
- Custom Kademlia DHT
- Community discovery systems
- Alternative decentralized registries

---

# Routing Layer

## Purpose

Responsible for:

- path creation
- path learning
- route propagation
- multi-hop forwarding

---

## Design Principle

Routing is global.

Discovery is scoped.

---

## Example

```text
Phone
  |
Public Relay
  |
Community Relay
  |
Friend Relay
  |
NAS
```

Traffic can traverse global infrastructure.

Visibility remains restricted.

---

# Relay System

## Objective

Enable any suitable node to become a relay.

---

## Relay Capability

Relay eligibility depends on:

```yaml
Public Reachability
Bandwidth
Policy
Resources
User Consent
```

---

## Relay Roles

A relay may:

- forward traffic
- advertise reachability
- assist NAT traversal

A relay must not gain automatic visibility into:

- services
- domains
- communities
- device inventories

---

# Gateway System

Gateways connect Mycelis to external systems.

---

## Internet Gateway

Examples:

- SOCKS Proxy
- HTTP Proxy
- DNS Proxy

Allows Mycelis-only devices to reach Internet resources.

---

## Service Gateway

Bridges:

- MQTT
- REST APIs
- Industrial protocols
- Local applications

---

# Transport Layer

## Required Transports

### Bluetooth LE

Local device communication.

---

### Bluetooth Classic

Legacy device support.

---

### Wi-Fi

Local and infrastructure networking.

---

### Ethernet

High-performance local networking.

---

### TCP

Universal compatibility transport.

---

### QUIC

Preferred Internet transport.

---

## Optional Transports

### Hysteria

Optional plugin.

Purpose:

- difficult networks
- censorship resistance
- lossy links

Mycelis must not depend on Hysteria.

---

### Tor

Anonymous transport layer.

---

### I2P

Anonymous decentralized transport.

---

### LoRa

Low-bandwidth long-range operation.

---

### Serial

Direct hardware communication.

---

# NAT Traversal

The system must support:

- Home NAT
- Carrier NAT
- Mobile networks
- Enterprise firewalls

Techniques may include:

- hole punching
- relay assist
- rendezvous services
- transport-specific solutions

---

# Capability-Based Node Design

Every node runs the same software.

Roles emerge automatically.

---

## Example: Mobile Phone

```yaml
Discovery: true

Relay: false

Gateway: false

Bluetooth: true

QUIC: true
```

---

## Example: Home Server

```yaml
Discovery: true

Relay: true

Gateway: true

Bluetooth: true

QUIC: true

LoRa: true
```

---

## Example: VPS

```yaml
Discovery: true

Relay: true

Gateway: optional
```

---

# Privacy Model

## Design Principle

```text
Route Globally
Discover Locally
Share Explicitly
```

---

## Network Visibility

A node must not automatically discover:

- all devices
- all domains
- all services

---

## Domain Isolation

Example:

```text
Global Mycelis Mesh
200,000 nodes

Domain: hvh-home
4 nodes
```

User sees:

```text
Phone
Laptop
NAS
Server
```

not:

```text
200,000 devices
```

---

## Discovery Authorization

Discovery may require:

- domain membership
- invitation
- capability tokens

Unauthorized requests should return:

```text
Not Found
```

rather than information leakage.

---

# Subnet Replacement Model

Traditional networking:

```text
IP Subnet
```

Mycelis:

```text
Cryptographic Domain
```

Advantages:

- transport independent
- location independent
- relay independent
- globally portable

---

# Scaling Model

The architecture should scale through:

- DHT discovery
- peer exchange
- route learning
- relay emergence
- scoped visibility

---

## Expected Deployment Scales

### Personal

```text
10–100 devices
```

---

### Family/Friends

```text
100–1,000 devices
```

---

### Neighborhood

```text
1,000–10,000 devices
```

---

### City

```text
100,000+ devices
```

---

### Nation

```text
Millions of devices
```

---

### Global

```text
Potentially tens of millions
```

The limiting factors become:

- relay capacity
- discovery traffic
- churn
- abuse mitigation

not architectural boundaries.

---

# Security Requirements

## Mandatory

- End-to-end encryption
- Self-sovereign identity
- Cryptographic authentication
- Secure route establishment
- Permission-controlled discovery

---

## Prohibited Dependencies

The system may not require:

- Central identity servers
- Certificate authorities
- Mandatory cloud infrastructure
- Central operator trust

---

# Software Architecture

## User Experience

Single executable:

```text
mycelisd
```

Examples:

```bash
mycelisd start
```

```bash
mycelisd status
```

---

# Internal Architecture

```text
mycelisd
│
├── Identity Manager
├── Domain Manager
├── Community Manager
├── Routing Engine
├── Discovery Manager
├── Relay Manager
├── Gateway Manager
├── Transport Manager
└── API Layer
```

---

# Plugin Architecture

## Discovery Plugins

```text
libp2p DHT
Nostr
Custom Kademlia
Future Systems
```

---

## Transport Plugins

```text
Bluetooth LE
Bluetooth Classic
WiFi
Ethernet
TCP
QUIC
Tor
I2P
LoRa
Hysteria
Serial
```

---

## Gateway Plugins

```text
SOCKS
HTTP
DNS
MQTT
Custom Bridges
```

---

# Example Use Cases

## Personal Mesh

```text
Phone
Laptop
NAS
Server
```

Connected securely across any transport.

---

## Friend Network

```text
Friends Domain
```

Shared services and applications.

---

## Community Mesh

```text
Neighborhood
|
WiFi
|
Local Relays
```

Independent local communication.

---

## Disaster Recovery Network

```text
LoRa
+
Portable Relays
+
Limited Internet
```

Resilient operation during infrastructure failures.

---

## Global Private Domain

```text
Phone
|
Global Mesh
|
Home Server
```

Uses public infrastructure.

Maintains private visibility.

---

# Project Identity

## Name

```text
Mycelis
```

## Meaning

Inspired by mycelial networks found in nature.

Mycelis represents:

- distributed growth
- resilience
- emergent structure
- decentralized coordination
- adaptive connectivity

---

# Vision Statement

Mycelis is a decentralized communication fabric built around self-sovereign identity, cryptographic domains, scoped discovery, emergent relays, and transport independence.

Mycelis enables people, devices, communities, and cities to communicate across Bluetooth, Wi-Fi, LoRa, Internet, Tor, I2P, and future transports without dependence on centralized infrastructure.

The network grows organically through participating nodes, forming a living communication system capable of operating locally, globally, online, offline, and everywhere in between.

A living network, a fabric for free communicatio.
