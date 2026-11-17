"""The execution boundary the graded build runs behind.

The recorder refuses a C compile by name, which is enough only while every name reaches
it. Two routes go around a name: a driver spelling the shim did not enumerate, and the
compiler proper called by its own absolute path, which no PATH can mediate. Both were
measured against this image: `aarch64-linux-gnu-gcc-14 -S` produced assembly that `rustc`
then assembled through its integrated assembler and exported from a cdylib, and
`/usr/libexec/gcc/<triple>/<version>/cc1` compiled C with no driver in the picture at all.

So the boundary is drawn around the compiler proper rather than around its drivers. The
image moves `cc1` and `lto1` into `PRIVATE`, a directory no rule here ever grants, and
this module restricts the graded build to an explicit allow-list of hierarchies it may
execute from. A driver stripped of `cc1` still links, which is all the recorder needs when
it passes a link through, and it can no longer turn C into assembly by any spelling or any
path. The verifier's own later stages run without this restriction and reach the real
compiler through `-B PRIVATE`, because the consumer routes must still compile C against
the submission's installed tree.

Landlock is an allow-list: a ruleset that handles a right denies it outside the
hierarchies an explicit rule grants, and a narrower nested rule cannot subtract a right
granted higher up. Carving an exception out of a granted tree was measured and does not
work, which is why `cc1` has to leave the tree instead.
"""

from __future__ import annotations

import ctypes
import os
import sys
from pathlib import Path

PRIVATE = Path("/opt/gcc-private")

# `/bin`, `/sbin` and `/lib` are symlinks into `/usr` on this base image and Landlock
# grants by inode, so naming either spelling grants the same hierarchy. Both are listed
# because a future base image may split them again.
ALLOW = (
    "/usr/bin",
    "/usr/local/bin",
    "/usr/sbin",
    "/usr/lib",
    "/usr/libexec",
    "/bin",
    "/sbin",
    "/lib",
    "/tmp",
    "/opt/rust",
    "/opt/verifier",
    "/workspace",
)

_CREATE_RULESET, _ADD_RULE, _RESTRICT_SELF = 444, 445, 446
_EXECUTE = 1 << 0
# Enforcing a ruleset that does not handle REFER makes the kernel deny every rename whose
# source and destination directories differ, reporting EXDEV. cargo renames a temporary
# rlib into place, so handling execution alone fails the build with "Invalid cross-device
# link" and nothing in that message names Landlock. This boundary restricts execution, not
# file movement, so REFER is handled and granted wherever execution is.
_REFER = 1 << 13
_GRANT = _EXECUTE | _REFER
_PATH_BENEATH = 1
_PR_SET_NO_NEW_PRIVS = 38


class _RulesetAttr(ctypes.Structure):
    _fields_ = [
        ("handled_access_fs", ctypes.c_uint64),
        ("handled_access_net", ctypes.c_uint64),
        ("scoped", ctypes.c_uint64),
    ]


class _PathBeneathAttr(ctypes.Structure):
    _pack_ = 1
    _fields_ = [("allowed_access", ctypes.c_uint64), ("parent_fd", ctypes.c_int32)]


def available() -> bool:
    """Whether this kernel accepts a ruleset at all, asked without enforcing one."""
    try:
        libc = ctypes.CDLL("libc.so.6", use_errno=True)
    except OSError:
        return False
    attr = _RulesetAttr(_GRANT, 0, 0)
    fd = libc.syscall(_CREATE_RULESET, ctypes.byref(attr), ctypes.sizeof(attr), 0)
    if fd < 0:
        return False
    os.close(fd)
    return True


def restrict(allow: tuple[str, ...] = ALLOW) -> None:
    """Deny this process execution outside `allow`, irrevocably and for every descendant."""
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    attr = _RulesetAttr(_GRANT, 0, 0)
    fd = libc.syscall(_CREATE_RULESET, ctypes.byref(attr), ctypes.sizeof(attr), 0)
    if fd < 0:
        raise OSError(ctypes.get_errno(), "landlock_create_ruleset")
    try:
        granted = 0
        for path in allow:
            if not os.path.exists(path):
                continue
            parent = os.open(path, os.O_PATH | os.O_CLOEXEC)
            try:
                rule = _PathBeneathAttr(_GRANT, parent)
                if libc.syscall(_ADD_RULE, fd, _PATH_BENEATH, ctypes.byref(rule), 0) < 0:
                    raise OSError(ctypes.get_errno(), f"landlock_add_rule {path}")
            finally:
                os.close(parent)
            granted += 1
        if granted == 0:
            raise OSError(0, "refusing to enforce a ruleset that grants nothing")
        if os.path.exists(PRIVATE) and any(
            Path(path) in PRIVATE.parents or Path(path) == PRIVATE for path in allow
        ):
            raise OSError(0, f"the allow-list reaches {PRIVATE}, which defeats the boundary")
        if libc.prctl(_PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0:
            raise OSError(ctypes.get_errno(), "prctl(PR_SET_NO_NEW_PRIVS)")
        if libc.syscall(_RESTRICT_SELF, fd, 0) < 0:
            raise OSError(ctypes.get_errno(), "landlock_restrict_self")
    finally:
        os.close(fd)


def main(argv: list[str]) -> int:
    if not argv:
        print("sandbox.py COMMAND [ARGUMENT ...]", file=sys.stderr)
        return 2
    restrict()
    os.execvp(argv[0], argv)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
