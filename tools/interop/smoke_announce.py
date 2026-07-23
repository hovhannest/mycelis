#!/usr/bin/env python3
"""Minimal interop placeholder — prints guidance until rns adapter is wired."""

def main() -> None:
    print("Mycelia interop smoke: install rns (see requirements.txt) and point at a running mycelisd.")
    print("Control plane e2e is covered by: cargo test -p mycelia-node --test e2e_domain_service")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
