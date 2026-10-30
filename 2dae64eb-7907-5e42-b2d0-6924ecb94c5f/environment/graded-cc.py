#!/usr/bin/env python3
"""Reproduce the graded build's compiler policy locally, so you can check your build under it.

Your workspace has a working C toolchain, because you need it to build and observe the
original library while it is still there. The build that produces the shipped artifacts is
graded under a different policy: it has no C, C++, Objective-C or assembly compiler. Put this
directory first on PATH to run your own build under that policy:

    PATH=/opt/graded/bin:$PATH cmake -S . -B build-check -DBUILD_SHARED_LIBS=ON
    PATH=/opt/graded/bin/:$PATH cmake --build build-check

Linking is not compiling, so link-only invocations are handed to the real driver; the Rust
toolchain links through `cc` and must keep working.
"""

from __future__ import annotations

import os
import shlex
import sys

REAL = {"cc": "/usr/bin/gcc", "gcc": "/usr/bin/gcc", "c++": "/usr/bin/g++", "g++": "/usr/bin/g++"}
SOURCE_SUFFIXES = (
    ".c .i .ii .cc .cp .cxx .cpp .c++ .C .CPP .m .mi .mm .M .mii .s .S .sx .asm".split()
)
LINK_SUFFIXES = ".o .obj .a .lo .la .rlib .rmeta .ld .lds".split()
LANGUAGES = (
    "c c-header cpp-output c++ c++-header c++-cpp-output objective-c objective-c-header "
    "objective-c-cpp-output objective-c++ objective-c++-header objective-c++-cpp-output "
    "assembler assembler-with-cpp".split()
)
VALUE_FLAGS = (
    "-o -I -L -l -D -U -include -imacros -isystem -iquote -idirafter -isysroot --sysroot -MF "
    "-MT -MQ -Xlinker -Xpreprocessor -Xassembler -z -u -e -T -soname -rpath -B -specs "
    "-aux-info -dumpbase -dumpdir -param -arch -target".split()
)
QUERY_FLAGS = (
    "--version -dumpversion -dumpfullversion -dumpmachine -dumpspecs --help --target-help -V "
    "-qversion".split()
)
STAGE_FLAGS = "-c -S -E -fsyntax-only".split()


def expand(argv: list[str], depth: int = 0) -> list[str]:
    if depth > 8:
        return argv
    out: list[str] = []
    for argument in argv:
        if not argument.startswith("@") or len(argument) == 1:
            out.append(argument)
            continue
        try:
            with open(argument[1:], encoding="utf-8", errors="replace") as handle:
                out.extend(expand(shlex.split(handle.read(), posix=True), depth + 1))
        except OSError:
            out.append(argument)
    return out


def suffix_of(name: str) -> str:
    index = name.rfind(".")
    return name[index:] if index > 0 else ""


def translation_units(tool: str, argv: list[str]) -> list[str]:
    if tool in {"cpp", "as"}:
        return [item for item in argv if not item.startswith("-")]
    language: str | None = None
    units: list[str] = []
    stage = False
    index = 0
    while index < len(argv):
        argument = argv[index]
        index += 1
        if argument in STAGE_FLAGS:
            stage = True
        elif argument == "-x":
            language = argv[index].lower() if index < len(argv) else None
            index += 1
        elif argument.startswith("-x") and len(argument) > 2:
            language = argument[2:].lower()
        elif argument in VALUE_FLAGS:
            index += 1
        elif argument.startswith("-"):
            continue
        elif argument == "-":
            if language in LANGUAGES:
                units.append("<stdin>")
        elif suffix_of(argument) in SOURCE_SUFFIXES or language in LANGUAGES:
            units.append(argument)
        elif stage and suffix_of(argument) not in LINK_SUFFIXES:
            units.append(argument)
    return units


def main(argv: list[str]) -> int:
    tool = os.path.basename(argv[0])
    arguments = expand(argv[1:])
    units = translation_units(tool, arguments)
    if units:
        sys.stderr.write(
            f"{tool}: refused: this build may not compile a C, C++, Objective-C or assembly "
            f"translation unit; offending input(s): {', '.join(units) or '<none>'}\n"
        )
        return 1
    if any(argument in QUERY_FLAGS for argument in arguments) or tool not in REAL:
        sys.stderr.write(f"{tool}: refused: no compiler is available to answer that\n")
        return 1
    os.execv(REAL[tool], [REAL[tool], *arguments])
    return 127


if __name__ == "__main__":
    sys.exit(main(sys.argv))
