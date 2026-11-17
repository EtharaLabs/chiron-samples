"""Write a PEP 503 simple index over a wheelhouse from its manifest.

usage: mkindex.py <manifest> <index dir> <href prefix>
Each link carries the file's sha256 fragment and its upload time, so uv's
exclude-newer setting resolves against the index offline.
"""
import html
import re
import sys
from pathlib import Path

manifest, out, prefix = sys.argv[1], Path(sys.argv[2]), sys.argv[3]
groups: dict[str, list[tuple[str, str, str]]] = {}
for line in Path(manifest).read_text().splitlines():
    if not line.strip():
        continue
    name, _url, sha, uploaded = line.split()
    dist = name[:-7].rsplit("-", 1)[0] if name.endswith(".tar.gz") else name.split("-")[0]
    groups.setdefault(re.sub(r"[-_.]+", "-", dist).lower(), []).append((name, sha, uploaded))
out.mkdir(parents=True, exist_ok=True)
rows = []
for project, files in sorted(groups.items()):
    rows.append(f'<a href="{project}/">{project}</a>\n')
    (out / project).mkdir(exist_ok=True)
    links = "".join(
        f'<a href="{prefix}/{html.escape(n)}#sha256={s}" data-upload-time="{u}">{html.escape(n)}</a>\n'
        for n, s, u in sorted(files)
    )
    (out / project / "index.html").write_text(f"<!DOCTYPE html><html><body>\n{links}</body></html>\n")
(out / "index.html").write_text("<!DOCTYPE html><html><body>\n" + "".join(rows) + "</body></html>\n")
