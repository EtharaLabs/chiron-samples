#!/usr/bin/env python3
"""The graded build's compiler driver: it records every invocation and refuses every compile.

The task moves yyjson's implementation off C, so the build that produces the shipped
artifacts must not compile a C, C++, Objective-C or assembly translation unit. This program
stands in for `cc`, `gcc`, `c++`, `g++`, `cpp`, `as` and `cargo` on the graded build's PATH.
It appends one JSON line per invocation to the ledger named by RECORDER_LEDGER, then either
refuses the invocation (a translation-unit compile) or hands it to the real tool (anything
else, linking included, because the Rust toolchain links through `cc`).

Refusing rather than silently recording is deliberate: a build that still needs a C compiler
cannot produce artifacts at all, so it scores zero on every graded target instead of shipping
a half-migrated library. The refusal text names the rule and the offending input.
"""

from __future__ import annotations

import json
import os
import shlex
import sys

LEDGER_VARIABLE = "RECORDER_LEDGER"
REAL_PREFIX = "RECORDER_REAL_"

# Every extension GCC treats as a translation unit it compiles, including the
# already-preprocessed and Objective-C forms, plus both assembly spellings.
SOURCE_SUFFIXES = frozenset(
    {
        ".c",
        ".i",
        ".ii",
        ".cc",
        ".cp",
        ".cxx",
        ".cpp",
        ".c++",
        ".C",
        ".CPP",
        ".m",
        ".mi",
        ".mm",
        ".M",
        ".mii",
        ".s",
        ".S",
        ".sx",
        ".asm",
    }
)
# Linker inputs and anything else a link-only invocation may name.
LINK_SUFFIXES = frozenset({".o", ".obj", ".a", ".lo", ".la", ".rlib", ".rmeta", ".ld", ".lds"})
# `-x LANG` values that make the following inputs translation units.
COMPILED_LANGUAGES = frozenset(
    {
        "c",
        "c-header",
        "cpp-output",
        "c++",
        "c++-header",
        "c++-cpp-output",
        "objective-c",
        "objective-c-header",
        "objective-c-cpp-output",
        "objective-c++",
        "objective-c++-header",
        "objective-c++-cpp-output",
        "assembler",
        "assembler-with-cpp",
    }
)
# Flags that take a separate argument which is never an input file.
VALUE_FLAGS = frozenset(
    {
        "-o",
        "-I",
        "-L",
        "-l",
        "-D",
        "-U",
        "-include",
        "-imacros",
        "-isystem",
        "-iquote",
        "-idirafter",
        "-isysroot",
        "--sysroot",
        "-MF",
        "-MT",
        "-MQ",
        "-Xlinker",
        "-Xpreprocessor",
        "-Xassembler",
        "-z",
        "-u",
        "-e",
        "-T",
        "-soname",
        "-rpath",
        "-B",
        "-specs",
        "-aux-info",
        "-dumpbase",
        "-dumpdir",
        "-param",
        "-arch",
        "-target",
    }
)
# Invocations that only ask the driver about itself; they build nothing.
QUERY_FLAGS = frozenset(
    {
        "--version",
        "-dumpversion",
        "-dumpfullversion",
        "-dumpmachine",
        "-dumpspecs",
        "--help",
        "--target-help",
        "-V",
        "-qversion",
    }
)
COMPILE_STAGE_FLAGS = frozenset({"-c", "-S", "-E", "-fsyntax-only"})


def expand(argv: list[str], depth: int = 0) -> list[str]:
    """`argv` with every GCC response file (`@file`) spliced in, as the driver would read it."""
    if depth > 8:
        return argv
    out: list[str] = []
    for argument in argv:
        if not argument.startswith("@") or len(argument) == 1:
            out.append(argument)
            continue
        try:
            with open(argument[1:], encoding="utf-8", errors="replace") as handle:
                nested = shlex.split(handle.read(), posix=True)
        except OSError:
            out.append(argument)
            continue
        out.extend(expand(nested, depth + 1))
    return out


def suffix_of(name: str) -> str:
    index = name.rfind(".")
    return name[index:] if index > 0 else ""


def classify(tool: str, argv: list[str]) -> tuple[str, list[str]]:
    """The invocation's kind (`compile`, `link`, `query`, `cargo`) and the inputs that decide it."""
    if tool == "cargo":
        # Recorded so the pipeline can ask each named manifest what it resolves, then
        # handed straight to cargo: Rust is the target language, not a refused one.
        return "cargo", [item for item in argv if not item.startswith("-")]
    if tool in {"cpp", "as"}:
        # Preprocessing or assembling a translation unit is still compiling one.
        return "compile", [item for item in argv if not item.startswith("-")]

    language: str | None = None
    inputs: list[str] = []
    translation_units: list[str] = []
    stage = False
    query = False
    index = 0
    while index < len(argv):
        argument = argv[index]
        index += 1
        if argument in QUERY_FLAGS:
            query = True
            continue
        if argument in COMPILE_STAGE_FLAGS:
            stage = True
            continue
        if argument == "-x":
            language = argv[index].lower() if index < len(argv) else None
            index += 1
            continue
        if argument.startswith("-x") and len(argument) > 2:
            language = argument[2:].lower()
            continue
        if argument in VALUE_FLAGS:
            index += 1
            continue
        if argument.startswith("-"):
            continue
        # A bare `-` is stdin, which only names a translation unit under `-x LANG`.
        inputs.append(argument)
        if argument == "-":
            if language in COMPILED_LANGUAGES:
                translation_units.append("<stdin>")
            continue
        if suffix_of(argument) in SOURCE_SUFFIXES or language in COMPILED_LANGUAGES:
            translation_units.append(argument)
            continue
        if stage and suffix_of(argument) not in LINK_SUFFIXES:
            translation_units.append(argument)

    if translation_units:
        return "compile", translation_units
    if query or not inputs:
        return "query", inputs
    return "link", inputs


def real_tool(tool: str) -> str | None:
    """The tool this shim stands in for, from the environment the pipeline sets.

    Every driver spelling the image ships is shimmed, including `gcc-14`, `c99-gcc` and the
    target-triple forms, so a compile cannot slip past by choosing another name. Only the
    canonical few get their own variable; the rest fall back to the default real driver, which
    is correct because this path is reached only for invocations that are not compiles.
    """
    named = os.environ.get(REAL_PREFIX + tool.replace("-", "_").replace("+", "x").upper())
    return named or os.environ.get(REAL_PREFIX + "DEFAULT")


def record(entry: dict[str, object]) -> None:
    ledger = os.environ.get(LEDGER_VARIABLE)
    if not ledger:
        return
    line = json.dumps(entry, sort_keys=True, ensure_ascii=True) + "\n"
    handle = os.open(ledger, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o644)
    try:
        os.write(handle, line.encode("ascii"))
    finally:
        os.close(handle)


def main(argv: list[str]) -> int:
    tool = os.path.basename(argv[0])
    arguments = expand(argv[1:])
    kind, inputs = classify(tool, arguments)
    target = real_tool(tool)
    record(
        {
            "tool": tool,
            "kind": kind,
            "cwd": os.getcwd(),
            "argv": arguments,
            "inputs": inputs,
            "refused": kind == "compile",
            "real": target,
        }
    )
    if kind == "compile":
        sys.stderr.write(
            f"{tool}: refused: this build may not compile a C, C++, Objective-C or assembly "
            f"translation unit; offending input(s): {', '.join(inputs) or '<none>'}\n"
        )
        return 1
    if target is None:
        sys.stderr.write(f"{tool}: refused: no real tool is configured for {tool}\n")
        return 1
    os.execv(target, [target, *arguments])
    return 127


if __name__ == "__main__":
    sys.exit(main(sys.argv))
