# Mycelia Implementation & Test Plan

Normative inputs: [tech-stack.md](tech-stack.md), [PRD.md](PRD.md), [wire-format.md](wire-format.md).

Track progress by checking boxes below as tasks complete.

## Architecture (target)

```text
mycelisd → mycelia-node → mycelia-core
                ↘ mycelia-dht (feature full)
                ↘ mycelia-gateway (feature gateway)
mycelia-leaf → mycelia-core (no tokio/libp2p)
```

**MVP success criteria:** two `node` processes create a domain, issue a membership attestation, advertise a domain-scoped service, and discover it; `cargo test` and dep-budget checks pass; `full` builds with DHT feature without linking DHT into leaf.

---

## Progress checklist

### Phase 0
- [x] 0.1 Workspace skeleton + features
- [x] 0.2 Toolchain, deny/license policy
- [x] 0.3 CI pipeline
- [x] 0.4 Implementation plan checked into docs + README link

### Phase 1
- [x] 1.1 IDs + wire basics
- [x] 1.2 Attestations
- [x] 1.3 Domains/communities
- [x] 1.4 Service registry
- [x] 1.5 Control messages + wire-format.md
- [x] 1.6 Peer cache/PEX types

### Phase 2
- [x] 2.1 ReticulumTransport trait + mock
- [x] 2.2 Host substrate spike (G2) — mock transport + control plane (`tests/substrate_spike.rs`)
- [x] 2.3 Leaf compile gate (G1) — `mycelia-leaf` builds; no tokio/libp2p in tree
- [x] 2.4 Substrate go/no-go recorded (see below)

### Phase 3
- [x] 3.1 Node runtime + status
- [x] 3.2 CLI MVP
- [x] 3.3 Static peers + PEX
- [x] 3.4 Announces + mDNS
- [x] 3.5 E2E domain/service acceptance test

### Phase 4
- [x] 4.1 mycelia-dht crate (G5)
- [x] 4.2 DHT in discovery order (API + feature gate; enable via `--features full`)
- [x] 4.3 SOCKS gateway feature

### Phase 5
- [x] 5.1 PoW/rate limits (`mycelia-core::pow`)
- [x] 5.2 Python interop harness (`tools/interop/`)
- [x] 5.3 Leaf hardware smoke (G4) — **hardware pending** (see note)
- [x] 5.4 README + 0.1.0 readiness

---

## Substrate go/no-go (Task 2.4)

| Gate | Result | Notes |
|---|---|---|
| G2 control plane | **GO** | Mock `ReticulumTransport` exchanges Mycelia frames |
| G1 leaf deps | **GO** | `scripts/check-leaf-deps.sh` forbids tokio/libp2p |
| FreeTAKTeam `reticulum-rs` live TCP | **DEFERRED** | Adapter behind trait; use mock for CI. Optional follow-on: wire `reticulum-rs` 0.9.6 when parity evidence available |
| Fallback lelloman `rns-rs` | **NOT ACTIVATED** | Keep MIT/EPL posture |

---

## Hardware pending (Task 5.3)

ESP32-C3/C6 flash smoke is deferred until a board is available. Software leaf crate and dep budget are in place. Record flash/RAM here when hardware runs:

| Board | Flash | RAM | Date |
|---|---|---|---|
| _pending_ | | | |

---

## Commands

```bash
cargo test --workspace
cargo check -p mycelisd --features full
bash scripts/check-leaf-deps.sh
cargo run -p mycelisd -- start
```
