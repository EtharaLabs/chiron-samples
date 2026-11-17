"""The graded pipeline: rebuild the submission offline, run the Stage I checks, replay Stage II and III.

usage: python grade.py <submitted tree> <results.json> [--work DIR]

Writes one results file: every compiled item with pass or fail and a reason,
the Stage II groups and the Stage III families. It writes no reward bytes;
Reward Kit scores tests/test_output.py over this file.
"""

from __future__ import annotations

import argparse
import ast
import io
import json
import keyword
import os
import re
import shutil
import subprocess
import tarfile
import time
import tokenize
import tomllib
from pathlib import Path

from packaging.requirements import InvalidRequirement, Requirement
from packaging.specifiers import SpecifierSet
from packaging.utils import canonicalize_name
from packaging.version import Version

from projections import as_recorded

HERE = Path(__file__).resolve().parent
DATA = Path(os.environ.get("GRADER_DATA", str(HERE / "data")))
CORPUS = Path(os.environ.get("GRADER_CORPUS", "/opt/grader-data/corpus"))
WHEELHOUSE = Path(os.environ.get("GRADER_WHEELHOUSE", "/opt/wheelhouse"))
STATE_A = Path(os.environ.get("GRADER_STATE_A", "/opt/state-a/pydatalab"))
PYTHON311 = os.environ.get("GRADER_PYTHON", "/opt/python/bin/python3.11")
MONGOD = os.environ.get("GRADER_MONGOD", "/opt/mongodb/bin/mongod")
UV = os.environ.get("GRADER_UV", "uv")
TARGET = {"pydantic": "2.13.4", "pydantic-core": "2.46.4", "pydantic-settings": "2.15.0"}
RETIRED_TOP = {"pydantic", "pydantic_core", "pydantic_settings", "pydantic_v1"}
GRADED_EXTRAS = ["server", "apps"]

STAGE_I = [
    "S1-build", "S1-manifest", "S1-lockfile", "S1-installed", "S1-v1-namespace",
    "S1-default-path", "S1-deprecated-aliases", "S1-transplant", "S1-shim", "S1-verifier-isolation",
]
TRANSPLANT_THRESHOLD = 0.01
TOKEN_K = 40
DEPRECATED_SHARE = 0.05

ARGS = argparse.ArgumentParser()
ARGS.add_argument("tree")
ARGS.add_argument("out")
ARGS.add_argument("--work", default=None)


def log(msg: str) -> None:
    print(f"[grade {time.strftime('%H:%M:%S')}] {msg}", flush=True)


# ------------------------------------------------------------- tree handling


def is_build_product(path: Path, root: Path) -> bool:
    rel = path.relative_to(root)
    if path.is_dir():
        if path.name == "__pycache__" or path.name.endswith(".egg-info"):
            return True
        if (path / "pyvenv.cfg").is_file():
            return True
        if len(rel.parts) == 1 and path.name in ("build", "dist"):
            return True
    return False


def clean_copy(src: Path, dst: Path) -> None:
    """Copy the submitted tree minus every build product (virtualenvs, __pycache__, *.egg-info, build/, dist/)."""

    def ignore(directory: str, names: list[str]) -> list[str]:
        base = Path(directory)
        return [n for n in names if (base / n).is_dir() and not (base / n).is_symlink() and is_build_product(base / n, src)]

    shutil.copytree(src, dst, symlinks=True, ignore=ignore)


def regular_files(root: Path):
    for path in sorted(root.rglob("*")):
        if path.is_file() and not path.is_symlink():
            yield path


def text_of(path: Path) -> str | None:
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if b"\0" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def python_files(root: Path):
    for path in regular_files(root):
        if path.suffix == ".py" and ".git" not in path.relative_to(root).parts:
            yield path


# ---------------------------------------------------------------- Stage I static


def check_manifest(tree: Path) -> tuple[bool, str]:
    retired = [Version(v) for v in json.loads((DATA / "pydantic-1x-releases.json").read_text())]
    found_main = False
    problems = []

    def judge(req_text: str, where: str, main: bool) -> None:
        nonlocal found_main
        try:
            req = Requirement(req_text)
        except InvalidRequirement:
            return
        if canonicalize_name(req.name) != "pydantic":
            return
        spec = req.specifier
        admitted = [str(v) for v in retired if spec.contains(v, prereleases=True)]
        if admitted:
            problems.append(f"{where}: {req_text!r} admits retired {admitted[-1]}")
        if main:
            found_main = True
            if not spec.contains(Version(TARGET["pydantic"]), prereleases=True):
                problems.append(f"{where}: {req_text!r} excludes {TARGET['pydantic']}")

    pyproject = tree / "pyproject.toml"
    if not pyproject.is_file():
        return False, "pyproject.toml absent"
    try:
        data = tomllib.loads(pyproject.read_text())
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as exc:
        return False, f"pyproject.toml unreadable: {exc}"
    project = data.get("project", {})
    for req in project.get("dependencies", []):
        judge(req, "project.dependencies", True)
    for extra, reqs in project.get("optional-dependencies", {}).items():
        for req in reqs:
            judge(req, f"optional-dependencies.{extra}", False)
    for group, reqs in data.get("dependency-groups", {}).items():
        for req in reqs:
            if isinstance(req, str):
                judge(req, f"dependency-groups.{group}", False)
    for path in regular_files(tree):
        if re.fullmatch(r"(requirements|constraints)[\w.-]*\.(txt|in)", path.name):
            for line in (text_of(path) or "").splitlines():
                line = line.split("#", 1)[0].strip()
                if line and not line.startswith("-"):
                    judge(line.split(";")[0].strip(), str(path.relative_to(tree)), False)
    if not found_main:
        problems.append("project.dependencies declares no pydantic requirement")
    return (not problems), "; ".join(problems) or "ok"


def wheelhouse_pins() -> set[tuple[str, str]]:
    pins = set()
    for path in WHEELHOUSE.iterdir():
        name = path.name
        if name.endswith(".whl"):
            dist, version = name.split("-")[:2]
        elif name.endswith(".tar.gz"):
            dist, version = name[:-7].rsplit("-", 1)
        else:
            continue
        pins.add((canonicalize_name(dist), version))
    return pins


def check_lockfile(tree: Path) -> tuple[bool, str]:
    lock = tree / "uv.lock"
    if not lock.is_file():
        return False, "uv.lock absent"
    try:
        data = tomllib.loads(lock.read_text())
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as exc:
        return False, f"uv.lock unreadable: {exc}"
    pins = wheelhouse_pins()
    problems = []
    for pkg in data.get("package", []):
        name, version = canonicalize_name(pkg.get("name", "")), pkg.get("version")
        source = pkg.get("source", {})
        if name == "pydantic" and version and Version(version) < Version("2"):
            problems.append(f"pins retired pydantic {version}")
        if "registry" not in source or version is None:
            continue
        if (name, version) not in pins:
            problems.append(f"{name}=={version} not in the wheelhouse")
    if not any(canonicalize_name(p.get("name", "")) == "pydantic" for p in data.get("package", [])):
        problems.append("no pydantic pin")
    return (not problems), "; ".join(problems[:8]) or "ok"


def corpus_windows() -> set[tuple[str, ...]]:
    windows: set[tuple[str, ...]] = set()
    with tarfile.open(CORPUS / "pydantic-1.10.26.tar.gz") as tar:
        for member in tar.getmembers():
            if not member.isfile() or not member.name.endswith(".py"):
                continue
            parts = set(Path(member.name).parts)
            if parts & {".git", ".venv", "venv", "site-packages", "node_modules", "__pycache__"}:
                continue
            handle = tar.extractfile(member)
            if handle:
                windows |= grams(norm_tokens(handle.read().decode("utf-8", "replace")), TOKEN_K)
    return windows


def norm_tokens(text: str) -> list[str]:
    out = []
    try:
        for tok in tokenize.generate_tokens(io.StringIO(text).readline):
            if tok.type == tokenize.NAME:
                out.append(tok.string if keyword.iskeyword(tok.string) else "N")
            elif tok.type == tokenize.STRING:
                out.append("S")
            elif tok.type == tokenize.NUMBER:
                out.append("0")
            elif tok.type == tokenize.OP:
                out.append(tok.string)
    except (tokenize.TokenError, IndentationError, SyntaxError):
        pass
    return out


def grams(seq: list[str], k: int) -> set[tuple[str, ...]]:
    return {tuple(seq[i : i + k]) for i in range(len(seq) - k + 1)}


PACKED = re.compile(r"\.(whl|egg|zip|tar|tgz|tbz2|txz|gz|bz2|xz|zst|7z|rar|jar|so|pyd|pyc|pyo|dylib|dll)$", re.I)


def check_transplant(tree: Path) -> tuple[bool, str, float]:
    retired = corpus_windows()
    seen: set[tuple[str, ...]] = set()
    for path in regular_files(tree):
        text = text_of(path)
        if text is not None:
            seen |= grams(norm_tokens(text), TOKEN_K) & retired
    share = len(seen) / len(retired)
    baseline = {str(p.relative_to(STATE_A)) for p in regular_files(STATE_A)}
    packed = [
        str(p.relative_to(tree)) for p in regular_files(tree)
        if PACKED.search(p.name) and str(p.relative_to(tree)) not in baseline and ".git" not in p.relative_to(tree).parts
    ]
    ok = share <= TRANSPLANT_THRESHOLD and not packed
    reason = f"retired-source share {share:.4f} (threshold {TRANSPLANT_THRESHOLD})"
    if packed:
        reason += f"; packed or compiled files absent from State A: {packed[:5]}"
    return ok, reason, round(share, 4)


SHIM_NAMES = re.compile(r"^(sitecustomize|usercustomize)\.py$|\.pth$")
REDIRECT = re.compile(r"sys\.(meta_path|path_hooks)\b|sys\.modules\s*\[[^\]]*\]\s*=|sys\.modules\.(update|setdefault)\(")


def check_shim(tree: Path) -> tuple[bool, str]:
    problems = []
    for path in regular_files(tree):
        rel = path.relative_to(tree)
        if ".git" in rel.parts:
            continue
        if SHIM_NAMES.search(path.name):
            problems.append(f"{rel}: startup hook file")
        stem = path.stem if path.suffix in (".py", ".pyi") else None
        if stem in RETIRED_TOP and rel.parts[0] in ("src", "plugins") or (stem in RETIRED_TOP and len(rel.parts) == 1):
            problems.append(f"{rel}: module named {stem}")
        if path.suffix == ".py":
            text = text_of(path) or ""
            for line_no, line in enumerate(text.splitlines(), 1):
                if REDIRECT.search(line):
                    problems.append(f"{rel}:{line_no}: import machinery redirect")
    for path in tree.rglob("*"):
        if path.is_dir() and path.name in RETIRED_TOP and ".git" not in path.relative_to(tree).parts:
            if (path / "__init__.py").exists() or any(path.glob("*.py")):
                problems.append(f"{path.relative_to(tree)}: package named {path.name}")
    return (not problems), "; ".join(problems[:8]) or "ok"


VERIFIER_NEEDLES = [
    "/tests/", "/tests\"", "/tests'", "/logs/verifier", "reward.json", "reward-details", "ctrf",
    "rewardkit", "GATE_VERDICT", "VETO_SCRATCH", "HARBOR_", "/opt/grader", "grader-data", "GRADER_",
]
# A needle that starts with "/" is an absolute path: it counts only where no path character,
# and no closing bracket of an interpolated prefix such as f"{ROOT}/tests" or "$(pwd)/tests",
# precedes it, so a relative path such as pydatalab/tests/server is not the grader's /tests.
NEEDLE_PATTERNS = [
    (needle, re.compile(("(?<![\\w.)}-])" if needle.startswith("/") else "") + re.escape(needle)))
    for needle in VERIFIER_NEEDLES
]


def check_isolation(tree: Path) -> tuple[bool, str]:
    baseline: dict[str, int] = {}
    for path in regular_files(STATE_A):
        text = text_of(path) or ""
        for needle, pattern in NEEDLE_PATTERNS:
            baseline[f"{path.relative_to(STATE_A)}|{needle}"] = len(pattern.findall(text))
    problems = []
    for path in regular_files(tree):
        rel = path.relative_to(tree)
        if ".git" in rel.parts:
            continue
        text = text_of(path)
        if text is None:
            continue
        for needle, pattern in NEEDLE_PATTERNS:
            count = len(pattern.findall(text))
            if count > baseline.get(f"{rel}|{needle}", 0):
                problems.append(f"{rel}: names {needle!r}")
    return (not problems), "; ".join(problems[:8]) or "ok"


def v1_ast_sites(tree: Path) -> list[str]:
    sites = []
    for path in python_files(tree):
        try:
            module = ast.parse(path.read_bytes())
        except (SyntaxError, ValueError):
            continue
        for node in ast.walk(module):
            names = []
            if isinstance(node, ast.ImportFrom) and node.module:
                names = [node.module] + [f"{node.module}.{a.name}" for a in node.names]
            elif isinstance(node, ast.Import):
                names = [a.name for a in node.names]
            if any(n == "pydantic.v1" or n.startswith("pydantic.v1.") for n in names):
                sites.append(f"{path.relative_to(tree)}:{node.lineno}")
            if isinstance(node, ast.Constant) and isinstance(node.value, str) and re.fullmatch(r"pydantic\.v1(\.\w+)*", node.value):
                sites.append(f"{path.relative_to(tree)}:{node.lineno}")
    return sorted(set(sites))


V1_METHODS = {"dict", "json", "copy", "parse_obj", "parse_raw", "parse_file", "schema", "schema_json", "construct",
              "update_forward_refs", "from_orm", "validate"}
V1_ATTRS = {"__fields__", "__fields_set__"}


def v1_interface_sites(tree: Path, *, with_methods: bool) -> set[str]:
    """Static v1-interface sites: validator and root_validator decorators, inner class Config,
    and (for the State A baseline) calls of the v1 model methods and field attributes."""
    sites = set()
    for path in python_files(tree):
        if path.relative_to(tree).parts[0] not in ("src", "plugins"):
            continue  # application code only, on both sides of the comparison
        try:
            module = ast.parse(path.read_bytes())
        except (SyntaxError, ValueError):
            continue
        rel = str(path.relative_to(tree))
        for node in ast.walk(module):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                for deco in node.decorator_list:
                    target = deco.func if isinstance(deco, ast.Call) else deco
                    name = target.attr if isinstance(target, ast.Attribute) else getattr(target, "id", None)
                    if name in ("validator", "root_validator"):
                        sites.add(f"{rel}:{node.lineno}")
            if isinstance(node, ast.ClassDef) and node.bases:
                for inner in node.body:
                    if isinstance(inner, ast.ClassDef) and inner.name == "Config":
                        sites.add(f"{rel}:{inner.lineno}")
            if with_methods:
                if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr in V1_METHODS:
                    sites.add(f"{rel}:{node.lineno}")
                if isinstance(node, ast.Attribute) and node.attr in V1_ATTRS:
                    sites.add(f"{rel}:{node.lineno}")
    return sites


# -------------------------------------------------------------------- build


def run(cmd: list[str], *, cwd: Path | None = None, env: dict | None = None, timeout: int = 1200) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True, timeout=timeout)


def graded_build(tree: Path, work: Path) -> tuple[bool, str, Path | None]:
    env = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": str(work / "home"),
        "UV_CACHE_DIR": str(work / "uv-cache"),
        "UV_PYTHON_DOWNLOADS": "never",
        "UV_OFFLINE": "1",
        "UV_NO_PROGRESS": "1",
    }
    (work / "home").mkdir(parents=True, exist_ok=True)
    if not (tree / "pyproject.toml").is_file():
        return False, "resolve: pyproject.toml absent", None
    if not (tree / "uv.lock").is_file():
        return False, "resolve: uv.lock absent", None
    export = run([UV, "export", "--frozen", "--no-emit-project", "--format", "requirements-txt", "--no-header",
                  *sum((["--extra", e] for e in GRADED_EXTRAS), [])], cwd=tree, env=env)
    if export.returncode != 0:
        return False, "resolve: " + export.stderr.strip()[-400:], None
    req = work / "graded-requirements.txt"
    req.write_text(export.stdout)
    venv = work / "venv"
    made = run([UV, "venv", "--no-config", "-q", "--python", PYTHON311, str(venv)], env=env)
    if made.returncode != 0:
        return False, "install: " + made.stderr.strip()[-400:], None
    deps = run([UV, "pip", "install", "--no-config", "-q", "--python", str(venv / "bin" / "python"), "--no-index",
                "--find-links", str(WHEELHOUSE), "--require-hashes", "-r", str(req)], env=env, timeout=1800)
    if deps.returncode != 0:
        return False, "install: " + deps.stderr.strip()[-400:], None
    proj = run([UV, "pip", "install", "--no-config", "-q", "--python", str(venv / "bin" / "python"), "--no-index",
                "--find-links", str(WHEELHOUSE), "--no-deps", str(tree)], env=env, timeout=1800)
    if proj.returncode != 0:
        return False, "build: " + proj.stderr.strip()[-400:], None
    return True, "ok", venv


def project_files(venv: Path) -> Path:
    probe = run([str(venv / "bin" / "python"), "-I", str(HERE / "installed.py"), "files"])
    out = venv.parent / "project-files.txt"
    out.write_text(probe.stdout)
    return out


def check_installed(venv: Path) -> tuple[bool, str]:
    probe = run([str(venv / "bin" / "python"), "-I", str(HERE / "installed.py"), "metadata"])
    if probe.returncode != 0:
        return False, "probe failed: " + probe.stderr.strip()[-300:]
    data = json.loads(probe.stdout)
    problems = []
    for dist, version in TARGET.items():
        if data["versions"].get(dist) != version:
            problems.append(f"{dist} installed {data['versions'].get(dist)}, expected {version}")
    for dist, tops in data["providers"].items():
        clash = sorted(set(tops) & RETIRED_TOP)
        if clash and canonicalize_name(dist) not in TARGET:
            problems.append(f"{dist} provides {clash}")
    return (not problems), "; ".join(problems) or "ok"


class Mongo:
    def __init__(self, work: Path, port: int):
        self.dbpath = work / f"mongo-{port}"
        self.dbpath.mkdir(parents=True, exist_ok=True)
        self.port = port

    def __enter__(self):
        subprocess.run([MONGOD, "--bind_ip", "127.0.0.1", "--port", str(self.port), "--dbpath", str(self.dbpath),
                        "--fork", "--logpath", str(self.dbpath / "mongod.log"), "--quiet"], check=True,
                       capture_output=True, timeout=120)
        return self

    def __exit__(self, *exc):
        subprocess.run([MONGOD, "--shutdown", "--dbpath", str(self.dbpath)], capture_output=True, timeout=120)


def replay(venv: Path, files: Path, cases: list, work: Path, port: int) -> dict:
    work.mkdir(parents=True, exist_ok=True)
    (work / "home").mkdir(exist_ok=True)
    spec = work / "cases.json"
    spec.write_text(json.dumps(cases))
    out = work / "observed.json"
    env = {k: v for k, v in os.environ.items() if not k.startswith(("PYDATALAB_", "GATE_VERDICT", "VETO_SCRATCH", "HARBOR_", "GRADER_", "OPENAI_"))}
    env.update({"GRADER_EXAMPLE_DATA": str(STATE_A / "example_data"), "HOME": str(work / "home")})
    drive = run([str(venv / "bin" / "python"), "-I", str(HERE / "drive.py"), str(spec), str(out), "--files", str(files),
                 "--work", str(work / "drive"), "--mongo", f"mongodb://127.0.0.1:{port}"], env=env, cwd=work, timeout=1800)
    if not out.is_file():
        return {"start": "error", "start_trace": (drive.stderr or "")[-1500:], "cases": {}, "probes": {}}
    return json.loads(out.read_text())


def sample(expected: dict, observed: dict, member: str) -> dict:
    """Expected and observed text of one differing step, truncated, for the verifier log only."""
    case_id, idx = member.split("#")
    got = observed.get(case_id)
    got_map = {str(i): o for i, o in got} if isinstance(got, list) else {}
    return {"step": member, "expected": json.dumps(expected[case_id][idx], sort_keys=True)[:1500],
            "observed": json.dumps(as_recorded(expected[case_id][idx], got_map.get(idx, "<missing>")), sort_keys=True)[:1500]}


def compare(expected: dict, observed: dict) -> dict:
    """Per case, the kept step indexes whose observation differs from the recording."""
    diffs = {}
    for case_id, steps in expected.items():
        got = observed.get(case_id)
        if not isinstance(got, list):
            got_map = {}
        else:
            got_map = {str(i): o for i, o in got}
        bad = [i for i, want in steps.items() if as_recorded(want, got_map.get(i)) != want]
        if bad:
            diffs[case_id] = bad
    return diffs


# ------------------------------------------------------------- judge input


def render_for_judge(tree: Path, out_dir: Path) -> None:
    """Line-numbered text of the application sources the migration concerns, for the judged items.

    Selected: pyproject.toml and every Python file under src/ or plugins/ whose
    submitted text or State A counterpart mentions pydantic. State A's copy of
    the same files is rendered beside it as read-only context.
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    chosen = ["pyproject.toml"]
    names = set()
    for root in (tree, STATE_A):
        for path in python_files(root):
            rel = path.relative_to(root)
            if rel.parts[0] in ("src", "plugins") and "pydantic" in (text_of(path) or ""):
                names.add(str(rel))
    chosen += sorted(names)
    for label, root, name in (("SUBMISSION", tree, "submission.txt"), ("STATE A (original, read-only context)", STATE_A, "state-a.txt")):
        parts = [f"# {label}: files of the datalab project at {'/app/pydatalab' if root == tree else 'State A'}\n"]
        for rel in chosen:
            path = root / rel
            text = text_of(path) if path.is_file() else None
            if text is None:
                parts.append(f"\n=== {rel} (absent) ===\n")
                continue
            body = "".join(f"{i:5d}| {line}\n" for i, line in enumerate(text.splitlines(), 1))
            parts.append(f"\n=== {rel} ===\n{body}")
        (out_dir / name).write_text("".join(parts))


# --------------------------------------------------------------------- main


def main() -> None:
    opts = ARGS.parse_args()
    work = Path(opts.work or "/tmp/grade")
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    items: dict[str, dict] = {}

    def put(item: str, ok: bool, reason: str, **extra) -> None:
        items[item] = {"status": "pass" if ok else "fail", "reason": reason, **extra}
        log(f"{item}: {'pass' if ok else 'fail'} ({reason[:160]})")

    stage2 = json.loads((DATA / "stage2.json").read_text())
    stage3 = json.loads((DATA / "stage3.json").read_text())
    submitted = Path(opts.tree)
    tree = work / "tree" / "pydatalab"
    if not submitted.is_dir():
        reason = f"no submitted tree at {submitted}"
        for item in STAGE_I:
            put(item, False, reason)
        groups = {g: {"status": "fail", "reason": reason} for g in stage2["groups"]}
        families = {f: {"status": "fail", "reason": reason} for f in stage3["families"]}
        Path(opts.out).write_text(json.dumps({"items": items, "groups": groups, "families": families}, indent=1))
        return
    clean_copy(submitted, tree)
    render_for_judge(tree, Path(os.environ.get("GRADE_JUDGE_INPUT", "/tmp/grade/judge")))

    put("S1-manifest", *check_manifest(tree))
    put("S1-lockfile", *check_lockfile(tree))
    ok, reason, share = check_transplant(tree)
    put("S1-transplant", ok, reason, share=share)
    put("S1-shim", *check_shim(tree))
    put("S1-verifier-isolation", *check_isolation(tree))
    static_v1 = v1_ast_sites(tree)

    built, reason, venv = graded_build(tree, work / "build")
    observed: dict = {"cases": {}, "probes": {}}
    if built and venv is not None:
        put("S1-installed", *check_installed(venv))
        files = project_files(venv)
        cases = stage2["cases"] + stage3["cases"]
        with Mongo(work, 27017):
            observed = replay(venv, files, cases, work / "replay", 27017)
        if observed.get("start") != "ok":
            built, reason = False, "start: " + (observed.get("start_trace") or "server did not start")[-400:]
    else:
        put("S1-installed", False, "graded build failed: " + reason)
    put("S1-build", built, reason)

    probes = observed.get("probes") or {}
    if built:
        runtime_v1 = [f"{f}:{line}" for f, line, _ in probes.get("v1_imports", [])]
        put("S1-v1-namespace", not static_v1 and not runtime_v1,
            "ok" if not (static_v1 or runtime_v1) else f"pydantic.v1 imported at {(static_v1 + runtime_v1)[:6]}")
        default = probes.get("default_path") or {}
        bad = default.get("not_v2")
        if "error" in default or bad is None:
            put("S1-default-path", False, f"probe failed: {default.get('error', 'no probe')}")
        else:
            put("S1-default-path", not bad and bool(probes.get("model_classes")),
                "ok" if not bad else f"model classes off the installed pydantic 2: {bad[:6]}")
        baseline = len(v1_interface_sites(STATE_A, with_methods=True))
        limit = int(baseline * DEPRECATED_SHARE)
        runtime = {f"{f}:{line}" for f, line, _ in probes.get("deprecations", [])}
        static = v1_interface_sites(tree, with_methods=False)
        count = len(runtime | {f"static:{s}" for s in static})
        put("S1-deprecated-aliases", count <= limit, f"{count} deprecated v1-alias sites (limit {limit} of State A's {baseline})",
            sites=sorted(runtime)[:20] + sorted(static)[:20])
    else:
        put("S1-v1-namespace", not static_v1, "static scan only, graded build failed" if not static_v1 else f"pydantic.v1 imported at {static_v1[:6]}")
        put("S1-default-path", False, "graded build failed")
        put("S1-deprecated-aliases", False, "graded build failed")

    got = observed.get("cases", {})
    diffs2 = compare(stage2["expected"], got)
    groups = {}
    for group in stage2["groups"]:
        members = stage2["groups"][group]
        failed = [f"{c}#{i}" for c, idxs in diffs2.items() for i in idxs if f"{c}#{i}" in members]
        groups[group] = {"status": "pass" if built and not failed else "fail",
                         "reason": "ok" if built and not failed else (f"{len(failed)} of {len(members)} cases differ: {failed[:5]}" if built else "graded build failed")}
        if built and failed:
            groups[group]["samples"] = [sample(stage2["expected"], got, m) for m in failed[:3]]
    diffs3 = compare(stage3["expected"], got)
    families = {}
    for family, case_id in stage3["families"].items():
        bad = case_id in diffs3
        families[family] = {"status": "pass" if built and not bad else "fail",
                            "reason": "ok" if built and not bad else ("differs from State A" if built else "graded build failed")}
    Path(opts.out).write_text(json.dumps({"items": items, "groups": groups, "families": families}, indent=1, sort_keys=True))
    log("done")


if __name__ == "__main__":
    main()
