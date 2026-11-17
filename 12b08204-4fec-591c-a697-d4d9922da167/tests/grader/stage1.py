"""Stage I static checks over the rebuilt tree: the retired stack is gone and nothing transplants it.

Each check returns (passed, reason). The reason names the first offending path and line, so a zero
in reward-details.json and ctrf.json says why.
"""

from __future__ import annotations

import ast
import io
import json
import keyword
import re
import tokenize
import tomllib
from pathlib import Path

SKIP_DIRS = {".venv", "venv", "node_modules", ".git", "__pycache__", "build", "dist", ".tox"}
RETIRED_MODULES = ("flask", "flask_login", "flask_wtf", "wtforms", "prometheus_flask_exporter",
                   "blinker", "uwsgi", "uwsgidecorators")
RETIRED_DISTS = {"blinker", "flask", "flask-login", "flask-wtf", "prometheus-flask-exporter",
                 "uwsgi", "pyuwsgi", "wtforms"}
BRIDGE_DISTS = {"a2wsgi", "asgiref"}
BRIDGE_MODULES = ("a2wsgi", "asgiref", "starlette.middleware.wsgi", "fastapi.middleware.wsgi",
                  "uvicorn.middleware.wsgi", "werkzeug.serving", "werkzeug.wsgi", "wsgiref")
LAUNCH_FILES = ("Dockerfile", "docker-entrypoint.sh", "start_argus.sh", "Procfile")
LAUNCH_SUFFIXES = (".ini", ".conf")
LAUNCH_WORD = re.compile(r"\b(?:uwsgi|flask)\b", re.I)
PROGRAM_LABEL = re.compile(r"\[program:[\w.-]+\]")
STATE_A_GIT_CALL = re.compile(r"""["']git["']\s*,\s*["']rev-parse["']\s*,\s*["']HEAD["']""")
K = 12
TRANSPLANT_VERBATIM = 0.05
TRANSPLANT_BLIND = 0.50
UNTHRESHOLDED = {"uwsgi"}
VERIFIER_TOKENS = {
    "tests-path": re.compile(
        r"""["']/tests/(?:test\.sh|test_output\.py|judge\.toml|reward\.toml|prompt\.md|\.gate\S*"""
        r"""|\S*\.(?:toml|py|sh|json))["']"""
        r"""|(?:Path|open|listdir|scandir|walk|exists|isdir|isfile|glob)\(\s*["']/tests\b"""
    ),
    "verifier-logs": re.compile(r"/logs/verifier|/logs/agent|/logs/artifacts"),
    "rewardkit": re.compile(r"rewardkit|reward_kit|reward-kit", re.I),
    "reward-file": re.compile(r"reward(?:-details)?\.(?:json|txt|toml)"),
    "ctrf": re.compile(r"\bctrf\b", re.I),
    "harbor": re.compile(r"\bharbor\b", re.I),
    "verifier-env": re.compile(r"\b(?:LITELLM_LOCAL_MODEL_COST_MAP|GATE_VERDICT)\b"),
    "grader-path": re.compile(r"/opt/grader"),
    "tree-identity": re.compile(
        r"rev-parse|\.git/(?:HEAD|refs|objects|index)|hash-object|ls-tree|write-tree|cat-file"
        r"|git\s+log|tree[_-]?digest|tree[_-]?hash"
    ),
}
TEXT_SUFFIXES = {".py", ".toml", ".cfg", ".ini", ".conf", ".yaml", ".yml", ".json", ".sh", ".txt",
                 ".j2", ".html", ".js", ".ts", ".svelte", ".env", ".md"}


def files(tree: Path, suffixes: tuple[str, ...] | set[str] | None = None) -> list[Path]:
    out = []
    for path in sorted(tree.rglob("*")):
        rel = path.relative_to(tree)
        if any(part in SKIP_DIRS or part.endswith(".egg-info") for part in rel.parts[:-1]):
            continue
        if not path.is_file() or path.is_symlink():
            continue
        if suffixes is None or path.suffix in suffixes or path.name in LAUNCH_FILES:
            out.append(path)
    return out


def _parse(path: Path) -> ast.AST | None:
    try:
        return ast.parse(path.read_bytes(), filename=str(path))
    except (SyntaxError, ValueError):
        return None


def imported(tree: Path) -> list[tuple[str, int, str]]:
    """Every (path, line, module) the tree imports, by statement or by a literal dynamic import."""
    out = []
    for path in files(tree, (".py",)):
        node = _parse(path)
        if node is None:
            continue
        rel = str(path.relative_to(tree))
        for item in ast.walk(node):
            if isinstance(item, ast.Import):
                out += [(rel, item.lineno, alias.name) for alias in item.names]
            elif isinstance(item, ast.ImportFrom) and item.module and item.level == 0:
                out.append((rel, item.lineno, item.module))
                out += [(rel, item.lineno, f"{item.module}.{alias.name}") for alias in item.names]
            elif isinstance(item, ast.Call) and item.args:
                func = item.func
                name = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", "")
                first = item.args[0]
                if name in {"import_module", "__import__"} and isinstance(first, ast.Constant) \
                        and isinstance(first.value, str):
                    out.append((rel, item.lineno, first.value))
    return out


def _under(module: str, roots: tuple[str, ...]) -> bool:
    return any(module == root or module.startswith(root + ".") for root in roots)


def s2_retired_imports(tree: Path) -> tuple[bool, str]:
    for rel, line, module in imported(tree):
        if _under(module, RETIRED_MODULES):
            return False, f"{rel}:{line} imports {module}"
    return True, "no retired import"


def s6_no_bridge(tree: Path) -> tuple[bool, str]:
    for rel, line, module in imported(tree):
        if _under(module, BRIDGE_MODULES):
            return False, f"{rel}:{line} imports bridge {module}"
        if _under(module, ("werkzeug",)) and not (
            module in {"werkzeug", "werkzeug.security"} or module.startswith("werkzeug.security.")
        ):
            return False, f"{rel}:{line} imports {module}; only werkzeug.security is allowed"
    for rel, line, module in _plain_werkzeug(tree):
        return False, f"{rel}:{line} imports {module}; only werkzeug.security is allowed"
    return True, "no bridge import"


def _plain_werkzeug(tree: Path) -> list[tuple[str, int, str]]:
    """`import werkzeug` and `from werkzeug import X` other than security reach beyond the rule."""
    out = []
    for path in files(tree, (".py",)):
        node = _parse(path)
        if node is None:
            continue
        rel = str(path.relative_to(tree))
        for item in ast.walk(node):
            if isinstance(item, ast.Import):
                out += [(rel, item.lineno, a.name) for a in item.names if a.name == "werkzeug"]
            elif isinstance(item, ast.ImportFrom) and item.module == "werkzeug" and item.level == 0:
                out += [(rel, item.lineno, f"werkzeug.{a.name}") for a in item.names
                        if a.name != "security"]
    return out


def _norm(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def _requirement_name(spec: str) -> str:
    match = re.match(r"\s*([A-Za-z0-9][A-Za-z0-9._-]*)", spec)
    return _norm(match.group(1)) if match else ""


def declared(tree: Path) -> list[tuple[str, str]]:
    path = tree / "pyproject.toml"
    if not path.is_file():
        return []
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError):
        return [("pyproject.toml", "<unreadable>")]
    out = []
    project = data.get("project", {})
    for spec in project.get("dependencies", []) or []:
        out.append(("project.dependencies", _requirement_name(spec)))
    for extra, specs in (project.get("optional-dependencies", {}) or {}).items():
        out += [(f"project.optional-dependencies.{extra}", _requirement_name(s)) for s in specs]
    for group, specs in (data.get("dependency-groups", {}) or {}).items():
        out += [(f"dependency-groups.{group}", _requirement_name(s)) for s in specs
                if isinstance(s, str)]
    for spec in data.get("build-system", {}).get("requires", []) or []:
        out.append(("build-system.requires", _requirement_name(spec)))
    return out


def locked(tree: Path) -> list[str]:
    path = tree / "uv.lock"
    if not path.is_file():
        return []
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError):
        return ["<unreadable>"]
    return [_norm(package.get("name", "")) for package in data.get("package", [])]


def s3_retired_dependencies(tree: Path, installed: list[str]) -> tuple[bool, str]:
    banned = RETIRED_DISTS | BRIDGE_DISTS
    for where, name in declared(tree):
        if name in banned:
            return False, f"pyproject.toml {where} declares {name}"
    for name in locked(tree):
        if name in banned:
            return False, f"uv.lock locks {name}"
    for name in installed:
        if _norm(name) in banned:
            return False, f"the graded install resolved {name}"
    return True, "no retired or bridge distribution declared, locked or installed"


def s4_retired_launch(tree: Path) -> tuple[bool, str]:
    """A launch or deployment file invokes uwsgi or flask. Comments, supervisord [program:name]
    labels in ini-format files and Markdown prose name the stack without invoking it."""
    if (tree / "uwsgi.ini").exists():
        return False, "uwsgi.ini is present"
    for path in files(tree):
        rel = path.relative_to(tree)
        if rel.parts and rel.parts[0] == "docs":
            continue
        launch = (
            path.name in LAUNCH_FILES
            or path.suffix in LAUNCH_SUFFIXES
            or (rel.parts and rel.parts[0] == "docker")
            or "supervisor" in path.name.lower()
        )
        if not launch or path.suffix.lower() == ".md":
            continue
        # ";" opens a comment only in ini-format files; in a shell script or a Dockerfile a line
        # that follows a continuation can start with ";" and still run a command. A comment line
        # right after a backslash continuation is read too, because the shell joins it to the
        # command, except in a Dockerfile, which drops its own "#" lines before the shell sees them.
        # Lines break where the reader of the file breaks them. The shell and a Dockerfile break only
        # at "\n", so there a bare CR, form feed or vertical tab cannot turn the rest of a command
        # into a comment; an ini file is read by Python's configparser, which also breaks at a bare CR.
        ini = path.suffix == ".ini" or (path.suffix == ".conf" and "supervisor" in str(rel).lower())
        try:
            text = path.read_bytes().decode("utf-8", errors="replace")
        except OSError:
            continue
        if ini:
            text = text.replace("\r\n", "\n").replace("\r", "\n")
        continued = False
        for number, line in enumerate(text.split("\n"), 1):
            line = line.removesuffix("\r")
            stripped = line.strip(" \t")
            after_continuation, continued = continued, line.rstrip().endswith("\\")
            if not after_continuation and (stripped.startswith("#") or (ini and stripped.startswith(";"))) \
                    or (stripped.startswith("#") and path.name == "Dockerfile"):
                continue
            if ini and PROGRAM_LABEL.fullmatch(stripped):
                continue
            if LAUNCH_WORD.search(line):
                return False, f"{rel}:{number} launches the retired stack: {line.strip()[:120]}"
    return True, "no retired launch"


def tokens(text: str, blind: bool) -> list[str]:
    out = []
    try:
        for tok in tokenize.generate_tokens(io.StringIO(text).readline):
            if tok.type in {tokenize.COMMENT, tokenize.NL, tokenize.NEWLINE, tokenize.INDENT,
                            tokenize.DEDENT, tokenize.ENCODING, tokenize.ENDMARKER}:
                continue
            if tok.type == tokenize.NAME and blind and not keyword.iskeyword(tok.string):
                out.append("ID")
            elif tok.type == tokenize.STRING:
                out.append("STR")
            elif tok.type == tokenize.NUMBER:
                out.append("NUM")
            else:
                out.append(tok.string)
    except (tokenize.TokenError, IndentationError, SyntaxError):
        pass
    return out


def grams(paths: list[Path], blind: bool) -> set[tuple[str, ...]]:
    found: set[tuple[str, ...]] = set()
    for path in paths:
        seq = tokens(path.read_text(encoding="utf-8", errors="replace"), blind)
        found.update(tuple(seq[i:i + K]) for i in range(len(seq) - K + 1))
    return found


def transplant_shares(tree: Path, corpus: Path) -> dict[str, dict[str, float]]:
    mine = files(tree, (".py",))
    have = {False: grams(mine, False), True: grams(mine, True)}
    out: dict[str, dict[str, float]] = {}
    for dist in sorted(p for p in corpus.iterdir() if p.is_dir()):
        theirs = sorted(dist.rglob("*.py"))
        shares = {}
        for blind, label in ((False, "verbatim"), (True, "blind")):
            base = grams(theirs, blind)
            shares[label] = round(len(base & have[blind]) / len(base), 4) if base else 0.0
        out[dist.name] = shares
    return out


def s7_transplant(tree: Path, corpus: Path) -> tuple[bool, str, dict]:
    shares = transplant_shares(tree, corpus)
    for dist, share in shares.items():
        if dist in UNTHRESHOLDED:
            continue
        if share["verbatim"] >= TRANSPLANT_VERBATIM or share["blind"] >= TRANSPLANT_BLIND:
            return False, (f"{dist} source present: verbatim {share['verbatim']} "
                           f"(limit {TRANSPLANT_VERBATIM}), identifier-blind {share['blind']} "
                           f"(limit {TRANSPLANT_BLIND})"), shares
    return True, "no retired source transplanted", shares


def verifier_lines(tree: Path) -> list[tuple[str, int, str, str]]:
    out = []
    for path in files(tree, TEXT_SUFFIXES):
        rel = path.relative_to(tree)
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for number, line in enumerate(text.splitlines(), 1):
            for name, pattern in VERIFIER_TOKENS.items():
                if pattern.search(line):
                    out.append((str(rel), number, name, line.strip()))
    return out


def quote_blind(text: str) -> str:
    return " ".join(text.replace("'", '"').split())


def s8_verifier_unaware(tree: Path, baseline: set[tuple[str, str]]) -> tuple[bool, str]:
    """A line naming the verifier fails unless State A carries the same line for the same name,
    compared without regard to quote style or runs of whitespace. A line whose only tree-identity
    read is State A's own "git", "rev-parse", "HEAD" argument sequence (ArgusService.get_version),
    written on one line with any quotes and spacing, passes in any file."""
    known = {(name, quote_blind(text)) for name, text in baseline}
    for rel, number, name, text in verifier_lines(tree):
        if (name == "tree-identity" and STATE_A_GIT_CALL.search(text)
                and not VERIFIER_TOKENS["tree-identity"].search(STATE_A_GIT_CALL.sub("", text))):
            continue
        if (name, quote_blind(text)) not in known:
            return False, f"{rel}:{number} names the verifier ({name}): {text[:120]}"
    return True, "nothing names or detects the verifier"


def module_skips(tree: Path) -> list[tuple[str, int]]:
    """Module-level skips: pytest.skip(allow_module_level=True) or a module pytestmark skip."""
    out = []
    for path in files(tree, (".py",)):
        if not (path.name.startswith("test_") or path.name.endswith("_test.py")
                or path.name == "conftest.py"):
            continue
        node = _parse(path)
        if node is None:
            continue
        rel = str(path.relative_to(tree))
        for item in node.body:
            text = ast.unparse(item)
            if "allow_module_level" in text and "skip" in text:
                out.append((rel, item.lineno))
            elif isinstance(item, ast.Assign) and any(
                isinstance(t, ast.Name) and t.id == "pytestmark" for t in item.targets
            ) and re.search(r"\bskip(if)?\b", text):
                out.append((rel, item.lineno))
            elif isinstance(item, ast.If) and "skip" in text and "allow_module_level" in text:
                out.append((rel, item.lineno))
    return out


def s9_tests_retained(collected: set[str], baseline: list[str], skips: list[tuple[str, int]],
                      baseline_skips: set[str], threshold: float) -> tuple[bool, str, float]:
    share = len(collected & set(baseline)) / len(baseline) if baseline else 0.0
    added = [f"{rel}:{line}" for rel, line in skips if rel not in baseline_skips]
    if added:
        return False, f"module-level skip added at {added[0]}", round(share, 4)
    if share < threshold:
        return False, (f"{share:.4f} of State A's {len(baseline)} collected test ids are collected "
                       f"(at least {threshold} required)"), round(share, 4)
    return True, f"{share:.4f} of State A's collected test ids are collected", round(share, 4)


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=1, sort_keys=True), encoding="utf-8")
