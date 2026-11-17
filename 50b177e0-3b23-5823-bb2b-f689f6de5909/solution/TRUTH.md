<!-- CANARY-BEGIN -->
<!-- CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6 -->
<!-- CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6 -->
<!-- CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17 -->
<!-- CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f -->
<!-- CANARY-END -->

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

Satisfies no graded item: task.toml is an authored bundle file, graded in no dimension.

## S-02

Action: Author instruction.md as the only statement the agent sees: State B, what must not change, the graded build's compiler policy and how to reproduce it, and how the work is judged, including the gate condition and the ten faults the review looks for.

Established: Every requirement the grading rests on is stated, and no check is enumerated.

Survives: Stating the checks would turn the task into a checklist; stating the requirements leaves the original implementation as the only specification.

Deliverable: D-02 (instruction.md, the only statement the agent sees)

Satisfies no graded item: instruction.md is an authored bundle file, graded in no dimension.

## S-03

Action: Author environment/: a two-stage image pinned by digest that installs the State A toolchain and the Rust toolchain, unpacks the verified archive into /workspace/repo as one squashed commit, and runs as uid 1000.

Established: The agent starts from State A with a working C toolchain, an offline cargo, and a local copy of the graded compiler policy at /opt/graded/bin.

Survives: Without a C toolchain the agent could not observe State A; without the policy shim the no-compiler rule would be a hidden trap.

Deliverable: D-03 (environment/, the agent's image and State A's archive)

Satisfies no graded item: environment/ is an authored bundle file, graded in no dimension.

## S-04

Action: Author solution/: the migration payload, this grounding record and the generator that derives every emitted file.

Established: The reference migration exists as data plus a pure generator, so TRUTH.md, solve.sh, provenance.yaml, the criteria registration and the reward table are all derived rather than hand-maintained.

Survives: A hand-maintained TRUTH.md drifts from the items it claims; a generated one cannot.

Deliverable: D-04 (solution/, the reference migration, this record and its generator)

Satisfies no graded item: solution/ is an authored bundle file, graded in no dimension.

## S-05

Action: Author tests/: the verifier image, the graded pipeline, the frozen State A records, the stage-2 harness, the stage-3 corpus split into sixty families, the judge packet builder and the compiler recorder.

Established: Grading reads only frozen records of State A and the submitted tree, and runs offline except for the judge route.

Survives: Recording State A at grading time would let a submission that breaks State A's build also break its own reference; freezing the records at image build time prevents it.

Deliverable: D-05 (tests/, the verifier image, the graded pipeline and the frozen records)

Satisfies no graded item: tests/ is an authored bundle file, graded in no dimension.

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
- S3-X01-bom: Every stage-3 case in X-01 under bom/ agrees with State A on every recorded metric.
- S3-X01-comma: Every stage-3 case in X-01 under comma/ agrees with State A on every recorded metric.
- S3-X01-comment: Every stage-3 case in X-01 under comment/ agrees with State A on every recorded metric.
- S3-X01-errpos: Every stage-3 case in X-01 under errpos/ agrees with State A on every recorded metric.
- S3-X01-esc: Every stage-3 case in X-01 under esc/ agrees with State A on every recorded metric.
- S3-X01-esckey: Every stage-3 case in X-01 under esckey/ agrees with State A on every recorded metric.
- S3-X01-json5: Every stage-3 case in X-01 under json5/ agrees with State A on every recorded metric.
- S3-X01-trunc: Every stage-3 case in X-01 under trunc/ agrees with State A on every recorded metric.
- S3-X01-utf8: Every stage-3 case in X-01 under utf8/ agrees with State A on every recorded metric.
- S3-X01-ws: Every stage-3 case in X-01 under ws/ agrees with State A on every recorded metric.
- S3-X01-trunc23: Every stage-3 case in X-01 under trunc2/ and trunc3/ agrees with State A on every recorded metric.

## S-10

Action: Port the number reader: the digit scanner, the big-integer and diy floating-point slow paths, the two fast paths, and the hex and raw forms (src/read_num.rs).

Established: Every numeric literal yields State A's exact bits, subtype and error.

Survives: Rounding mutants inside diy_fp and the exactness windows of the fast paths are separated by the numeric round-trip family.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-08: Every recorded case of module B-03 (numbers) agrees with State A on every recorded metric.
- S3-X02-d17: Every stage-3 case in X-02 under d17/ agrees with State A on every recorded metric.
- S3-X02-d768: Every stage-3 case in X-02 under d768/ agrees with State A on every recorded metric.
- S3-X02-diyfp: Every stage-3 case in X-02 under diyfp/ agrees with State A on every recorded metric.
- S3-X02-e21: Every stage-3 case in X-02 under e21/ agrees with State A on every recorded metric.
- S3-X02-expext: Every stage-3 case in X-02 under expext/ agrees with State A on every recorded metric.
- S3-X02-fuzz: Every stage-3 case in X-02 under fuzz/ agrees with State A on every recorded metric.
- S3-X02-halfrnd: Every stage-3 case in X-02 under halfrnd/ agrees with State A on every recorded metric.
- S3-X02-halfway: Every stage-3 case in X-02 under halfway/ agrees with State A on every recorded metric.
- S3-X02-hexinf: Every stage-3 case in X-02 under hexinf/ agrees with State A on every recorded metric.
- S3-X02-intbound: Every stage-3 case in X-02 under intbound/ agrees with State A on every recorded metric.
- S3-X02-longdig: Every stage-3 case in X-02 under longdig/ agrees with State A on every recorded metric.
- S3-X02-pow2: Every stage-3 case in X-02 under pow2/ agrees with State A on every recorded metric.
- S3-X02-rnd: Every stage-3 case in X-02 under rnd/ agrees with State A on every recorded metric.
- S3-X02-round: Every stage-3 case in X-02 under round/ agrees with State A on every recorded metric.
- S3-X02-subnormal: Every stage-3 case in X-02 under subnormal/ agrees with State A on every recorded metric.

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
- S3-X03-num: Every stage-3 case in X-03 under num/ agrees with State A on every recorded metric.
- S3-X03-str: Every stage-3 case in X-03 under str/ agrees with State A on every recorded metric.

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
- S3-X04-build_then_patch: Every stage-3 case in X-04 from build_then_patch.vm3 agrees with State A on every recorded metric.
- S3-X04-copy_roundtrip: Every stage-3 case in X-04 from copy_roundtrip.vm3 agrees with State A on every recorded metric.
- S3-X04-copy_scalars: Every stage-3 case in X-04 from copy_scalars.vm3 agrees with State A on every recorded metric.
- S3-X04-deep_nest: Every stage-3 case in X-04 from deep_nest.vm3 agrees with State A on every recorded metric.
- S3-X04-equality: Every stage-3 case in X-04 from equality.vm3 agrees with State A on every recorded metric.
- S3-X04-mutpatch_all_mutable: Every stage-3 case in X-04 from mutpatch_all_mutable.vm3 agrees with State A on every recorded metric.
- S3-X04-patch_fails_midway: Every stage-3 case in X-04 from patch_fails_midway.vm3 agrees with State A on every recorded metric.
- S3-X04-patch_on_empty: Every stage-3 case in X-04 from patch_on_empty.vm3 agrees with State A on every recorded metric.
- S3-X04-patch_op_names: Every stage-3 case in X-04 from patch_op_names.vm3 agrees with State A on every recorded metric.
- S3-X04-ptr_array_tokens: Every stage-3 case in X-04 from ptr_array_tokens.vm3 agrees with State A on every recorded metric.
- S3-X04-ptr_ctx_after_mutation: Every stage-3 case in X-04 from ptr_ctx_after_mutation.vm3 agrees with State A on every recorded metric.
- S3-X04-ptr_escapes: Every stage-3 case in X-04 from ptr_escapes.vm3 agrees with State A on every recorded metric.
- S3-X04-ptr_then_mutate: Every stage-3 case in X-04 from ptr_then_mutate.vm3 agrees with State A on every recorded metric.
- S3-X04-fuzz: Every stage-3 case in X-04 from the fuzz_NNN.vm3 scripts agrees with State A on every recorded metric.

## S-16

Action: Honour the allocator contract: the three callbacks, the context pointer, the old size handed to realloc, and the order and size of every request, including the null, default, pool and dynamic allocators (src/alloc.rs).

Established: A caller tracing its allocator sees State A's exact request sequence, and a pool too small to serve it fails where State A fails.

Survives: Allocation-failure injection separates a port that allocates different sizes or in a different order even when its output is identical.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-12: Every recorded case of module B-07 (allocators) agrees with State A on every recorded metric.
- S3-X05-build_arr_grow: Every stage-3 case in X-05 from build_arr_grow.vm3 agrees with State A on every recorded metric.
- S3-X05-build_obj: Every stage-3 case in X-05 from build_obj.vm3 agrees with State A on every recorded metric.
- S3-X05-copy_cycle: Every stage-3 case in X-05 from copy_cycle.vm3 agrees with State A on every recorded metric.
- S3-X05-dupkey_copy: Every stage-3 case in X-05 from dupkey_copy.vm3 agrees with State A on every recorded metric.
- S3-X05-dyn_strings: Every stage-3 case in X-05 from dyn_strings.vm3 agrees with State A on every recorded metric.
- S3-X05-merge_patch: Every stage-3 case in X-05 from merge_patch.vm3 agrees with State A on every recorded metric.
- S3-X05-mutpatch_mutable: Every stage-3 case in X-05 from mutpatch_mutable.vm3 agrees with State A on every recorded metric.
- S3-X05-numbers: Every stage-3 case in X-05 from numbers.vm3 agrees with State A on every recorded metric.
- S3-X05-patch_add_copy: Every stage-3 case in X-05 from patch_add_copy.vm3 agrees with State A on every recorded metric.
- S3-X05-patch_fail_mid: Every stage-3 case in X-05 from patch_fail_mid.vm3 agrees with State A on every recorded metric.
- S3-X05-ptr_create_parent: Every stage-3 case in X-05 from ptr_create_parent.vm3 agrees with State A on every recorded metric.
- S3-X05-ptr_mod_seq: Every stage-3 case in X-05 from ptr_mod_seq.vm3 agrees with State A on every recorded metric.
- S3-X05-read_escapes: Every stage-3 case in X-05 from read_escapes.vm3 agrees with State A on every recorded metric.
- S3-X05-read_write: Every stage-3 case in X-05 from read_write.vm3 agrees with State A on every recorded metric.
- S3-X05-strpool: Every stage-3 case in X-05 from strpool.vm3 agrees with State A on every recorded metric.
- S3-X05-valpool: Every stage-3 case in X-05 from valpool.vm3 agrees with State A on every recorded metric.
- S3-X05-write_big: Every stage-3 case in X-05 from write_big.vm3 agrees with State A on every recorded metric.
- S3-X05-fuzz: Every stage-3 case in X-05 from the fuzz_NNN.vm3 scripts agrees with State A on every recorded metric.

## S-17

Action: Keep file input and output on the same libc stdio calls, with the same error mapping (src/reader.rs, src/writer.rs, src/ffi.rs).

Established: Reading and writing files reports State A's codes and leaves the same bytes on disk.

Survives: Buffering the whole file where State A streams it changes the failure point on a short write.

Deliverable: D-06 (the shipped library and its installed tree)

Satisfies:

- T-15: Every recorded case of module B-10 (files) agrees with State A on every recorded metric.

## S-18

Action: Replace CMakeLists.txt: project(... LANGUAGES NONE) so no compiled language is probed, cargo driven through add_custom_command with the source directory as its working directory, the SONAME set by a link argument in .cargo/config.toml, two hand-written package-config templates in place of the export set CMake would have generated, and every build option State A compiles into the library still declared but refused with FATAL_ERROR naming it when a caller takes it off its default.

Established: The unchanged CMake commands configure, build and install both library kinds with no C compiler present, producing State A's install tree and package configuration, and every build option State A compiles into the library is refused rather than accepted and ignored.

Survives: Leaving LANGUAGES C in place makes the configure step itself ask for a compiler, which the graded build refuses. Carrying State A's option declarations forward and warning instead of refusing would configure, build and install a library the option does not apply to.

Deliverable: D-06 (the shipped library and its installed tree)

Red line: R-01 (any compile of a C, C++, Objective-C or assembly translation unit by the graded build)

Satisfies:

- T-03: The shared library's DT_SONAME equals State A's.
- T-04: The install prefix holds exactly State A's installed paths: the header, the libraries and their links, the CMake export set and the pkg-config file.
- T-16: Every recorded case of module B-12 (consumers) agrees with State A on every recorded metric.
- T-23: Under each of the twelve build options State A compiles into the library, configuring the submission either exits non-zero or yields a shared library exporting State A's own symbol set for that option.
- T-24: For each of the two depth-limit build options State A compiles into the library, configuring the submission with a positive limit either exits non-zero or yields an installed library that accepts and refuses exactly the nesting depths State A accepts and refuses under that same limit (module B-13).
- P-01: The graded build attempts to compile a C, C++, Objective-C or assembly translation unit, or the submitted tree carries assembly handed to the assembler inside rustc, a compiler's assembly output under any name, or an executable.

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

Action: Write the Rust so a reviewer reading it finds none of the ten prohibited acts and no transformed copy of the C: the caller's yyjson_alc for every document, value, string, write buffer and pool; no write to stdout or stderr; no .unwrap, .expect, panic!, unreachable!, todo! or unimplemented! on a shipped path; #[repr(C)] on every type the header exposes; no mutable global and no thread_local!; the original's failure value rather than an abort when memory runs out; the reader and the writer walking nesting with a loop and an explicit stack; the original's own byte handling rather than Rust's UTF-8 machinery; no environment variable and no file the original does not open; and the port's own digit generation rather than core::fmt.

Established: The shipped library's Rust is the port's own code, holds the original's allocator, failure and byte handling contracts, and carries no transformed copy of the C implementation.

Survives: A port that passes every recorded comparison can still allocate from Rust's global heap, abort on a failed allocation, recurse on nesting or carry the C through a build-time translation; none of those is visible in a record of return values and output bytes.

Deliverable: D-07 (the Rust source of the shipped library, as the judged review reads it)

Satisfies no graded item: D-07 is graded by the judged layer (Q-01 to Q-10) and by the judged gate (G-01), neither of which carries a compiled criterion.

## How the reward is shaped

The reward is a product of two gates and a weighted mean of two layers:

    R = [every stage I and stage II item holds] x [the judged gate holds]
          x (0.8 x D + 0.2 x Q)

Stage I and Stage II are one set here rather than two, because every item in either of
them costs the run rather than a share of it: there is no group among them whose failure
costs less than the whole score, so the two indicators the benchmark writes separately
are the same indicator for this task.

D is the weighted mean over all 84 compiled items at their own weights, which
sum to 300. Q is the weighted mean over the 10 judged items, whose weights sum
to 30. The outer table splits `test_output` and `judge` 4 to 1, which is the
0.8 and 0.2 above read as 4/5 and 1/5. Nothing is scaled anywhere: every weight
below is the weight the reward table carries.

Stage I and Stage II are the 24 compiled items with no stage, each at weight 5. They
are the compiled veto tests/.veto/veto.toml declares: one set, each item a group of its own,
k = 1. One of them false sends the run down the veto route, which scores a zero table where
every item keeps its identifier and its weight and scores zero, names each failed item in a
criterion of weight 0, and calls no judge at all.

Stage III is the 60 differential families, each at weight 3, so they carry
180 of the 300 compiled weight: three fifths of D exactly. A family costs a share of
the score and never the run.

### Stage I and Stage II

| id | weight | criterion |
|---|---|---|
| T-02 | 5 | The shipped shared library exports exactly State A's 52 dynamic symbols. |
| T-03 | 5 | The shared library's DT_SONAME equals State A's. |
| T-04 | 5 | The install prefix holds exactly State A's installed paths: the header, the libraries and their links, the CMake export set and the pkg-config file. |
| T-05 | 5 | Every DT_NEEDED entry of the shared library is in State A's set. |
| T-06 | 5 | Every recorded case of module B-01 (reader) agrees with State A on every recorded metric. |
| T-07 | 5 | Every recorded case of module B-02 (errors) agrees with State A on every recorded metric. |
| T-08 | 5 | Every recorded case of module B-03 (numbers) agrees with State A on every recorded metric. |
| T-09 | 5 | Every recorded case of module B-04 (writer) agrees with State A on every recorded metric. |
| T-10 | 5 | Every recorded case of module B-05 (mutable documents) agrees with State A on every recorded metric. |
| T-11 | 5 | Every recorded case of module B-06 (pointer and patch) agrees with State A on every recorded metric. |
| T-12 | 5 | Every recorded case of module B-07 (allocators) agrees with State A on every recorded metric. |
| T-13 | 5 | Every recorded case of module B-08 (incremental reader) agrees with State A on every recorded metric. |
| T-14 | 5 | Every recorded case of module B-09 (header interop) agrees with State A on every recorded metric. |
| T-15 | 5 | Every recorded case of module B-10 (files) agrees with State A on every recorded metric. |
| T-16 | 5 | Every recorded case of module B-12 (consumers) agrees with State A on every recorded metric. |
| T-17 | 5 | Reconfiguring, a no-op rebuild and reinstalling leave the installed tree byte-identical (module B-11). |
| T-23 | 5 | Under each of the twelve build options State A compiles into the library, configuring the submission either exits non-zero or yields a shared library exporting State A's own symbol set for that option. |
| T-24 | 5 | For each of the two depth-limit build options State A compiles into the library, configuring the submission with a positive limit either exits non-zero or yields an installed library that accepts and refuses exactly the nesting depths State A accepts and refuses under that same limit (module B-13). |
| P-01 | 5 | The graded build attempts to compile a C, C++, Objective-C or assembly translation unit, or the submitted tree carries assembly handed to the assembler inside rustc, a compiler's assembly output under any name, or an executable. |
| P-02 | 5 | The graded build links an object, archive or shared library it did not build from the submitted source. |
| P-03 | 5 | The shipped library imports dlopen, dlmopen, dlsym, fork, vfork, execve, posix_spawn, system or popen. |
| P-04 | 5 | The submitted src/yyjson.h differs from State A's by at least one byte. |
| P-05 | 5 | The graded build compiles a crate from a registry, a git source or a vendored third-party copy. |
| P-06 | 5 | src/yyjson.c or another State A translation unit remains in the submitted tree, or the tree carries at least a quarter of State A's implementation as verbatim spans under any name. |

### Stage III, the differential families

Each family is a group of cases of one stage-3 harness module. The module still runs once
through the harness; a family's verdict is the record lines whose case names one of its own
cases, compared in order against the frozen archive. A record that stops early fails the
families whose lines never arrived and leaves the earlier ones intact.

| id | weight | criterion |
|---|---|---|
| S3-X01-bom | 3 | Every stage-3 case in X-01 under bom/ agrees with State A on every recorded metric. |
| S3-X01-comma | 3 | Every stage-3 case in X-01 under comma/ agrees with State A on every recorded metric. |
| S3-X01-comment | 3 | Every stage-3 case in X-01 under comment/ agrees with State A on every recorded metric. |
| S3-X01-errpos | 3 | Every stage-3 case in X-01 under errpos/ agrees with State A on every recorded metric. |
| S3-X01-esc | 3 | Every stage-3 case in X-01 under esc/ agrees with State A on every recorded metric. |
| S3-X01-esckey | 3 | Every stage-3 case in X-01 under esckey/ agrees with State A on every recorded metric. |
| S3-X01-json5 | 3 | Every stage-3 case in X-01 under json5/ agrees with State A on every recorded metric. |
| S3-X01-trunc | 3 | Every stage-3 case in X-01 under trunc/ agrees with State A on every recorded metric. |
| S3-X01-utf8 | 3 | Every stage-3 case in X-01 under utf8/ agrees with State A on every recorded metric. |
| S3-X01-ws | 3 | Every stage-3 case in X-01 under ws/ agrees with State A on every recorded metric. |
| S3-X01-trunc23 | 3 | Every stage-3 case in X-01 under trunc2/ and trunc3/ agrees with State A on every recorded metric. |
| S3-X02-d17 | 3 | Every stage-3 case in X-02 under d17/ agrees with State A on every recorded metric. |
| S3-X02-d768 | 3 | Every stage-3 case in X-02 under d768/ agrees with State A on every recorded metric. |
| S3-X02-diyfp | 3 | Every stage-3 case in X-02 under diyfp/ agrees with State A on every recorded metric. |
| S3-X02-e21 | 3 | Every stage-3 case in X-02 under e21/ agrees with State A on every recorded metric. |
| S3-X02-expext | 3 | Every stage-3 case in X-02 under expext/ agrees with State A on every recorded metric. |
| S3-X02-fuzz | 3 | Every stage-3 case in X-02 under fuzz/ agrees with State A on every recorded metric. |
| S3-X02-halfrnd | 3 | Every stage-3 case in X-02 under halfrnd/ agrees with State A on every recorded metric. |
| S3-X02-halfway | 3 | Every stage-3 case in X-02 under halfway/ agrees with State A on every recorded metric. |
| S3-X02-hexinf | 3 | Every stage-3 case in X-02 under hexinf/ agrees with State A on every recorded metric. |
| S3-X02-intbound | 3 | Every stage-3 case in X-02 under intbound/ agrees with State A on every recorded metric. |
| S3-X02-longdig | 3 | Every stage-3 case in X-02 under longdig/ agrees with State A on every recorded metric. |
| S3-X02-pow2 | 3 | Every stage-3 case in X-02 under pow2/ agrees with State A on every recorded metric. |
| S3-X02-rnd | 3 | Every stage-3 case in X-02 under rnd/ agrees with State A on every recorded metric. |
| S3-X02-round | 3 | Every stage-3 case in X-02 under round/ agrees with State A on every recorded metric. |
| S3-X02-subnormal | 3 | Every stage-3 case in X-02 under subnormal/ agrees with State A on every recorded metric. |
| S3-X03-num | 3 | Every stage-3 case in X-03 under num/ agrees with State A on every recorded metric. |
| S3-X03-str | 3 | Every stage-3 case in X-03 under str/ agrees with State A on every recorded metric. |
| S3-X04-build_then_patch | 3 | Every stage-3 case in X-04 from build_then_patch.vm3 agrees with State A on every recorded metric. |
| S3-X04-copy_roundtrip | 3 | Every stage-3 case in X-04 from copy_roundtrip.vm3 agrees with State A on every recorded metric. |
| S3-X04-copy_scalars | 3 | Every stage-3 case in X-04 from copy_scalars.vm3 agrees with State A on every recorded metric. |
| S3-X04-deep_nest | 3 | Every stage-3 case in X-04 from deep_nest.vm3 agrees with State A on every recorded metric. |
| S3-X04-equality | 3 | Every stage-3 case in X-04 from equality.vm3 agrees with State A on every recorded metric. |
| S3-X04-mutpatch_all_mutable | 3 | Every stage-3 case in X-04 from mutpatch_all_mutable.vm3 agrees with State A on every recorded metric. |
| S3-X04-patch_fails_midway | 3 | Every stage-3 case in X-04 from patch_fails_midway.vm3 agrees with State A on every recorded metric. |
| S3-X04-patch_on_empty | 3 | Every stage-3 case in X-04 from patch_on_empty.vm3 agrees with State A on every recorded metric. |
| S3-X04-patch_op_names | 3 | Every stage-3 case in X-04 from patch_op_names.vm3 agrees with State A on every recorded metric. |
| S3-X04-ptr_array_tokens | 3 | Every stage-3 case in X-04 from ptr_array_tokens.vm3 agrees with State A on every recorded metric. |
| S3-X04-ptr_ctx_after_mutation | 3 | Every stage-3 case in X-04 from ptr_ctx_after_mutation.vm3 agrees with State A on every recorded metric. |
| S3-X04-ptr_escapes | 3 | Every stage-3 case in X-04 from ptr_escapes.vm3 agrees with State A on every recorded metric. |
| S3-X04-ptr_then_mutate | 3 | Every stage-3 case in X-04 from ptr_then_mutate.vm3 agrees with State A on every recorded metric. |
| S3-X04-fuzz | 3 | Every stage-3 case in X-04 from the fuzz_NNN.vm3 scripts agrees with State A on every recorded metric. |
| S3-X05-build_arr_grow | 3 | Every stage-3 case in X-05 from build_arr_grow.vm3 agrees with State A on every recorded metric. |
| S3-X05-build_obj | 3 | Every stage-3 case in X-05 from build_obj.vm3 agrees with State A on every recorded metric. |
| S3-X05-copy_cycle | 3 | Every stage-3 case in X-05 from copy_cycle.vm3 agrees with State A on every recorded metric. |
| S3-X05-dupkey_copy | 3 | Every stage-3 case in X-05 from dupkey_copy.vm3 agrees with State A on every recorded metric. |
| S3-X05-dyn_strings | 3 | Every stage-3 case in X-05 from dyn_strings.vm3 agrees with State A on every recorded metric. |
| S3-X05-merge_patch | 3 | Every stage-3 case in X-05 from merge_patch.vm3 agrees with State A on every recorded metric. |
| S3-X05-mutpatch_mutable | 3 | Every stage-3 case in X-05 from mutpatch_mutable.vm3 agrees with State A on every recorded metric. |
| S3-X05-numbers | 3 | Every stage-3 case in X-05 from numbers.vm3 agrees with State A on every recorded metric. |
| S3-X05-patch_add_copy | 3 | Every stage-3 case in X-05 from patch_add_copy.vm3 agrees with State A on every recorded metric. |
| S3-X05-patch_fail_mid | 3 | Every stage-3 case in X-05 from patch_fail_mid.vm3 agrees with State A on every recorded metric. |
| S3-X05-ptr_create_parent | 3 | Every stage-3 case in X-05 from ptr_create_parent.vm3 agrees with State A on every recorded metric. |
| S3-X05-ptr_mod_seq | 3 | Every stage-3 case in X-05 from ptr_mod_seq.vm3 agrees with State A on every recorded metric. |
| S3-X05-read_escapes | 3 | Every stage-3 case in X-05 from read_escapes.vm3 agrees with State A on every recorded metric. |
| S3-X05-read_write | 3 | Every stage-3 case in X-05 from read_write.vm3 agrees with State A on every recorded metric. |
| S3-X05-strpool | 3 | Every stage-3 case in X-05 from strpool.vm3 agrees with State A on every recorded metric. |
| S3-X05-valpool | 3 | Every stage-3 case in X-05 from valpool.vm3 agrees with State A on every recorded metric. |
| S3-X05-write_big | 3 | Every stage-3 case in X-05 from write_big.vm3 agrees with State A on every recorded metric. |
| S3-X05-fuzz | 3 | Every stage-3 case in X-05 from the fuzz_NNN.vm3 scripts agrees with State A on every recorded metric. |

### The judged layer

Ten prohibited acts, read by one judge call over the staged judge packet. Each is binary
and negated: a judge that answers yes has found the act, and the item scores zero.

| id | weight | criterion |
|---|---|---|
| Q-01 | 5 | The library takes memory for the documents, values, strings, write buffers or pools it builds from Rust's global allocator (Box, Vec, String, vec!, format!, to_string, to_owned, alloc::alloc or a #[global_allocator]) instead of the allocator in effect for the call. |
| Q-02 | 1 | The library writes to the process's standard output or standard error (println!, eprintln!, print!, eprint!, dbg!, std::io::stdout or std::io::stderr). |
| Q-03 | 5 | Code compiled into the shipped library contains an explicit panic path: .unwrap(), .expect(...), panic!, unreachable!, todo! or unimplemented!. Compile-time const assertions, unwrap_unchecked and code under #[cfg(test)] do not count. |
| Q-04 | 5 | A Rust type standing for a type the header exposes (yyjson_val, yyjson_mut_val, yyjson_doc, yyjson_mut_doc, yyjson_alc, an iterator or an error struct) is declared without #[repr(C)] or #[repr(transparent)], so its layout is not the one the header's inline functions assume. |
| Q-05 | 3 | The port keeps mutable global state the original does not have: a static mut, a static holding an Atomic, Mutex, RwLock, OnceCell, OnceLock, RefCell or UnsafeCell, or a thread_local!. |
| Q-06 | 3 | When memory cannot be obtained, the library ends the process (handle_alloc_error, abort, exit or a panic) instead of returning the failure value the original returns. The body of the #[panic_handler] does not count. |
| Q-07 | 3 | The reader or the writer walks nested arrays and objects by recursion (a function that calls itself, directly or through another, once per nesting level), where the original walks them with a loop and an explicit stack. |
| Q-08 | 3 | The reader validates or decodes input bytes through Rust's UTF-8 machinery (core::str::from_utf8, from_utf8_unchecked, str::from_utf8_mut, or iterating .chars()) instead of handling the bytes as the original does. |
| Q-09 | 1 | Library behaviour depends on the process environment: it reads environment variables (std::env, getenv) or opens files the original's code does not open. |
| Q-10 | 1 | Doubles are written through Rust's formatting machinery (core::fmt Display or Debug, {} or {:?}, write! or format!) instead of the port's own digit generation. |

Every judged criterion, and the gate below, ends with the same clause:

> Answer yes only when you can cite a file path, a line number and a verbatim quote from the files that show this act; if you cannot cite all three, or the quoted text is not at that path and line, answer no.

### The judged gate

One criterion, scored on its own in /tests/.gate before anything else is paid. It is the
second gate of the product above: a tree that carries the original implementation in
another form and turns it into the library during the build scores zero whatever else it
did.

| id | weight | criterion |
|---|---|---|
| G-01 | 5 | The tree carries the original C implementation, or machine code built from it, in another form (encoded, compressed, split, reformatted, or translated by a tool during the build) and uses it to produce the library: (a) a build script (build.rs), a CMake step or a macro reads C source or a blob derived from it and emits Rust, object code or assembly during the build; (b) include_bytes!, include_str! or include! pulls in C source, a blob derived from it, or Rust generated during the build; or (c) a large embedded payload, such as a base64 or hex blob or a byte array of machine code, is executed or reinterpreted as code. Lookup tables transcribed into committed plain Rust ahead of time, a few lines of the C quoted in a comment, and an honest port written by hand are not this act. |

## The judge

Model anthropic/claude-opus-5-5, reasoning effort high, mode
batched, timeout 600 seconds, reading the 3 packet parts
/logs/verifier/judge-packet-1.txt, /logs/verifier/judge-packet-2.txt and /logs/verifier/judge-packet-3.txt, and
prompted by tests/judge-prompt.md. One call scores all 10 items: the packet is
about 830 kB of inlined source for the reference port and individual mode would send it
once per item. Batched mode reads every entry of `files` in that one call, in the order
the block lists them, which is the order the parts are filled. The gate uses the same
model at the same effort in individual mode over the same 3 parts,
prompted by tests/.gate/prompt.md, with its scoring aggregated
all-pass.

Source of those values: the operator's choice of 2026-11-05, made for the smol-data delivery and not yet bound at a Phase approve, is the source of the judge model anthropic/claude-opus-5-5, the batched mode, the high reasoning effort and the 600 second timeout this block carries; record 47 of 2026-10-30, which bound openai/gpt-6.1-sol in individual mode against a design note, and record 13, where the judged layer began, are history.

Why this mode: mode is batched because no judged item carries a files entry of its own: every one of them reads the same packet, so one call scores all ten and Reward Kit refuses per-criterion files in batched mode anyway. The packet is about 830 kB of inlined source for the reference port, which individual mode would send ten times over. files names three paths rather than one because Reward Kit replaces a judge input over rewardkit.judges._MAX_FILE_SIZE (1 MiB) with "[skipped: file too large]" and applies that limit to each entry of this list separately, so one packet file would cap the inlined evidence near 1 MB and a fault that sat only in a file past the cap could be cited by nobody; three parts of at most 1,000,000 bytes carry the 2,400,000-byte inline cap this task declares. Batched mode reads every entry of this list in one call, in this order, so the order here is the order the parts are filled. files names the staged parts rather than the submitted tree, because tests/verifier/stage_note.py unlinks all three paths and then writes the packet into them only for a submission whose graded build produced the shipped library and whose six prohibition items all hold. An ungraded tree leaves all three absent and the judge reads each as not found, so no act can be cited and every judged item scores its negated full credit on a tree that never reached the gate; the routing in tests/test.sh is what keeps that from being reachable, by sending an unstaged run to the zero route instead.

Why the gate's: one criterion, so individual and batched run the same single call; individual is the shape the gate route was built against. files names the same three packet parts as the judge block, in the same order, and for the same reason: Reward Kit applies its 1 MiB input limit to each entry separately, so a single part would cap the evidence the gate reads at about 950 kB. The gate is scored on its own in /tests/.gate and its outcome multiplies the rest: tests/test.sh grades it first, and only a gate that scores 1 reaches the table that pays anything.

## The judge packet

The judged layer reads the port, not a note about it. tests/verifier/stage_note.py builds
the packet at the same write-last moment it used to stage a note, and writes it to the
3 part files
/logs/verifier/judge-packet-1.txt, /logs/verifier/judge-packet-2.txt and /logs/verifier/judge-packet-3.txt:

- one block per collected file the submission added or changed against State A, in sorted
  path order, opening with `==> <path> (<n> lines) <==` and then every line of the file
  under its own 1-based number;
- then `==> tree listing <==`, carrying every file of the tree with its size, its state
  against State A, whether it was inlined, and a digest prefix for every file that was not.

Collected means the paths a build can turn into the shipped library: Rust sources, the
cargo manifests and lock file, build.rs, the cargo configuration, the CMake files and
templates, and scripts, anywhere outside target/, .git/ and build directories. There is no
per-file size limit: every collected text file the submission added or changed is read to
its last line, in chunks across the parts when one part cannot hold it. Only a collected
BINARY file is listed rather than inlined, because a judge can cite no line of one.

It is 3 files and not one because Reward Kit replaces a judge input over 1 MiB
with `[skipped: file too large]`, and applies that limit to each entry of a judge's `files`
list separately. Each part holds at most 1000000 bytes, so the 2400000 byte inline
cap this task declares reaches the judge instead of the 1 MiB one file would have capped it
at, and a fault that sits only in a late file can still be cited. How the parts are filled:

- every part opens with one header line naming its own number and all three part paths, so
  a judge holding one part knows how many there are and where the rest is;
- blocks go into part 1 until it is full, then part 2, then part 3, and the cursor never
  goes back. A file whose whole block fits the room left in the current part is written
  there in one piece under its `==> <path> (<n> lines) <==` header;
- any other file is SPLIT at a line boundary: as many whole lines as fit go into the
  current part under `==> <path> (lines <a>-<b> of <n>) <==`, and the rest continues in
  further chunks of that same form in the next part and the one after. A line is never
  split, and a line keeps the file's own 1-based number in whichever chunk carries it, so
  a citation of path and line means the same thing in every part;
- the tree listing goes at the end of the last part that received anything, and room for it
  is reserved out of every part so it fits wherever that turns out to be. It says, per file,
  `inlined`, `inlined in <k> chunks`, or why the file carried no lines;
- all 3 paths are always written. A part that received nothing carries one line
  saying it is an empty part, so an absent part always means the stager refused to stage;
- the 2400000 byte cap is the only thing that can leave source unread, and it is
  enforced as a REFUSAL rather than an omission. `packet.build` returns the paths whose
  last line reached no part, and a submission with any of them has NO part staged and is
  routed to zero naming them, because every judged item is a negated act and source the
  judge never saw would score as clean.

The packet is written only after the process sweep, only for a submission whose graded
build produced the shipped library and whose six prohibition items all hold, and every part
of it is withdrawn again if anything foreign is found alive before the verdicts are
published. `judged.staged` means all three parts were written: a judge handed two of three
reads `[not found]` for the rest. That is the note's old security model unchanged; only the
content it stages, and the number of files it stages into, are different.

## The routes

tests/test.sh runs the graded pipeline once, publishes its verdicts into the environment
and then asks the routes below in this order; the first that applies scores the run:

- zero, when the stager found no execution boundary on the host. The build was refused
  rather than run, so no verdict describes the port. /tests/.zero scores every compiled
  item and every judged item at its own weight and returns zero for all of them, and no
  judge is called. The stager's own route reason travels in the verdict JSON and is what
  the zero criteria name.
- the veto route, which grades the compiled items alone in a scratch copy, with no judge,
  and fires when one of the 24 stage I/II items tests/.veto/veto.toml declares is false.
  It then scores a zero table where every compiled and judged item keeps its identifier
  and its weight and scores zero, and a criterion of weight 0 names each failed item.
- zero, when the stager says zero for another reason: no shipped library, or a packet
  that could not carry the submission's whole collected source. Scored as the first.
- the gate route, when all 3 packet parts were staged AND the sweep converged AND
  nothing foreign is alive. It grades /tests/.gate first; a gate that scores 1 goes on to
  /tests with the judge, and a gate that does not is scored against /tests/.gate/zero.toml,
  whose `reward` is all-pass over a gate input that scored zero.
- zero again, when the packet was not staged in full or the tree did not go quiet. The gate
  cannot be read on a packet the submission may have rewritten, so the run fails closed and
  the zero criteria name that reason, which reaches them through the
  VERIFIER_ZERO_REASON variable.

graded-verdicts.json and ctrf.json carry the stager's own route, which never includes the
veto: a run the veto route scores still shows the stager's `pass` or `zero` there, and
its reward-details.json names the failed items in a criterion of weight 0.

## Routes that do not solve the task

- oracle: The reference migration exactly as solution/ ships it. Expected: 1.0: every compiled item true, the gate passes, and the judge finds none of the ten acts, so D is 1.0 and Q is 1.0.
- state-a: State A unchanged. Expected: 0.0: the configure step asks for a C compiler and the graded build refuses it, so no graded target is written and the run takes the veto route.
- empty: An empty submission. Expected: 0.0, by the same fail-closed path.
- c-wrapper: A Rust shim whose build compiles State A's C and links it in. Expected: 0.0: the compile is refused, P-01 is false, and a false Stage I/II item takes the veto route.
- header-edit: The reference migration with one byte changed in src/yyjson.h. Expected: 0.0: P-04 is a Stage I/II item, so the veto route takes the whole score rather than a share of it.
- partial: The reference migration with a defect planted in the writer, the mutable API, the allocator trace and one stage-3 family. Expected: 0.0: the writer, mutable and allocator defects are Stage I/II items and gate the run; a defect in the stage-3 family alone would instead cost 0.8 x 3/300 of D, with the failing family named.
- one-family: The reference migration with a defect planted in one stage-3 family and nowhere else. Expected: 0.992: every Stage I/II item holds, the gate passes, Q is 1.0, and D loses 3 of 300, so R = 0.8 x 297/300 + 0.2.
- prebuilt: The reference migration shipping a prebuilt archive the build links. Expected: 0.0: P-02 is false and the run takes the veto route.
- vendored: The reference migration resolving a vendored third-party crate. Expected: 0.0: P-05 is false and the run takes the veto route.
- embedded: The reference migration carrying State A's implementation as a non-C resource. Expected: 0.0: P-06 is false; and were the embedding transformed past P-06's verbatim-span relation, the judged gate G-01 would read it and take the score to zero on its own.
- fair-quotation: The reference migration quoting eighty-eight lines of State A's C in a comment in src/reader.rs. Expected: 1.0: G-01 exempts a few lines of the C quoted in a comment, and no compiled item reads comments.
- global-alloc: The reference migration with its document arena taken from Vec instead of the caller's yyjson_alc. Expected: Below 1.0 with Q-01 false. The allocator trace items would also move, so in practice T-12 and the X-05 families fail first and the run takes the veto route; Q-01 exists for the port that keeps the traced sequence and allocates its scratch beside it.
- panic-path: The reference migration with one .unwrap() left on a shipped path. Expected: 0.98: every compiled item still holds and the gate passes, so R = 0.8 + 0.2 x 25/30.
- recursive-reader: The reference migration with the reader walking nesting by recursion. Expected: 0.98: the corpus nests no deeper than the recursion survives, so D is 1.0 and Q loses 3 of 30.
- packet-withheld: The reference migration on a grading host where the tree does not go quiet after the sweep. Expected: 0.0: the gate cannot be judged on a packet the submission may have rewritten. A writer still alive when the criteria run fails every compiled item closed, so the veto route takes the run and names the stage I/II items; one that went quiet only after the stager gave up sends the run to the zero route, whose criteria name that reason.
- one-file-port: The reference migration collapsed into a single src/lib.rs larger than one packet part. Expected: 1.0: the one file is read whole, in as many chunks as the parts need, with each chunk headed by the lines of it that chunk carries, so every line of the port is citable and the submission is judged exactly as the sixteen-file reference is.
- packet-oversize: A migration whose collected text, once line-numbered, exceeds the 2400000 bytes the three packet parts hold. Expected: 0.0: packet.build reports the files whose last line reached no part, the stager stages NO part at all, and the run takes the zero route with a reason naming them. The judged items are never asked about source the judge was not shown, so a port cannot buy negated full credit by being too large to read.

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
- Eight of the one hundred and seven mutants survive the stage-3 corpus, and all eight were judged unclosable. Three are output-equivalent by construction. Two turn a bounds guard into an out-of-bounds read observable only for bytes outside the buffer. One needs an array of 10^18 elements. Two are reachable only within one chunk header of the maximum object size.
- Sixteen further one-fault mutations of the same rounding decisions in the numeric reader and writer survive because they are unobservable rather than unreached. Three are output-equivalent by construction. Four only move work between the accuracy window and bigcomp, and 1.5 million boundary candidates separated none of them. Two need a round-to-odd sticky word of exactly one, which lies outside both the 2^24 and the 2^53 significand spaces. Four are Schubfach u1 and w1 comparisons, and 4 million candidates separated none of them. One is the irregular threshold, exhaustively unobservable across all 2046 powers of two. One is the significand bound of fast path 1. One is the big-cut dot fixup.
- The verbatim-span relation behind P-06 does not detect a transformed embedding of State A's implementation, such as a base64, compressed or reformatted copy. The judged gate G-01 reads the packet for exactly that and is the remaining defence; a transformation no reader of the collected source can see is outside both.
- T-04 compares installed paths, kinds and link targets rather than the contents of the CMake export set; wrong contents are separated instead by the consumer module's find_package route.
- T-17 compares installed bytes after folding the install prefix string, because the pkg-config file and the CMake package configuration legitimately record their own prefix.
- P-03 reads the shared library's dynamic imports; the static archive's undefined symbols are not an import relation and are not read.
- The reference port is verified on linux/arm64 only. State A's ten behavioural record files are byte-identical on linux/amd64 and both images build for both platforms, but rustc cannot run under the authoring host's emulator, so the port itself was not exercised there. The frozen baseline is recorded per platform inside the verifier image, so an amd64-only difference would surface as an oracle control failure rather than as a wrong baseline.
- T-05 is a subset relation, so a shipped library declaring no dynamic dependency at all would satisfy it. No working submission can avoid the C runtime, so the relation is left as the contract binds it.
- T-23 reads an exported symbol set, and only six of the twelve options State A compiles into the library change that set: YYJSON_DISABLE_READER, _WRITER, _INCR_READER, _FILE, _UTILS and YYJSON_FREESTANDING, which remove 7, 17, 3, 10, 9 and 10 of State A's 52 exports. Under the other six, YYJSON_DISABLE_FAST_FP_CONV, _NON_STANDARD, _UTF8_VALIDATION, _UNALIGNED_MEMORY_ACCESS and the two depth limits, State A exports all 52, so a submission that accepts one of those and ignores it satisfies that option's half of the relation. Those six are separated only when the submission refuses them, and YYJSON_DISABLE_UNALIGNED_MEMORY_ACCESS changes no observable behaviour of State A at all, only whether a load goes through memcpy.
- The caller's buffer after an in-situ read and errno after a failing file call are not compared. Sixty in-situ cases and the errno and depth behaviour were measured byte-identical across the reference port and both collected agent ports, so the observables separate nothing, and reaching them would have changed the frozen record of an existing stage-2 module and so the right-hand side of an item already graded.
- The static archive carries rustc's compiler-builtins members, so it holds 406 members and 603 defined global text symbols against State A's single member with 52. Exactly 52 carry default visibility and 551 are hidden, so a final link still exports State A's set, and no compiled criterion reads archive membership.
- A stage-3 family's verdict reads the record lines whose case names one of its own cases. The HDR and END lines carry no case and belong to no family, so a module that agreed on every case and disagreed on its header or its trailer would fail no family; the library version HDR reports is held by T-02 and T-14 instead.
- The judged layer reads the collected source of the submitted tree, not the machine code. A fault the Rust does not state but a macro of the submission's own expands into is outside what a reader of the packet can cite, and no compiled item reads expanded Rust either.
- The judge packet inlines at most 2400000 bytes, spread over three part files of at most 1000000 bytes each, because Reward Kit does not read a judge input over 1 MiB at all and applies that limit to each named file separately. Every collected text file the submission added or changed is read whole, to its last line, in chunks across the parts when one part cannot hold it: there is no per-file size limit and a large file is never withheld. What the cap bounds is the whole submission, not any one file, and it bounds it by refusal rather than by omission -- a submission whose line-numbered collected source does not fit in the parts' 2400000 bytes has NO part staged and scores zero by the zero route, naming the files that did not fit. So there is no state in which a judge is asked a negated question about source it was not shown. The residue is only that such a submission scores the same zero as one that failed a Stage I/II item: the zero criteria's route reason and the veto route's named items are the only places the difference is recorded. A collected BINARY file is still listed rather than inlined, because a judge can cite no line of one, and that is not counted as source the packet failed to carry.
