GENERATED SECTION. DO NOT HAND-EDIT.

<!-- FORGE-CANARY-BEGIN -->
<!-- FORGE-CANARY-SLOT-1: 60ae0f0c079c366733615a2e6551dc4e64c530f1c56ffed9db295862b101ddc8 -->
<!-- FORGE-CANARY-SLOT-2: 87ab65e4ae3bd13b9cfb24e8b811c4ab101c8552a8b3d1f0d91c32bbd429958a -->
<!-- FORGE-CANARY-SLOT-3: 76135f235d13f30c4446a51ada3f9a19b6fba2e250fa6b2ffc5a14d1b3f55ffb -->
<!-- FORGE-CANARY-SLOT-4: e415e80ce167e9922758ef5d43162679b0fe51046883994e0fc30102c1e183f5 -->
<!-- FORGE-CANARY-END -->

# How this task is solved

The repository is github.com/ibireme/yyjson at release 0.13.0, commit
6447536015f3d600f3d65323b10976103b337ca7, under the MIT licence. The task moves its
implementation from C to Rust and holds everything observable fixed.

Each step below names what was done, the state it established, the mistake it survives,
and the graded items it satisfies. The steps are ordered: a later step assumes the state
an earlier one established.

## S-01

Action: Author task.toml: the Harbor contact, the artifact closure collected from /workspace/repo, the resource and network policy, and the upstream provenance record.

Established: The task declares what is collected and under which policy, and names its upstream source, base commit and fork-ancestry digest.

Survives: An artifact closure that collected build output would let a prebuilt library pass as a built one; the exclusion list prevents it.

Deliverable: D-01 (task.toml, the Harbor contact and the upstream provenance record)

Satisfies no graded item: task.toml is a FORGE-authored bundle file, graded in no dimension.

## S-02

Action: Author instruction.md as the only statement the agent sees: State B, what must not change, the graded build's compiler policy and how to reproduce it, the note to write, and how the work is judged.

Established: Every requirement the grading rests on is stated, and no check is enumerated.

Survives: Stating the checks would turn the task into a checklist; stating the requirements leaves the original implementation as the only specification.

Deliverable: D-02 (instruction.md, the only statement the agent sees)

Satisfies no graded item: instruction.md is a FORGE-authored bundle file, graded in no dimension.

## S-03

Action: Author environment/: a two-stage image pinned by digest that installs the State A toolchain and the Rust toolchain, unpacks the verified archive into /workspace/repo as one squashed commit, and runs as uid 1000.

Established: The agent starts from State A with a working C toolchain, an offline cargo, and a local copy of the graded compiler policy at /opt/graded/bin.

Survives: Without a C toolchain the agent could not observe State A; without the policy shim the no-compiler rule would be a hidden trap.

Deliverable: D-03 (environment/, the agent's image and State A's archive)

Satisfies no graded item: environment/ is a FORGE-authored bundle file, graded in no dimension.

## S-04

Action: Author solution/: the migration payload, this grounding record, the generator that derives every emitted file, and the reference note.

Established: The reference migration exists as data plus a pure generator, so TRUTH.md, solve.sh, provenance.yaml, the criteria registration and the reward table are all derived rather than hand-maintained.

Survives: A hand-maintained TRUTH.md drifts from the items it claims; a generated one cannot.

Deliverable: D-04 (solution/, the reference migration, this record and its generator)

Satisfies no graded item: solution/ is a FORGE-authored bundle file, graded in no dimension.

## S-05

Action: Author tests/: the verifier image, the graded pipeline, the frozen State A records, the stage-2 harness, the stage-3 corpus and the compiler recorder.

Established: Grading reads only frozen records of State A and the submitted tree, and runs offline except for the judge route.

Survives: Recording State A at grading time would let a submission that breaks State A's build also break its own reference; freezing the records at image build time prevents it.

Deliverable: D-05 (tests/, the verifier image, the graded pipeline and the frozen records)

Satisfies no graded item: tests/ is a FORGE-authored bundle file, graded in no dimension.

## S-06

Action: Keep src/yyjson.h byte for byte and derive the Rust layouts from it: repr(C) yyjson_val, yyjson_mut_val, yyjson_doc, yyjson_mut_doc, the string and value pools, and the tag encoding, with compile-time size and offset assertions (src/types.rs, src/hdr.rs).

Established: Every static inline function in the unchanged header reads and writes the port's memory correctly.

Survives: A reordered field or a changed tag layout leaves the header compiling and silently wrong; the header-interop module reads every accessor against State A.

Deliverable: D-06 (the shipped library and its installed tree)

Red line: R-04 (any change to src/yyjson.h)

Satisfies:

- P-04: The submitted src/yyjson.h differs from State A's by at least one byte.
- T-14: Every recorded case of module B-09 (header interop) agrees with State A on every recorded metric.

## S-07

Action: Delete src/yyjson.c and leave no other C, C++, Objective-C or assembly translation unit, in any file name, and no verbatim copy of the implementation under another extension.

Established: The library's machine code can only come from Rust.

Survives: Renaming the C file, or embedding it as a resource and generating Rust from it during the build, both reproduce State A without migrating it.

Deliverable: D-06 (the shipped library and its installed tree)

Red line: R-06 (src/yyjson.c or any other State A translation unit left in the tree)

Satisfies:

- P-06: src/yyjson.c or another State A translation unit remains in the submitted tree, or the tree carries at least a quarter of State A's implementation as verbatim spans under any name.

## S-08

Action: Create the crate as #![no_std], edition 2024, with no dependency, no alloc crate and no global allocator, and export exactly the 52 entry points as #[unsafe(no_mangle)] pub unsafe extern "C" (src/lib.rs, src/ffi.rs).

Established: The shipped library offers State A's symbol set and nothing else, and resolves no crate from a registry, a git source or a vendored copy.

Survives: Pulling in libc or any convenience crate changes the dependency surface and the import set.

Deliverable: D-06 (the shipped library and its installed tree)

Red line: R-05 (a registry, git or vendored third-party dependency)

Satisfies:

- T-02: The shipped shared library exports exactly State A's 52 dynamic symbols.
- P-05: The graded build compiles a crate from a registry, a git source or a vendored third-party copy.

## S-09

Action: Port the reader: trivia and comment skipping, literals, strings, containers, the flag matrix and the in-situ path (src/reader.rs, src/util.rs, src/utf8_seq.rs, src/tables.rs).

Established: Every document and every malformed input produces State A's value tree, return value and consumed length.

Survives: Reader mutants that shift a bound, accept a trailing comma or mis-handle a BOM are separated by the parse-and-error family.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-06: Every recorded case of module B-01 (reader) agrees with State A on every recorded metric.
- T-18: Every stage-3 case in family X-01 (parse and error) agrees with State A on every recorded metric.

## S-10

Action: Port the number reader: the digit scanner, the big-integer and diy floating-point slow paths, the two fast paths, and the hex and raw forms (src/read_num.rs).

Established: Every numeric literal yields State A's exact bits, subtype and error.

Survives: Rounding mutants inside diy_fp and the exactness windows of the fast paths are separated by the numeric round-trip family.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-08: Every recorded case of module B-03 (numbers) agrees with State A on every recorded metric.
- T-19: Every stage-3 case in family X-02 (numeric round-trip) agrees with State A on every recorded metric.

## S-11

Action: Port the incremental reader so a document split at an arbitrary byte resumes with the same state (src/incr.rs).

Established: Chunked reading returns MORE at the same boundaries and completes to the same document as a single read.

Survives: A resume that re-scans or drops a partial token diverges on inputs split inside a literal, a string escape or a number.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-13: Every recorded case of module B-08 (incremental reader) agrees with State A on every recorded metric.

## S-12

Action: Port error reporting: the code, the constant message text and the byte position for every failure path (src/reader.rs, src/util.rs).

Established: Every failure reports State A's code, its exact message bytes and its exact position.

Survives: A message reworded or a position off by one is invisible to a caller that only checks success.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-07: Every recorded case of module B-02 (errors) agrees with State A on every recorded metric.

## S-13

Action: Port the writer and the number writer: the escaping tables, minified and pretty layouts, the flag matrix, and digit generation including the fixed and float forms (src/writer.rs, src/write_num.rs).

Established: Serialising any document produces State A's bytes exactly, including its escaping and formatting choices.

Survives: Writer mutants that change an escape table entry, a separator or the fixed-notation cut-over are separated by the escaping and formatting family.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-09: Every recorded case of module B-04 (writer) agrees with State A on every recorded metric.
- T-20: Every stage-3 case in family X-03 (writer escaping and formatting) agrees with State A on every recorded metric.

## S-14

Action: Port the mutable document API: the string and value pools, the circular child lists, the copies and the equality relation (src/value.rs).

Established: Building, copying and comparing mutable documents matches State A, including iteration order and duplicate keys.

Survives: List surgery that loses the last-child invariant appears correct until a container is iterated twice.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-10: Every recorded case of module B-05 (mutable documents) agrees with State A on every recorded metric.

## S-15

Action: Port JSON Pointer, JSON Patch and JSON Merge Patch, including the context struct and the parent-creating forms (src/pointer.rs, src/patch.rs).

Established: Every pointer and patch operation returns State A's result, error code and error position.

Survives: Pointer mutants that mis-parse an escaped token or an index are separated by the mutation and patch family.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-11: Every recorded case of module B-06 (pointer and patch) agrees with State A on every recorded metric.
- T-21: Every stage-3 case in family X-04 (mutation, pointer and patch sequences) agrees with State A on every recorded metric.

## S-16

Action: Honour the allocator contract: the three callbacks, the context pointer, the old size handed to realloc, and the order and size of every request, including the null, default, pool and dynamic allocators (src/alloc.rs).

Established: A caller tracing its allocator sees State A's exact request sequence, and a pool too small to serve it fails where State A fails.

Survives: Allocation-failure injection separates a port that allocates different sizes or in a different order even when its output is identical.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-12: Every recorded case of module B-07 (allocators) agrees with State A on every recorded metric.
- T-22: Every stage-3 case in family X-05 (allocation-failure injection) agrees with State A on every recorded metric.

## S-17

Action: Keep file input and output on the same libc stdio calls, with the same error mapping (src/reader.rs, src/writer.rs, src/ffi.rs).

Established: Reading and writing files reports State A's codes and leaves the same bytes on disk.

Survives: Buffering the whole file where State A streams it changes the failure point on a short write.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-15: Every recorded case of module B-10 (files) agrees with State A on every recorded metric.

## S-18

Action: Replace CMakeLists.txt: project(... LANGUAGES NONE) so no compiled language is probed, cargo driven through add_custom_command with the source directory as its working directory, the SONAME set by a link argument in .cargo/config.toml, and two hand-written package-config templates in place of the export set CMake would have generated.

Established: The unchanged CMake commands configure, build and install both library kinds with no C compiler present, producing State A's install tree and package configuration.

Survives: Leaving LANGUAGES C in place makes the configure step itself ask for a compiler, which the graded build refuses.

Deliverable: D-06 (the shipped library and its installed tree)

Red line: R-01 (any compile of a C, C++, Objective-C or assembly translation unit by the graded build)

Satisfies:

- T-03: The shared library's DT_SONAME equals State A's.
- T-04: The install prefix holds exactly State A's installed paths: the header, the libraries and their links, the CMake export set and the pkg-config file.
- T-16: Every recorded case of module B-12 (consumers) agrees with State A on every recorded metric.
- P-01: The graded build attempts to compile a C, C++, Objective-C or assembly translation unit.

## S-19

Action: Make the build idempotent: reconfiguring, rebuilding with nothing changed and reinstalling leave the installed tree as it was.

Established: The install tree is a function of the source, not of the build directory's history.

Survives: A build that regenerates a timestamped or path-dependent artefact fails a second install without failing the first.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-17: Reconfiguring, a no-op rebuild and reinstalling leave the installed tree byte-identical (module B-11).

## S-20

Action: Keep the shipped library's dependency and import surface inside State A's: only libc, and none of the dynamic-loading or process-spawning entry points.

Established: The library parses and writes in its own code, with State A's shared-library dependencies.

Survives: Delegating the work through dlopen or a helper process would pass behavioural comparison while moving the implementation outside the library.

Deliverable: D-06 (the shipped library and its installed tree)

Red line: R-03 (dlopen, LD_PRELOAD or a helper process doing the parsing or writing)

Satisfies:

- T-05: Every DT_NEEDED entry of the shared library is in State A's set.
- P-03: The shipped library imports dlopen, dlmopen, dlsym, fork, vfork, execve, posix_spawn, system or popen.

## S-21

Action: Build only from submitted source: ship no object file, archive or shared library for the build to link.

Established: Everything the library is made of is rebuilt from the tree the grading collects.

Survives: A prebuilt archive linked into the library reproduces State A's behaviour with none of its source.

Deliverable: D-06 (the shipped library and its installed tree)

Red line: R-02 (a prebuilt object, archive or shared library shipped in the submission and linked in)

Satisfies:

- P-02: The graded build links an object, archive or shared library it did not build from the submitted source.

## S-22

Action: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.

Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

Survives: A port that passes every recorded comparison by accident cannot explain why its unsafe operations are sound.

Deliverable: D-07 (doc/rust-port.md, the root-cause and design note the agent writes)

Satisfies no graded item: D-07 is graded by the judged layer (J-01 to J-14), which carries no compiled criterion.

## The judged note

The note at doc/rust-port.md answers the obligations below. Each is read on its own against this record.

- J-01: The note gives a Rust layout for yyjson_val, yyjson_mut_val, yyjson_doc and yyjson_mut_doc whose field order, sizes and alignment are the ones the reference states keep every inline function in the unchanged yyjson.h valid.
- J-02: The note states how the 64-bit tag packs a value's type, subtype and length, as the reference states it.
- J-03: The note states how an immutable document stores its values in one contiguous arena and how a container's offset skips its children, as the reference states it.
- J-04: The note states how mutable containers keep their children in a circular list reached through the last child, as the reference states it.
- J-05: The note states the yyjson_alc contract the port honours, namely the three callbacks, the context pointer, the old size passed to realloc and the order of requests, as the reference states it.
- J-06: For every category of unsafe operation the reference lists, the note gives the invariant that makes the port's use of it sound.
- J-07: The note states how an allocation failure at any request returns State A's error result instead of aborting, as the reference states it.
- J-08: Every State A behaviour the reference lists as easy to mistake for a defect is named in the note as kept on purpose.
- J-09: The note states how numbers are read and written so that output matches State A digit for digit, as the reference states it.
- J-10: The note states how the incremental reader resumes across arbitrary chunk boundaries, as the reference states it.
- J-11: The note states how the unchanged CMake invocation drives cargo and installs State A's tree, SONAME and export set while hiding Rust runtime symbols, as the reference states it.
- J-12: The note states how the port was checked against State A, naming the comparisons that were run.
- J-13: Each design decision the note states names the Rust module, type or function that implements it.
- J-14: The note states how the port keeps a Rust panic from unwinding into a C caller at every exported entry point, as the reference states it.

## Routes that do not solve the task

- oracle: The reference migration exactly as solution/ ships it. Expected: 1.0, every item true.
- state-a: State A unchanged. Expected: 0.0: the configure step asks for a C compiler and the graded build refuses it, so no graded target is written.
- empty: An empty submission. Expected: 0.0, by the same fail-closed path.
- c-wrapper: A Rust shim whose build compiles State A's C and links it in. Expected: 0.0: the compile is refused.
- header-edit: The reference migration with one byte changed in src/yyjson.h. Expected: Below 1.0 with P-04 false and every other item true.
- partial: The reference migration with a defect planted in the writer, the mutable API, the allocator trace and one stage-3 family. Expected: Below 1.0, naming the failing modules and cases.
- prebuilt: The reference migration shipping a prebuilt archive the build links. Expected: Below 1.0 with P-02 false.
- vendored: The reference migration resolving a vendored third-party crate. Expected: Below 1.0 with P-05 false.
- embedded: The reference migration carrying State A's implementation as a non-C resource. Expected: Below 1.0 with P-06 false.
- fair-quotation: The reference migration quoting eighty-eight lines of State A's C in the design note. Expected: 1.0: quoting a passage to explain a decision is not shipping the implementation.

## What the grading does not separate

- Incremental reading past completion is undefined behaviour in State A, not an error path, so no case exercises it.
- State A's "number too long" message is unreachable through the public API, because truncation is reported first.
- YYJSON_READ_ERROR_DEPTH never compiles at the default depth limits, so no case reaches it.
- Permission-denied file paths are left out of the file module: the outcome depends on the effective user id, which would fail a correct port.
- unsafe_yyjson_mut_str_alc with a null document and unsafe_yyjson_mut_strncpy with a null string are unguarded in State A, so neither is compared.
- yyjson_mem_* and yyjson_strlen are not declared in the public header and are not part of the compared surface.
- The scratch contents of a failing buffer writer are undocumented in State A and are not compared.
- Stage 3 separates no mutant in mutable list surgery or the merge-patch walk: array ordering, duplicate-key handling, rotation, renaming, merge-patch sequences, raw and no-escape writing, the non-copying string constructors and pointer parent creation are exercised but discriminate nothing. Stage 2 modules B-05, B-06 and B-07 cover them.
- Stage 3 separates no mutant in incremental reading, file input and output, or the pool and dynamic allocators; stage 2 modules B-07, B-08 and B-10 cover them.
- Fourteen of the one hundred and four mutants survive the stage-3 corpus. Three are output-equivalent by construction. Six are rounding decisions no candidate reached. Two turn a bounds guard into an out-of-bounds read observable only for bytes outside the buffer. One needs an array of 10^18 elements. Two are reachable only within one chunk header of the maximum object size.
- The verbatim-span relation behind P-06 does not detect a transformed embedding of State A's implementation, such as a base64, compressed or reformatted copy. The judged note and the Phase attack anti-shortcut lane are the remaining defences.
- T-04 compares installed paths, kinds and link targets rather than the contents of the CMake export set; wrong contents are separated instead by the consumer module's find_package route.
- T-17 compares installed bytes after folding the install prefix string, because the pkg-config file and the CMake package configuration legitimately record their own prefix.
- P-03 reads the shared library's dynamic imports; the static archive's undefined symbols are not an import relation and are not read.
- The reference port is verified on linux/arm64 only. State A's ten behavioural record files are byte-identical on linux/amd64 and both images build for both platforms, but rustc cannot run under the authoring host's emulator, so the port itself was not exercised there. The frozen baseline is recorded per platform inside the verifier image, so an amd64-only difference would surface as an oracle control failure rather than as a wrong baseline.
- T-05 is a subset relation, so a shipped library declaring no dynamic dependency at all would satisfy it. No working submission can avoid the C runtime, so the relation is left as the contract binds it.
- The static archive carries rustc's compiler-builtins members, so it holds 406 members and 603 defined global text symbols against State A's single member with 52. Exactly 52 carry default visibility and 551 are hidden, so a final link still exports State A's set, and no compiled criterion reads archive membership.
