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
cargo run -p mycelisd -- --data-dir .mycelis-a communities create mesh
cargo run -p mycelisd -- --data-dir .mycelis-a gateway status
```

Default transport is FreeTAKTeam `reticulum-rs` (`transport = "rns"`). For mock/tests:

```bash
cargo run -p mycelisd -- start --transport mock
# or: MYCELIS_TRANSPORT=mock ...
```

Control plane: localhost JSON over TCP; address written to `<data-dir>/control.addr`.

### Config keys (TOML)

| Key | Default | Notes |
|---|---|---|
| `transport` | `"rns"` | `"mock"` forces in-process hub |
| `[[interfaces]]` | _(empty → TCP from listen/peers)_ | See [docs/interfaces.md](docs/interfaces.md) |
| `gateway_bind` | `127.0.0.1:1080` | SOCKS5 listen when `enable_gateway` |
| `enable_dht` | `false` | requires `--features discovery-dht` / `full` |
| `enable_gateway` | `false` | requires `--features gateway` / `full` + GATEWAY attestation |
| `pow_difficulty` | `8` | ServiceAnnounce PoW (0 disables) |

### Features

| Feature | Effect |
|---|---|
| default / `node` | Standard node + CLI; `mycelia-node` defaults include `transport-rns` |
| `full` | `discovery-dht` + `gateway` |
| `discovery-dht` | Enable `mycelia-dht` (libp2p Kad locator) |
| `gateway` | Enable SOCKS5 gateway crate |
| `transport-rns` | Live RNS adapter (default on `mycelia-node`) |
| `iface-ble` | BLE RNode / VR-N76 interfaces (`mycelia-node`) |

## Docs

| Document | Purpose |
|---|---|
| [docs/PRD.md](docs/PRD.md) | Product requirements |
| [docs/tech-stack.md](docs/tech-stack.md) | Normative language, profiles, crates |
| [docs/implementation-plan.md](docs/implementation-plan.md) | Build/test plan + **progress checklist** |
| [docs/wire-format.md](docs/wire-format.md) | Control-plane binary format + MYC1 |
| [docs/interfaces.md](docs/interfaces.md) | Pluggable Reticulum `[[interfaces]]` |
| [docs/substrate-notes.md](docs/substrate-notes.md) | `reticulum-rs` embedding notes |
| [docs/leaf-hardware.md](docs/leaf-hardware.md) | ESP32 leaf cross-compile (hardware pending) |
| [docs/landscape-survey.md](docs/landscape-survey.md) | Competitors / prior art |
| [docs/market-and-research.md](docs/market-and-research.md) | Market research notes |

## Status

**v0.1.0.** Core protocol, pluggable RNS interfaces (TCP/UDP/…), node runtime, CLI, DHT/gateway features, persistence, interop smoke, and CI are in-tree. Leaf **hardware** flash pending board access.

## License

MIT — see [LICENSE](LICENSE). Substrate crates may use EPL-2.0 when linked (see tech-stack).
