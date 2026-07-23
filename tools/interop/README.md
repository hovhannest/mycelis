# Python RNS interop harness (Task 5.2)

Optional smoke tests against the Python Reticulum reference (`rns`).

## Setup

```bash
cd tools/interop
python -m venv .venv
# Windows: .venv\Scripts\activate
source .venv/bin/activate
pip install -r requirements.txt
```

## Smoke

```bash
# Terminal 1: Rust node
cargo run -p mycelisd -- --data-dir /tmp/mycelis-a start

# Terminal 2: Python announce probe (requires rns configured)
python smoke_announce.py
```

CI: run manually or with workflow label `interop` (not required for default PR CI).

## Status

Harasses Mycelia control-plane readiness. Full announce/link parity with Python `rns` depends on the deferred `reticulum-rs` adapter (see implementation-plan Task 2.4).
