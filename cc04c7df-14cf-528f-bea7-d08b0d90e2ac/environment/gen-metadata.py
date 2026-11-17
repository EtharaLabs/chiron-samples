"""gen-metadata.py <local-repository>: write the local metadata Maven reads offline.

A repository rebuilt from release artifacts lacks the maven-metadata-central.xml files Maven
writes beside what it downloads. Two kinds matter offline: the version list of an artifact (for
version ranges and LATEST/RELEASE) and the plugin-prefix map of a plugin group (so that `mvn
dependency:tree` finds its plugin). Both are derived here from what the repository holds, so
they describe exactly the set and nothing else.
"""

from __future__ import annotations

import re
import sys
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(sys.argv[1])
NAME = "maven-metadata-central.xml"


def version_key(version: str) -> list[tuple[int, object]]:
    return [(0, int(p)) if p.isdigit() else (1, p) for p in re.split(r"[.\-]", version)]


plugins: dict[str, list[tuple[str, str, str]]] = {}
for pom in sorted(ROOT.rglob("*.pom")):
    version_dir = pom.parent
    artifact_dir = version_dir.parent
    if not pom.name.startswith(f"{artifact_dir.name}-{version_dir.name}"):
        continue
    group = artifact_dir.parent.relative_to(ROOT).as_posix().replace("/", ".")
    jar = version_dir / f"{artifact_dir.name}-{version_dir.name}.jar"
    if jar.is_file():
        try:
            with zipfile.ZipFile(jar) as archive:
                descriptor = archive.read("META-INF/maven/plugin.xml").decode("utf-8", "replace")
        except (KeyError, zipfile.BadZipFile):
            descriptor = ""
        prefix = re.search(r"<goalPrefix>([^<]+)</goalPrefix>", descriptor)
        name = re.search(r"<name>([^<]+)</name>", descriptor)
        if prefix:
            plugins.setdefault(group, []).append(
                (name.group(1) if name else artifact_dir.name, prefix.group(1), artifact_dir.name)
            )

for artifact_dir in sorted({p.parent.parent for p in ROOT.rglob("*.pom")}):
    versions = sorted((d.name for d in artifact_dir.iterdir() if d.is_dir()), key=version_key)
    if not versions:
        continue
    group = artifact_dir.parent.relative_to(ROOT).as_posix().replace("/", ".")
    releases = [v for v in versions if not v.endswith("-SNAPSHOT")]
    body = "\n".join(f"      <version>{escape(v)}</version>" for v in versions)
    (artifact_dir / NAME).write_text(
        '<?xml version="1.0" encoding="UTF-8"?>\n<metadata>\n'
        f"  <groupId>{escape(group)}</groupId>\n  <artifactId>{escape(artifact_dir.name)}</artifactId>\n"
        "  <versioning>\n"
        f"    <latest>{escape(versions[-1])}</latest>\n"
        + (f"    <release>{escape(releases[-1])}</release>\n" if releases else "")
        + f"    <versions>\n{body}\n    </versions>\n"
        "    <lastUpdated>20261101000000</lastUpdated>\n  </versioning>\n</metadata>\n",
        encoding="utf-8",
    )

for group, rows in sorted(plugins.items()):
    unique = sorted({(prefix, artifact): name for name, prefix, artifact in rows}.items())
    body = "\n".join(
        f"    <plugin>\n      <name>{escape(name)}</name>\n      <prefix>{escape(prefix)}</prefix>\n"
        f"      <artifactId>{escape(artifact)}</artifactId>\n    </plugin>"
        for (prefix, artifact), name in unique
    )
    (ROOT / group.replace(".", "/") / NAME).write_text(
        f'<?xml version="1.0" encoding="UTF-8"?>\n<metadata>\n  <plugins>\n{body}\n  </plugins>\n</metadata>\n',
        encoding="utf-8",
    )
print(f"gen-metadata: {ROOT}: plugin groups {len(plugins)}")
