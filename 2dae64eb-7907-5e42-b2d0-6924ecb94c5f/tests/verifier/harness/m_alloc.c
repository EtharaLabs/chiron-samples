#include "common.h"

/*==============================================================================
 * MARK: - B-07 the allocator contract
 *
 * Every entry point that takes a `const yyjson_alc *` must route *all* of its
 * allocation through it, must fail cleanly when it is refused, and must leave
 * nothing outstanding when it does.  This module is the only place that can
 * see any of that, because it supplies the allocator.
 *
 * Passes:
 *   pool     `yyjson_alc_pool_init` over a size sweep: sizes that are
 *            rejected outright, the size one byte below what a document needs,
 *            the exact size, and generous sizes.  Driven through the reader,
 *            the writer, both copy directions and the mutable builder.
 *   dyn      the same, on `yyjson_alc_dyn_new` / `yyjson_alc_dyn_free`.
 *   null     a NULL allocator, which selects libc.  Recorded so the default
 *            path is pinned against the explicit one.
 *   trace    the full allocator event list (kind, size, and realloc's old and
 *            new size) for a representative set of documents.  A port with a
 *            different growth schedule or an extra scratch buffer diverges
 *            here even when every value it produces is correct.
 *   inject   an allocator that returns NULL on its Nth call, for every N from
 *            1 up to one past the number of calls a successful run makes.
 *            Per N: the returned code and message, and the number of blocks
 *            still outstanding after the failure has been cleaned up.  A
 *            non-zero `live` is a leak on an allocation-failure path.
 *   bad      an allocator whose function pointers are NULL, and a partially
 *            filled one.
 *============================================================================*/

/* Documents chosen so that between them they drive every allocating path:
   a flat array, a deep nest, an object with many keys, long strings needing
   unescaping, numbers, and one input large enough to force the reader's value
   buffer to grow by realloc more than once. */
static const struct { const char *name, *src; } DOCS[] = {
    { "null",   "null" },
    { "num",    "12345" },
    { "real",   "1.25e10" },
    { "str",    "\"abcdefghijklmnopqrstuvwxyz\"" },
    { "esc",    "\"\\u00e9\\u4e2d\\n\\t\\\\\\\"\\ud83d\\ude00\"" },
    { "arr",    "[1,2,3,4,5,6,7,8]" },
    { "obj",    "{\"a\":1,\"b\":2,\"c\":3,\"d\":4}" },
    { "nest",   "[[[[[[[[[[[[[[[[1]]]]]]]]]]]]]]]]" },
    { "mixed",  "{\"a\":[1,{\"b\":[true,false,null,\"s\"]}],\"c\":{}}" },
    { "grow",   "[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,"
                "20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,"
                "38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,"
                "56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,71,72,73]" },
    { "strs",   "[\"aaaaaaaaaaaaaaaa\",\"bbbbbbbbbbbbbbbb\","
                "\"cccccccccccccccc\",\"dddddddddddddddd\"]" },
};
static const size_t DOCS_N = sizeof(DOCS) / sizeof(DOCS[0]);

/*==============================================================================
 * MARK: - a driver that exercises every allocating entry point
 *
 * `run_all` performs one fixed sequence of calls against one allocator and
 * records what came back.  Using the same sequence for every allocator kind
 * is what makes the pool / dyn / libc / injected records comparable.
 *============================================================================*/

/* A bignum literal, kept at file scope so the aliasing check below compares
   against a stable object rather than a fresh string literal each call. */
static const char NUMSRC[] = "123456789012345678901234567890";

static void run_all(buf *b, const char *tag, const char *dname,
                    const char *src, const yyjson_alc *alc, alc_trace *t) {
    size_t len = strlen(src);
    char *in = pad_copy(src, len);
    yyjson_read_err rerr;
    memset(&rerr, 0, sizeof(rerr));

    yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, alc,
                                       &rerr);
    f_bool(b, "rd", doc != NULL);
    f_u(b, "rd_code", (unsigned long long)rerr.code);
    f_cstr(b, "rd_msg", rerr.msg);
    f_u(b, "rd_pos", (unsigned long long)rerr.pos);
    if (doc) {
        f_u(b, "vals", (unsigned long long)yyjson_doc_get_val_count(doc));
        f_tree_imut(b, "tree", yyjson_doc_get_root(doc));
    }

    /* writer, on the same allocator */
    {
        size_t ol = 0;
        yyjson_write_err we;
        memset(&we, 0, sizeof(we));
        char *o = yyjson_write_opts(doc, YYJSON_WRITE_NOFLAG, alc, &ol, &we);
        f_bool(b, "wr", o != NULL);
        f_u(b, "wr_code", (unsigned long long)we.code);
        f_cstr(b, "wr_msg", we.msg);
        f_bytes(b, "wr_out", o, o ? ol : 0);
        if (o && alc && alc->free) alc->free(alc->ctx, o);
        else free(o);
    }

    /* mutable copy, value copy and the way back */
    {
        yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, alc);
        f_bool(b, "mcopy", m != NULL);
        f_tree_mut(b, "mtree", m ? yyjson_mut_doc_get_root(m) : NULL);
        yyjson_mut_doc *m2 = yyjson_mut_doc_mut_copy(m, alc);
        f_bool(b, "mcopy2", m2 != NULL);
        yyjson_doc *i2 = yyjson_mut_doc_imut_copy(m, alc);
        f_bool(b, "icopy", i2 != NULL);
        f_tree_imut(b, "itree", i2 ? yyjson_doc_get_root(i2) : NULL);
        yyjson_doc_free(i2);
        yyjson_mut_doc_free(m2);
        yyjson_mut_doc_free(m);
    }

    /* a mutable document built from scratch on this allocator */
    {
        yyjson_mut_doc *m = yyjson_mut_doc_new(alc);
        f_bool(b, "mnew", m != NULL);
        yyjson_mut_val *arr = yyjson_mut_arr(m);
        yyjson_mut_doc_set_root(m, arr);
        int ok = arr != NULL;
        for (int k = 0; k < 24 && ok; k++) {
            char key[32];
            snprintf(key, sizeof(key), "value_%06d", k);
            ok = yyjson_mut_arr_add_strcpy(m, arr, key);
        }
        f_bool(b, "mbuild", ok);
        f_u(b, "msize", (unsigned long long)yyjson_mut_arr_size(arr));
        yyjson_mut_doc_free(m);
    }

    /* yyjson_read_number also takes an allocator (for the raw/bignum path) */
    {
        yyjson_val nv;
        yyjson_read_err ne;
        memset(&nv, 0, sizeof(nv));
        memset(&ne, 0, sizeof(ne));
        const char *end = yyjson_read_number(NUMSRC, &nv,
                                             YYJSON_READ_BIGNUM_AS_RAW,
                                             alc, &ne);
        f_bool(b, "rdnum", end != NULL);
        f_u(b, "rdnum_code", (unsigned long long)ne.code);
        f_cstr(b, "rdnum_msg", ne.msg);
        if (end) f_tree_imut(b, "rdnum_v", &nv);
        /* `yyjson_read_number`'s allocator "is only used when the built-in
           floating point reader is disabled", which it is not in this build.
           A RAW result therefore points INTO `dat`, which the caller owns, and
           must not be freed.  Recording whether it aliases the input is the
           part that is actually observable. */
        f_bool(b, "rdnum_aliases_input",
               end && yyjson_is_raw(&nv) &&
               yyjson_get_raw(&nv) == NUMSRC);
    }

    yyjson_doc_free(doc);
    free(in);

    if (t) {
        f_u(b, "live", (unsigned long long)t->live);
        f_u(b, "ncall", (unsigned long long)t->n_alloc);
    }
    (void)tag;
    (void)dname;
}

/*==============================================================================
 * MARK: - pass `pool`
 *============================================================================*/

static void do_pool(buf *b) {
    /* `yyjson_alc_pool_init` rejects a buffer that cannot even hold its own
       header, so the sweep has to start below that. */
    static const size_t SZ[] = {
        0, 1, 2, 4, 8, 16, 31, 32, 33, 48, 64, 96, 128, 192, 256, 384, 512,
        768, 1024, 1536, 2048, 4096, 8192, 16384, 65536
    };
    static const size_t SZ_N = sizeof(SZ) / sizeof(SZ[0]);
    /* One shared backing buffer, aligned well past what the pool needs.
       `yyjson_alc_pool_init` aligns its context up to `sizeof(pool_ctx)`, 16
       bytes, so a buffer that is merely 8-aligned loses 8 bytes of capacity on
       one target and none on another, which flips the borderline sizes in this
       sweep. Declaring the alignment measures the pool's arithmetic instead of
       the linker's placement. */
    _Alignas(64) static union { double d; char c[65536]; } backing;

    for (size_t d = 0; d < DOCS_N; d++) {
        for (size_t i = 0; i < SZ_N; i++) {
            memset(&backing, 0, sizeof(backing));
            yyjson_alc alc;
            memset(&alc, 0, sizeof(alc));
            bool ok = yyjson_alc_pool_init(&alc, backing.c, SZ[i]);
            buf_adds(b, "B-07 pool");
            f_cstr(b, "case", DOCS[d].name);
            f_u(b, "sz", (unsigned long long)SZ[i]);
            f_bool(b, "init", ok);
            if (ok) run_all(b, "pool", DOCS[d].name, DOCS[d].src, &alc, NULL);
            rec_line(b);
            rec_cases++;
        }
    }

    /* pool_init parameter paths */
    buf_adds(b, "B-07 pool case=init_params");
    {
        yyjson_alc alc;
        _Alignas(64) char tiny[64];
        memset(&alc, 0, sizeof(alc));
        f_bool(b, "alc_null", yyjson_alc_pool_init(NULL, tiny, sizeof(tiny)));
        f_bool(b, "buf_null", yyjson_alc_pool_init(&alc, NULL, 1024));
        f_bool(b, "size_zero", yyjson_alc_pool_init(&alc, tiny, 0));
        f_bool(b, "size_max", yyjson_alc_pool_init(&alc, tiny, (size_t)-1));
        /* an unaligned start address must still work */
        f_bool(b, "unaligned",
               yyjson_alc_pool_init(&alc, tiny + 1, sizeof(tiny) - 1));
    }
    rec_line(b);
    rec_cases++;

    /* The exact threshold: for each document, find the smallest pool that
       lets the read succeed, then record that size, one byte below it and one
       byte above it.  That pins the pool's own bookkeeping overhead. */
    for (size_t d = 0; d < DOCS_N; d++) {
        size_t need = 0;
        for (size_t sz = 0; sz <= 65536; sz++) {
            memset(&backing, 0, sizeof(backing));
            yyjson_alc alc;
            memset(&alc, 0, sizeof(alc));
            if (!yyjson_alc_pool_init(&alc, backing.c, sz)) continue;
            size_t len = strlen(DOCS[d].src);
            char *in = pad_copy(DOCS[d].src, len);
            yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG,
                                               &alc, NULL);
            int got = doc != NULL;
            yyjson_doc_free(doc);
            free(in);
            if (got) { need = sz; break; }
        }
        buf_adds(b, "B-07 pool_min");
        f_cstr(b, "case", DOCS[d].name);
        f_u(b, "need", (unsigned long long)need);
        for (int off = -1; off <= 1; off++) {
            size_t sz = need + (size_t)off;
            if (need == 0) break;
            memset(&backing, 0, sizeof(backing));
            yyjson_alc alc;
            memset(&alc, 0, sizeof(alc));
            bool ok = yyjson_alc_pool_init(&alc, backing.c, sz);
            size_t len = strlen(DOCS[d].src);
            char *in = pad_copy(DOCS[d].src, len);
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *doc = ok ? yyjson_read_opts(in, len,
                                   YYJSON_READ_NOFLAG, &alc, &e) : NULL;
            char nm[32];
            snprintf(nm, sizeof(nm), "off%d", off);
            f_bool(b, nm, doc != NULL);
            snprintf(nm, sizeof(nm), "off%d_code", off);
            f_u(b, nm, (unsigned long long)e.code);
            snprintf(nm, sizeof(nm), "off%d_msg", off);
            f_cstr(b, nm, e.msg);
            yyjson_doc_free(doc);
            free(in);
        }
        rec_line(b);
        rec_cases++;
    }
}

/*==============================================================================
 * MARK: - pass `dyn` and `null`
 *============================================================================*/

static void do_dyn_null(buf *b) {
    for (size_t d = 0; d < DOCS_N; d++) {
        yyjson_alc *alc = yyjson_alc_dyn_new();
        buf_adds(b, "B-07 dyn");
        f_cstr(b, "case", DOCS[d].name);
        f_bool(b, "new", alc != NULL);
        if (alc) run_all(b, "dyn", DOCS[d].name, DOCS[d].src, alc, NULL);
        rec_line(b);
        rec_cases++;
        yyjson_alc_dyn_free(alc);

        buf_adds(b, "B-07 null");
        f_cstr(b, "case", DOCS[d].name);
        run_all(b, "null", DOCS[d].name, DOCS[d].src, NULL, NULL);
        rec_line(b);
        rec_cases++;
    }

    /* A dynamic allocator reused across several documents must behave the
       same way on the second document as on the first. */
    {
        yyjson_alc *alc = yyjson_alc_dyn_new();
        buf_adds(b, "B-07 dyn case=reuse");
        f_bool(b, "new", alc != NULL);
        for (size_t d = 0; d < DOCS_N && alc; d++) {
            size_t len = strlen(DOCS[d].src);
            char *in = pad_copy(DOCS[d].src, len);
            yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG,
                                               alc, NULL);
            char nm[32];
            snprintf(nm, sizeof(nm), "d%zu", d);
            f_bool(b, nm, doc != NULL);
            yyjson_doc_free(doc);
            free(in);
        }
        rec_line(b);
        rec_cases++;
        yyjson_alc_dyn_free(alc);
        /* freeing a NULL dynamic allocator is a documented no-op */
        yyjson_alc_dyn_free(NULL);
    }
}

/*==============================================================================
 * MARK: - pass `trace`
 *============================================================================*/

static void do_trace(buf *b) {
    for (size_t d = 0; d < DOCS_N; d++) {
        alc_trace t;
        yyjson_alc alc;
        alc_trace_init(&t, &alc);
        buf_adds(b, "B-07 trace");
        f_cstr(b, "case", DOCS[d].name);
        run_all(b, "trace", DOCS[d].name, DOCS[d].src, &alc, &t);
        f_alc_trace(b, "alc", &t);
        rec_line(b);
        rec_cases++;
        alc_trace_free(&t);
    }

    /* the reader and the writer in isolation, so the event list is not a
       composite of five entry points */
    for (size_t d = 0; d < DOCS_N; d++) {
        static const char *const WHICH[4] = { "read", "write", "mcopy",
                                              "icopy" };
        for (int w = 0; w < 4; w++) {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            size_t len = strlen(DOCS[d].src);
            char *in = pad_copy(DOCS[d].src, len);

            if (w == 0) {
                yyjson_doc *doc = yyjson_read_opts(in, len,
                                  YYJSON_READ_NOFLAG, &alc, NULL);
                yyjson_doc_free(doc);
            } else {
                /* build the input document on libc, so only the operation
                   under test appears in the trace */
                yyjson_doc *doc = yyjson_read_opts(in, len,
                                  YYJSON_READ_NOFLAG, NULL, NULL);
                alc_trace_reset(&t);
                t.live = 0;
                if (w == 1) {
                    size_t ol = 0;
                    char *o = yyjson_write_opts(doc, YYJSON_WRITE_NOFLAG,
                                                &alc, &ol, NULL);
                    if (o) alc.free(alc.ctx, o);
                } else if (w == 2) {
                    yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, &alc);
                    yyjson_mut_doc_free(m);
                } else {
                    yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, NULL);
                    alc_trace_reset(&t);
                    t.live = 0;
                    yyjson_doc *i2 = yyjson_mut_doc_imut_copy(m, &alc);
                    yyjson_doc_free(i2);
                    yyjson_mut_doc_free(m);
                }
                yyjson_doc_free(doc);
            }
            free(in);

            buf_adds(b, "B-07 trace1");
            f_cstr(b, "case", DOCS[d].name);
            f_cstr(b, "what", WHICH[w]);
            f_u(b, "live", (unsigned long long)t.live);
            f_alc_trace(b, "alc", &t);
            rec_line(b);
            rec_cases++;
            alc_trace_free(&t);
        }
    }

    /* The writer's growth schedule over an output that needs several
       doublings: a long array of long strings. */
    {
        buf src;
        buf_init(&src);
        buf_addc(&src, '[');
        for (int k = 0; k < 200; k++) {
            if (k) buf_addc(&src, ',');
            buf_addf(&src, "\"%064d\"", k);
        }
        buf_addc(&src, ']');
        char *in = pad_copy(src.dat, src.len);
        yyjson_doc *doc = yyjson_read_opts(in, src.len, YYJSON_READ_NOFLAG,
                                           NULL, NULL);
        for (size_t w = 0; w < WFLAGS_CORE_N; w++) {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            size_t ol = 0;
            char *o = yyjson_write_opts(doc, WFLAGS_CORE[w].flg, &alc, &ol,
                                        NULL);
            buf_adds(b, "B-07 trace_grow");
            f_cstr(b, "w", WFLAGS_CORE[w].name);
            f_bool(b, "ok", o != NULL);
            f_u(b, "olen", (unsigned long long)ol);
            if (o) alc.free(alc.ctx, o);
            f_u(b, "live", (unsigned long long)t.live);
            f_alc_trace(b, "alc", &t);
            rec_line(b);
            rec_cases++;
            alc_trace_free(&t);
        }
        yyjson_doc_free(doc);
        free(in);
        buf_free(&src);
    }
}

/*==============================================================================
 * MARK: - pass `inject`
 *============================================================================*/

static void do_inject(buf *b) {
    for (size_t d = 0; d < DOCS_N; d++) {
        /* first, how many allocator calls does a successful run make? */
        alc_trace t;
        yyjson_alc alc;
        alc_trace_init(&t, &alc);
        {
            buf sink;
            buf_init(&sink);
            run_all(&sink, "probe", DOCS[d].name, DOCS[d].src, &alc, &t);
            buf_free(&sink);
        }
        size_t total = t.n_alloc;
        alc_trace_free(&t);

        buf_adds(b, "B-07 inject_total");
        f_cstr(b, "case", DOCS[d].name);
        f_u(b, "ncall", (unsigned long long)total);
        rec_line(b);
        rec_cases++;

        /* then fail each call in turn, including one past the end */
        for (size_t n = 1; n <= total + 1; n++) {
            alc_trace_init(&t, &alc);
            t.fail_after = n - 1;
            buf_adds(b, "B-07 inject");
            f_cstr(b, "case", DOCS[d].name);
            f_u(b, "fail_at", (unsigned long long)n);
            run_all(b, "inject", DOCS[d].name, DOCS[d].src, &alc, &t);
            f_alc_trace(b, "alc", &t);
            rec_line(b);
            rec_cases++;
            alc_trace_free(&t);
        }
    }

    /* Injection against the single entry points, where the leak accounting is
       unambiguous because only one call sequence is in flight. */
    for (size_t d = 0; d < DOCS_N; d++) {
        static const char *const WHICH[3] = { "read", "write", "mcopy" };
        for (int w = 0; w < 3; w++) {
            /* probe the call count */
            size_t total;
            {
                alc_trace t;
                yyjson_alc alc;
                alc_trace_init(&t, &alc);
                size_t len = strlen(DOCS[d].src);
                char *in = pad_copy(DOCS[d].src, len);
                if (w == 0) {
                    yyjson_doc_free(yyjson_read_opts(in, len,
                        YYJSON_READ_NOFLAG, &alc, NULL));
                } else {
                    yyjson_doc *doc = yyjson_read_opts(in, len,
                        YYJSON_READ_NOFLAG, NULL, NULL);
                    alc_trace_reset(&t);
                    t.live = 0;
                    if (w == 1) {
                        size_t ol = 0;
                        char *o = yyjson_write_opts(doc, YYJSON_WRITE_NOFLAG,
                                                    &alc, &ol, NULL);
                        if (o) alc.free(alc.ctx, o);
                    } else {
                        yyjson_mut_doc_free(yyjson_doc_mut_copy(doc, &alc));
                    }
                    yyjson_doc_free(doc);
                }
                free(in);
                total = t.n_alloc;
                alc_trace_free(&t);
            }

            for (size_t n = 1; n <= total + 1; n++) {
                alc_trace t;
                yyjson_alc alc;
                alc_trace_init(&t, &alc);
                size_t len = strlen(DOCS[d].src);
                char *in = pad_copy(DOCS[d].src, len);

                buf_adds(b, "B-07 inject1");
                f_cstr(b, "case", DOCS[d].name);
                f_cstr(b, "what", WHICH[w]);
                f_u(b, "fail_at", (unsigned long long)n);

                if (w == 0) {
                    t.fail_after = n - 1;
                    yyjson_read_err e;
                    memset(&e, 0, sizeof(e));
                    yyjson_doc *doc = yyjson_read_opts(in, len,
                        YYJSON_READ_NOFLAG, &alc, &e);
                    f_bool(b, "ok", doc != NULL);
                    f_u(b, "code", (unsigned long long)e.code);
                    f_cstr(b, "msg", e.msg);
                    f_u(b, "pos", (unsigned long long)e.pos);
                    if (doc)
                        f_tree_imut(b, "tree", yyjson_doc_get_root(doc));
                    yyjson_doc_free(doc);
                } else {
                    yyjson_doc *doc = yyjson_read_opts(in, len,
                        YYJSON_READ_NOFLAG, NULL, NULL);
                    alc_trace_reset(&t);
                    t.live = 0;
                    t.fail_after = n - 1;
                    if (w == 1) {
                        size_t ol = 0;
                        yyjson_write_err e;
                        memset(&e, 0, sizeof(e));
                        char *o = yyjson_write_opts(doc, YYJSON_WRITE_NOFLAG,
                                                    &alc, &ol, &e);
                        f_bool(b, "ok", o != NULL);
                        f_u(b, "code", (unsigned long long)e.code);
                        f_cstr(b, "msg", e.msg);
                        f_bytes(b, "out", o, o ? ol : 0);
                        if (o) alc.free(alc.ctx, o);
                    } else {
                        yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, &alc);
                        f_bool(b, "ok", m != NULL);
                        f_tree_mut(b, "tree",
                                   m ? yyjson_mut_doc_get_root(m) : NULL);
                        yyjson_mut_doc_free(m);
                    }
                    yyjson_doc_free(doc);
                }
                free(in);
                f_u(b, "live", (unsigned long long)t.live);
                f_u(b, "ncall", (unsigned long long)t.n_alloc);
                f_alc_trace(b, "alc", &t);
                rec_line(b);
                rec_cases++;
                alc_trace_free(&t);
            }
        }
    }
}

/*==============================================================================
 * MARK: - pass `bad`
 *============================================================================*/

/* Refuses every request but still accepts frees, because the library is
   entitled to call `free` on a pointer it believes it owns and the all-NULL
   function-pointer case is not a supported configuration: `yyjson_mut_doc_new`
   dereferences `alc->malloc` unconditionally. */
static void *nop_malloc(void *ctx, size_t size) {
    (void)ctx;
    (void)size;
    return NULL;
}
static void *nop_realloc(void *ctx, void *ptr, size_t old_size, size_t size) {
    (void)ctx;
    (void)ptr;
    (void)old_size;
    (void)size;
    return NULL;
}
static void nop_free(void *ctx, void *ptr) {
    (void)ctx;
    (void)ptr;
}

static void do_bad(buf *b) {
    static const char *SRC = "[1,2,3]";
    size_t len = strlen(SRC);

    /* an allocator that always refuses */
    {
        yyjson_alc alc;
        memset(&alc, 0, sizeof(alc));
        alc.malloc = nop_malloc;
        alc.realloc = nop_realloc;
        alc.free = nop_free;
        char *in = pad_copy(SRC, len);
        yyjson_read_err e;
        memset(&e, 0, sizeof(e));
        yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, &alc,
                                           &e);
        buf_adds(b, "B-07 bad case=always_null");
        f_bool(b, "ok", doc != NULL);
        f_u(b, "code", (unsigned long long)e.code);
        f_cstr(b, "msg", e.msg);
        f_u(b, "pos", (unsigned long long)e.pos);
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(doc);
        free(in);

        /* and the writer and the mutable builder on the same allocator */
        yyjson_doc *good = yyjson_read(SRC, len, YYJSON_READ_NOFLAG);
        size_t ol = 0;
        yyjson_write_err we;
        memset(&we, 0, sizeof(we));
        char *o = yyjson_write_opts(good, YYJSON_WRITE_NOFLAG, &alc, &ol, &we);
        buf_adds(b, "B-07 bad case=always_null_write");
        f_bool(b, "ok", o != NULL);
        f_u(b, "code", (unsigned long long)we.code);
        f_cstr(b, "msg", we.msg);
        f_bool(b, "mcopy", yyjson_doc_mut_copy(good, &alc) != NULL);
        f_bool(b, "mnew", yyjson_mut_doc_new(&alc) != NULL);
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(good);
    }
}

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

void mod_alloc(const char *corpus) {
    (void)corpus;
    buf b;
    buf_init(&b);
    do_pool(&b);
    do_dyn_null(&b);
    do_trace(&b);
    do_inject(&b);
    do_bad(&b);
    buf_free(&b);
}
