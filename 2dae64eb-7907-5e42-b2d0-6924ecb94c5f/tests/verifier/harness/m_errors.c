#include "common.h"

/*==============================================================================
 * MARK: - B-02 errors
 *
 * Malformed and truncated inputs: exact code, message text and byte position.
 *
 * Three passes:
 *   1. `direct`    - the API's own parameter and allocator error paths, which
 *                    no corpus file can produce.
 *   2. `case`      - every file in corpus/err against every individual read
 *                    flag, each file crafted for one reader diagnostic.
 *   3. `trunc`     - every prefix of every corpus/err file, which is what
 *                    reaches UNEXPECTED_END from every reader state.
 *============================================================================*/

static void emit(buf *b, const char *kind, const char *name, const char *fname,
                 yyjson_doc *doc, const yyjson_read_err *err) {
    buf_adds(b, "B-02 ");
    buf_adds(b, kind);
    f_cstr(b, "case", name);
    if (fname) f_cstr(b, "f", fname);
    f_bool(b, "ok", doc != NULL);
    f_u(b, "code", (unsigned long long)err->code);
    f_cstr(b, "msg", err->msg);
    f_u(b, "pos", (unsigned long long)err->pos);
    rec_line(b);
    rec_cases++;
}

/* A pool allocator over a buffer of `n` bytes: the cheapest deterministic way
   to drive YYJSON_READ_ERROR_MEMORY_ALLOCATION. */
static void read_with_pool(buf *b, const char *name, const char *dat,
                           size_t len, yyjson_read_flag flg, size_t pool_sz) {
    char *in = pad_copy(dat, len);
    void *pool = malloc(pool_sz ? pool_sz : 1);
    yyjson_alc alc;
    memset(&alc, 0, sizeof(alc));
    bool ok = yyjson_alc_pool_init(&alc, pool, pool_sz);
    yyjson_read_err err;
    memset(&err, 0, sizeof(err));
    yyjson_doc *doc = ok ? yyjson_read_opts(in, len, flg, &alc, &err) : NULL;

    buf_adds(b, "B-02 pool");
    f_cstr(b, "case", name);
    f_u(b, "poolsz", (unsigned long long)pool_sz);
    f_bool(b, "poolinit", ok);
    f_bool(b, "ok", doc != NULL);
    f_u(b, "code", (unsigned long long)err.code);
    f_cstr(b, "msg", err.msg);
    f_u(b, "pos", (unsigned long long)err.pos);
    if (doc) f_tree_imut(b, "tree", yyjson_doc_get_root(doc));
    rec_line(b);
    rec_cases++;

    yyjson_doc_free(doc);
    free(pool);
    free(in);
}

void mod_errors(const char *corpus) {
    buf b;
    buf_init(&b);
    char path[4096];
    yyjson_read_err err;

    /* ---- pass 1: direct parameter / allocator paths ---- */
    {
        memset(&err, 0, sizeof(err));
        yyjson_doc *d = yyjson_read_opts(NULL, 8, 0, NULL, &err);
        emit(&b, "direct", "dat_null", NULL, d, &err);
        yyjson_doc_free(d);
    }
    {
        char in[8] = "[1,2]";
        memset(&err, 0, sizeof(err));
        yyjson_doc *d = yyjson_read_opts(in, 0, 0, NULL, &err);
        emit(&b, "direct", "len_zero", NULL, d, &err);
        yyjson_doc_free(d);
    }
    {
        /* len >= USIZE_MAX - YYJSON_PADDING_SIZE -> MEMORY_ALLOCATION before
           any allocation is attempted.  The pointer is never dereferenced on
           this path. */
        char in[8] = "[1,2]";
        memset(&err, 0, sizeof(err));
        yyjson_doc *d = yyjson_read_opts(in, (size_t)-1, 0, NULL, &err);
        emit(&b, "direct", "len_overflow", NULL, d, &err);
        yyjson_doc_free(d);
    }
    {
        /* yyjson_read_number parameter paths */
        yyjson_val v;
        memset(&v, 0, sizeof(v));
        memset(&err, 0, sizeof(err));
        const char *e = yyjson_read_number(NULL, &v, 0, NULL, &err);
        buf_adds(&b, "B-02 direct");
        f_cstr(&b, "case", "read_number_dat_null");
        f_bool(&b, "ret", e != NULL);
        f_u(&b, "code", (unsigned long long)err.code);
        f_cstr(&b, "msg", err.msg);
        f_u(&b, "pos", (unsigned long long)err.pos);
        rec_line(&b);
        rec_cases++;

        memset(&err, 0, sizeof(err));
        e = yyjson_read_number("1", NULL, 0, NULL, &err);
        buf_adds(&b, "B-02 direct");
        f_cstr(&b, "case", "read_number_val_null");
        f_bool(&b, "ret", e != NULL);
        f_u(&b, "code", (unsigned long long)err.code);
        f_cstr(&b, "msg", err.msg);
        f_u(&b, "pos", (unsigned long long)err.pos);
        rec_line(&b);
        rec_cases++;
    }

    /* ---- pass 2+3: the crafted error corpus ---- */
    flist fl = corpus_list(corpus, "err");
    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;

        for (size_t f = 0; f < RFLAGS_ALL_N; f++) {
            char *in = pad_copy(dat, len);
            memset(&err, 0, sizeof(err));
            yyjson_doc *doc =
                yyjson_read_opts(in, len, RFLAGS_ALL[f].flg, NULL, &err);
            buf_adds(&b, "B-02 case");
            f_cstr(&b, "case", fl.name[i]);
            f_cstr(&b, "f", RFLAGS_ALL[f].name);
            f_read_result(&b, doc, &err);
            rec_line(&b);
            rec_cases++;
            yyjson_doc_free(doc);
            free(in);
        }

        /* every prefix, default flags and the all-permissive flag set */
        static const yyjson_read_flag TRUNC_FLAGS[2] = {
            YYJSON_READ_NOFLAG,
            YYJSON_READ_ALLOW_COMMENTS | YYJSON_READ_ALLOW_TRAILING_COMMAS |
            YYJSON_READ_ALLOW_INF_AND_NAN | YYJSON_READ_ALLOW_EXT_NUMBER |
            YYJSON_READ_ALLOW_EXT_ESCAPE | YYJSON_READ_ALLOW_EXT_WHITESPACE |
            YYJSON_READ_ALLOW_SINGLE_QUOTED_STR |
            YYJSON_READ_ALLOW_UNQUOTED_KEY | YYJSON_READ_ALLOW_INVALID_UNICODE
        };
        static const char *TRUNC_NAMES[2] = { "none", "permissive" };
        for (int tf = 0; tf < 2; tf++) {
            for (size_t n = 0; n <= len; n++) {
                char *in = pad_copy(dat, n);
                memset(&err, 0, sizeof(err));
                yyjson_doc *doc =
                    yyjson_read_opts(in, n, TRUNC_FLAGS[tf], NULL, &err);
                buf_adds(&b, "B-02 trunc");
                f_cstr(&b, "case", fl.name[i]);
                f_cstr(&b, "f", TRUNC_NAMES[tf]);
                f_u(&b, "n", (unsigned long long)n);
                f_bool(&b, "ok", doc != NULL);
                f_u(&b, "code", (unsigned long long)err.code);
                f_cstr(&b, "msg", err.msg);
                f_u(&b, "pos", (unsigned long long)err.pos);
                if (doc) f_tree_imut(&b, "tree", yyjson_doc_get_root(doc));
                rec_line(&b);
                rec_cases++;
                yyjson_doc_free(doc);
                free(in);
            }
        }

        free(dat);
    }
    flist_free(&fl);

    /* ---- MEMORY_ALLOCATION via a too-small pool allocator ---- */
    {
        static const char *SRC[] = {
            "[1,2,3]",
            "{\"a\":[1,{\"b\":\"ccccccccccccccccccccccccccccc\"}]}",
            "\"\\u00e9\\u00e9\\u00e9\\u00e9\\u00e9\\u00e9\\u00e9\\u00e9\"",
            "[[[[[[[[[[1]]]]]]]]]]"
        };
        static const char *NAME[] = { "arr", "obj", "escstr", "nest" };
        for (int s = 0; s < 4; s++) {
            size_t len = strlen(SRC[s]);
            char nm[64];
            for (size_t sz = 0; sz <= 512; sz += 8) {
                snprintf(nm, sizeof(nm), "%s/%zu", NAME[s], sz);
                read_with_pool(&b, nm, SRC[s], len, YYJSON_READ_NOFLAG, sz);
            }
            /* and one comfortably large pool, to pin the success side */
            snprintf(nm, sizeof(nm), "%s/big", NAME[s]);
            read_with_pool(&b, nm, SRC[s], len, YYJSON_READ_NOFLAG, 65536);
        }
    }

    buf_free(&b);
}
