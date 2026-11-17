#!/usr/bin/env python3
"""The judge packet: the submission's own Rust and build files, against State A, in three parts.

The judged layer reads the port rather than a note about it, so the thing the judge is handed
is the code the submission added or changed. This module builds that text and nothing else: it
runs no build, reads no record and reaches nothing outside the two trees it is given.

The packet is THREE files, not one, and the reason is a limit in the harness rather than
anything about the port. Reward Kit replaces a judge input larger than
`rewardkit.judges._MAX_FILE_SIZE` (1 MiB) with the text `[skipped: file too large]`, and it
applies that limit to each named file separately: `judges._build_user_content` walks the
judge's `files` list and calls `_read_file_blocks` once per entry, which stats that entry on
its own. A single packet is therefore capped near 1 MB however much source the submission has,
and a port larger than that would have its later files listed but not inlined -- a fault that
sits only there could be cited by nobody and would score as absent, which on a negated item is
full credit. Three parts of at most `PART_LIMIT` bytes each are read in full, so about
`INLINE_CAP` bytes of source reach the judge instead of about 950 kB.

What goes in, in this order:

  * one block per collected file the submission ADDED or CHANGED against State A, in sorted
    path order, carrying every line of the file prefixed with its 1-based number and ": ". A
    changed file is shown whole rather than as a diff, because every criterion asks about a
    property of the submitted code and a hunk would hide the lines around the property;
  * `==> tree listing <==`, then every file of the tree with its size, its state against
    State A, whether it was inlined, and, when it was not, the first twelve hex digits of its
    sha256. The listing is what lets the judge see that a file exists and was not shown, so a
    submission cannot hide code by calling it a binary.

Collected means: `*.rs`, `Cargo.toml`, `Cargo.lock`, `build.rs`, `.cargo/config`,
`.cargo/config.toml`, `CMakeLists.txt`, `*.cmake`, `*.in`, and scripts (`*.py`, `*.sh`,
`*.pl`), at any depth, outside `target/`, `.git/` and build directories. That is every path a
build can turn into the shipped library, which is what the ten criteria and the gate ask
about. The tree listing is not restricted that way: it carries every file.

How the parts are filled:

  * every part opens with one header line naming its own number and all three part paths, so
    a judge reading part 2 knows it is holding a third of the evidence and which files carry
    the rest;
  * blocks are placed in sorted path order, into part 1 until it is full, then part 2, then
    part 3. The cursor only moves forward and never returns to a part it has left;
  * a file whose WHOLE block fits the room left in the part the cursor is on is written there
    in one piece, headed `==> <path> (<n> lines) <==`;
  * ANY OTHER file is SPLIT. As many whole lines as fit go into the part the cursor is on as a
    chunk headed `==> <path> (lines <a>-<b> of <n>) <==`, and the rest continues in further
    chunks, under that same header form, in the next part and the one after. There is no
    per-file size limit: a one-file port is read in full, in as many chunks as it takes. A
    line is NEVER split, and a line keeps the file's own 1-based number wherever its chunk
    lands, so a citation of "path, line" means the same thing in every part;
  * the tree listing goes at the end of the LAST part that received anything, which is where a
    judge finds it after reading the inlined source. `LISTING_BUDGET` is reserved out of every
    part, so the listing fits in whichever part turns out to be the last one;
  * all three paths are always written. A part that received nothing holds its header line and
    one line saying it is an empty part of the packet, so an absent file always means the
    stager refused to stage and never means "this part happened to be empty";
  * chunking stops when `INLINE_CAP` would be exceeded or when no part has room left for a
    header and one more line. A file the parts could not carry to its last line is listed as
    `not inlined: packet limit` and is reported in `build().incomplete`. A collected BINARY
    file is not inlined either -- a judge cannot cite a line of it -- but it is not incomplete:
    it gets the one-line stub and the listing row it has always had.

Three limits bound the output, and all three are deterministic:

  * `PART_LIMIT`, per part file, which is what keeps each part under Reward Kit's own limit.
  * `INLINE_CAP`, over the inlined text of all three parts together. This is the cap the task
    declares, and it is what binds: three parts could hold about 2.85 MB and the cap allows
    2.4 MB.
  * `LISTING_BUDGET`, over the tree listing, which is truncated with a line saying how many
    paths it dropped.

`build()` reports both the parts and the paths that did not fit, because a judge asked a
NEGATED question about source it was never shown would answer "no act found" and score that
as clean. tests/verifier/stage_note.py stages nothing at all when anything is incomplete and
sends the run to the zero route instead, so the cap is a refusal rather than a blind spot.

Nothing here depends on the clock, the locale, the environment, the walk order of the
filesystem or the identity of the process: the same two trees produce the same bytes in the
same three parts.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path
from typing import NamedTuple

# The inlined-text cap this task declares, over all three parts together.
INLINE_CAP = 2_400_000

# `rewardkit.judges._MAX_FILE_SIZE`. A judge input larger than this is not read at all: the
# judge is handed `[skipped: file too large]` in its place. Reward Kit applies it to each
# entry of the judge's `files` list separately, which is what makes three parts worth having.
JUDGE_FILE_LIMIT = 1024 * 1024

# What one part file may occupy, with about 48 kB of headroom under JUDGE_FILE_LIMIT. A round
# number rather than the limit itself: the margin covers nothing in particular, which is the
# point of a margin.
PART_LIMIT = 1_000_000

# How many parts the packet is. Three is what the inline cap needs: 2,400,000 bytes of source
# does not fit in two parts of 1,000,000.
PART_COUNT = 3

# Where the stager writes them. tests/verifier/stage_note.py takes the file names from here and
# joins them onto its own log directory, and /tests/judge.toml and /tests/.gate/judge.toml name
# these same three paths, in this same order, in their `files` entries.
LOG_DIRECTORY = "/logs/verifier"
PART_NUMBERS = tuple(range(1, PART_COUNT + 1))
PART_NAMES = tuple(f"judge-packet-{number}.txt" for number in PART_NUMBERS)
PART_PATHS = tuple(f"{LOG_DIRECTORY}/{name}" for name in PART_NAMES)

# Reserved out of every part for the tree listing, which is written last, into whichever part
# turned out to be the last non-empty one, and truncated to fit. About 500 paths at the width
# the listing uses. Reserving it in every part rather than only in the last one is what lets
# the listing be placed after the packing rather than before it.
LISTING_BUDGET = 48 * 1024

# Directories whose contents are no part of the submission: build output and the git database.
# `target` and `.git` are named outright; the rest is what task.toml's artifact closure already
# excludes, so the packet shows the tree Harbor collects and not a different one.
EXCLUDED_DIRECTORIES = frozenset({"target", ".git", "_build", "CMakeFiles"})
EXCLUDED_DIRECTORY_PREFIXES = ("build", "cmake-build")

# Collected by exact name, by suffix, and by name inside a `.cargo` directory.
COLLECTED_NAMES = frozenset({"Cargo.toml", "Cargo.lock", "build.rs", "CMakeLists.txt"})
COLLECTED_SUFFIXES = (".rs", ".cmake", ".in", ".py", ".sh", ".pl")
CARGO_CONFIG_NAMES = frozenset({"config", "config.toml"})

ADDED = "added"
CHANGED = "changed"
UNCHANGED = "unchanged"

LISTING_HEADER = "==> tree listing <=="

# The body of a part that received no block. One line, so a part file is never empty and never
# ambiguous: the judge is told it is holding an empty part rather than a truncated one.
EMPTY_PART_LINE = "==> this part of the judge packet carries no files <=="


def part_header(number: int) -> str:
    """The first line of part `number`: which part it is, and where all three parts are.

    Every part carries all three paths because the judge sees the parts as separate blocks of
    one user message and has no other way to know how many there are. The line is the same
    length for every part, so the budget below is the same for every part too.
    """
    joined = ", ".join(PART_PATHS)
    return (
        f"==> judge packet part {number} of {PART_COUNT}: the packet is the {PART_COUNT} "
        f"files {joined}, read in that order <==\n"
    )


def part_budget(number: int) -> int:
    """How many bytes of blocks part `number` may hold: its limit, less its header and reserve."""
    return PART_LIMIT - LISTING_BUDGET - len(part_header(number).encode("utf-8"))


def excluded_directory(name: str) -> bool:
    """Whether a directory of this name is build output rather than submitted source."""
    if name in EXCLUDED_DIRECTORIES:
        return True
    return any(name.startswith(prefix) for prefix in EXCLUDED_DIRECTORY_PREFIXES)


def collected(relative: Path) -> bool:
    """Whether this relative path is one of the kinds the judged layer reads."""
    if relative.name in COLLECTED_NAMES:
        return True
    if relative.suffix in COLLECTED_SUFFIXES:
        return True
    return relative.parent.name == ".cargo" and relative.name in CARGO_CONFIG_NAMES


def walk(root: Path) -> list[Path]:
    """Every regular file of `root`, relative and sorted, outside the excluded directories.

    Symbolic links are not followed and are not listed: a link is not content, and following
    one could walk out of the tree or around the exclusions.
    """
    found: list[Path] = []

    def descend(directory: Path, relative: Path) -> None:
        try:
            entries = sorted(directory.iterdir(), key=lambda item: item.name)
        except OSError:
            return
        for entry in entries:
            if entry.is_symlink():
                continue
            child = relative / entry.name
            if entry.is_dir():
                if not excluded_directory(entry.name):
                    descend(entry, child)
                continue
            if entry.is_file():
                found.append(child)

    descend(root, Path())
    return sorted(found, key=lambda item: item.as_posix())


def _read(path: Path) -> bytes | None:
    try:
        return path.read_bytes()
    except OSError:
        return None


def _text(data: bytes) -> str | None:
    """`data` as text, or None when it is binary.

    A NUL byte decides it before the decode does, because a UTF-16 source would decode under
    some codecs and is not text a judge can cite lines of.
    """
    if b"\x00" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def _state(data: bytes, original: Path | None) -> str:
    """Whether the submission added this file or changed it, compared by content."""
    if original is None:
        return ADDED
    previous = _read(original)
    if previous is None:
        return ADDED
    return UNCHANGED if previous == data else CHANGED


def _numbered(text: str) -> list[str]:
    """Every line of `text` under its own 1-based number, which is what a judge cites."""
    return [f"{number}: {line}" for number, line in enumerate(text.splitlines(), start=1)]


def whole_header(relative: str, total: int) -> str:
    """The header of a block that carries the file entire, which is today's form unchanged."""
    return f"==> {relative} ({total} lines) <=="


def chunk_header(relative: str, first: int, last: int, total: int) -> str:
    """The header of one chunk of a split file: which of its own lines this part carries."""
    return f"==> {relative} (lines {first}-{last} of {total}) <=="


def _render(header: str, rows: list[str]) -> str:
    return "\n".join([header, *rows]) + "\n"


def _width(text: str) -> int:
    return len(text.encode("utf-8"))


class Placement(NamedTuple):
    """What became of one file's block: how many of its lines were inlined, in how many chunks."""

    lines: int
    chunks: int
    total: int

    @property
    def complete(self) -> bool:
        return self.lines == self.total


class _Packer:
    """Blocks placed into the three parts in sorted order, split across parts where they must be.

    The cursor only ever moves forward. A block that fits the room left in the part the cursor
    is on goes there whole; anything else is cut at a line boundary and continues in the next
    part, so the tail of a part is filled rather than wasted and no file is ever withheld for
    being large. The cursor never goes back, so two trees differing in one late file still
    partition every earlier file identically: the packing of a file depends only on the files
    sorted before it.
    """

    def __init__(self) -> None:
        self.budgets = [part_budget(number) for number in PART_NUMBERS]
        self.blocks: list[list[str]] = [[] for _ in PART_NUMBERS]
        self.used = [0 for _ in PART_NUMBERS]
        self.cursor = 0
        self.spent = 0

    def _room(self) -> int:
        """Bytes the part at the cursor can still take, bounded by what is left of the cap."""
        if self.cursor >= PART_COUNT:
            return 0
        return min(
            self.budgets[self.cursor] - self.used[self.cursor],
            INLINE_CAP - self.spent,
        )

    def _append(self, text: str) -> None:
        width = _width(text)
        self.blocks[self.cursor].append(text)
        self.used[self.cursor] += width
        self.spent += width

    def place(self, text: str) -> bool:
        """Put the one-line stub `text` in the first part from the cursor on that can hold it.

        This is the path for a file with no lines to show -- a binary, or a file the cap shut
        out before a single chunk of it could be written. False means no part can take even
        that: either `INLINE_CAP` is spent or the cursor has run past the last part.
        """
        width = _width(text)
        if self.spent + width > INLINE_CAP:
            return False
        if width > max(self.budgets):
            return False
        while self.cursor < PART_COUNT:
            if self.used[self.cursor] + width <= self.budgets[self.cursor]:
                self._append(text)
                return True
            self.cursor += 1
        return False

    def place_file(self, relative: str, text: str) -> Placement:
        """Inline `relative` whole where it fits, in chunks where it does not; say how far it got.

        The whole block is tried against the room left in the part the cursor is on. When it
        fits, the block carries today's `(<n> lines)` header and nothing is split. When it does
        not, as many whole lines as fit go in under a `(lines <a>-<b> of <n>)` header and the
        walk continues into the next part, and the one after, until the file is done or the
        parts and the cap are out of room.
        """
        rows = _numbered(text)
        total = len(rows)
        if total == 0:
            # An empty file: a header and no lines, which still fits the whole-block form.
            block = _render(whole_header(relative, 0), [])
            if _width(block) <= self._room():
                self._append(block)
                return Placement(0, 1, 0)
            return Placement(0, 0, 0)

        whole = _render(whole_header(relative, total), rows)
        if _width(whole) <= self._room():
            self._append(whole)
            return Placement(total, 1, total)

        placed = 0
        chunks = 0
        while placed < total and self.cursor < PART_COUNT:
            room = self._room()
            # The header is measured at its widest -- `<b>` is at most `<n>`, so a header
            # written for the real last line can only be shorter than the one budgeted here.
            reserve = _width(chunk_header(relative, placed + 1, total, total)) + 1
            taken = _fitting(rows, placed, room - reserve)
            if taken == 0:
                # Nothing more fits here. When the whole-packet cap is what ran out, no later
                # part can help and the file ends short; when it is only this part's budget,
                # the cursor moves on. The two are told apart by asking the cap alone.
                need = reserve + _width(rows[placed]) + 1
                if INLINE_CAP - self.spent < need:
                    break
                self.cursor += 1
                continue
            header = chunk_header(relative, placed + 1, placed + taken, total)
            self._append(_render(header, rows[placed : placed + taken]))
            placed += taken
            chunks += 1
        return Placement(placed, chunks, total)

    def last_filled(self) -> int:
        """The index of the last part that received anything, or 0 when none did.

        The cursor only moves forward and text is only ever placed at the cursor, so the parts
        that received something are a prefix and this is the end of it. When nothing was
        inlined at all the listing goes into part 1, which makes part 1 the non-empty one.
        """
        filled = [index for index in range(PART_COUNT) if self.blocks[index]]
        return filled[-1] if filled else 0


def _fitting(rows: list[str], start: int, limit: int) -> int:
    """How many whole rows from `start` fit in `limit` bytes, counting each row's newline."""
    if limit <= 0:
        return 0
    width = 0
    count = 0
    for index in range(start, len(rows)):
        step = _width(rows[index]) + 1
        if width + step > limit:
            break
        width += step
        count += 1
    return count


def _listing(rows: list[str]) -> str:
    """The tree listing, truncated to `LISTING_BUDGET` with a line saying what it dropped."""
    out = [LISTING_HEADER]
    width = len(LISTING_HEADER.encode("utf-8")) + 1
    dropped = 0
    for line in rows:
        chunk = len(line.encode("utf-8")) + 1
        if width + chunk > LISTING_BUDGET - 128:
            dropped += 1
            continue
        width += chunk
        out.append(line)
    if dropped:
        out.append(f"... {dropped} further path(s) omitted: listing limit")
    return "\n".join(out) + "\n"


class Packet(NamedTuple):
    """The built packet: its `PART_COUNT` parts, and the collected source they could not carry.

    `incomplete` is the point of this type. It holds the sorted posix paths of the collected
    text files the submission added or changed whose LAST line did not reach a part -- the
    source a judge would be asked a negated question about without ever having seen it. A
    binary is not in it: a judge could cite no line of one however much room there was.
    """

    parts: list[str]
    incomplete: list[str]


def build(workspace: Path, original: Path) -> Packet:
    """The packet for `workspace`, read against State A at `original`, and what did not fit.

    Exactly `PART_COUNT` strings come back, in part order, every one of them non-empty and
    every one of them at most `PART_LIMIT` bytes when encoded as UTF-8.
    """
    paths = walk(workspace)
    packer = _Packer()
    listing: list[str] = []
    incomplete: list[str] = []
    for relative in paths:
        posix = relative.as_posix()
        source = workspace / relative
        data = _read(source)
        if data is None:
            listing.append(f"{posix}  unreadable")
            continue
        previous = original / relative
        state = _state(data, previous if previous.is_file() else None)
        digest = hashlib.sha256(data).hexdigest()[:12]
        if not collected(relative):
            listing.append(f"{posix}  {len(data)} bytes  {state}  not collected  sha256 {digest}")
            continue
        if state == UNCHANGED:
            listing.append(f"{posix}  {len(data)} bytes  {state}  not inlined: unchanged")
            continue
        text = _text(data)
        if text is None:
            packer.place(f"==> {posix} <== not inlined: binary\n")
            listing.append(f"{posix}  {len(data)} bytes  {state}  not inlined: binary  sha256 {digest}")
            continue
        placed = packer.place_file(posix, text)
        if placed.complete:
            shown = "inlined" if placed.chunks <= 1 else f"inlined in {placed.chunks} chunks"
            listing.append(f"{posix}  {len(data)} bytes  {state}  {shown}")
            continue
        incomplete.append(posix)
        if placed.chunks == 0:
            packer.place(f"==> {posix} <== not inlined: packet limit\n")
            short = "not inlined: packet limit"
        else:
            short = (
                f"not inlined: packet limit, {placed.lines} of {placed.total} lines "
                f"in {placed.chunks} chunk(s)"
            )
        listing.append(f"{posix}  {len(data)} bytes  {state}  {short}  sha256 {digest}")

    tail = packer.last_filled()
    listed = _listing(listing)
    parts: list[str] = []
    for index in range(PART_COUNT):
        body = part_header(index + 1) + "".join(packer.blocks[index])
        if index == tail:
            body += listed
        elif not packer.blocks[index]:
            body += EMPTY_PART_LINE + "\n"
        parts.append(body)
    return Packet(parts, sorted(incomplete))


def build_parts(workspace: Path, original: Path) -> list[str]:
    """Just the parts, for the callers and the tests that ask nothing about completeness."""
    return build(workspace, original).parts


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(f"usage: {argv[0]} <submitted-tree> <state-a-tree>", file=sys.stderr)
        return 2
    built = build(Path(argv[1]), Path(argv[2]))
    for part in built.parts:
        sys.stdout.write(part)
    for path in built.incomplete:
        print(f"packet: {path} did not fit in the {PART_COUNT} parts", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
