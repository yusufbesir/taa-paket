#!/usr/bin/env python3
"""Kurucu dosyasının SHA256'sını harici.toml satırı olarak yazar: python tools/harici_sha.py kurucu.exe"""
import hashlib, os, sys

if len(sys.argv) != 2:
    sys.exit("Kullanım: harici_sha.py <kurucu.exe>")
h = hashlib.sha256()
with open(sys.argv[1], "rb") as f:
    for blok in iter(lambda: f.read(1 << 20), b""):
        h.update(blok)
print(f'sha256 = "{h.hexdigest()}"   # {os.path.basename(sys.argv[1])}')
