# Run by the fetch stage of tests/Dockerfile: writes the retired packages' Python sources
# under /opt/grader/retired, which the grader's transplant check reads.
import tarfile, zipfile
from pathlib import Path

out = Path("/opt/grader/retired")
packages = {"blinker": "blinker", "flask": "flask", "flask_login": "flask_login",
            "flask_wtf": "flask_wtf", "wtforms": "wtforms",
            "prometheus_flask_exporter": "prometheus_flask_exporter",
            "a2wsgi": "a2wsgi", "asgiref": "asgiref"}
for wheel in sorted(Path("/wheels/retired").glob("*.whl")):
    with zipfile.ZipFile(wheel) as archive:
        for name in archive.namelist():
            top = name.split("/", 1)[0]
            if top in packages and name.endswith(".py"):
                target = out / packages[top] / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.read(name))
for sdist in sorted(Path("/wheels/retired").glob("uwsgi-*.tar.gz")):
    with tarfile.open(sdist) as archive:
        for member in archive.getmembers():
            if member.isfile() and member.name.endswith(".py"):
                target = out / "uwsgi" / member.name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.extractfile(member).read())
