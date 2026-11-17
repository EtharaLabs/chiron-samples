"""Transplant measurement: containment of retired-stack source in a tree, by normalised token k-grams."""
import hashlib
import json
import re
import sys
import zipfile
from pathlib import Path

KEYWORDS = set("""abstract assert boolean break byte case catch char class const continue default do double else
enum extends final finally float for goto if implements import instanceof int interface long native new package
private protected public return short static strictfp super switch synchronized this throw throws transient try void
volatile while true false null var record yield sealed permits""".split())
TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[A-Za-z_$][A-Za-z0-9_$]*|\d[\w.]*|\S')
COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.S)
HEADER = re.compile(r"^\s*(package|import)\s[^;]*;", re.M)


def tokens(text):
    text = COMMENT.sub(" ", text)
    text = HEADER.sub(" ", text)
    out = []
    for t in TOKEN.findall(text):
        if t[0] in "\"'":
            out.append("S")
        elif re.match(r"[A-Za-z_$]", t):
            out.append(t if t in KEYWORDS else "I")
        else:
            out.append(t)
    return out


def shingles(toks, k):
    return {hashlib.blake2b(" ".join(toks[i:i + k]).encode(), digest_size=8).digest() for i in range(len(toks) - k + 1)}


def corpus_files(corpus_dir):
    for jar in sorted(Path(corpus_dir).glob("*-sources.jar")):
        with zipfile.ZipFile(jar) as z:
            for n in z.namelist():
                if n.endswith(".java") and not n.endswith("module-info.java") and not n.endswith("package-info.java"):
                    yield f"{jar.name}!{n}", z.read(n).decode("utf-8", "replace")


def tree_files(root):
    for p in Path(root).rglob("*.java"):
        if "/target/" in str(p):
            continue
        yield str(p), p.read_text("utf-8", "replace")


def measure(corpus_dir, tree, k, min_tokens):
    sub = set()
    for _, text in tree_files(tree):
        sub |= shingles(tokens(text), k)
    rows = []
    total = matched = 0
    for name, text in corpus_files(corpus_dir):
        toks = tokens(text)
        if len(toks) < min_tokens:
            continue
        sh = shingles(toks, k)
        if not sh:
            continue
        hit = len(sh & sub)
        total += len(sh)
        matched += hit
        rows.append((hit / len(sh), name))
    rows.sort(reverse=True)
    return {
        "k": k, "min_tokens": min_tokens, "corpus_files": len(rows),
        "max_file_containment": round(rows[0][0], 4) if rows else 0.0,
        "max_file": rows[0][1] if rows else None,
        "files_over_0.5": sum(1 for c, _ in rows if c >= 0.5),
        "aggregate_share": round(matched / total, 5) if total else 0.0,
        "top5": [[round(c, 4), n] for c, n in rows[:5]],
    }


if __name__ == "__main__":
    corpus, tree, k, mt = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    print(json.dumps(measure(corpus, tree, k, mt)))
