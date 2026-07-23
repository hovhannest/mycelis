# Substrate notes — FreeTAKTeam `reticulum-rs` 0.9.6

## Status

Phase 6 wires a live adapter (`mycelia-node::rns_transport`) behind feature `transport-rns` (default). Mock transport remains for tests (`hub.is_some()`, `transport = "mock"`, or `MYCELIS_TRANSPORT=mock`).

## APIs used

| Concern | API |
|---|---|
| Identity | `PrivateIdentity::new_from_rand` / `new_from_hex_string` / `to_hex_string` |
| Transport | `TransportConfig::new("mycelis", &id, true)`, `Transport::new` |
| Interfaces | Config-driven via `rns_ifaces::spawn_all` — TCP/UDP/serial/KISS/LoRa/pipe/I2P/weave/meshtastic; BLE behind `iface-ble` |
| Destination | `DestinationName::new("mycelis", "v1")`, `add_destination` |
| Send control | `send_announce(&dest, Some(envelope))` (broadcast path; no links required) |
| Receive | `recv_announces()`, `received_data_events()` |

**Import path:** use types from `reticulum_rs::transport::{identity, destination, transport}` and `reticulum_rs::iface::*`. Do not mix `reticulum_rs::{identity,destination}` (core) with transport types.

See [interfaces.md](interfaces.md) for TOML `[[interfaces]]` kinds.

## NodeId vs RNS AddressHash

- Mycelia `NodeId` = first 16 bytes of SHA-512(Ed25519 verifying key) from the Mycelia signing identity (`identity.key`).
- RNS `AddressHash` is derived from the RNS `PrivateIdentity` (X25519 + Ed25519) and destination name hash — **different**.
- Mapping is application-layer: MYC1 envelope carries Mycelia `from`/`to` NodeIds inside announce `app_data`. RNS routing identity is persisted separately as `rns.identity`.

## MYC1 envelope

```
"MYC1" | from_node_id[16] | to_node_id[16] or 16×0x00 | payload
```

Directed filter: accept if `to` is all zeros (broadcast) or equals local NodeId.

## Windows / sqlite

`reticulum-rs-transport` depends on `rusqlite` with the **`bundled`** feature (ships sqlite amalgamation). No system sqlite install is required on Windows. If a future upstream drops bundled, override:

```toml
[patch.crates-io]
# rusqlite = { version = "0.37", features = ["bundled"] }
```

## Smoke

```bash
cargo test -p mycelia-node --features transport-rns --test rns_substrate_spike
cargo run -p mycelia-node --example rns_tcp_ping --features transport-rns
```
