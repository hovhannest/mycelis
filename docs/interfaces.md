# Mycelia Reticulum interfaces

Mycelia’s live substrate (`transport = "rns"`) attaches FreeTAKTeam `reticulum-rs` **interfaces** from config. Control frames stay MYC1 announces; RNS routes them over whatever carriers you enable.

## Config

```toml
transport = "rns"

# When [[interfaces]] is omitted or empty, Mycelia synthesizes:
#   tcp_server from `listen`
#   tcp_client for each `static_peers` entry

[[interfaces]]
type = "tcp_server"
bind = "0.0.0.0:4242"

[[interfaces]]
type = "tcp_client"
target = "192.168.1.10:4242"

[[interfaces]]
type = "udp"
bind = "0.0.0.0:4243"
forward = "192.168.1.10:4243"

[[interfaces]]
type = "serial"
device = "COM3"          # or /dev/ttyUSB0
baud = 115200

[[interfaces]]
type = "kiss"
device = "/dev/ttyUSB0"
baud = 9600

[[interfaces]]
type = "kiss_tcp_client"
target = "127.0.0.1:8001"

[[interfaces]]
type = "lora"
device = "/dev/ttyACM0"  # or tcp://host:port
baud = 115200
region = "US915"

[[interfaces]]
type = "rnode_multi"
device = "/dev/ttyACM0"

[[interfaces]]
type = "pipe"
command = "my-bridge-tool"

[[interfaces]]
type = "i2p"
name = "i2p"
sam = "127.0.0.1:7656"
peers = []
connectable = false

[[interfaces]]
type = "weave"
device = "/dev/ttyUSB0"

[[interfaces]]
type = "meshtastic"
name = "mesh0"

# Unix only:
# [[interfaces]]
# type = "local"
# path = "/tmp/mycelis-rns.sock"

# Requires: cargo build -p mycelisd --features iface-ble
# [[interfaces]]
# type = "reticulum_ble"
# peripheral_id = "AA:BB:CC:DD:EE:FF"

# [[interfaces]]
# type = "vrn76_kiss_ble"
# peripheral_id = "VR-N76"
```

Set `enabled = false` on a row to skip it.

## Cargo features

| Feature | Effect |
|---|---|
| `transport-rns` (default) | Live RNS adapter |
| `iface-ble` | Enable `reticulum_ble` / `vrn76_kiss_ble` (btleplug) |

```bash
cargo check -p mycelia-node --features iface-ble
```

## Status

`mycelisd status` includes `interfaces` (kind names) and `listen`.

## Notes

- `auto` (reticulumd AutoInterface) is **not** implemented — use explicit `udp`.
- Hardware-backed kinds (serial, LoRa, BLE) need the device present; spawn may fail at runtime.
- See also [substrate-notes.md](substrate-notes.md) and FreeTAKTeam `interfaces-reference.toml`.
