"""Unpack the target pydantic distributions as readable sources (no compiled modules).

usage: mksrc.py <wheelhouse> <out> <wheel filename>...
"""
import sys
import zipfile
from pathlib import Path

wheelhouse, out = Path(sys.argv[1]), Path(sys.argv[2])
for name in sys.argv[3:]:
    dist = "-".join(name.split("-")[:2])
    with zipfile.ZipFile(wheelhouse / name) as wheel:
        for member in wheel.infolist():
            if member.is_dir() or member.filename.endswith((".so", ".pyd")):
                continue
            target = out / dist / member.filename
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(wheel.read(member))
