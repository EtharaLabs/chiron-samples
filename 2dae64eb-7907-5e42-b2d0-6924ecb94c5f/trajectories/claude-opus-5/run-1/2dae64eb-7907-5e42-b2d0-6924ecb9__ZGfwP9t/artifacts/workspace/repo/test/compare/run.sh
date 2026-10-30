#!/bin/bash
# Differential test: build each driver below twice -- once against the original
# C implementation and once against the Rust library -- and compare the output
# byte for byte.
#
# The original `src/yyjson.c` is no longer part of the tree; the script takes
# it from the first commit of this repository (`git show 40440d6:src/yyjson.c`,
# i.e. the "yyjson 0.13.0" import), or from $YYJSON_C if that is set.
#
# The drivers are kept here as heredocs rather than as `.c` files so that the
# repository contains no C translation unit at all.
#
# Usage:  test/compare/run.sh [extra-input-dir ...]
# Env:    WORK=<dir>      scratch directory (default /tmp/yyjson-compare)
#         YYJSON_C=<file> the original C implementation
set -u

cd "$(dirname "$0")/../.."
ROOT=$(pwd)
WORK=${WORK:-/tmp/yyjson-compare}
YYJSON_C=${YYJSON_C:-$WORK/yyjson-orig.c}
CC=${CC:-cc}
mkdir -p "$WORK"

if [ ! -f "$YYJSON_C" ]; then
    git -C "$ROOT" show 40440d6:src/yyjson.c > "$YYJSON_C" || {
        echo "the original src/yyjson.c is not available; set YYJSON_C" >&2
        exit 1
    }
fi

( cd rust && cargo build --release --offline >/dev/null ) || exit 1
RUST_LIB=$ROOT/rust/target/release/libyyjson.a

cat > "$WORK/dump.c" <<'YYJSON_DRIVER_EOF'
/* A differential test driver: reads JSON from stdin (or a literal given on the
   command line), parses it with a set of flag combinations and dumps every
   observable result. The output of this program, linked against the original C
   library and against the Rust library, must be byte identical. */

#include "yyjson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void dump_val(yyjson_val *val, int depth);

static void indent(int depth) {
    int i;
    for (i = 0; i < depth; i++) fputs("  ", stdout);
}

static void dump_raw_bytes(const char *s, size_t len) {
    size_t i;
    if (!s) { printf("<nullstr>"); return; }
    for (i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c >= 0x20 && c < 0x7F && c != '\\') putchar(c);
        else printf("\\x%02X", c);
    }
}

static void dump_val(yyjson_val *val, int depth) {
    indent(depth);
    if (!val) { printf("<null ptr>\n"); return; }
    printf("tag=%u/%u len=%zu ", (unsigned)yyjson_get_type(val),
           (unsigned)yyjson_get_subtype(val), yyjson_get_len(val));
    switch (yyjson_get_type(val)) {
        case YYJSON_TYPE_NULL: printf("null\n"); break;
        case YYJSON_TYPE_BOOL: printf("bool %d\n", (int)yyjson_get_bool(val)); break;
        case YYJSON_TYPE_NUM:
            switch (yyjson_get_subtype(val)) {
                case YYJSON_SUBTYPE_UINT:
                    printf("uint %llu\n", (unsigned long long)yyjson_get_uint(val));
                    break;
                case YYJSON_SUBTYPE_SINT:
                    printf("sint %lld\n", (long long)yyjson_get_sint(val));
                    break;
                default: {
                    double d = yyjson_get_real(val);
                    unsigned long long bits;
                    memcpy(&bits, &d, 8);
                    printf("real %.17g bits=%016llX\n", d, bits);
                    break;
                }
            }
            break;
        case YYJSON_TYPE_STR:
            printf("str \"");
            dump_raw_bytes(yyjson_get_str(val), yyjson_get_len(val));
            printf("\"\n");
            break;
        case YYJSON_TYPE_RAW:
            printf("raw \"");
            dump_raw_bytes(yyjson_get_raw(val), yyjson_get_len(val));
            printf("\"\n");
            break;
        case YYJSON_TYPE_ARR: {
            size_t idx, max;
            yyjson_val *v;
            printf("arr\n");
            yyjson_arr_foreach(val, idx, max, v) dump_val(v, depth + 1);
            break;
        }
        case YYJSON_TYPE_OBJ: {
            size_t idx, max;
            yyjson_val *k, *v;
            printf("obj\n");
            yyjson_obj_foreach(val, idx, max, k, v) {
                indent(depth + 1);
                printf("key \"");
                dump_raw_bytes(yyjson_get_str(k), yyjson_get_len(k));
                printf("\"\n");
                dump_val(v, depth + 2);
            }
            break;
        }
        default: printf("none\n"); break;
    }
}

typedef struct { const char *name; yyjson_read_flag flg; } flag_case;

static const flag_case cases[] = {
    { "default",   0 },
    { "insitu",    YYJSON_READ_INSITU },
    { "stop",      YYJSON_READ_STOP_WHEN_DONE },
    { "comma",     YYJSON_READ_ALLOW_TRAILING_COMMAS },
    { "comment",   YYJSON_READ_ALLOW_COMMENTS },
    { "infnan",    YYJSON_READ_ALLOW_INF_AND_NAN },
    { "numraw",    YYJSON_READ_NUMBER_AS_RAW },
    { "invuni",    YYJSON_READ_ALLOW_INVALID_UNICODE },
    { "bignum",    YYJSON_READ_BIGNUM_AS_RAW },
    { "bom",       YYJSON_READ_ALLOW_BOM },
    { "json5",     YYJSON_READ_JSON5 },
    { "all",       YYJSON_READ_JSON5 | YYJSON_READ_ALLOW_INVALID_UNICODE |
                   YYJSON_READ_ALLOW_BOM | YYJSON_READ_BIGNUM_AS_RAW },
};

int main(int argc, char **argv) {
    char *dat;
    size_t len;
    size_t i;
    static char buf[1 << 22];

    if (argc > 1) {
        dat = argv[1];
        len = strlen(dat);
    } else {
        len = fread(buf, 1, sizeof(buf), stdin);
        dat = buf;
    }

    printf("version=%08X\n", yyjson_version());
    printf("input len=%zu\n", len);

    for (i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        yyjson_read_err err;
        yyjson_doc *doc;
        char *copy = (char *)malloc(len + 1 + YYJSON_PADDING_SIZE);
        memset(copy, 0, len + 1 + YYJSON_PADDING_SIZE);
        memcpy(copy, dat, len);
        copy[len] = 0;
        memset(&err, 0, sizeof(err));
        doc = yyjson_read_opts(copy, len, cases[i].flg, NULL, &err);
        printf("== case %s\n", cases[i].name);
        printf("   err code=%u msg=%s pos=%zu\n", (unsigned)err.code,
               err.msg ? err.msg : "(null)", err.pos);
        if (doc) {
            size_t line = 0, col = 0, chr = 0;
            printf("   read=%zu vals=%zu\n", yyjson_doc_get_read_size(doc),
                   yyjson_doc_get_val_count(doc));
            dump_val(yyjson_doc_get_root(doc), 1);
            yyjson_locate_pos(dat, len, len / 2, &line, &col, &chr);
            printf("   locate(%zu) line=%zu col=%zu chr=%zu\n", len / 2,
                   line, col, chr);
            yyjson_doc_free(doc);
        } else {
            size_t line = 0, col = 0, chr = 0;
            int ok = yyjson_locate_pos(dat, len, err.pos, &line, &col, &chr);
            printf("   locate ok=%d line=%zu col=%zu chr=%zu\n", ok,
                   line, col, chr);
        }
        free(copy);
    }
    return 0;
}
YYJSON_DRIVER_EOF

cat > "$WORK/rw.c" <<'YYJSON_DRIVER_EOF'
/* Differential test driver for the writer: read the input, then serialize it
   with every interesting write-flag combination, through the immutable
   writer, the mutable writer, the buffer writer and the file writer. */

#include "yyjson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void dump_bytes(const char *s, size_t len) {
    size_t i;
    if (!s) { printf("<null>"); return; }
    for (i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c >= 0x20 && c < 0x7F) putchar(c);
        else printf("\\x%02X", c);
    }
}

typedef struct { const char *name; yyjson_write_flag flg; } wcase;

static const wcase wcases[] = {
    { "min",        0 },
    { "pretty",     YYJSON_WRITE_PRETTY },
    { "pretty2",    YYJSON_WRITE_PRETTY_TWO_SPACES },
    { "escuni",     YYJSON_WRITE_ESCAPE_UNICODE },
    { "escuni_lo",  YYJSON_WRITE_ESCAPE_UNICODE | YYJSON_WRITE_LOWERCASE_HEX },
    { "escslash",   YYJSON_WRITE_ESCAPE_SLASHES },
    { "infnan",     YYJSON_WRITE_ALLOW_INF_AND_NAN },
    { "infnannull", YYJSON_WRITE_INF_AND_NAN_AS_NULL },
    { "invuni",     YYJSON_WRITE_ALLOW_INVALID_UNICODE },
    { "invuni_esc", YYJSON_WRITE_ALLOW_INVALID_UNICODE |
                    YYJSON_WRITE_ESCAPE_UNICODE },
    { "newline",    YYJSON_WRITE_NEWLINE_AT_END },
    { "tofloat",    YYJSON_WRITE_FP_TO_FLOAT },
    { "fixed1",     YYJSON_WRITE_FP_TO_FIXED(1) },
    { "fixed3",     YYJSON_WRITE_FP_TO_FIXED(3) },
    { "fixed6",     YYJSON_WRITE_FP_TO_FIXED(6) },
    { "fixed15",    YYJSON_WRITE_FP_TO_FIXED(15) },
    { "pretty_all", YYJSON_WRITE_PRETTY | YYJSON_WRITE_ESCAPE_UNICODE |
                    YYJSON_WRITE_ESCAPE_SLASHES |
                    YYJSON_WRITE_ALLOW_INF_AND_NAN |
                    YYJSON_WRITE_NEWLINE_AT_END },
};

static void write_all(const char *tag, yyjson_doc *doc, yyjson_mut_doc *mdoc) {
    size_t i;
    for (i = 0; i < sizeof(wcases) / sizeof(wcases[0]); i++) {
        yyjson_write_err err;
        size_t len = 0;
        char *out;
        static char sbuf[1 << 20];

        memset(&err, 0, sizeof(err));
        out = yyjson_write_opts(doc, wcases[i].flg, NULL, &len, &err);
        printf("   %s/%s imut len=%zu code=%u msg=%s\n   >", tag,
               wcases[i].name, len, (unsigned)err.code,
               err.msg ? err.msg : "(null)");
        dump_bytes(out, len);
        printf("<\n");
        if (out) free(out);

        memset(&err, 0, sizeof(err));
        len = 0;
        out = yyjson_mut_write_opts(mdoc, wcases[i].flg, NULL, &len, &err);
        printf("   %s/%s mut len=%zu code=%u msg=%s\n   >", tag,
               wcases[i].name, len, (unsigned)err.code,
               err.msg ? err.msg : "(null)");
        dump_bytes(out, len);
        printf("<\n");
        if (out) free(out);

        memset(&err, 0, sizeof(err));
        len = yyjson_write_buf(sbuf, sizeof(sbuf), doc, wcases[i].flg, &err);
        printf("   %s/%s buf len=%zu code=%u\n   >", tag, wcases[i].name,
               len, (unsigned)err.code);
        dump_bytes(len ? sbuf : NULL, len);
        printf("<\n");

        /* small buffer to exercise the out-of-space path */
        memset(&err, 0, sizeof(err));
        len = yyjson_write_buf(sbuf, 8, doc, wcases[i].flg, &err);
        printf("   %s/%s buf8 len=%zu code=%u msg=%s\n", tag,
               wcases[i].name, len, (unsigned)err.code,
               err.msg ? err.msg : "(null)");
    }
}

int main(int argc, char **argv) {
    char *dat;
    size_t len;
    static char buf[1 << 22];
    yyjson_read_err rerr;
    yyjson_doc *doc;
    yyjson_mut_doc *mdoc;
    yyjson_doc *idoc;
    char *copy;

    if (argc > 1) {
        dat = argv[1];
        len = strlen(dat);
    } else {
        len = fread(buf, 1, sizeof(buf), stdin);
        dat = buf;
    }

    copy = (char *)malloc(len + 1 + YYJSON_PADDING_SIZE);
    memset(copy, 0, len + 1 + YYJSON_PADDING_SIZE);
    memcpy(copy, dat, len);

    memset(&rerr, 0, sizeof(rerr));
    doc = yyjson_read_opts(copy, len,
                           YYJSON_READ_JSON5 |
                           YYJSON_READ_ALLOW_INVALID_UNICODE |
                           YYJSON_READ_ALLOW_BOM, NULL, &rerr);
    printf("read code=%u pos=%zu\n", (unsigned)rerr.code, rerr.pos);
    if (!doc) { free(copy); return 0; }

    mdoc = yyjson_doc_mut_copy(doc, NULL);
    write_all("a", doc, mdoc);

    /* immutable copy of the mutable document, then write again */
    idoc = yyjson_mut_doc_imut_copy(mdoc, NULL);
    if (idoc) {
        yyjson_mut_doc *mdoc2 = yyjson_doc_mut_copy(idoc, NULL);
        printf("imut vals=%zu read=%zu\n", yyjson_doc_get_val_count(idoc),
               yyjson_doc_get_read_size(idoc));
        write_all("b", idoc, mdoc2);
        printf("equals=%d\n", (int)yyjson_equals(yyjson_doc_get_root(doc),
                                                 yyjson_doc_get_root(idoc)));
        printf("mut_equals=%d\n",
               (int)yyjson_mut_equals(yyjson_mut_doc_get_root(mdoc),
                                      yyjson_mut_doc_get_root(mdoc2)));
        yyjson_mut_doc_free(mdoc2);
        yyjson_doc_free(idoc);
    }

    /* yyjson_write_number for every number in the document */
    {
        yyjson_val *root = yyjson_doc_get_root(doc);
        size_t idx, max;
        yyjson_val *v;
        char nbuf[64];
        if (yyjson_is_arr(root)) {
            yyjson_arr_foreach(root, idx, max, v) {
                if (yyjson_is_num(v)) {
                    char *e = yyjson_write_number(v, nbuf);
                    printf("num[%zu] = %s (end=%d)\n", idx,
                           e ? nbuf : "(null)", e ? (int)(e - nbuf) : -1);
                }
            }
        }
    }

    /* file round trip */
    {
        yyjson_write_err err;
        memset(&err, 0, sizeof(err));
        if (yyjson_write_file("/tmp/yycmp_out.json", doc,
                              YYJSON_WRITE_PRETTY, NULL, &err)) {
            yyjson_read_err e2;
            yyjson_doc *d2;
            memset(&e2, 0, sizeof(e2));
            d2 = yyjson_read_file("/tmp/yycmp_out.json", 0, NULL, &e2);
            printf("file roundtrip ok=%d code=%u vals=%zu\n", d2 != NULL,
                   (unsigned)e2.code,
                   d2 ? yyjson_doc_get_val_count(d2) : (size_t)0);
            if (d2) yyjson_doc_free(d2);
        } else {
            printf("file write failed code=%u msg=%s\n", (unsigned)err.code,
                   err.msg ? err.msg : "(null)");
        }
    }

    yyjson_mut_doc_free(mdoc);
    yyjson_doc_free(doc);
    free(copy);
    return 0;
}
YYJSON_DRIVER_EOF

cat > "$WORK/api.c" <<'YYJSON_DRIVER_EOF'
/* Differential test driver for the rest of the API: JSON Pointer, JSON Patch,
   JSON Merge Patch, the incremental reader, the pool/dynamic allocators, a
   malloc-call trace, and the mutable value API through the header's inline
   functions. */

#include "yyjson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void dump_bytes(const char *s, size_t len) {
    size_t i;
    if (!s) { printf("<null>"); return; }
    for (i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c >= 0x20 && c < 0x7F) putchar(c);
        else printf("\\x%02X", c);
    }
}

static void print_mut(const char *tag, yyjson_mut_val *val) {
    size_t len = 0;
    char *out = yyjson_mut_val_write_opts(val, YYJSON_WRITE_ALLOW_INF_AND_NAN,
                                          NULL, &len, NULL);
    printf("%s = ", tag);
    dump_bytes(out, len);
    printf("\n");
    if (out) free(out);
}

/* ---------------------------------------------------------- alloc trace */

static size_t trace_n = 0;
static char trace_buf[1 << 16];
static size_t trace_len = 0;

static void trace_add(const char *fmt, size_t a, size_t b) {
    int n = snprintf(trace_buf + trace_len, sizeof(trace_buf) - trace_len,
                     fmt, (unsigned long long)a, (unsigned long long)b);
    if (n > 0) trace_len += (size_t)n;
    trace_n++;
}

static void *trace_malloc(void *ctx, size_t size) {
    trace_add("  malloc(%llu)\n", size, 0);
    return malloc(size);
}
static void *trace_realloc(void *ctx, void *ptr, size_t old_size, size_t size) {
    trace_add("  realloc(%llu -> %llu)\n", old_size, size);
    return realloc(ptr, size);
}
static void trace_free(void *ctx, void *ptr) {
    trace_add("  free()%.0llu%.0llu\n", 0, 0);
    free(ptr);
}

static const yyjson_alc trace_alc = {
    trace_malloc, trace_realloc, trace_free, NULL
};

/* ------------------------------------------------------------- pointers */

static const char *ptrs[] = {
    "", "/", "/a", "/a/0", "/a/1", "/a/2", "/a/-", "/a/3", "/b",
    "/b/c", "/b/c/d", "/x", "/0", "/a~0b", "/a~1b", "/a~2b", "/~",
    "/-", "/00", "/01", "/999999999999999999999999",
    "/e", "/e/0", "/e/0/f"
};

static void test_ptr(yyjson_doc *doc, yyjson_mut_doc *mdoc) {
    size_t i;
    for (i = 0; i < sizeof(ptrs) / sizeof(ptrs[0]); i++) {
        yyjson_ptr_err err;
        yyjson_val *v;
        yyjson_mut_val *mv;
        memset(&err, 0, sizeof(err));
        v = yyjson_doc_ptr_getx(doc, ptrs[i], strlen(ptrs[i]), &err);
        printf("ptr \"%s\" -> %s code=%u pos=%zu msg=%s\n", ptrs[i],
               v ? "val" : "null", (unsigned)err.code, err.pos,
               err.msg ? err.msg : "(null)");
        if (v) {
            size_t len = 0;
            char *out = yyjson_val_write_opts(v, 0, NULL, &len, NULL);
            printf("   > ");
            dump_bytes(out, len);
            printf("\n");
            if (out) free(out);
        }
        memset(&err, 0, sizeof(err));
        mv = yyjson_mut_doc_ptr_getx(mdoc, ptrs[i], strlen(ptrs[i]),
                                     NULL, &err);
        printf("mptr \"%s\" -> %s code=%u pos=%zu\n", ptrs[i],
               mv ? "val" : "null", (unsigned)err.code, err.pos);
    }
}

static void test_ptr_modify(yyjson_doc *doc) {
    size_t i;
    for (i = 0; i < sizeof(ptrs) / sizeof(ptrs[0]); i++) {
        const char *p = ptrs[i];
        size_t plen = strlen(p);
        int op;
        for (op = 0; op < 4; op++) {
            yyjson_mut_doc *d = yyjson_doc_mut_copy(doc, NULL);
            yyjson_ptr_err err;
            yyjson_ptr_ctx ctx;
            yyjson_mut_val *nv = yyjson_mut_int(d, 42);
            bool ok = false;
            yyjson_mut_val *old = NULL;
            memset(&err, 0, sizeof(err));
            memset(&ctx, 0, sizeof(ctx));
            switch (op) {
                case 0: ok = yyjson_mut_doc_ptr_addx(d, p, plen, nv, true,
                                                     &ctx, &err); break;
                case 1: ok = yyjson_mut_doc_ptr_setx(d, p, plen, nv, true,
                                                     &ctx, &err); break;
                case 2: old = yyjson_mut_doc_ptr_replacex(d, p, plen, nv,
                                                          &ctx, &err);
                        ok = old != NULL; break;
                case 3: old = yyjson_mut_doc_ptr_removex(d, p, plen,
                                                         &ctx, &err);
                        ok = old != NULL; break;
            }
            printf("mod%d \"%s\" ok=%d code=%u pos=%zu ctn=%d pre=%d old=%d\n",
                   op, p, (int)ok, (unsigned)err.code, err.pos,
                   ctx.ctn != NULL, ctx.pre != NULL, ctx.old != NULL);
            print_mut("   doc", yyjson_mut_doc_get_root(d));
            yyjson_mut_doc_free(d);
        }
    }
}

/* ---------------------------------------------------------------- patch */

static const char *patches[] = {
    "[]",
    "[{\"op\":\"add\",\"path\":\"/z\",\"value\":1}]",
    "[{\"op\":\"add\",\"path\":\"\",\"value\":1}]",
    "[{\"op\":\"remove\",\"path\":\"/a\"}]",
    "[{\"op\":\"remove\",\"path\":\"/zz\"}]",
    "[{\"op\":\"replace\",\"path\":\"/a\",\"value\":[9]}]",
    "[{\"op\":\"move\",\"from\":\"/a\",\"path\":\"/b\"}]",
    "[{\"op\":\"copy\",\"from\":\"/a\",\"path\":\"/c\"}]",
    "[{\"op\":\"test\",\"path\":\"/a\",\"value\":[1,2.5,\"x\"]}]",
    "[{\"op\":\"test\",\"path\":\"/a\",\"value\":1}]",
    "[{\"op\":\"bogus\",\"path\":\"/a\"}]",
    "[{\"op\":\"add\"}]",
    "[{\"path\":\"/a\"}]",
    "[{\"op\":1,\"path\":\"/a\"}]",
    "[1]",
    "{}",
    "[{\"op\":\"add\",\"path\":\"/a/9\",\"value\":1}]",
    "[{\"op\":\"add\",\"path\":\"/a/-\",\"value\":1},"
    " {\"op\":\"remove\",\"path\":\"/a/0\"}]",
    "[{\"op\":\"move\",\"from\":\"\",\"path\":\"\"}]",
    "[{\"op\":\"copy\",\"from\":\"/a\",\"path\":\"\"}]"
};

static const char *merges[] = {
    "{}", "null", "1", "[1,2]", "{\"a\":null}", "{\"a\":{\"b\":1}}",
    "{\"b\":{\"c\":null}}", "{\"new\":{\"deep\":{\"x\":1}}}"
};

static void test_patch(yyjson_doc *doc) {
    size_t i;
    for (i = 0; i < sizeof(patches) / sizeof(patches[0]); i++) {
        char *buf = (char *)malloc(strlen(patches[i]) + 1 +
                                   YYJSON_PADDING_SIZE);
        yyjson_doc *pdoc;
        yyjson_mut_doc *d;
        yyjson_patch_err perr;
        yyjson_mut_val *res;
        memset(buf, 0, strlen(patches[i]) + 1 + YYJSON_PADDING_SIZE);
        memcpy(buf, patches[i], strlen(patches[i]));
        pdoc = yyjson_read(buf, strlen(patches[i]), 0);
        d = yyjson_mut_doc_new(NULL);
        memset(&perr, 0, sizeof(perr));
        res = yyjson_patch(d, yyjson_doc_get_root(doc),
                           pdoc ? yyjson_doc_get_root(pdoc) : NULL, &perr);
        printf("patch %zu code=%u idx=%zu msg=%s ptr=%u/%zu\n", i,
               (unsigned)perr.code, perr.idx,
               perr.msg ? perr.msg : "(null)",
               (unsigned)perr.ptr.code, perr.ptr.pos);
        print_mut("   res", res);

        /* and the mutable variant */
        {
            yyjson_mut_doc *md = yyjson_doc_mut_copy(doc, NULL);
            yyjson_mut_doc *mp = pdoc ? yyjson_doc_mut_copy(pdoc, NULL) : NULL;
            yyjson_mut_doc *d2 = yyjson_mut_doc_new(NULL);
            yyjson_patch_err perr2;
            yyjson_mut_val *res2;
            memset(&perr2, 0, sizeof(perr2));
            res2 = yyjson_mut_patch(d2, yyjson_mut_doc_get_root(md),
                                    mp ? yyjson_mut_doc_get_root(mp) : NULL,
                                    &perr2);
            printf("mpatch %zu code=%u idx=%zu msg=%s ptr=%u/%zu\n", i,
                   (unsigned)perr2.code, perr2.idx,
                   perr2.msg ? perr2.msg : "(null)",
                   (unsigned)perr2.ptr.code, perr2.ptr.pos);
            print_mut("   res", res2);
            yyjson_mut_doc_free(d2);
            if (mp) yyjson_mut_doc_free(mp);
            yyjson_mut_doc_free(md);
        }
        yyjson_mut_doc_free(d);
        if (pdoc) yyjson_doc_free(pdoc);
        free(buf);
    }

    for (i = 0; i < sizeof(merges) / sizeof(merges[0]); i++) {
        char *buf = (char *)malloc(strlen(merges[i]) + 1 +
                                   YYJSON_PADDING_SIZE);
        yyjson_doc *pdoc;
        yyjson_mut_doc *d;
        yyjson_mut_val *res;
        memset(buf, 0, strlen(merges[i]) + 1 + YYJSON_PADDING_SIZE);
        memcpy(buf, merges[i], strlen(merges[i]));
        pdoc = yyjson_read(buf, strlen(merges[i]), 0);
        d = yyjson_mut_doc_new(NULL);
        res = yyjson_merge_patch(d, yyjson_doc_get_root(doc),
                                 pdoc ? yyjson_doc_get_root(pdoc) : NULL);
        printf("merge %zu\n", i);
        print_mut("   res", res);
        {
            yyjson_mut_doc *md = yyjson_doc_mut_copy(doc, NULL);
            yyjson_mut_doc *mp = pdoc ? yyjson_doc_mut_copy(pdoc, NULL) : NULL;
            yyjson_mut_val *res2 = yyjson_mut_merge_patch(
                d, yyjson_mut_doc_get_root(md),
                mp ? yyjson_mut_doc_get_root(mp) : NULL);
            printf("mmerge %zu\n", i);
            print_mut("   res", res2);
            if (mp) yyjson_mut_doc_free(mp);
            yyjson_mut_doc_free(md);
        }
        yyjson_mut_doc_free(d);
        if (pdoc) yyjson_doc_free(pdoc);
        free(buf);
    }
}

/* ------------------------------------------------------ incremental read */

static void test_incr(const char *dat, size_t len) {
    size_t step;
    for (step = 1; step <= 7; step++) {
        char *buf = (char *)malloc(len + YYJSON_PADDING_SIZE + 1);
        yyjson_incr_state *state;
        size_t pos = 0;
        memset(buf, 0, len + YYJSON_PADDING_SIZE + 1);
        memcpy(buf, dat, len);
        state = yyjson_incr_new(buf, len, 0, NULL);
        printf("incr step=%zu\n", step);
        if (!state) { printf("  state=NULL\n"); free(buf); continue; }
        while (true) {
            yyjson_read_err err;
            yyjson_doc *doc;
            pos += step;
            if (pos > len) pos = len;
            memset(&err, 0, sizeof(err));
            doc = yyjson_incr_read(state, pos, &err);
            printf("  read(%zu) -> %s code=%u pos=%zu msg=%s\n", pos,
                   doc ? "doc" : "null", (unsigned)err.code, err.pos,
                   err.msg ? err.msg : "(null)");
            if (doc) {
                size_t olen = 0;
                char *out = yyjson_write_opts(doc, 0, NULL, &olen, NULL);
                printf("  > ");
                dump_bytes(out, olen);
                printf("\n");
                if (out) free(out);
                yyjson_doc_free(doc);
                break;
            }
            if (err.code != YYJSON_READ_ERROR_MORE) break;
            if (pos >= len) break;
        }
        yyjson_incr_free(state);
        free(buf);
    }
}

/* ---------------------------------------------------------- allocators */

static void test_alc(const char *dat, size_t len) {
    /* pool allocator with a range of buffer sizes */
    size_t sizes[] = { 0, 1, 31, 32, 63, 64, 100, 1024, 1 << 16, 1 << 20 };
    size_t i;
    static char pool[1 << 20];
    for (i = 0; i < sizeof(sizes) / sizeof(sizes[0]); i++) {
        yyjson_alc alc;
        bool ok = yyjson_alc_pool_init(&alc, pool, sizes[i]);
        yyjson_doc *doc;
        yyjson_read_err err;
        char *buf = (char *)malloc(len + YYJSON_PADDING_SIZE + 1);
        memset(buf, 0, len + YYJSON_PADDING_SIZE + 1);
        memcpy(buf, dat, len);
        memset(&err, 0, sizeof(err));
        doc = yyjson_read_opts(buf, len, 0, ok ? &alc : NULL, &err);
        printf("pool %zu ok=%d doc=%d code=%u\n", sizes[i], (int)ok,
               doc != NULL, (unsigned)err.code);
        if (doc) {
            size_t olen = 0;
            char *out = yyjson_write_opts(doc, 0, ok ? &alc : NULL, &olen,
                                          NULL);
            printf("  > ");
            dump_bytes(out, olen);
            printf("\n");
            if (out && ok) alc.free(alc.ctx, out);
            else if (out) free(out);
            yyjson_doc_free(doc);
        }
        free(buf);
    }

    /* dynamic allocator */
    {
        yyjson_alc *alc = yyjson_alc_dyn_new();
        yyjson_doc *doc;
        char *buf = (char *)malloc(len + YYJSON_PADDING_SIZE + 1);
        memset(buf, 0, len + YYJSON_PADDING_SIZE + 1);
        memcpy(buf, dat, len);
        doc = yyjson_read_opts(buf, len, 0, alc, NULL);
        printf("dyn doc=%d\n", doc != NULL);
        if (doc) {
            size_t olen = 0;
            char *out = yyjson_write_opts(doc, YYJSON_WRITE_PRETTY, alc,
                                          &olen, NULL);
            printf("  > ");
            dump_bytes(out, olen);
            printf("\n");
            if (out) alc->free(alc->ctx, out);
            yyjson_doc_free(doc);
        }
        yyjson_alc_dyn_free(alc);
        free(buf);
    }

    /* trace of allocator calls */
    {
        yyjson_doc *doc;
        char *buf = (char *)malloc(len + YYJSON_PADDING_SIZE + 1);
        memset(buf, 0, len + YYJSON_PADDING_SIZE + 1);
        memcpy(buf, dat, len);
        trace_len = 0;
        trace_n = 0;
        trace_buf[0] = 0;
        doc = yyjson_read_opts(buf, len, 0, &trace_alc, NULL);
        if (doc) {
            size_t olen = 0;
            char *out = yyjson_write_opts(doc, 0, &trace_alc, &olen, NULL);
            yyjson_mut_doc *mdoc = yyjson_doc_mut_copy(doc, &trace_alc);
            if (out) trace_alc.free(trace_alc.ctx, out);
            if (mdoc) {
                size_t mlen = 0;
                char *mout = yyjson_mut_write_opts(mdoc, 0, &trace_alc,
                                                   &mlen, NULL);
                if (mout) trace_alc.free(trace_alc.ctx, mout);
                yyjson_mut_doc_free(mdoc);
            }
            yyjson_doc_free(doc);
        }
        printf("trace calls=%zu\n%s", trace_n, trace_buf);
        free(buf);
    }
}

/* ------------------------------------------------- mutable value API */

static void test_mut_api(void) {
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *arr = yyjson_mut_arr(doc);
    yyjson_mut_val *obj = yyjson_mut_obj(doc);
    size_t i;
    double reals[] = { 0.0, -0.0, 1.0, 0.1, 1e-7, 1e21, 1e22, 1.0 / 3.0,
                       123456789012345678.0, 5e-324, 1.7976931348623157e308 };

    yyjson_mut_doc_set_root(doc, obj);
    yyjson_mut_obj_add_val(doc, obj, "arr", arr);
    yyjson_mut_arr_add_int(doc, arr, -1);
    yyjson_mut_arr_add_uint(doc, arr, 18446744073709551615ULL);
    yyjson_mut_arr_add_sint(doc, arr, -9223372036854775807LL - 1);
    yyjson_mut_arr_add_bool(doc, arr, true);
    yyjson_mut_arr_add_null(doc, arr);
    yyjson_mut_arr_add_str(doc, arr, "plain");
    yyjson_mut_arr_add_strcpy(doc, arr, "copied\"\\\n");
    yyjson_mut_arr_append(arr, yyjson_mut_raw(doc, "{\"raw\":1}"));
    for (i = 0; i < sizeof(reals) / sizeof(reals[0]); i++) {
        yyjson_mut_arr_add_real(doc, arr, reals[i]);
    }
    for (i = 0; i < sizeof(reals) / sizeof(reals[0]); i++) {
        yyjson_mut_val *v = yyjson_mut_real(doc, reals[i]);
        yyjson_mut_set_fp_to_fixed(v, (int)(i % 16));
        yyjson_mut_arr_append(arr, v);
    }
    for (i = 0; i < sizeof(reals) / sizeof(reals[0]); i++) {
        yyjson_mut_val *v = yyjson_mut_real(doc, reals[i]);
        yyjson_mut_set_fp_to_float(v, true);
        yyjson_mut_arr_append(arr, v);
    }
    for (i = 0; i < sizeof(reals) / sizeof(reals[0]); i++) {
        yyjson_mut_arr_add_float(doc, arr, (float)reals[i]);
    }
    yyjson_mut_obj_add_int(doc, obj, "i", 7);
    yyjson_mut_obj_add_str(doc, obj, "s", "v");
    yyjson_mut_obj_add_obj(doc, obj, "o");

    print_mut("mut", obj);
    {
        size_t len = 0;
        char *out = yyjson_mut_write_opts(doc, YYJSON_WRITE_PRETTY |
                                          YYJSON_WRITE_ALLOW_INF_AND_NAN,
                                          NULL, &len, NULL);
        dump_bytes(out, len);
        printf("\n");
        if (out) free(out);
    }
    /* number writer for each value */
    {
        yyjson_mut_arr_iter iter = yyjson_mut_arr_iter_with(arr);
        yyjson_mut_val *v;
        char nbuf[64];
        while ((v = yyjson_mut_arr_iter_next(&iter))) {
            if (yyjson_mut_is_num(v)) {
                char *e = yyjson_mut_write_number(v, nbuf);
                printf("wnum %s\n", e ? nbuf : "(null)");
            }
        }
    }
    /* array / object modification through the header inline API */
    {
        yyjson_mut_val *v = yyjson_mut_arr_remove_first(arr);
        printf("removed_first=%d size=%zu\n", v != NULL,
               yyjson_mut_arr_size(arr));
        v = yyjson_mut_arr_remove_last(arr);
        printf("removed_last=%d size=%zu\n", v != NULL,
               yyjson_mut_arr_size(arr));
        yyjson_mut_arr_insert(arr, yyjson_mut_int(doc, 100), 2);
        yyjson_mut_arr_rotate(arr, 3);
        yyjson_mut_arr_replace(arr, 1, yyjson_mut_int(doc, 101));
        printf("obj_remove=%d\n",
               yyjson_mut_obj_remove_str(obj, "i") != NULL);
        yyjson_mut_obj_rename_key(doc, obj, "s", "s2");
        print_mut("mut2", obj);
    }

    /* immutable copy and back */
    {
        yyjson_doc *idoc = yyjson_mut_doc_imut_copy(doc, NULL);
        if (idoc) {
            size_t len = 0;
            char *out = yyjson_write_opts(idoc, 0, NULL, &len, NULL);
            printf("imut vals=%zu read=%zu\n",
                   yyjson_doc_get_val_count(idoc),
                   yyjson_doc_get_read_size(idoc));
            dump_bytes(out, len);
            printf("\n");
            if (out) free(out);
            yyjson_doc_free(idoc);
        }
    }
    yyjson_mut_doc_free(doc);
}

/* --------------------------------------------------------- read_number */

static const char *numbers[] = {
    "0", "-0", "1", "-1", "1.5", "1e3", "1E3", "1e+3", "1e-3",
    "18446744073709551615", "18446744073709551616",
    "-9223372036854775808", "-9223372036854775809",
    "1.7976931348623157e308", "1.7976931348623159e308",
    "5e-324", "2.4703282292062327e-324", "0.1", "0.2", "0.3",
    "1e1000", "-1e1000", "1e-1000", "123456789012345678901234567890",
    "nan", "inf", "-inf", "Infinity", "NaN", "0x1f", "+1", ".5", "1.",
    "00", "01", "1a", "", " 1", "1 ", "1e", "1e+", "1.e3",
    "9007199254740993", "4.9406564584124654e-324"
};

static void test_read_number(void) {
    size_t i;
    yyjson_read_flag flags[] = {
        0, YYJSON_READ_ALLOW_INF_AND_NAN, YYJSON_READ_NUMBER_AS_RAW,
        YYJSON_READ_BIGNUM_AS_RAW, YYJSON_READ_ALLOW_EXT_NUMBER,
        YYJSON_READ_JSON5
    };
    size_t f;
    for (f = 0; f < sizeof(flags) / sizeof(flags[0]); f++) {
        for (i = 0; i < sizeof(numbers) / sizeof(numbers[0]); i++) {
            yyjson_val val;
            yyjson_read_err err;
            const char *end;
            char nbuf[64];
            memset(&val, 0, sizeof(val));
            memset(&err, 0, sizeof(err));
            end = yyjson_read_number(numbers[i], &val, flags[f], NULL, &err);
            printf("num[%zu] \"%s\" end=%d code=%u pos=%zu msg=%s",
                   f, numbers[i], end ? (int)(end - numbers[i]) : -1,
                   (unsigned)err.code, err.pos,
                   err.msg ? err.msg : "(null)");
            if (end) {
                char *e = yyjson_write_number(&val, nbuf);
                printf(" type=%u/%u out=%s", (unsigned)yyjson_get_type(&val),
                       (unsigned)yyjson_get_subtype(&val),
                       e ? nbuf : "(null)");
                if (yyjson_is_raw(&val)) {
                    printf(" raw=");
                    dump_bytes(yyjson_get_raw(&val), yyjson_get_len(&val));
                }
                if (yyjson_is_real(&val)) {
                    unsigned long long bits;
                    double d = yyjson_get_real(&val);
                    memcpy(&bits, &d, 8);
                    printf(" bits=%016llX", bits);
                }
            }
            printf("\n");
        }
    }
}

int main(int argc, char **argv) {
    char *dat;
    size_t len;
    static char buf[1 << 22];
    char *copy;
    yyjson_doc *doc;
    yyjson_mut_doc *mdoc;

    if (argc > 1) {
        dat = argv[1];
        len = strlen(dat);
    } else {
        len = fread(buf, 1, sizeof(buf), stdin);
        dat = buf;
    }

    test_read_number();
    test_mut_api();
    test_incr(dat, len);
    test_alc(dat, len);

    copy = (char *)malloc(len + 1 + YYJSON_PADDING_SIZE);
    memset(copy, 0, len + 1 + YYJSON_PADDING_SIZE);
    memcpy(copy, dat, len);
    doc = yyjson_read_opts(copy, len, 0, NULL, NULL);
    if (doc) {
        mdoc = yyjson_doc_mut_copy(doc, NULL);
        test_ptr(doc, mdoc);
        test_ptr_modify(doc);
        test_patch(doc);
        yyjson_mut_doc_free(mdoc);
        yyjson_doc_free(doc);
    } else {
        printf("input not readable\n");
    }
    free(copy);
    return 0;
}
YYJSON_DRIVER_EOF

cat > "$WORK/num.c" <<'YYJSON_DRIVER_EOF'
/* Differential stress test for the number reader and writer: walks a large
   deterministic set of double bit patterns and integers through
   yyjson_write_number() / yyjson_read_number() with every floating-point
   output format, plus a few exhaustive sweeps over small exponents. */

#include "yyjson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned long long rng_state = 0x853c49e6748fea9bULL;

static unsigned long long rng(void) {
    rng_state ^= rng_state >> 12;
    rng_state ^= rng_state << 25;
    rng_state ^= rng_state >> 27;
    return rng_state * 2685821657736338717ULL;
}

static void check_real(unsigned long long bits) {
    static const yyjson_write_flag fmts[] = {
        0,
        YYJSON_WRITE_FP_TO_FLOAT,
        YYJSON_WRITE_FP_TO_FIXED(1),
        YYJSON_WRITE_FP_TO_FIXED(2),
        YYJSON_WRITE_FP_TO_FIXED(3),
        YYJSON_WRITE_FP_TO_FIXED(6),
        YYJSON_WRITE_FP_TO_FIXED(9),
        YYJSON_WRITE_FP_TO_FIXED(15)
    };
    size_t i;
    char buf[64];
    double d;
    memcpy(&d, &bits, 8);
    for (i = 0; i < sizeof(fmts) / sizeof(fmts[0]); i++) {
        yyjson_val val;
        char *end;
        memset(&val, 0, sizeof(val));
        yyjson_set_real(&val, d);
        val.tag |= ((unsigned long long)fmts[i]) << 32;
        end = yyjson_write_number(&val, buf);
        printf("%016llX/%zu %s", bits, i, end ? buf : "(null)");
        if (end) {
            /* read it back and report the bits, so that a shortest-round-trip
               difference shows up even if the digits happen to agree */
            yyjson_val back;
            const char *e2;
            memset(&back, 0, sizeof(back));
            e2 = yyjson_read_number(buf, &back,
                                    YYJSON_READ_ALLOW_INF_AND_NAN, NULL, NULL);
            if (e2 && yyjson_is_real(&back)) {
                unsigned long long b2;
                double d2 = yyjson_get_real(&back);
                memcpy(&b2, &d2, 8);
                printf(" -> %016llX", b2);
            } else if (e2) {
                printf(" -> int %llu", (unsigned long long)yyjson_get_uint(&back));
            } else {
                printf(" -> err");
            }
        }
        printf("\n");
    }
}

static void check_int(unsigned long long u) {
    yyjson_val val;
    char buf[64], *end;
    memset(&val, 0, sizeof(val));
    yyjson_set_uint(&val, u);
    end = yyjson_write_number(&val, buf);
    printf("u %llu -> %s\n", u, end ? buf : "(null)");
    yyjson_set_sint(&val, (long long)u);
    end = yyjson_write_number(&val, buf);
    printf("s %lld -> %s\n", (long long)u, end ? buf : "(null)");
}

int main(void) {
    int i;
    int exp;

    /* random bit patterns */
    for (i = 0; i < 4000; i++) {
        check_real(rng());
    }
    /* small significands over the whole exponent range */
    for (exp = 0; exp < 2047; exp++) {
        check_real(((unsigned long long)exp << 52) | 1);
        check_real(((unsigned long long)exp << 52) | 0xFFFFFFFFFFFFFULL);
        check_real(((unsigned long long)exp << 52));
    }
    /* subnormals */
    for (i = 0; i < 64; i++) {
        check_real(1ULL << i);
        check_real((1ULL << i) - 1);
    }
    /* integers */
    for (i = 0; i < 2000; i++) {
        check_int(rng());
    }
    for (i = 0; i < 64; i++) {
        check_int(1ULL << i);
        check_int((1ULL << i) - 1);
        check_int(~(1ULL << i));
    }
    /* decimal strings with every exponent */
    for (exp = -340; exp <= 320; exp++) {
        char txt[64];
        yyjson_val val;
        const char *end;
        char out[64], *oe;
        int k;
        for (k = 0; k < 4; k++) {
            static const char *sigs[] = { "1", "9", "1234567890123456789",
                                          "4.9406564584124654" };
            snprintf(txt, sizeof(txt), "%se%d", sigs[k], exp);
            memset(&val, 0, sizeof(val));
            end = yyjson_read_number(txt, &val, 0, NULL, NULL);
            oe = end ? yyjson_write_number(&val, out) : NULL;
            printf("dec %s -> %s\n", txt, oe ? out : "(null)");
        }
    }
    return 0;
}
YYJSON_DRIVER_EOF

cat > "$WORK/io.c" <<'YYJSON_DRIVER_EOF'
/* Differential test driver for file/fp input and output and for the
   fixed-buffer writer at every buffer size. */

#include "yyjson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void dump_bytes(const char *s, size_t len) {
    size_t i;
    if (!s) { printf("<null>"); return; }
    for (i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c >= 0x20 && c < 0x7F) putchar(c);
        else printf("\\x%02X", c);
    }
}

int main(int argc, char **argv) {
    static char buf[1 << 22];
    size_t len;
    char *path = argc > 1 ? argv[1] : "/tmp/yyjson_io_test";
    char *copy;
    yyjson_doc *doc;
    yyjson_read_err rerr;

    len = fread(buf, 1, sizeof(buf), stdin);

    /* write the input to a file, then read it back through both file APIs */
    {
        FILE *fp = fopen(path, "wb");
        if (fp) { fwrite(buf, 1, len, fp); fclose(fp); }
    }

    memset(&rerr, 0, sizeof(rerr));
    doc = yyjson_read_file(path, 0, NULL, &rerr);
    printf("read_file doc=%d code=%u pos=%zu msg=%s\n", doc != NULL,
           (unsigned)rerr.code, rerr.pos, rerr.msg ? rerr.msg : "(null)");
    if (doc) {
        printf("  read=%zu vals=%zu\n", yyjson_doc_get_read_size(doc),
               yyjson_doc_get_val_count(doc));
        yyjson_doc_free(doc);
    }

    /* unseekable stream: exercises the chunked read path */
    {
        FILE *pipe_fp;
        char cmd[512];
        snprintf(cmd, sizeof(cmd), "cat %s", path);
        pipe_fp = popen(cmd, "r");
        if (pipe_fp) {
            memset(&rerr, 0, sizeof(rerr));
            doc = yyjson_read_fp(pipe_fp, 0, NULL, &rerr);
            printf("read_fp(pipe) doc=%d code=%u pos=%zu msg=%s\n", doc != NULL,
                   (unsigned)rerr.code, rerr.pos,
                   rerr.msg ? rerr.msg : "(null)");
            if (doc) {
                printf("  read=%zu vals=%zu\n", yyjson_doc_get_read_size(doc),
                       yyjson_doc_get_val_count(doc));
                yyjson_doc_free(doc);
            }
            pclose(pipe_fp);
        }
    }

    /* read from a file opened at a non-zero offset */
    {
        FILE *fp = fopen(path, "rb");
        if (fp && len > 1) {
            fseek(fp, 1, SEEK_SET);
            memset(&rerr, 0, sizeof(rerr));
            doc = yyjson_read_fp(fp, 0, NULL, &rerr);
            printf("read_fp(ofs1) doc=%d code=%u pos=%zu\n", doc != NULL,
                   (unsigned)rerr.code, rerr.pos);
            if (doc) yyjson_doc_free(doc);
        }
        if (fp) fclose(fp);
    }

    printf("read_file(missing) doc=%d\n",
           yyjson_read_file("/tmp/yyjson_definitely_missing_file", 0, NULL,
                            NULL) != NULL);

    /* buffer writer at every size */
    copy = (char *)malloc(len + 1 + YYJSON_PADDING_SIZE);
    memset(copy, 0, len + 1 + YYJSON_PADDING_SIZE);
    memcpy(copy, buf, len);
    doc = yyjson_read_opts(copy, len, 0, NULL, NULL);
    if (doc) {
        size_t full = 0;
        char *out = yyjson_write_opts(doc, 0, NULL, &full, NULL);
        size_t i;
        static char small[1 << 16];
        if (out) free(out);
        for (i = 0; i <= full + 4 && i < sizeof(small); i++) {
            yyjson_write_err werr;
            size_t n;
            memset(&werr, 0, sizeof(werr));
            memset(small, '#', 32);
            n = yyjson_write_buf(small, i, doc, 0, &werr);
            printf("buf %zu -> %zu code=%u msg=%s\n", i, n,
                   (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
            if (n) {
                printf("  > ");
                dump_bytes(small, n);
                printf("\n");
            }
        }
        /* write to file and fp */
        {
            yyjson_write_err werr;
            FILE *fp;
            memset(&werr, 0, sizeof(werr));
            printf("write_file=%d code=%u\n",
                   (int)yyjson_write_file(path, doc, YYJSON_WRITE_PRETTY, NULL,
                                          &werr), (unsigned)werr.code);
            memset(&werr, 0, sizeof(werr));
            printf("write_file(bad)=%d code=%u msg=%s\n",
                   (int)yyjson_write_file("/nonexistent-dir/x.json", doc, 0,
                                          NULL, &werr),
                   (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
            memset(&werr, 0, sizeof(werr));
            printf("write_file(empty)=%d code=%u msg=%s\n",
                   (int)yyjson_write_file("", doc, 0, NULL, &werr),
                   (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
            fp = fopen("/dev/null", "wb");
            if (fp) {
                memset(&werr, 0, sizeof(werr));
                printf("write_fp=%d code=%u\n",
                       (int)yyjson_write_fp(fp, doc, 0, NULL, &werr),
                       (unsigned)werr.code);
                fclose(fp);
            }
            memset(&werr, 0, sizeof(werr));
            printf("write_fp(null)=%d code=%u msg=%s\n",
                   (int)yyjson_write_fp(NULL, doc, 0, NULL, &werr),
                   (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
        }
        /* NULL input handling */
        {
            yyjson_write_err werr;
            memset(&werr, 0, sizeof(werr));
            printf("write(null doc)=%d code=%u msg=%s\n",
                   yyjson_write_opts(NULL, 0, NULL, NULL, &werr) != NULL,
                   (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
            memset(&werr, 0, sizeof(werr));
            printf("write_buf(null buf)=%zu code=%u msg=%s\n",
                   yyjson_write_buf(NULL, 10, doc, 0, &werr),
                   (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
        }
        yyjson_doc_free(doc);
    }
    {
        yyjson_read_err e;
        memset(&e, 0, sizeof(e));
        printf("read(null)=%d code=%u msg=%s\n",
               yyjson_read_opts(NULL, 1, 0, NULL, &e) != NULL,
               (unsigned)e.code, e.msg ? e.msg : "(null)");
        memset(&e, 0, sizeof(e));
        printf("read(len0)=%d code=%u msg=%s\n",
               yyjson_read_opts(copy, 0, 0, NULL, &e) != NULL,
               (unsigned)e.code, e.msg ? e.msg : "(null)");
        memset(&e, 0, sizeof(e));
        printf("read_file(null)=%d code=%u msg=%s\n",
               yyjson_read_file(NULL, 0, NULL, &e) != NULL,
               (unsigned)e.code, e.msg ? e.msg : "(null)");
        memset(&e, 0, sizeof(e));
        printf("read_fp(null)=%d code=%u msg=%s\n",
               yyjson_read_fp(NULL, 0, NULL, &e) != NULL,
               (unsigned)e.code, e.msg ? e.msg : "(null)");
    }
    free(copy);
    return 0;
}
YYJSON_DRIVER_EOF

cat > "$WORK/extra.c" <<'YYJSON_DRIVER_EOF'
/* Differential test driver for the remaining exported entry points: the
   mutable document pool-size setters, the mutable value writers,
   yyjson_locate_pos over every byte position, and a pool-allocator stress
   loop that forces the split/merge/realloc paths. */

#include "yyjson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void dump_bytes(const char *s, size_t len) {
    size_t i;
    if (!s) { printf("<null>"); return; }
    for (i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c >= 0x20 && c < 0x7F) putchar(c);
        else printf("\\x%02X", c);
    }
}

static size_t calls = 0;
static char log_buf[1 << 18];
static size_t log_len = 0;

static void logf2(const char *fmt, unsigned long long a, unsigned long long b) {
    int n = snprintf(log_buf + log_len, sizeof(log_buf) - log_len, fmt, a, b);
    if (n > 0) log_len += (size_t)n;
    calls++;
}
static void *t_malloc(void *c, size_t s) { logf2("m %llu\n", s, 0); return malloc(s); }
static void *t_realloc(void *c, void *p, size_t o, size_t s) {
    logf2("r %llu %llu\n", o, s); return realloc(p, s);
}
static void t_free(void *c, void *p) { logf2("f\n", 0, 0); free(p); }
static const yyjson_alc t_alc = { t_malloc, t_realloc, t_free, NULL };

int main(int argc, char **argv) {
    static char buf[1 << 22];
    size_t len = fread(buf, 1, sizeof(buf), stdin);
    char *copy;
    yyjson_doc *doc;
    size_t i;

    /* yyjson_locate_pos over every byte position, plus out-of-range */
    for (i = 0; i <= len + 2; i++) {
        size_t line = 12345, col = 12345, chr = 12345;
        int ok = yyjson_locate_pos(buf, len, i, &line, &col, &chr);
        printf("loc %zu ok=%d %zu/%zu/%zu\n", i, ok, line, col, chr);
    }
    printf("loc null=%d\n", (int)yyjson_locate_pos(NULL, 1, 0, NULL, NULL, NULL));

    /* pool-size setters */
    {
        size_t sizes[] = { 0, 1, 2, 7, 16, 255, 4096, (size_t)-1,
                           (size_t)-2, (size_t)-32 };
        for (i = 0; i < sizeof(sizes) / sizeof(sizes[0]); i++) {
            yyjson_mut_doc *d = yyjson_mut_doc_new(NULL);
            int a = (int)yyjson_mut_doc_set_str_pool_size(d, sizes[i]);
            int b = (int)yyjson_mut_doc_set_val_pool_size(d, sizes[i]);
            yyjson_mut_val *o = yyjson_mut_obj(d);
            int c;
            yyjson_mut_doc_set_root(d, o);
            c = (int)yyjson_mut_obj_add_strcpy(d, o, "key",
                                               "a string long enough to grow");
            printf("poolsize %zu str=%d val=%d add=%d\n", sizes[i], a, b, c);
            {
                size_t ol = 0;
                char *out = yyjson_mut_write_opts(d, 0, NULL, &ol, NULL);
                printf("  > ");
                dump_bytes(out, ol);
                printf("\n");
                if (out) free(out);
            }
            printf("  null str=%d val=%d\n",
                   (int)yyjson_mut_doc_set_str_pool_size(NULL, 16),
                   (int)yyjson_mut_doc_set_val_pool_size(NULL, 16));
            yyjson_mut_doc_free(d);
        }
    }

    /* misaligned pool buffers */
    {
        static char raw[4096];
        for (i = 0; i < 8; i++) {
            yyjson_alc alc;
            int ok = (int)yyjson_alc_pool_init(&alc, raw + i, 256 - i);
            void *p1 = ok ? alc.malloc(alc.ctx, 32) : NULL;
            void *p2 = ok ? alc.malloc(alc.ctx, 64) : NULL;
            void *p3;
            printf("palign %zu ok=%d p1=%d p2=%d\n", i, ok, p1 != NULL,
                   p2 != NULL);
            if (p1) {
                p3 = alc.realloc(alc.ctx, p1, 32, 48);
                printf("  realloc=%d\n", p3 != NULL);
                if (p3) alc.free(alc.ctx, p3);
            }
            if (p2) alc.free(alc.ctx, p2);
            printf("  pool_init(null)=%d\n",
                   (int)yyjson_alc_pool_init(NULL, raw, 256));
            printf("  pool_init(nobuf)=%d\n",
                   (int)yyjson_alc_pool_init(&alc, NULL, 256));
        }
    }

    /* pool allocator churn: split, merge and realloc paths. The allocator
       contract requires the true `old_size` and `size > old_size`, so the
       requested sizes are tracked per slot. */
    {
        static char raw[1 << 16];
        yyjson_alc alc;
        void *ptrs[64];
        size_t szs[64];
        size_t k;
        unsigned long long s = 1;
        if (yyjson_alc_pool_init(&alc, raw, sizeof(raw))) {
            memset(ptrs, 0, sizeof(ptrs));
            memset(szs, 0, sizeof(szs));
            for (k = 0; k < 4000; k++) {
                size_t idx;
                s = s * 6364136223846793005ULL + 1442695040888963407ULL;
                idx = (size_t)((s >> 33) % 64);
                if (ptrs[idx]) {
                    if ((s >> 20) & 1) {
                        size_t ns = szs[idx] + 1 + (size_t)((s >> 44) % 200);
                        void *q = alc.realloc(alc.ctx, ptrs[idx], szs[idx], ns);
                        if (q) { ptrs[idx] = q; szs[idx] = ns; }
                    } else {
                        alc.free(alc.ctx, ptrs[idx]);
                        ptrs[idx] = NULL;
                        szs[idx] = 0;
                    }
                } else {
                    szs[idx] = (size_t)((s >> 40) % 300) + 1;
                    ptrs[idx] = alc.malloc(alc.ctx, szs[idx]);
                    if (!ptrs[idx]) szs[idx] = 0;
                }
            }
            for (k = 0; k < 64; k++) {
                if (ptrs[k]) alc.free(alc.ctx, ptrs[k]);
            }
            /* after returning everything the whole pool must be free again */
            printf("churn final=%d\n", alc.malloc(alc.ctx,
                                                  sizeof(raw) - 64) != NULL);
        }
    }

    /* dynamic allocator churn */
    {
        yyjson_alc *alc = yyjson_alc_dyn_new();
        void *ptrs[32];
        size_t szs[32];
        size_t k;
        unsigned long long s = 7;
        memset(ptrs, 0, sizeof(ptrs));
        memset(szs, 0, sizeof(szs));
        for (k = 0; k < 2000; k++) {
            size_t idx;
            s = s * 6364136223846793005ULL + 1442695040888963407ULL;
            idx = (size_t)((s >> 33) % 32);
            if (ptrs[idx]) {
                if ((s >> 20) & 1) {
                    size_t ns = szs[idx] + 1 + (size_t)((s >> 44) % 9000);
                    void *q = alc->realloc(alc->ctx, ptrs[idx], szs[idx], ns);
                    if (q) { ptrs[idx] = q; szs[idx] = ns; }
                } else {
                    alc->free(alc->ctx, ptrs[idx]);
                    ptrs[idx] = NULL;
                    szs[idx] = 0;
                }
            } else {
                szs[idx] = (size_t)((s >> 40) % 20000) + 1;
                ptrs[idx] = alc->malloc(alc->ctx, szs[idx]);
                if (!ptrs[idx]) szs[idx] = 0;
            }
        }
        for (k = 0; k < 32; k++) {
            if (ptrs[k]) alc->free(alc->ctx, ptrs[k]);
        }
        printf("dyn churn ok=1\n");
        yyjson_alc_dyn_free(alc);
        yyjson_alc_dyn_free(NULL);
    }

    /* mutable value writers + allocator trace */
    copy = (char *)malloc(len + 1 + YYJSON_PADDING_SIZE);
    memset(copy, 0, len + 1 + YYJSON_PADDING_SIZE);
    memcpy(copy, buf, len);
    doc = yyjson_read_opts(copy, len, 0, NULL, NULL);
    if (doc) {
        yyjson_mut_doc *mdoc = yyjson_doc_mut_copy(doc, NULL);
        yyjson_mut_val *root = yyjson_mut_doc_get_root(mdoc);
        yyjson_write_err werr;
        static char sbuf[1 << 20];
        size_t n;
        FILE *fp;

        memset(&werr, 0, sizeof(werr));
        n = yyjson_mut_val_write_buf(sbuf, sizeof(sbuf), root, 0, &werr);
        printf("mutvalbuf=%zu code=%u\n  > ", n, (unsigned)werr.code);
        dump_bytes(n ? sbuf : NULL, n);
        printf("\n");

        memset(&werr, 0, sizeof(werr));
        n = yyjson_mut_val_write_buf(sbuf, 4, root, 0, &werr);
        printf("mutvalbuf4=%zu code=%u msg=%s\n", n, (unsigned)werr.code,
               werr.msg ? werr.msg : "(null)");

        memset(&werr, 0, sizeof(werr));
        printf("mutvalfile=%d code=%u\n",
               (int)yyjson_mut_val_write_file(argv[1] ? argv[1] : "/tmp/x.json",
                                              root, YYJSON_WRITE_PRETTY, NULL,
                                              &werr),
               (unsigned)werr.code);
        memset(&werr, 0, sizeof(werr));
        printf("mutvalfile(empty)=%d code=%u msg=%s\n",
               (int)yyjson_mut_val_write_file("", root, 0, NULL, &werr),
               (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
        fp = fopen("/dev/null", "wb");
        if (fp) {
            memset(&werr, 0, sizeof(werr));
            printf("mutvalfp=%d code=%u\n",
                   (int)yyjson_mut_val_write_fp(fp, root, 0, NULL, &werr),
                   (unsigned)werr.code);
            fclose(fp);
        }
        memset(&werr, 0, sizeof(werr));
        printf("mutvalfp(null)=%d code=%u msg=%s\n",
               (int)yyjson_mut_val_write_fp(NULL, root, 0, NULL, &werr),
               (unsigned)werr.code, werr.msg ? werr.msg : "(null)");
        memset(&werr, 0, sizeof(werr));
        printf("mutvalopts(null)=%d code=%u msg=%s\n",
               yyjson_mut_val_write_opts(NULL, 0, NULL, NULL, &werr) != NULL,
               (unsigned)werr.code, werr.msg ? werr.msg : "(null)");

        /* yyjson_mut_val_imut_copy of a sub-value */
        {
            yyjson_doc *id = yyjson_mut_val_imut_copy(root, NULL);
            if (id) {
                size_t ol = 0;
                char *out = yyjson_write_opts(id, 0, NULL, &ol, NULL);
                printf("valimut vals=%zu read=%zu\n  > ",
                       yyjson_doc_get_val_count(id),
                       yyjson_doc_get_read_size(id));
                dump_bytes(out, ol);
                printf("\n");
                if (out) free(out);
                yyjson_doc_free(id);
            }
            printf("valimut(null)=%d\n",
                   yyjson_mut_val_imut_copy(NULL, NULL) != NULL);
            printf("docimut(null)=%d\n",
                   yyjson_mut_doc_imut_copy(NULL, NULL) != NULL);
        }

        /* allocator trace for the mutable-side operations */
        log_len = 0; calls = 0; log_buf[0] = 0;
        {
            yyjson_mut_doc *d2 = yyjson_mut_doc_mut_copy(mdoc, &t_alc);
            yyjson_doc *i2 = yyjson_mut_doc_imut_copy(mdoc, &t_alc);
            yyjson_mut_doc *d3 = yyjson_doc_mut_copy(doc, &t_alc);
            if (i2) yyjson_doc_free(i2);
            if (d2) yyjson_mut_doc_free(d2);
            if (d3) yyjson_mut_doc_free(d3);
        }
        printf("trace2 calls=%zu\n%s", calls, log_buf);

        yyjson_mut_doc_free(mdoc);
        yyjson_doc_free(doc);
    }
    printf("mut_doc_mut_copy(null)=%d\n",
           yyjson_mut_doc_mut_copy(NULL, NULL) != NULL);
    printf("doc_mut_copy(null)=%d\n",
           yyjson_doc_mut_copy(NULL, NULL) != NULL);
    printf("val_mut_copy(null)=%d\n",
           yyjson_val_mut_copy(NULL, NULL) != NULL);
    printf("mut_val_mut_copy(null)=%d\n",
           yyjson_mut_val_mut_copy(NULL, NULL) != NULL);
    free(copy);
    return 0;
}
YYJSON_DRIVER_EOF

for drv in dump rw api num io extra; do
    $CC -O1 -std=gnu99 -I "$ROOT/src" -o "$WORK/$drv.orig" \
        "$WORK/$drv.c" "$YYJSON_C" -lm || exit 1
    $CC -O1 -std=gnu99 -I "$ROOT/src" -o "$WORK/$drv.rust" \
        "$WORK/$drv.c" "$RUST_LIB" -lm || exit 1
done

fail=0
total=0

# `num` takes no input: it walks its own set of bit patterns.
"$WORK/num.orig" > "$WORK/num.a" 2>&1
"$WORK/num.rust" > "$WORK/num.b" 2>&1
total=$((total + 1))
if ! cmp -s "$WORK/num.a" "$WORK/num.b"; then
    echo "MISMATCH num"
    diff "$WORK/num.a" "$WORK/num.b" | head -20
    fail=$((fail + 1))
else
    echo "checked num ($(wc -l < "$WORK/num.a") cases)"
fi

for dir in "$ROOT/test/compare/inputs" "$@"; do
    [ -d "$dir" ] || continue
    for drv in dump rw api io extra; do
        for f in "$dir"/*; do
            [ -f "$f" ] || continue
            total=$((total + 1))
            "$WORK/$drv.orig" "$WORK/io_a.json" < "$f" > "$WORK/a.txt" 2>&1
            ra=$?
            "$WORK/$drv.rust" "$WORK/io_b.json" < "$f" > "$WORK/b.txt" 2>&1
            rb=$?
            if [ "$ra" != "$rb" ] || ! cmp -s "$WORK/a.txt" "$WORK/b.txt"; then
                echo "MISMATCH $drv $(basename "$f") (exit $ra vs $rb)"
                diff "$WORK/a.txt" "$WORK/b.txt" | head -20
                fail=$((fail + 1))
            fi
        done
        echo "checked $drv over $(basename "$dir")"
    done
done

echo "comparisons=$total mismatches=$fail"
exit $((fail > 0))
