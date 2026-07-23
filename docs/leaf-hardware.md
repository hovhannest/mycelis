# Leaf hardware notes (Phase 10)

Software prep for ESP32-C3 / ESP32-C6 leaf nodes. **Board flash is pending** — no hardware required to land this doc.

## Flash / RAM budget (fill when measured)

| Board | Flash used | RAM used | Date | Notes |
|---|---|---|---|---|
| ESP32-C3 | _pending_ | _pending_ | | |
| ESP32-C6 | _pending_ | _pending_ | | |

Record results in [implementation-plan.md](implementation-plan.md) Task 5.3 / Phase 10 table as well.

## Cross-compile sketch

Toolchain (example with `espup` / `esp-rs`):

```bash
# Install ESP Rust toolchain (host-specific; see https://esp-rs.github.io/)
espup install
. ~/export-esp.sh   # or Windows equivalent

# Check leaf crate (host) — must stay free of tokio/libp2p
cargo check -p mycelia-leaf
bash scripts/check-leaf-deps.sh

# Target check once Xtensa/RISC-V target is installed, e.g.:
# cargo check -p mycelia-leaf --target riscv32imc-unknown-none-elf
```

## `rns-embedded` integration (planned)

FreeTAKTeam publishes embedded-oriented crates (`rns-embedded-*`) alongside `reticulum-rs`. Leaf profile should:

1. Depend on `mycelia-core` with `--no-default-features --features crypto,alloc`.
2. Link the embedded RNS stack for LoRa / serial carriers (not host TCP).
3. Reuse Mycelia control codec (`ControlMessage`, attestations) unchanged; transport adapter implements `ReticulumTransport` with poll-style RX.

Do **not** pull `mycelia-node`, `tokio`, or `libp2p` into the leaf binary.

## Software-prep checklist

- [x] `mycelia-leaf` crate + dep budget script
- [x] Cross-compile / board notes documented
- [ ] Actual flash + RAM numbers on C3/C6
- [ ] CI job for `riscv32imc-unknown-none-elf` (optional later)
