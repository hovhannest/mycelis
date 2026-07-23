# Mycelia — Technology Stack Decision (ADR)

**Status:** Accepted  
**Date:** 2026-07-22  
**Supersedes:** open language question in [PRD.md](PRD.md)  
**Constraints (binding):**

1. As few dependencies as possible  
2. Cross-platform (Linux, macOS, Windows; later mobile via FFI)  
3. Deployable on microcontrollers / IoT (ESP32-class and similar), not only desktops  
4. Obey [PRD.md](PRD.md): Reticulum substrate, pluggable discovery, optional Internet locator  

This document is **normative for implementation**. Do not introduce languages, frameworks, or crates that violate the profiles and allowlists below without updating this ADR.

---

## 1. Decision summary

| Topic | Decision |
|---|---|
| Language | **Rust** (edition 2021+; MSRV **1.97.1**) |
| Product shape | **One workspace, three profiles** (`leaf` / `node` / `full`) — not one fat binary on every device |
| Networking substrate | **FreeTAKTeam Reticulum Rust stack** (`reticulum-rs*` + embedded crates), **EPL-2.0** |
| Control plane | **Mycelia-native compact messages over Reticulum first**; optional LXMF interop later |
| Internet locator | **`libp2p` Kad DHT**, **full profile only**, feature-gated |
| LAN discovery | **`mdns-sd`**, node/full only |
| Attestations / authz | Compact Ed25519-signed blobs; **no** full W3C DID/VC stack in MVP |
| Async runtime | **`tokio`** on `node`/`full` only; **no Tokio on `leaf`** |
| Forbidden as core | Python runtime, Go, Node/Electron, full IPFS, second crypto library zoo |

---

## 2. Why Rust (validated)

| Requirement | Validation |
|---|---|
| Cross-platform daemon | Single static-ish binary via Cargo; Windows/macOS/Linux first-class |
| MCU / IoT | `no_std` + `alloc`/`heapless` ecosystem; Embassy / esp-hal / embedded-hal for leaf boards |
| Crypto safety | Memory safety matters for mesh identity and authz |
| Minimal deps | Feature flags keep leaf crates tiny; avoid pulling DHT/gateway into firmware |
| Ecosystem | Published Reticulum/LXMF Rust crates exist (see §4) |

**Rejected for production:**

- **Python RNS** — excellent for reference interop tests; **not** MCU-capable; license packaging issues on upstream `rns`  
- **Go** — good daemons; weak bare-metal MCU story  
- **Zig** — strong cross-compile, thin Reticulum/crypto ecosystem today  
- **C/C++ only** — viable for ports (e.g. RTReticulum), but Mycelia fabric logic stays in Rust; C++ only as optional leaf FFI later if needed  

---

## 3. Profiles (binding)

```
mycelia-core     (no_std-capable)     domains, attestations, registry records, PEX types
mycelia-node     (std)                + RNS transport + peer cache + local discovery
mycelisd         (std binary)         CLI/daemon over mycelia-node
mycelia-dht      (optional)           libp2p locator — linked only into `full`
mycelia-gateway  (optional)           SOCKS/HTTP — linked only into `full`
mycelia-leaf     (no_std firmware)    subset of core + thin RNS embedded runtime
```

| Profile | Targets | Discovery | DHT | Gateway | Typical deps |
|---|---|---|---|---|---|
| **leaf** | ESP32-C3/C6, nRF52, STM32WL-class | Peer cache + RNS announces + LoRa/BLE/serial | ✗ | ✗ | `mycelia-core` + embedded RNS + `heapless` |
| **node** | Pi, desktop, phone companion | + mDNS + static peers | ✗ | ✗ | + `reticulum-rs*` + `mdns-sd` + `tokio` |
| **full** | Always-on Internet hosts | + libp2p DHT | ✓ | optional | + `libp2p` (kad/tcp/noise/yamux/dns) |

A leaf that needs Internet uses a **node/full neighbor as a mesh hop or gateway** — it does not embed libp2p.

---

## 4. Substrate choice (validated)

### 4.1 Primary: FreeTAKTeam LXMF-rs / reticulum-rs family

| Crate (crates.io) | Role | License | Verified |
|---|---|---|---|
| [`reticulum-rs`](https://crates.io/crates/reticulum-rs) `0.9.6` | Umbrella RNS stack | **EPL-2.0** | Published 2026-07-21 |
| [`reticulum-rs-core`](https://crates.io/crates/reticulum-rs-core) | Crypto/packet primitives | EPL-2.0 | Same family |
| [`reticulum-rs-transport`](https://crates.io/crates/reticulum-rs-transport) | Transport / interfaces | EPL-2.0 | Same family |
| [`rns-embedded-core`](https://crates.io/crates/rns-embedded-core) | Embedded-friendly core | EPL-2.0 | Published |
| [`rns-embedded-runtime`](https://crates.io/crates/rns-embedded-runtime) | Embedded runtime | EPL-2.0 | Published |
| [`rns-embedded-mininode`](https://crates.io/crates/rns-embedded-mininode) | Minimal embedded node helpers | EPL-2.0 | Published |
| [`rns-embedded-ffi`](https://crates.io/crates/rns-embedded-ffi) | C ABI for leaf hosts | EPL-2.0 | Published |
| [`lxmf`](https://crates.io/crates/lxmf) / [`lxmf-wire`](https://crates.io/crates/lxmf-wire) | LXMF (optional interop) | EPL-2.0 | Published |
| [`lxmf-embedded-mini`](https://crates.io/crates/lxmf-embedded-mini) `0.9.6` | No-alloc mini LXMF (`heapless`) | EPL-2.0 | Published; deps: `heapless` only |

**Why primary:**

- Available on crates.io and actively released (0.9.x line through Jul 2026)  
- Explicit **embedded** crate surface (critical for MCU constraint)  
- **EPL-2.0** is packagable in distros (unlike Reticulum License anti-AI / no-harm clauses)  
- Optional LXMF path for Nomad/Sideband interop without forcing LXMF into leaf MVP  

**Caveats (accepted):**

- Upstream states it is **not** a complete drop-in for every Python RNS/LXMF behavior  
- **Interop gate required:** continuous tests against Python `rns` / Nomad/Sideband where claims matter  
- Mycelia application code stays **MIT**; EPL applies to those library components (respect EPL when modifying them)

### 4.2 Evaluated alternatives (not primary)

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| **lelloman `rns-crypto` / `rns-core`** | Strong `no_std` claim; dalek crypto; 900+ interop tests; tiny dep tree | **Reticulum License** (anti-AI + no-harm) — packaging/distribution risk | **Fallback only** if FreeTAKTeam fails MCU/interop spike |
| **Beechat `reticulum` 0.1.0** | MIT | Tokio + tonic/prost heavy; not MCU-first; immature | Reject as substrate |
| **Python `rns` / RetiNet** | Reference truth | Not MCU; license/runtime | Interop/test harness only |
| **RTReticulum (C++)** | Native MCU LoRa | Different language; WIP | Optional later leaf via FFI, not Mycelia core |

### 4.3 Fallback procedure

If the FreeTAKTeam stack fails the spike criteria in §8:

1. Document failure evidence in this ADR (append changelog).  
2. Switch leaf/`mycelia-core` transport bindings to **lelloman rns-rs**, accepting Reticulum License on that dependency only.  
3. Keep Mycelia protocol/attestation code MIT and substrate-agnostic behind a trait (`ReticulumTransport`).  

---

## 5. Dependency allowlist (validated available)

### 5.1 Always allowed (any profile that needs them)

| Crate | License | Purpose | MCU? |
|---|---|---|---|
| Substrate crates in §4.1 (as needed per profile) | EPL-2.0 | Reticulum | leaf via embedded crates |
| [`heapless`](https://crates.io/crates/heapless) | MIT OR Apache-2.0 | Fixed buffers on leaf | ✓ |
| [`log`](https://crates.io/crates/log) | MIT OR Apache-2.0 | Logging façade | ✓ |
| `serde` + `serde_bytes` (optional) | MIT OR Apache-2.0 | Config/host only; **prefer raw bytes on leaf** | host |

Do **not** add a second Ed25519/X25519 stack. Use keys/crypto exposed by the chosen Reticulum substrate. If a host-only helper is unavoidable, pin dalek crates already used by the ecosystem (`ed25519-dalek` is `no_std`-capable, BSD-3-Clause, crates.io).

### 5.2 `node` / `full` only

| Crate | License | Purpose | Validated |
|---|---|---|---|
| [`tokio`](https://crates.io/crates/tokio) `1.x` (feature-minimal: `rt`, `net`, `time`, `sync`, `macros`) | MIT | Async I/O | ✓ 1.53.x on crates.io |
| [`mdns-sd`](https://crates.io/crates/mdns-sd) `0.20.x` | Apache-2.0 OR MIT | LAN DNS-SD; **no async runtime required** | ✓ |

### 5.3 `full` only (feature `discovery-dht`)

| Crate | License | Purpose | Validated |
|---|---|---|---|
| [`libp2p`](https://crates.io/crates/libp2p) `0.56.x` with features: `kad`, `tcp`, `noise`, `yamux`, `dns`, `tokio`, `identify`, `macros` | MIT | Disposable Internet locator → RNS identity/reachability | ✓ MSRV 1.83; Kad provider/bootstrap APIs present |

**Explicitly do not enable** on Mycelia by default: gossipsub, floodsub, relay-as-app-transport, wasm, full feature umbrella. DHT is a phone book, not the data plane.

### 5.4 Optional later (not MVP)

| Feature flag | Direction | Notes |
|---|---|---|
| `control-lxmf` | `lxmf-wire` / `lxmf-embedded-mini` | Ecosystem interop; not required for Mycelia MVP control plane |
| `gateway-socks` | Tiny custom SOCKS5 or minimal deps | Internet sharing; policy-gated |
| `discovery-ble` | platform BLE APIs / `btleplug` on host | Leaf may use vendor HAL instead |

### 5.5 Denied as core dependencies

- Python, PyO3 as runtime dependency of `mycelisd`  
- Full IPFS / Kubo  
- Electron / Node for the daemon  
- W3C DID method + full VC libraries in MVP  
- Embedding `libp2p` into leaf firmware  
- Beechat `reticulum` as the default substrate  

---

## 6. Control plane & authz (minimal-deps design)

**MVP:** Mycelia control messages (invites, membership attestations, service ads, policy deltas) as **compact binary records** carried over Reticulum links/destinations — implemented in `mycelia-core`.

**Attestation format (MVP):**

- Ed25519 signature over canonical bytes  
- Issuer identity, subject identity, domain/community id, capability bits, expiry, optional attenuation chain  
- Sized for LoRa (budget aggressively; measure in spike)

**Not MVP:** full VC-JSON-LD, Biscuit, MLS. Revisit after wire format stabilizes.

**LXMF:** optional adapter for “speak to Nomad/Sideband users,” not the internal schema.

---

## 7. Workspace layout (to implement)

```
mycelis/
  Cargo.toml                 # workspace
  crates/
    mycelia-core/            # no_std + alloc feature
    mycelia-node/            # std node runtime
    mycelia-dht/             # optional libp2p locator
    mycelia-gateway/         # optional
    mycelisd/                # CLI binary
    mycelia-leaf/            # firmware example (ESP32 target)
  docs/
    tech-stack.md            # this ADR
```

Cargo features on `mycelisd`:

```toml
[features]
default = ["node"]
node = []
full = ["node", "discovery-dht"]
discovery-dht = ["dep:mycelia-dht"]
gateway = ["dep:mycelia-gateway"]
control-lxmf = []  # off by default
```

---

## 8. Validation gates before locking further

Must pass before claiming production readiness of the substrate choice:

| # | Gate | Pass criteria |
|---|---|---|
| G1 | **Compile leaf** | `mycelia-core` + embedded RNS crates build for at least one MCU target (e.g. `riscv32imc-unknown-none-elf` or `thumbv7em-none-eabihf`) |
| G2 | **Host interop** | Rust node exchanges announce/link or equivalent with Python `rns` reference |
| G3 | **Dep budget** | `cargo tree -p mycelia-leaf` (or equivalent) reviewed; no libp2p/tokio in leaf |
| G4 | **Size budget** | Document flash/RAM for leaf demo; reject if DHT accidentally linked |
| G5 | **DHT optional** | `full` publishes/resolves a Mycelia-namespaced record to an RNS destination hint; `node` builds without libp2p |
| G6 | **License scan** | `cargo deny` (or equivalent) policy: allow MIT/Apache/BSD/EPL; **deny** unexpected copyleft; flag Reticulum License if fallback used |

---

## 9. License posture

| Component | License |
|---|---|
| Mycelia code (this repo) | **MIT** (see root `LICENSE`) |
| FreeTAKTeam substrate | **EPL-2.0** (dependency) |
| libp2p, tokio, mdns-sd | MIT / Apache-2.0 |
| Fallback lelloman rns-rs | **Reticulum License** — only if §4.3 triggered; document redistribution limits |

Do not re-license Mycelia as Reticulum License. Keep substrate behind the transport trait so a swap remains possible.

---

## 10. What implementers must do

1. Read this ADR before adding any dependency.  
2. Default to **smallest profile** that meets the use case.  
3. Prefer implementing Mycelia logic in `mycelia-core` over pulling another protocol stack.  
4. Run gates G1–G6 in CI as they become automatable.  
5. If tempted by Python for speed: use it only in `/tools` or tests, never as `mycelisd` runtime.

---

## 11. Changelog

| Date | Change |
|---|---|
| 2026-07-23 | MSRV / toolchain pin raised to **1.97.1** (current stable) |
| 2026-07-22 | Accepted: Rust + tiered profiles; FreeTAKTeam reticulum-rs primary; libp2p full-only; validated crates.io availability and licenses |
