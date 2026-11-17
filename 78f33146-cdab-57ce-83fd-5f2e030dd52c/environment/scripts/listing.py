"""Write the wheelhouse listing: distribution, version, file and sha256 of every carried file."""
import sys
from pathlib import Path

manifest, out = sys.argv[1], Path(sys.argv[2])
rows = ["# distribution version file sha256\n"]
for line in Path(manifest).read_text().splitlines():
    if not line.strip():
        continue
    name, _url, sha, *_ = line.split()
    if name.endswith(".tar.gz"):
        dist, version = name[:-7].rsplit("-", 1)
    else:
        dist, version = name.split("-")[:2]
    rows.append(f"{dist} {version} {name} {sha}\n")
out.write_text("".join(rows))
