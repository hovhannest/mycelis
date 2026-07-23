# Mycelia

> A living network without a center.

Mycelia is a **decentralized domain, community, and service fabric** built to run above the [Reticulum Network Stack](https://reticulum.network). This repository contains the Rust implementation (`mycelisd`) and design docs.

| Layer | Responsibility |
|---|---|
| **Applications** | Nomad Network, Sideband, MeshChat, custom apps |
| **Mycelia** | Domains · communities · service registry · authz · scoped discovery |
| **Reticulum** | Identity · routing · mesh · transports · store-and-forward |
| **Carriers** | LoRa · Wi‑Fi · TCP/UDP · BLE · serial · I2P · … |

## Build & test

Requirements: Rust **1.97.1+** (see `rust-toolchain.toml`).

```bash
cargo test --workspace
cargo check -p mycelisd --features full
bash scripts/check-leaf-deps.sh
```

### Run a node

```bash
cargo run -p mycelisd -- --data-dir .mycelis-a start
# another terminal:
cargo run -p mycelisd -- --data-dir .mycelis-a status
cargo run -p mycelisd -- --data-dir .mycelis-a domains create home
```

Control plane: localhost JSON over TCP; address written to `<data-dir>/control.addr`.

### Features

| Feature | Effect |
|---|---|
| default / `node` | Standard node + CLI |
| `full` / `discovery-dht` | Enable `mycelia-dht` (libp2p Kad locator) |
| `gateway` | Enable SOCKS5 gateway crate |

## Docs

| Document | Purpose |
|---|---|
| [docs/PRD.md](docs/PRD.md) | Product requirements |
| [docs/tech-stack.md](docs/tech-stack.md) | Normative language, profiles, crates |
| [docs/implementation-plan.md](docs/implementation-plan.md) | Build/test plan + **progress checklist** |
| [docs/wire-format.md](docs/wire-format.md) | Control-plane binary format |
| [docs/landscape-survey.md](docs/landscape-survey.md) | Competitors / prior art |
| [docs/market-and-research.md](docs/market-and-research.md) | Market research notes |

## Status

**v0.1.0 implementation in progress.** Core protocol, node runtime, CLI, e2e domain/service tests, optional DHT/gateway crates, and CI are in-tree. Live FreeTAKTeam `reticulum-rs` TCP adapter is deferred behind `ReticulumTransport` (mock hub used in CI). Leaf hardware smoke pending board access.

## License

MIT — see [LICENSE](LICENSE). Substrate crates may use EPL-2.0 when linked (see tech-stack).
