# Mycelia Wire Format

**Version:** 1  
**Endianness:** little-endian  
**Scope:** Mycelia-native control plane (not LXMF)

## Conventions

- Multi-byte integers are little-endian.
- Blobs are `u16` length + bytes.
- Frames: `version:u8 | type:u8 | len:u16 | payload`.
- Max frame payload: **1024** bytes (`MAX_FRAME_PAYLOAD`).
- Attestation max encoded size: **256** bytes (LoRa budget).

## Identifiers

| Type | Size |
|---|---|
| `NodeId` / `DomainId` / `CommunityId` / `ServiceId` | 16 bytes |

## Attestation (v1)

```
version:u8 = 1
issuer:NodeId
subject:NodeId
scope:u8 (1=domain, 2=community)
scope_id:[u8;16]
caps:u32
not_before:u64
not_after:u64
parent_flag:u8 (0|1)
parent_sig:[u8;64]?  # if parent_flag=1
sig:[u8;64]          # Ed25519 over all fields above except sig
```

## Service record (v1)

```
version:u8 = 1
service_id:ServiceId
name:blob (<=32)
owner:NodeId
visibility:u8 (1 public … 5 hidden)
audience:[u8;16]
endpoint:NodeId
meta:blob (<=64)
expires_at:u64
sig:[u8;64]
```

## Control message types

| Type | Id | Payload |
|---|---|---|
| Invite | 1 | from, domain, attestation blob |
| AttestationAnnounce | 2 | attestation blob |
| ServiceAnnounce | 3 | service record blob |
| ServiceQuery | 4 | from, filter flag + optional domain |
| ServiceQueryResponse | 5 | count + record blobs |
| PeerExchange | 6 | count + PeerInfo entries |
| PolicyUpdate | 7 | epoch + payload blob (stub) |

### PeerInfo

```
node_id:NodeId
last_seen:u64
interface_hint:blob (<=64)
```

## Phase 6+ — RNS announce envelope (MYC1)

Mycelia control frames over live FreeTAKTeam `reticulum-rs` ride in announce `app_data`:

```
magic:"MYC1"
from:NodeId          # 16 bytes
to:NodeId | zeros    # 16 bytes; zeros = broadcast
payload:bytes        # typically a Mycelia control frame
```

Receivers accept envelopes where `to` is all zeros **or** equals the local NodeId. Mycelia `NodeId` is not the RNS `AddressHash`; see [substrate-notes.md](substrate-notes.md).

### ServiceAnnounce PoW wrap (MPW1)

When `pow_difficulty > 0`, advertised ServiceAnnounce frames are wrapped:

```
magic:"MPW1"
PowStamp { version:u8, difficulty:u8, nonce:u64 }
frame: ControlMessage bytes
```

`PowStamp` proves leading-zero bits of SHA-256(frame \|\| nonce_le). Undersized stamps are rejected in `handle_control_bytes`.
