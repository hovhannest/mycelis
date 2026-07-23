# Python RNS interop harness (Phase 9)

Optional smoke tests against a running `mycelisd` control plane (and optionally Python `rns`).

## One-command smoke

```bash
# Terminal 1
cargo run -p mycelisd -- --data-dir .mycelis-interop start --transport mock

# Terminal 2
cd tools/interop
python smoke_announce.py
# or:
MYCELIS_DATA_DIR=.mycelis-interop python smoke_announce.py
```

Skip intentionally (exit 0):

```bash
MYCELIS_INTEROP_SKIP=1 python smoke_announce.py
```

## Setup (optional Python RNS)

```bash
cd tools/interop
python -m venv .venv
# Windows: .venv\Scripts\activate
source .venv/bin/activate
pip install -r requirements.txt
```

## CI

Workflow [`.github/workflows/interop.yml`](../../.github/workflows/interop.yml) is **manual** / `interop` label only — not part of default PR CI.
