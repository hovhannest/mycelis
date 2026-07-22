# Mycelia

> A living network without a center.

Mycelia is a **decentralized domain, community, and service fabric** built on the [Reticulum Network Stack](https://reticulum.network). Reticulum supplies identity, routing, path discovery, transport multiplexing, and store-and-forward. Mycelia adds the missing middle layer: **domains**, **communities**, **scoped service discovery**, **authorization**, and **policy** — without mandatory central infrastructure.

| Layer | Responsibility |
|---|---|
| **Applications** | Nomad Network, Sideband, MeshChat, custom apps |
| **Mycelia** | Domains · communities · service registry · authz · scoped discovery |
| **Reticulum** | Identity · routing · mesh · transports · store-and-forward |
| **Carriers** | LoRa · Wi‑Fi · TCP/UDP · BLE · serial · I2P · … |

## Why Mycelia

Mesh substrates (Reticulum, Yggdrasil, libp2p) and apps (Nomad Network, Sideband, Meshtastic) are well served. What is largely missing is an organization fabric that turns a flat cryptographic mesh into governable ownership boundaries, participation groups, and discoverable services — without a coordination SaaS.

That niche is Mycelia’s wedge. Closest neighbors: **Veilid** (keyed networks), **GNU Name System** (self-sovereign zones), **Nebula** (cert groups), and the **Reticulum app layer** (interop target, not competitor).

## Core principles

1. **No mandatory central infrastructure** — no required cloud, CA, IdP, or single relay operator.
2. **Zero-config Internet join** — connecting to the Internet should be enough to find the network; no single load-bearing bootstrap anchor.
3. **Local autonomy** — useful when offline or partitioned.
4. **Self-sovereign identity** — Reticulum identities; no registration authority.
5. **Privacy by default** — reachable ≠ visible ≠ authorized ≠ discoverable.
6. **Modular under hostility** — swap transports, obfuscation, discovery providers, and gateways without rewriting the stack.

## Quick links

| Document | Purpose |
|---|---|
| [docs/PRD.md](docs/PRD.md) | Product requirements and architecture |
| [docs/tech-stack.md](docs/tech-stack.md) | **Normative** language, profiles, crates, and dependency policy |
| [docs/landscape-survey.md](docs/landscape-survey.md) | Competitors, prior art, comparison matrix, design decisions |
| [docs/market-and-research.md](docs/market-and-research.md) | Market context and researched tech/protocol notes (2026) |

## Status

**Draft / pre-implementation.** Product design in the PRD; competitive context in the landscape survey; **implementation stack locked in [docs/tech-stack.md](docs/tech-stack.md)** (Rust, tiered `leaf`/`node`/`full` profiles, FreeTAKTeam Reticulum crates, optional libp2p on full nodes only). Proposed daemon: `mycelisd`.

```bash
mycelisd start
mycelisd status
mycelisd domains list
mycelisd communities list
```

## Naming

- **Product:** Mycelia  
- **Repository directory:** `mycelis` (historical spelling)  
- **Daemon:** `mycelisd`  

A Mycelia *domain* is a cryptographic ownership boundary — **not** a DNS domain.

## License

Documentation in this repository is project material under active drafting. Runtime licensing will be chosen with Reticulum ecosystem constraints in mind (see [market-and-research.md](docs/market-and-research.md#reticulum-license--ecosystem-risk)).
