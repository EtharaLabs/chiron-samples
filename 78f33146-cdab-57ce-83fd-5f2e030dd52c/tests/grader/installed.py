"""Run inside the graded virtualenv: report installed metadata or the project's installed files.

usage: python -I installed.py metadata|files
metadata: exact versions of the target distributions and, per installed
distribution, the top-level names its RECORD provides.
files: every installed file of the submitted project's distribution(s), the
distributions whose RECORD includes a pydatalab module.
"""
import importlib.metadata as md
import json
import sys
from pathlib import Path


def tops(dist):
    names = set()
    for f in dist.files or []:
        parts = Path(str(f)).parts
        if not parts or parts[0].endswith((".dist-info", ".data")) or parts[0] in ("..", "bin"):
            continue
        name = parts[0]
        if name.endswith(".py"):
            name = name[:-3]
        elif name.endswith(".pth"):
            name = name
        names.add(name.split(".")[0] if not name.endswith(".pth") else name)
    return names


mode = sys.argv[1]
dists = list(md.distributions())
if mode == "metadata":
    versions, providers = {}, {}
    for d in dists:
        name = (d.metadata["Name"] or "").lower().replace("_", "-")
        versions[name] = d.version
        providers[d.metadata["Name"]] = sorted(tops(d))
    print(json.dumps({"versions": versions, "providers": providers}))
else:
    for d in dists:
        files = [str(Path(d.locate_file(f)).resolve()) for f in d.files or []]
        if any("/pydatalab/" in f for f in files):
            print("\n".join(files))
