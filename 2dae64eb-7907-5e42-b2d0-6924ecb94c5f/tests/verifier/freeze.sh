#!/bin/bash
# Freeze State A into the verifier image, at image build time, once.
#
# Everything a submission is later compared against is produced here by State A itself: its
# two install trees, its shared library's symbol table and dynamic section, its public
# header's digest, the six consumer link routes, and the 280,974 stage-2 plus 27,527 stage-3
# record lines its own library writes. The graded pipeline then re-runs only the right-hand
# side of each comparison against the submission, so no expected value is ever written by
# hand and no reference text travels in /tests.
#
# The harness objects are compiled here and shipped, never recompiled at verify time: the
# submission's library is linked against the same machine code State A's was, which is what
# makes a record difference attributable to the library rather than to the instrument.
#
# The records are ~150 MB of text and are stored xz-compressed; the pipeline compares
# digests and only decompresses to locate a divergence for the run log.
set -euo pipefail

CTX=${1:-/ctx}
OUT=/opt/verifier
WORK=/freeze
# State A is installed where the graded pipeline later installs the submission, so the link
# line that produces State A's frozen records and the one that produces the submission's are
# character-identical. The harness's pool records depend on the harness binary's layout, and
# an identical link line is the cheapest way to hold that constant.
SCRATCH=${VERIFIER_SCRATCH:-/tmp/verifier-run}

export LC_ALL=C LANG=C TZ=UTC SOURCE_DATE_EPOCH=1788891432

CFLAGS="-O2 -Wall -Wextra -std=c99 -D_POSIX_C_SOURCE=200809L -I$CTX/statea/src"
HARNESS_SOURCES="common flags vm main m_reader m_errors m_numbers m_writer m_mutable
                 m_ptrpatch m_alloc m_incr m_hdr m_files"
STAGE3_SHARED="common flags vm m_reader"
STAGE3_SOURCES="vm3 main3 x01_parse x02_numeric x03_writer x04_mutate x05_allocfail"

echo "== payload"
mkdir -p "$OUT"/{statea,shim,records/stage2,records/stage3,obj/stage2,obj/stage3,stage3}
cp "$CTX"/recorder.py "$CTX"/probe.py "$CTX"/stages.py "$CTX"/pipeline.py "$OUT"/
chmod 0755 "$OUT"/recorder.py
cp -a "$CTX"/corpus "$OUT"/corpus
cp -a "$CTX"/consumer "$OUT"/consumer
cp -a "$CTX"/stage3/corpus "$OUT"/stage3/corpus
cp "$CTX"/statea/src/yyjson.h "$OUT"/statea/yyjson.h
# The implementation stays beside the header so the P-06 relation can be recomputed from the
# original bytes at grading time. It is verifier-private and never reaches the agent, which
# starts from this very file anyway.
cp "$CTX"/statea/src/yyjson.c "$OUT"/statea/yyjson.c
# The recorder stands in for every compiler driver, preprocessor and assembler this image
# ships, not just the canonical names: `gcc-14`, `c99-gcc` and the target-triple spellings are
# all real drivers, and a build that reached one of them unshimmed could compile C and still
# be graded. The list is computed from the image rather than fixed, so it follows the platform
# (aarch64-linux-gnu-* here, x86_64-linux-gnu-* on amd64). `cargo` is shimmed to be recorded,
# never refused.
for tool in $(
    ls /usr/bin /usr/local/bin 2>/dev/null \
        | grep -E '^((|[a-z0-9_.-]+-)(cc|gcc|g\+\+|c\+\+|cpp|as)|c[0-9]+-gcc|(gcc|g\+\+|cpp)-[0-9.]+|clang(\+\+)?(-[0-9.]+)?|tcc|icc|icx)$' \
        | sort -u
) cc gcc c++ g++ cpp as cargo; do
    ln -sf ../recorder.py "$OUT/shim/$tool"
done
echo "   shimmed: $(ls "$OUT/shim" | tr '\n' ' ')"

echo "== State A install trees"
mkdir -p "$WORK/repo" "$SCRATCH"
cp -a "$CTX"/statea/. "$WORK/repo/"
for kind in shared static; do
    if [ "$kind" = shared ]; then flag=ON; else flag=OFF; fi
    rm -rf "$SCRATCH/prefix-$kind"
    cmake -S "$WORK/repo" -B "$WORK/b-$kind" -DBUILD_SHARED_LIBS=$flag \
          -DCMAKE_INSTALL_PREFIX="$SCRATCH/prefix-$kind" > "$WORK/cfg-$kind.log"
    cmake --build "$WORK/b-$kind" --parallel > "$WORK/build-$kind.log"
    cmake --install "$WORK/b-$kind" > "$WORK/install-$kind.log"
done

echo "== stage-2 harness objects"
: > "$OUT/obj/stage2/link-order.txt"
for unit in $HARNESS_SOURCES; do
    gcc $CFLAGS -c "$CTX/harness/$unit.c" -o "$OUT/obj/stage2/$unit.o"
    echo "$unit.o" >> "$OUT/obj/stage2/link-order.txt"
done

echo "== stage-3 harness objects"
: > "$OUT/obj/stage3/link-order.txt"
for unit in $STAGE3_SHARED; do
    gcc $CFLAGS -I"$CTX/harness" -c "$CTX/harness/$unit.c" -o "$OUT/obj/stage3/$unit.o"
    echo "$unit.o" >> "$OUT/obj/stage3/link-order.txt"
done
for unit in $STAGE3_SOURCES; do
    gcc $CFLAGS -I"$CTX/harness" -c "$CTX/stage3/src/$unit.c" -o "$OUT/obj/stage3/$unit.o"
    echo "$unit.o" >> "$OUT/obj/stage3/link-order.txt"
done

echo "== stage-3 corpus integrity"
python3 "$CTX/freeze.py" corpus "$CTX/stage3/manifest.jsonl" "$OUT/stage3/corpus"

echo "== State A records"
LIB="$SCRATCH/prefix-shared/lib"
for stage in 2 3; do
    objects=""
    while read -r object; do objects="$objects $OUT/obj/stage$stage/$object"; done \
        < "$OUT/obj/stage$stage/link-order.txt"
    # shellcheck disable=SC2086
    gcc $objects -o "$WORK/harness$stage" -L"$LIB" -lyyjson -lm \
        -Wl,-rpath,"$LIB" -Wl,--disable-new-dtags
done
mkdir -p "$WORK/out2" "$WORK/out3" "$WORK/tmp"
for module in B-01 B-02 B-03 B-04 B-05 B-06 B-07 B-08 B-09 B-10; do
    rm -rf "$WORK/tmp/$module" && mkdir -p "$WORK/tmp/$module"
    "$WORK/harness2" "$module" "$OUT/corpus" "$WORK/out2/$module.txt" "$WORK/tmp/$module"
done
for family in X-01 X-02 X-03 X-04 X-05; do
    rm -rf "$WORK/tmp/$family" && mkdir -p "$WORK/tmp/$family"
    "$WORK/harness3" "$family" "$OUT/stage3/corpus" "$WORK/out3/$family.txt" "$WORK/tmp/$family"
done
rm -rf "$WORK/tmp"

# The records the components were verified against, before they were put in an image. A
# difference here means the image's State A is not the State A the harness was proved on.
echo "== frozen record digests"
( cd "$WORK/out2" && sha256sum --check --strict "$CTX/frozen-stage2.sha256" )
( cd "$WORK/out3" && sha256sum --check --strict "$CTX/frozen-stage3.sha256" )

echo "== State A manifest"
python3 "$CTX/freeze.py" manifest "$WORK" "$SCRATCH"

echo "== compress records"
for module in "$WORK"/out2/*.txt; do
    xz -3 -T0 -c "$module" > "$OUT/records/stage2/$(basename "$module").xz"
done
for family in "$WORK"/out3/*.txt; do
    xz -3 -T0 -c "$family" > "$OUT/records/stage3/$(basename "$family").xz"
done

rm -rf "$WORK" "$SCRATCH"
find "$OUT" -name __pycache__ -prune -exec rm -rf {} +
find "$OUT" -type d -exec chmod 0755 {} +
echo "== frozen"
du -sh "$OUT"
