#!/usr/bin/env python3
"""Mycelia interop smoke (Phase 9).

Prefer a control-plane ping against a running mycelisd. Optionally attempt a
Python `rns` announce if the package is installed.

Exit codes:
  0 — success, or intentional skip (MYCELIS_INTEROP_SKIP=1 / missing deps)
  1 — hard failure when interop was expected to run
"""

from __future__ import annotations

import json
import os
import socket
import sys
from pathlib import Path


def skip(msg: str) -> int:
    print(f"SKIP: {msg}")
    return 0


def control_ping(addr: str) -> bool:
    host, _, port_s = addr.rpartition(":")
    host = host.strip("[]") or "127.0.0.1"
    port = int(port_s)
    req = json.dumps({"cmd": "status"}).encode()
    with socket.create_connection((host, port), timeout=5) as s:
        s.sendall(req)
        s.shutdown(socket.SHUT_WR)
        data = b""
        while True:
            chunk = s.recv(65536)
            if not chunk:
                break
            data += chunk
    resp = json.loads(data.decode())
    print("control status:", json.dumps(resp, indent=2))
    return bool(resp.get("ok"))


def resolve_control() -> str | None:
    env = os.environ.get("MYCELIS_CONTROL")
    if env:
        return env
    data_dir = Path(os.environ.get("MYCELIS_DATA_DIR", ".mycelis"))
    path = data_dir / "control.addr"
    if path.exists():
        return path.read_text(encoding="utf-8").strip()
    return None


def try_python_rns_announce(listen_hint: str | None) -> None:
    try:
        import RNS  # type: ignore
    except ImportError:
        print("Python rns not installed; skipping RNS announce probe")
        return
    print(f"rns module present ({RNS}); listen hint={listen_hint!r}")
    print("Full Python↔Rust announce parity is optional; control ping is authoritative.")


def main() -> int:
    if os.environ.get("MYCELIS_INTEROP_SKIP") == "1":
        return skip("MYCELIS_INTEROP_SKIP=1")

    control = resolve_control()
    if not control:
        print(
            "No control address. Start a node first:\n"
            "  cargo run -p mycelisd -- --data-dir .mycelis-interop start --transport mock\n"
            "Then re-run, or set MYCELIS_CONTROL=127.0.0.1:PORT / MYCELIS_INTEROP_SKIP=1"
        )
        # Soft skip so manual CI label jobs without a daemon still exit 0.
        return skip("mycelisd control.addr not found")

    if not control_ping(control):
        print("control ping failed", file=sys.stderr)
        return 1

    try_python_rns_announce(os.environ.get("MYCELIS_RNS_LISTEN"))
    print("interop smoke ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
