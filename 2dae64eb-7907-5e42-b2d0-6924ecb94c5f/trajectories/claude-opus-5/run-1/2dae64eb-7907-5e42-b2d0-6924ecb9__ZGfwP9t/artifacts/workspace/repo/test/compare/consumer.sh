#!/bin/bash
# Check that an installed prefix can be consumed the three documented ways:
# pkg-config, find_package(yyjson) + yyjson::yyjson, and a bare -lyyjson.
#
# Usage: test/compare/consumer.sh <install-prefix> <shared|static>
set -u
PREFIX=$1
KIND=$2
WORK=${WORK:-/tmp/yyconsumer}
rm -rf "$WORK"
mkdir -p "$WORK"
cd "$WORK"

cat > main.c <<'EOF'
#include "yyjson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    const char *json = "{\"a\":[1,2.5,\"x\"],\"b\":null}";
    yyjson_doc *doc = yyjson_read(json, strlen(json), 0);
    char *out;
    yyjson_mut_doc *mdoc;
    yyjson_mut_val *v;
    if (!doc) return 1;
    out = yyjson_write(doc, YYJSON_WRITE_PRETTY, NULL);
    printf("version=%08X\n%s\n", yyjson_version(), out);
    free(out);
    mdoc = yyjson_doc_mut_copy(doc, NULL);
    v = yyjson_mut_doc_ptr_get(mdoc, "/a/1");
    printf("ptr=%.17g\n", yyjson_mut_get_real(v));
    out = yyjson_mut_write(mdoc, 0, NULL);
    printf("%s\n", out);
    free(out);
    yyjson_mut_doc_free(mdoc);
    yyjson_doc_free(doc);
    return 0;
}
EOF

cat > CMakeLists.txt <<'EOF'
cmake_minimum_required(VERSION 3.10)
project(consumer C)
find_package(yyjson REQUIRED)
add_executable(consumer main.c)
target_link_libraries(consumer PRIVATE yyjson::yyjson)
EOF

fail=0

echo "--- pkg-config ($KIND)"
flags=$(PKG_CONFIG_PATH=$PREFIX/lib/pkgconfig pkg-config --cflags --libs yyjson) || fail=1
gcc -o c1 main.c $flags || fail=1
LD_LIBRARY_PATH=$PREFIX/lib ./c1 > out1.txt || fail=1

echo "--- bare -lyyjson ($KIND)"
gcc -o c2 main.c -I "$PREFIX/include" -L "$PREFIX/lib" -lyyjson || fail=1
LD_LIBRARY_PATH=$PREFIX/lib ./c2 > out2.txt || fail=1

echo "--- find_package ($KIND)"
cmake -S . -B b -DCMAKE_PREFIX_PATH="$PREFIX" > /dev/null || fail=1
cmake --build b > /dev/null || fail=1
LD_LIBRARY_PATH=$PREFIX/lib ./b/consumer > out3.txt || fail=1

cmp out1.txt out2.txt || fail=1
cmp out1.txt out3.txt || fail=1
cat out1.txt

if [ "$KIND" = shared ]; then
    echo "--- SONAME / symbols"
    readelf -d "$PREFIX/lib/libyyjson.so.0.13.0" | grep -E 'SONAME|NEEDED'
    nm -D --defined-only "$PREFIX/lib/libyyjson.so.0.13.0" | awk '{print $3}' | sort \
        > syms.txt
    echo "exported: $(wc -l < syms.txt)"
fi

exit $fail
