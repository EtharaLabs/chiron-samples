#include "common.h"

/*==============================================================================
 * MARK: - B-06 JSON Pointer (RFC 6901) and JSON Patch (RFC 6902 / 7386)
 *
 * Passes:
 *   ptr_i    every pointer in PTRS resolved against every document in
 *            corpus/doc on the IMMUTABLE side: doc_ptr_get / getn / getx and
 *            val_ptr_get / getn / getx, plus the typed `ptr_get_<type>`
 *            accessors and the deprecated `get_pointer` spellings.
 *   ptr_m    the same pointers on the MUTABLE side, through the `getx` form
 *            that also yields a `yyjson_ptr_ctx`, whose three fields are
 *            recorded as trees.
 *   ptr_err  the pointer syntax corpus: one record per pointer per entry
 *            point, pinning every `yyjson_ptr_err` code, message and `pos`.
 *   ptr_mod  add / set / replace / remove on a fresh copy of every document
 *            for every pointer, with `create_parent` both ways.
 *   ptr_ctx  the three `yyjson_ptr_ctx` helpers driven off a preceding
 *            `getx`, including on a context that resolved nothing.
 *   vm       scripted pointer sequences, so a mutation's effect on the next
 *            lookup is recorded step by step.
 *   patch    every file in corpus/patch applied to every document in
 *            corpus/doc, through all four patch entry points, recording
 *            `err.code`, `err.idx`, `err.msg` and the nested `err.ptr`.
 *   mpatch   the same pairs through the two merge-patch entry points.
 *============================================================================*/

/* Pointers chosen to cover: the empty pointer (the whole document), object
   keys, array indices in range and out, `-` (the past-the-end token), the two
   RFC 6901 escapes `~0` and `~1`, a stray `~2`, a missing leading '/', an
   empty reference token, indices with leading zeros and signs, a very long
   index, and nesting several levels deep. */
static const char *const PTRS[] = {
    "",
    "/",
    "//",
    "/foo",
    "/foo/0",
    "/foo/1",
    "/foo/2",
    "/foo/-",
    "/foo/00",
    "/foo/01",
    "/foo/+1",
    "/foo/-1",
    "/foo/1x",
    "/foo/99999999999999999999",
    "/a~1b",
    "/c%d",
    "/e^f",
    "/g|h",
    "/i\\j",
    "/k\"l",
    "/ ",
    "/m~0n",
    "/m~2n",
    "/~0",
    "/~1",
    "/~",
    "/0",
    "/1",
    "/2",
    "/-",
    "/a",
    "/a/b",
    "/a/b/c",
    "/a/b/c/0",
    "/a/b/c/1/d",
    "/a/b/c/2/0",
    "/a/b/c/-",
    "/a/b/zz",
    "/f/0",
    "/f/0/-",
    "/g",
    "/g/new",
    "nope",
    "/nope/deep",
    "/b/c",
    "/b/e",
    "/d/0",
};
static const size_t PTRS_N = sizeof(PTRS) / sizeof(PTRS[0]);

/*==============================================================================
 * MARK: - field helpers
 *============================================================================*/

static void fp_err(buf *b, const char *pfx, const yyjson_ptr_err *e) {
    char nm[32];
    snprintf(nm, sizeof(nm), "%scode", pfx);
    f_u(b, nm, (unsigned long long)e->code);
    snprintf(nm, sizeof(nm), "%smsg", pfx);
    f_cstr(b, nm, e->msg);
    snprintf(nm, sizeof(nm), "%spos", pfx);
    f_u(b, nm, (unsigned long long)e->pos);
}

static void fp_ctx(buf *b, const char *pfx, const yyjson_ptr_ctx *c) {
    char nm[32];
    snprintf(nm, sizeof(nm), "%sctn", pfx);
    f_tree_mut(b, nm, c->ctn);
    snprintf(nm, sizeof(nm), "%spre", pfx);
    f_tree_mut(b, nm, c->pre);
    snprintf(nm, sizeof(nm), "%sold", pfx);
    f_tree_mut(b, nm, c->old);
}

/*==============================================================================
 * MARK: - pass `ptr_i`: the immutable side
 *============================================================================*/

static void do_ptr_imut(buf *b, const char *key, yyjson_doc *doc) {
    yyjson_val *root = yyjson_doc_get_root(doc);
    for (size_t p = 0; p < PTRS_N; p++) {
        const char *ptr = PTRS[p];
        size_t pl = strlen(ptr);
        yyjson_ptr_err e1, e2;
        memset(&e1, 0, sizeof(e1));
        memset(&e2, 0, sizeof(e2));

        yyjson_val *a = yyjson_doc_ptr_get(doc, ptr);
        yyjson_val *c = yyjson_doc_ptr_getn(doc, ptr, pl);
        yyjson_val *d = yyjson_doc_ptr_getx(doc, ptr, pl, &e1);
        yyjson_val *f = yyjson_ptr_get(root, ptr);
        yyjson_val *g = yyjson_ptr_getn(root, ptr, pl);
        yyjson_val *h = yyjson_ptr_getx(root, ptr, pl, &e2);

        buf_adds(b, "B-06 ptr_i");
        f_cstr(b, "case", key);
        f_cstr(b, "ptr", ptr);
        f_tree_imut(b, "doc_get", a);
        f_bool(b, "same_n", a == c);
        f_bool(b, "same_x", a == d);
        fp_err(b, "d", &e1);
        f_tree_imut(b, "val_get", f);
        f_bool(b, "vsame_n", f == g);
        f_bool(b, "vsame_x", f == h);
        fp_err(b, "v", &e2);
        /* The deprecated spellings are still exported and still part of the
           ABI a consumer may be linked against, so they are covered on
           purpose; the diagnostic is suppressed only across these calls. */
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-declarations"
        f_bool(b, "dep_doc", yyjson_doc_get_pointer(doc, ptr) == a);
        f_bool(b, "dep_docn", yyjson_doc_get_pointern(doc, ptr, pl) == a);
        f_bool(b, "dep_val", yyjson_get_pointer(root, ptr) == f);
        f_bool(b, "dep_valn", yyjson_get_pointern(root, ptr, pl) == f);
#pragma GCC diagnostic pop

        /* the typed accessors: each must leave `*value` untouched when it
           returns false, so the recorded value proves that too */
        {
            bool bv = true;
            uint64_t uv = 0xAAAA;
            int64_t sv = -0x5555;
            double rv = -1.5, nv = -2.5;
            const char *sp = "SENTINEL";
            f_bool(b, "t_bool", yyjson_ptr_get_bool(root, ptr, &bv));
            f_bool(b, "t_boolv", bv);
            f_bool(b, "t_uint", yyjson_ptr_get_uint(root, ptr, &uv));
            f_u(b, "t_uintv", (unsigned long long)uv);
            f_bool(b, "t_sint", yyjson_ptr_get_sint(root, ptr, &sv));
            f_i(b, "t_sintv", (long long)sv);
            f_bool(b, "t_real", yyjson_ptr_get_real(root, ptr, &rv));
            {
                uint64_t bits;
                memcpy(&bits, &rv, 8);
                f_x64(b, "t_realv", (unsigned long long)bits);
            }
            f_bool(b, "t_num", yyjson_ptr_get_num(root, ptr, &nv));
            {
                uint64_t bits;
                memcpy(&bits, &nv, 8);
                f_x64(b, "t_numv", (unsigned long long)bits);
            }
            f_bool(b, "t_str", yyjson_ptr_get_str(root, ptr, &sp));
            f_cstr(b, "t_strv", sp);
            /* NULL out-params */
            f_bool(b, "t_bool_n", yyjson_ptr_get_bool(root, ptr, NULL));
            f_bool(b, "t_str_n", yyjson_ptr_get_str(root, ptr, NULL));
        }
        rec_line(b);
        rec_cases++;
    }
}

/*==============================================================================
 * MARK: - pass `ptr_m`: the mutable side, with context
 *============================================================================*/

static void do_ptr_mut(buf *b, const char *key, yyjson_doc *idoc) {
    for (size_t p = 0; p < PTRS_N; p++) {
        const char *ptr = PTRS[p];
        size_t pl = strlen(ptr);
        /* a fresh copy per pointer: `getx` does not mutate, but the context it
           hands back points into the document, so isolation keeps the record
           independent of case order */
        yyjson_mut_doc *m = yyjson_doc_mut_copy(idoc, NULL);
        yyjson_mut_val *root = yyjson_mut_doc_get_root(m);
        yyjson_ptr_ctx c1, c2;
        yyjson_ptr_err e1, e2;
        memset(&c1, 0, sizeof(c1));
        memset(&c2, 0, sizeof(c2));
        memset(&e1, 0, sizeof(e1));
        memset(&e2, 0, sizeof(e2));

        yyjson_mut_val *a = yyjson_mut_doc_ptr_get(m, ptr);
        yyjson_mut_val *d = yyjson_mut_doc_ptr_getn(m, ptr, pl);
        yyjson_mut_val *f = yyjson_mut_doc_ptr_getx(m, ptr, pl, &c1, &e1);
        yyjson_mut_val *g = yyjson_mut_ptr_get(root, ptr);
        yyjson_mut_val *h = yyjson_mut_ptr_getn(root, ptr, pl);
        yyjson_mut_val *i = yyjson_mut_ptr_getx(root, ptr, pl, &c2, &e2);

        buf_adds(b, "B-06 ptr_m");
        f_cstr(b, "case", key);
        f_cstr(b, "ptr", ptr);
        f_tree_mut(b, "doc_get", a);
        f_bool(b, "same_n", a == d);
        f_bool(b, "same_x", a == f);
        fp_err(b, "d", &e1);
        fp_ctx(b, "d", &c1);
        f_tree_mut(b, "val_get", g);
        f_bool(b, "vsame_n", g == h);
        f_bool(b, "vsame_x", g == i);
        fp_err(b, "v", &e2);
        fp_ctx(b, "v", &c2);
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-declarations"
        f_bool(b, "dep_doc", yyjson_mut_doc_get_pointer(m, ptr) == a);
        f_bool(b, "dep_docn", yyjson_mut_doc_get_pointern(m, ptr, pl) == a);
        f_bool(b, "dep_val", yyjson_mut_get_pointer(root, ptr) == g);
        f_bool(b, "dep_valn", yyjson_mut_get_pointern(root, ptr, pl) == g);
#pragma GCC diagnostic pop
        rec_line(b);
        rec_cases++;

        yyjson_mut_doc_free(m);
    }
}

/*==============================================================================
 * MARK: - pass `ptr_mod`: add / set / replace / remove
 *============================================================================*/

static void do_ptr_mod(buf *b, const char *key, yyjson_doc *idoc) {
    static const char *const OPN[4] = { "add", "set", "replace", "remove" };

    for (size_t p = 0; p < PTRS_N; p++) {
        const char *ptr = PTRS[p];
        size_t pl = strlen(ptr);
        for (int op = 0; op < 4; op++) {
            /* `create_parent` only exists for add/set */
            int cp_max = (op < 2) ? 2 : 1;
            for (int cp = 0; cp < cp_max; cp++) {
                yyjson_mut_doc *m = yyjson_doc_mut_copy(idoc, NULL);
                yyjson_mut_val *root = yyjson_mut_doc_get_root(m);
                yyjson_mut_val *nv = yyjson_mut_strcpy(m, "NEW");
                yyjson_ptr_ctx c;
                yyjson_ptr_err e;
                memset(&c, 0, sizeof(c));
                memset(&e, 0, sizeof(e));

                buf_adds(b, "B-06 ptr_mod");
                f_cstr(b, "case", key);
                f_cstr(b, "ptr", ptr);
                f_cstr(b, "op", OPN[op]);
                f_bool(b, "cp", cp);

                if (op == 0) {
                    f_bool(b, "ret", yyjson_mut_doc_ptr_addx(m, ptr, pl, nv,
                                                             cp != 0, &c, &e));
                } else if (op == 1) {
                    f_bool(b, "ret", yyjson_mut_doc_ptr_setx(m, ptr, pl, nv,
                                                             cp != 0, &c, &e));
                } else if (op == 2) {
                    yyjson_mut_val *old =
                        yyjson_mut_doc_ptr_replacex(m, ptr, pl, nv, &c, &e);
                    f_tree_mut(b, "ret", old);
                } else {
                    yyjson_mut_val *old =
                        yyjson_mut_doc_ptr_removex(m, ptr, pl, &c, &e);
                    f_tree_mut(b, "ret", old);
                }
                fp_err(b, "", &e);
                fp_ctx(b, "", &c);
                f_tree_mut(b, "root", yyjson_mut_doc_get_root(m));
                {
                    size_t ol = 0;
                    char *o = yyjson_mut_write(m, YYJSON_WRITE_NOFLAG, &ol);
                    f_bytes(b, "json", o, o ? ol : 0);
                    free(o);
                }
                /* the value-rooted spellings on an untouched copy, so the
                   `_val` family is not shadowed by the `_doc` family's result */
                {
                    yyjson_mut_doc *m2 = yyjson_doc_mut_copy(idoc, NULL);
                    yyjson_mut_val *r2 = yyjson_mut_doc_get_root(m2);
                    yyjson_mut_val *n2 = yyjson_mut_strcpy(m2, "NEW");
                    yyjson_ptr_ctx c2;
                    yyjson_ptr_err e2;
                    memset(&c2, 0, sizeof(c2));
                    memset(&e2, 0, sizeof(e2));
                    if (op == 0) {
                        f_bool(b, "vret",
                               yyjson_mut_ptr_addx(r2, ptr, pl, n2, m2,
                                                   cp != 0, &c2, &e2));
                    } else if (op == 1) {
                        f_bool(b, "vret",
                               yyjson_mut_ptr_setx(r2, ptr, pl, n2, m2,
                                                   cp != 0, &c2, &e2));
                    } else if (op == 2) {
                        f_tree_mut(b, "vret",
                                   yyjson_mut_ptr_replacex(r2, ptr, pl, n2,
                                                           &c2, &e2));
                    } else {
                        f_tree_mut(b, "vret",
                                   yyjson_mut_ptr_removex(r2, ptr, pl, &c2,
                                                          &e2));
                    }
                    fp_err(b, "v", &e2);
                    f_tree_mut(b, "vroot", r2);
                    yyjson_mut_doc_free(m2);
                }
                (void)root;
                rec_line(b);
                rec_cases++;
                yyjson_mut_doc_free(m);
            }
        }
    }

    /* The short spellings, once per document.  They differ from the `x` forms
       only in defaulting `create_parent` and dropping the outputs, so one pass
       is enough to catch a wrong default. */
    {
        static const char *const SHORT_PTR[6] = {
            "", "/a", "/0", "/nope/deep", "/foo/-", "nope"
        };
        for (int s = 0; s < 6; s++) {
            const char *ptr = SHORT_PTR[s];
            size_t pl = strlen(ptr);
            yyjson_mut_doc *m = yyjson_doc_mut_copy(idoc, NULL);
            yyjson_mut_val *root = yyjson_mut_doc_get_root(m);
            buf_adds(b, "B-06 ptr_short");
            f_cstr(b, "case", key);
            f_cstr(b, "ptr", ptr);
            f_bool(b, "add", yyjson_mut_doc_ptr_add(m, ptr,
                                                    yyjson_mut_uint(m, 1)));
            f_bool(b, "addn", yyjson_mut_doc_ptr_addn(m, ptr, pl,
                                                      yyjson_mut_uint(m, 2)));
            f_bool(b, "set", yyjson_mut_doc_ptr_set(m, ptr,
                                                    yyjson_mut_uint(m, 3)));
            f_bool(b, "setn", yyjson_mut_doc_ptr_setn(m, ptr, pl,
                                                      yyjson_mut_uint(m, 4)));
            f_tree_mut(b, "rep", yyjson_mut_doc_ptr_replace(m, ptr,
                                                    yyjson_mut_uint(m, 5)));
            f_tree_mut(b, "repn", yyjson_mut_doc_ptr_replacen(m, ptr, pl,
                                                    yyjson_mut_uint(m, 6)));
            f_tree_mut(b, "rem", yyjson_mut_doc_ptr_remove(m, ptr));
            f_tree_mut(b, "remn", yyjson_mut_doc_ptr_removen(m, ptr, pl));
            f_bool(b, "vadd", yyjson_mut_ptr_add(root, ptr,
                                                 yyjson_mut_uint(m, 7), m));
            f_bool(b, "vaddn", yyjson_mut_ptr_addn(root, ptr, pl,
                                                   yyjson_mut_uint(m, 8), m));
            f_bool(b, "vset", yyjson_mut_ptr_set(root, ptr,
                                                 yyjson_mut_uint(m, 9), m));
            f_bool(b, "vsetn", yyjson_mut_ptr_setn(root, ptr, pl,
                                                   yyjson_mut_uint(m, 10), m));
            f_tree_mut(b, "vrep", yyjson_mut_ptr_replace(root, ptr,
                                                   yyjson_mut_uint(m, 11)));
            f_tree_mut(b, "vrepn", yyjson_mut_ptr_replacen(root, ptr, pl,
                                                   yyjson_mut_uint(m, 12)));
            f_tree_mut(b, "vrem", yyjson_mut_ptr_remove(root, ptr));
            f_tree_mut(b, "vremn", yyjson_mut_ptr_removen(root, ptr, pl));
            f_tree_mut(b, "root", yyjson_mut_doc_get_root(m));
            rec_line(b);
            rec_cases++;
            yyjson_mut_doc_free(m);
        }
    }
}

/*==============================================================================
 * MARK: - pass `ptr_null`: the NULL parameter paths
 *============================================================================*/

static void do_ptr_null(buf *b) {
    yyjson_doc *doc = yyjson_read("{\"a\":[1,2]}", 11, YYJSON_READ_NOFLAG);
    yyjson_val *root = yyjson_doc_get_root(doc);
    yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, NULL);
    yyjson_mut_val *mroot = yyjson_mut_doc_get_root(m);
    yyjson_ptr_err e;
    yyjson_ptr_ctx c;

    buf_adds(b, "B-06 ptr_null case=imut");
    memset(&e, 0, sizeof(e));
    f_bool(b, "doc_null", yyjson_doc_ptr_getx(NULL, "/a", 2, &e) != NULL);
    fp_err(b, "a", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "ptr_null", yyjson_doc_ptr_getx(doc, NULL, 2, &e) != NULL);
    fp_err(b, "b", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "val_null", yyjson_ptr_getx(NULL, "/a", 2, &e) != NULL);
    fp_err(b, "c", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "len_zero", yyjson_ptr_getx(root, "/a", 0, &e) != NULL);
    fp_err(b, "d", &e);
    /* no err out-parameter at all */
    f_bool(b, "noerr", yyjson_doc_ptr_getx(doc, "/a", 2, NULL) != NULL);
    rec_line(b);
    rec_cases++;

    buf_adds(b, "B-06 ptr_null case=mut");
    memset(&e, 0, sizeof(e));
    memset(&c, 0, sizeof(c));
    f_bool(b, "doc_null",
           yyjson_mut_doc_ptr_getx(NULL, "/a", 2, &c, &e) != NULL);
    fp_err(b, "a", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "ptr_null",
           yyjson_mut_doc_ptr_getx(m, NULL, 2, &c, &e) != NULL);
    fp_err(b, "b", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "add_val_null",
           yyjson_mut_doc_ptr_addx(m, "/z", 2, NULL, false, &c, &e));
    fp_err(b, "c", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "add_doc_null",
           yyjson_mut_ptr_addx(mroot, "/z", 2, yyjson_mut_uint(m, 1), NULL,
                               false, &c, &e));
    fp_err(b, "d", &e);
    memset(&e, 0, sizeof(e));
    /* `set` with a NULL value is documented as a remove */
    f_bool(b, "set_val_null",
           yyjson_mut_doc_ptr_setx(m, "/a", 2, NULL, false, &c, &e));
    fp_err(b, "e", &e);
    f_tree_mut(b, "root", yyjson_mut_doc_get_root(m));
    memset(&e, 0, sizeof(e));
    f_bool(b, "rep_val_null",
           yyjson_mut_doc_ptr_replacex(m, "/a", 2, NULL, &c, &e) != NULL);
    fp_err(b, "f", &e);
    rec_line(b);
    rec_cases++;

    /* a document whose root is NULL: YYJSON_PTR_ERR_NULL_ROOT */
    {
        yyjson_mut_doc *em = yyjson_mut_doc_new(NULL);
        buf_adds(b, "B-06 ptr_null case=null_root");
        memset(&e, 0, sizeof(e));
        memset(&c, 0, sizeof(c));
        f_bool(b, "get", yyjson_mut_doc_ptr_getx(em, "/a", 2, &c, &e) != NULL);
        fp_err(b, "g", &e);
        memset(&e, 0, sizeof(e));
        f_bool(b, "get_root",
               yyjson_mut_doc_ptr_getx(em, "", 0, &c, &e) != NULL);
        fp_err(b, "h", &e);
        memset(&e, 0, sizeof(e));
        f_bool(b, "add", yyjson_mut_doc_ptr_addx(em, "/a", 2,
               yyjson_mut_uint(em, 1), false, &c, &e));
        fp_err(b, "i", &e);
        memset(&e, 0, sizeof(e));
        f_bool(b, "add_cp", yyjson_mut_doc_ptr_addx(em, "/a", 2,
               yyjson_mut_uint(em, 1), true, &c, &e));
        fp_err(b, "j", &e);
        memset(&e, 0, sizeof(e));
        /* the root pointer on a rootless document: SET_ROOT territory */
        f_bool(b, "add_empty", yyjson_mut_doc_ptr_addx(em, "", 0,
               yyjson_mut_uint(em, 2), false, &c, &e));
        fp_err(b, "k", &e);
        f_tree_mut(b, "root", yyjson_mut_doc_get_root(em));
        memset(&e, 0, sizeof(e));
        f_bool(b, "rem_empty",
               yyjson_mut_doc_ptr_removex(em, "", 0, &c, &e) != NULL);
        fp_err(b, "l", &e);
        f_tree_mut(b, "root2", yyjson_mut_doc_get_root(em));
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(em);
    }

    /* MEMORY_ALLOCATION: a pool allocator with nothing left to give */
    {
        char small[1];
        yyjson_alc alc;
        memset(&alc, 0, sizeof(alc));
        bool pok = yyjson_alc_pool_init(&alc, small, sizeof(small));
        buf_adds(b, "B-06 ptr_null case=alloc_fail");
        f_bool(b, "poolinit", pok);
        if (pok) {
            yyjson_mut_doc *pm = yyjson_mut_doc_new(&alc);
            f_bool(b, "docnew", pm != NULL);
        }
        /* a document on a pool just big enough for the doc but not for the
           parent containers `create_parent` has to build */
        char pool[512];
        memset(&alc, 0, sizeof(alc));
        pok = yyjson_alc_pool_init(&alc, pool, sizeof(pool));
        f_bool(b, "poolinit2", pok);
        yyjson_mut_doc *pm = yyjson_mut_doc_new(&alc);
        f_bool(b, "docnew2", pm != NULL);
        if (pm) {
            yyjson_mut_doc_set_root(pm, yyjson_mut_obj(pm));
            memset(&e, 0, sizeof(e));
            memset(&c, 0, sizeof(c));
            int n = 0;
            bool ok = true;
            /* keep creating deep parents until the pool is exhausted */
            while (ok && n < 64) {
                char ptr[256];
                int w = 0;
                for (int k = 0; k <= n; k++)
                    w += snprintf(ptr + w, sizeof(ptr) - (size_t)w, "/d%d", k);
                memset(&e, 0, sizeof(e));
                ok = yyjson_mut_doc_ptr_addx(pm, ptr, (size_t)w,
                                             yyjson_mut_uint(pm, 1), true,
                                             &c, &e);
                n++;
            }
            f_u(b, "depth", (unsigned long long)n);
            fp_err(b, "m", &e);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(pm);
    }

    yyjson_mut_doc_free(m);
    yyjson_doc_free(doc);
}

/*==============================================================================
 * MARK: - pass `ptr_ctx`: the three context helpers
 *============================================================================*/

static void do_ptr_ctx(buf *b, const char *key, yyjson_doc *idoc) {
    /* Every helper is driven off a preceding `getx` so the context is the one
       the library built, never one the harness forged. */
    static const char *const CPTR[8] = {
        "", "/a", "/0", "/foo/0", "/foo/-", "/nope", "/a/b", "/d/0"
    };
    static const char *const CHELP[3] = { "append", "replace", "remove" };

    for (int s = 0; s < 8; s++) {
        for (int w = 0; w < 3; w++) {
            const char *ptr = CPTR[s];
            size_t pl = strlen(ptr);
            yyjson_mut_doc *m = yyjson_doc_mut_copy(idoc, NULL);
            yyjson_ptr_ctx c;
            yyjson_ptr_err e;
            memset(&c, 0, sizeof(c));
            memset(&e, 0, sizeof(e));
            yyjson_mut_val *got =
                yyjson_mut_doc_ptr_getx(m, ptr, pl, &c, &e);

            buf_adds(b, "B-06 ptr_ctx");
            f_cstr(b, "case", key);
            f_cstr(b, "ptr", ptr);
            f_cstr(b, "helper", CHELP[w]);
            f_tree_mut(b, "got", got);
            fp_err(b, "g", &e);
            fp_ctx(b, "pre_", &c);

            if (w == 0) {
                yyjson_mut_val *k = yyjson_mut_strcpy(m, "CK");
                yyjson_mut_val *v = yyjson_mut_strcpy(m, "CV");
                f_bool(b, "ret", yyjson_ptr_ctx_append(&c, k, v));
                /* the NULL-argument paths of append */
                f_bool(b, "ret_nullv", yyjson_ptr_ctx_append(&c, k, NULL));
                f_bool(b, "ret_nullk", yyjson_ptr_ctx_append(&c, NULL, v));
                f_bool(b, "ret_nullc", yyjson_ptr_ctx_append(NULL, k, v));
            } else if (w == 1) {
                yyjson_mut_val *v = yyjson_mut_strcpy(m, "RV");
                f_bool(b, "ret", yyjson_ptr_ctx_replace(&c, v));
                f_bool(b, "ret_nullv", yyjson_ptr_ctx_replace(&c, NULL));
                f_bool(b, "ret_nullc", yyjson_ptr_ctx_replace(NULL, v));
            } else {
                f_bool(b, "ret", yyjson_ptr_ctx_remove(&c));
                f_bool(b, "ret2", yyjson_ptr_ctx_remove(&c));
                f_bool(b, "ret_nullc", yyjson_ptr_ctx_remove(NULL));
            }
            fp_ctx(b, "post_", &c);
            f_tree_mut(b, "root", yyjson_mut_doc_get_root(m));
            {
                size_t ol = 0;
                char *o = yyjson_mut_write(m, YYJSON_WRITE_NOFLAG, &ol);
                f_bytes(b, "json", o, o ? ol : 0);
                free(o);
            }
            rec_line(b);
            rec_cases++;
            yyjson_mut_doc_free(m);
        }
    }
}

/*==============================================================================
 * MARK: - pass `patch` / `mpatch`
 *============================================================================*/

static void fpatch_err(buf *b, const char *pfx, const yyjson_patch_err *e) {
    char nm[32];
    snprintf(nm, sizeof(nm), "%scode", pfx);
    f_u(b, nm, (unsigned long long)e->code);
    snprintf(nm, sizeof(nm), "%sidx", pfx);
    f_u(b, nm, (unsigned long long)e->idx);
    snprintf(nm, sizeof(nm), "%smsg", pfx);
    f_cstr(b, nm, e->msg);
    snprintf(nm, sizeof(nm), "%sp", pfx);
    fp_err(b, nm, &e->ptr);
}

static void do_patch(buf *b, const char *dkey, yyjson_doc *idoc,
                     const char *pkey, yyjson_doc *ipatch) {
    yyjson_val *orig = yyjson_doc_get_root(idoc);
    yyjson_val *patch = yyjson_doc_get_root(ipatch);

    /* ---- yyjson_patch: immutable orig, immutable patch ---- */
    {
        yyjson_mut_doc *out = yyjson_mut_doc_new(NULL);
        yyjson_patch_err e;
        memset(&e, 0, sizeof(e));
        yyjson_mut_val *r = yyjson_patch(out, orig, patch, &e);
        yyjson_mut_doc_set_root(out, r);
        buf_adds(b, "B-06 patch");
        f_cstr(b, "doc", dkey);
        f_cstr(b, "patch", pkey);
        f_cstr(b, "via", "imut");
        f_bool(b, "ok", r != NULL);
        fpatch_err(b, "", &e);
        f_tree_mut(b, "out", r);
        {
            size_t ol = 0;
            char *o = r ? yyjson_mut_write(out, YYJSON_WRITE_NOFLAG, &ol)
                        : NULL;
            f_bytes(b, "json", o, o ? ol : 0);
            free(o);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(out);
    }

    /* ---- yyjson_mut_patch: both sides converted to mutable first ---- */
    {
        yyjson_mut_doc *mo = yyjson_doc_mut_copy(idoc, NULL);
        yyjson_mut_doc *mp = yyjson_doc_mut_copy(ipatch, NULL);
        yyjson_mut_doc *out = yyjson_mut_doc_new(NULL);
        yyjson_patch_err e;
        memset(&e, 0, sizeof(e));
        yyjson_mut_val *r = yyjson_mut_patch(out,
            yyjson_mut_doc_get_root(mo), yyjson_mut_doc_get_root(mp), &e);
        yyjson_mut_doc_set_root(out, r);
        buf_adds(b, "B-06 patch");
        f_cstr(b, "doc", dkey);
        f_cstr(b, "patch", pkey);
        f_cstr(b, "via", "mut");
        f_bool(b, "ok", r != NULL);
        fpatch_err(b, "", &e);
        f_tree_mut(b, "out", r);
        {
            size_t ol = 0;
            char *o = r ? yyjson_mut_write(out, YYJSON_WRITE_NOFLAG, &ol)
                        : NULL;
            f_bytes(b, "json", o, o ? ol : 0);
            free(o);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(out);
        yyjson_mut_doc_free(mp);
        yyjson_mut_doc_free(mo);
    }

    /* ---- merge patch, both entry points ---- */
    {
        yyjson_mut_doc *out = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *r = yyjson_merge_patch(out, orig, patch);
        yyjson_mut_doc_set_root(out, r);
        yyjson_mut_doc *mo = yyjson_doc_mut_copy(idoc, NULL);
        yyjson_mut_doc *mp = yyjson_doc_mut_copy(ipatch, NULL);
        yyjson_mut_doc *out2 = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *r2 = yyjson_mut_merge_patch(out2,
            yyjson_mut_doc_get_root(mo), yyjson_mut_doc_get_root(mp));
        yyjson_mut_doc_set_root(out2, r2);

        buf_adds(b, "B-06 mpatch");
        f_cstr(b, "doc", dkey);
        f_cstr(b, "patch", pkey);
        f_bool(b, "ok", r != NULL);
        f_tree_mut(b, "out", r);
        f_bool(b, "ok2", r2 != NULL);
        f_tree_mut(b, "out2", r2);
        f_bool(b, "same", yyjson_mut_equals(r, r2));
        {
            size_t ol = 0;
            char *o = r ? yyjson_mut_write(out, YYJSON_WRITE_NOFLAG, &ol)
                        : NULL;
            f_bytes(b, "json", o, o ? ol : 0);
            free(o);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(out2);
        yyjson_mut_doc_free(mp);
        yyjson_mut_doc_free(mo);
        yyjson_mut_doc_free(out);
    }
}

static void do_patch_null(buf *b) {
    yyjson_doc *d = yyjson_read("{\"a\":1}", 7, YYJSON_READ_NOFLAG);
    yyjson_doc *p = yyjson_read("[{\"op\":\"remove\",\"path\":\"/a\"}]", 29,
                                YYJSON_READ_NOFLAG);
    yyjson_val *dv = yyjson_doc_get_root(d);
    yyjson_val *pv = yyjson_doc_get_root(p);
    yyjson_mut_doc *out = yyjson_mut_doc_new(NULL);
    yyjson_patch_err e;

    buf_adds(b, "B-06 patch_null case=params");
    memset(&e, 0, sizeof(e));
    f_bool(b, "doc_null", yyjson_patch(NULL, dv, pv, &e) != NULL);
    fpatch_err(b, "a", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "orig_null", yyjson_patch(out, NULL, pv, &e) != NULL);
    fpatch_err(b, "b", &e);
    memset(&e, 0, sizeof(e));
    f_bool(b, "patch_null", yyjson_patch(out, dv, NULL, &e) != NULL);
    fpatch_err(b, "c", &e);
    /* no err out-parameter */
    f_bool(b, "noerr", yyjson_patch(out, dv, pv, NULL) != NULL);
    memset(&e, 0, sizeof(e));
    f_bool(b, "mut_all_null", yyjson_mut_patch(NULL, NULL, NULL, &e) != NULL);
    fpatch_err(b, "d", &e);
    f_bool(b, "merge_doc_null",
           yyjson_merge_patch(NULL, dv, pv) != NULL);
    f_bool(b, "merge_orig_null",
           yyjson_merge_patch(out, NULL, pv) != NULL);
    f_bool(b, "merge_patch_null",
           yyjson_merge_patch(out, dv, NULL) != NULL);
    f_bool(b, "mmerge_all_null",
           yyjson_mut_merge_patch(NULL, NULL, NULL) != NULL);
    rec_line(b);
    rec_cases++;

    /* YYJSON_PATCH_ERROR_MEMORY_ALLOCATION.
       A pool allocator is a poor instrument for this: the sizes that are small
       enough to starve the patch are also small enough to fail
       `yyjson_mut_doc_new`, so the patch is never entered and the code is
       never produced.  Failing the Nth allocation instead puts the refusal
       exactly where it is needed, and sweeping N walks it through every
       allocating step of the patch.  The outstanding-block count on each
       failure is what shows the error path does not leak. */
    {
        static const char *const BIGPATCH =
            "[{\"op\":\"add\",\"path\":\"/arr\",\"value\":"
            "[\"aaaaaaaaaaaaaaaa\",\"bbbbbbbbbbbbbbbb\",{\"k\":[1,2,3]}]},"
            "{\"op\":\"copy\",\"from\":\"/arr\",\"path\":\"/arr2\"},"
            "{\"op\":\"add\",\"path\":\"/s\",\"value\":\"tttttttttttttttt\"}]";
        yyjson_doc *bp = yyjson_read(BIGPATCH, strlen(BIGPATCH),
                                     YYJSON_READ_NOFLAG);
        yyjson_val *bpv = yyjson_doc_get_root(bp);

        /* how many allocator calls does a successful patch make? */
        size_t total;
        {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            yyjson_mut_doc *po = yyjson_mut_doc_new(&alc);
            yyjson_mut_val *r = yyjson_patch(po, dv, bpv, NULL);
            yyjson_mut_doc_set_root(po, r);
            yyjson_mut_doc_free(po);
            total = t.n_alloc;
            alc_trace_free(&t);
        }
        buf_adds(b, "B-06 patch_null case=inject_total");
        f_u(b, "ncall", (unsigned long long)total);
        rec_line(b);
        rec_cases++;

        for (size_t n = 1; n <= total + 1; n++) {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            t.fail_after = n - 1;
            yyjson_mut_doc *po = yyjson_mut_doc_new(&alc);
            memset(&e, 0, sizeof(e));
            yyjson_mut_val *r = po ? yyjson_patch(po, dv, bpv, &e) : NULL;
            buf_adds(b, "B-06 patch_null");
            buf_addf(b, " case=inject/%zu", n);
            f_bool(b, "docnew", po != NULL);
            f_bool(b, "ok", r != NULL);
            fpatch_err(b, "", &e);
            f_tree_mut(b, "out", r);
            /* merge-patch has no err out-parameter, so a refusal is only
               visible as a NULL return */
            yyjson_patch_err e2;
            memset(&e2, 0, sizeof(e2));
            yyjson_mut_val *mr = po ? yyjson_merge_patch(po, dv, bpv) : NULL;
            f_bool(b, "merge_ok", mr != NULL);
            yyjson_mut_val *pr = po
                ? yyjson_mut_patch(po, yyjson_mut_doc_get_root(po), NULL, &e2)
                : NULL;
            f_bool(b, "mut_ok", pr != NULL);
            fpatch_err(b, "m", &e2);
            yyjson_mut_doc_free(po);
            f_u(b, "live", (unsigned long long)t.live);
            f_u(b, "ncall", (unsigned long long)t.n_alloc);
            rec_line(b);
            rec_cases++;
            alc_trace_free(&t);
        }
        yyjson_doc_free(bp);
    }

    yyjson_mut_doc_free(out);
    yyjson_doc_free(p);
    yyjson_doc_free(d);
}

/*==============================================================================
 * MARK: - pass `vm`: scripted pointer sequences
 *============================================================================*/

static const struct { const char *name; const char *src; } VMS[] = {

{ "ptr_add_seq",
  "new\n"
  "obj r1\n"
  "root r1\n"
  "uint r2 1\n"
  "padd /a r2 0\n"
  "uint r2 2\n"
  "padd /b r2 0\n"
  "pget r3 /a\n"
  "pget r3 /b\n"
  "arr r4\n"
  "padd /c r4 0\n"
  "uint r2 3\n"
  "padd /c/0 r2 0\n"
  "uint r2 4\n"
  "padd /c/- r2 0\n"
  "uint r2 5\n"
  "padd /c/1 r2 0\n"
  "uint r2 6\n"
  "padd /c/9 r2 0\n"
  "write\n"
},

{ "ptr_create_parent",
  "new\n"
  "obj r1\n"
  "root r1\n"
  "uint r2 1\n"
  "padd /x/y/z r2 0\n"
  "uint r2 2\n"
  "padd /x/y/z r2 1\n"
  "pget r3 /x\n"
  "pget r3 /x/y\n"
  "pget r3 /x/y/z\n"
  "uint r2 3\n"
  "padd /p/0/q r2 1\n"
  "write\n"
},

{ "ptr_set_vs_add",
  "new\n"
  "parse r1 {\"a\":1,\"b\":[1,2]}\n"
  "root r1\n"
  "uint r2 9\n"
  "padd /a r2 0\n"
  "uint r2 8\n"
  "pset /a r2 0\n"
  "uint r2 7\n"
  "padd /b/0 r2 0\n"
  "uint r2 6\n"
  "pset /b/0 r2 0\n"
  "pset /a ! 0\n"
  "pget r3 /a\n"
  "write\n"
},

{ "ptr_replace_remove",
  "new\n"
  "parse r1 {\"a\":1,\"b\":[1,2,3],\"c\":{\"d\":4}}\n"
  "root r1\n"
  "uint r2 9\n"
  "prep r3 /a r2\n"
  "uint r2 8\n"
  "prep r3 /zz r2\n"
  "pdel r3 /b/1\n"
  "pdel r3 /b/9\n"
  "pdel r3 /c/d\n"
  "pdel r3 /c\n"
  "pdel r3 /nope\n"
  "pdel r3 ~\n"
  "write\n"
},

{ "ptr_root_ops",
  "new\n"
  "parse r1 {\"a\":1}\n"
  "root r1\n"
  "arr r2\n"
  "padd ~ r2 0\n"
  "pget r3 ~\n"
  "uint r4 1\n"
  "pset ~ r4 0\n"
  "pget r3 ~\n"
  "prep r3 ~ r2\n"
  "pdel r3 ~\n"
  "pget r3 ~\n"
  "write\n"
},

{ "ptr_escapes",
  "new\n"
  "obj r1\n"
  "root r1\n"
  "uint r2 1\n"
  "padd /a~1b r2 0\n"
  "uint r2 2\n"
  "padd /m~0n r2 0\n"
  "uint r2 3\n"
  "padd /~0~1 r2 0\n"
  "uint r2 4\n"
  "padd /%20 r2 0\n"
  "uint r2 5\n"
  "padd /~ r2 0\n"
  "uint r2 6\n"
  "padd /~2 r2 0\n"
  "pget r3 /a~1b\n"
  "pget r3 /m~0n\n"
  "pget r3 /a%2Fb\n"
  "write\n"
},

{ "ptr_ctx_append",
  "new\n"
  "parse r1 {\"a\":[1,2]}\n"
  "root r1\n"
  "pget r2 /a/-\n"
  "str r3 k\n"
  "uint r4 9\n"
  "cappend r3 r4\n"
  "pctx\n"
  "write\n"
  "pget r2 /a/0\n"
  "uint r4 8\n"
  "creplace r4\n"
  "pctx\n"
  "write\n"
  "pget r2 /a/0\n"
  "cremove\n"
  "pctx\n"
  "write\n"
},

{ "ptr_ctx_obj",
  "new\n"
  "parse r1 {\"a\":1,\"b\":2}\n"
  "root r1\n"
  "pget r2 /b\n"
  "uint r4 7\n"
  "creplace r4\n"
  "write\n"
  "pget r2 /a\n"
  "cremove\n"
  "write\n"
  "pget r2 /zz\n"
  "pctx\n"
  "str r3 zz\n"
  "uint r4 6\n"
  "cappend r3 r4\n"
  "write\n"
},

{ "ptr_on_scalar",
  "new\n"
  "uint r1 42\n"
  "root r1\n"
  "pget r2 ~\n"
  "pget r2 /a\n"
  "pget r2 /0\n"
  "uint r3 1\n"
  "padd /a r3 0\n"
  "padd /a r3 1\n"
  "pdel r2 /a\n"
  "write\n"
},

{ "ptr_valrooted",
  "new\n"
  "parse r1 {\"a\":{\"b\":[1,2]}}\n"
  "root r1\n"
  "pget r2 /a\n"
  "pgetv r3 /b r2\n"
  "pgetv r3 /b/1 r2\n"
  "pgetv r3 ~ r2\n"
  "pgetv r3 /b r1\n"
  "pgetv r3 /zz r2\n"
  "pgetv r3 /b/0 !\n"
},
};
static const size_t VMS_N = sizeof(VMS) / sizeof(VMS[0]);

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

typedef struct {
    char       *name;
    yyjson_doc *doc;
} loaded;

static loaded *load_dir(const char *corpus, const char *sub, size_t *out_n) {
    flist fl = corpus_list(corpus, sub);
    loaded *out = (loaded *)calloc(fl.n, sizeof(loaded));
    if (!out) { fprintf(stderr, "harness: oom\n"); exit(2); }
    size_t n = 0;
    char path[4096];
    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;
        char *in = pad_copy(dat, len);
        yyjson_doc *d = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                         NULL);
        free(in);
        free(dat);
        if (!d) continue;
        out[n].name = strdup(fl.name[i]);
        out[n].doc = d;
        n++;
    }
    flist_free(&fl);
    *out_n = n;
    return out;
}

static void free_dir(loaded *l, size_t n) {
    for (size_t i = 0; i < n; i++) {
        free(l[i].name);
        yyjson_doc_free(l[i].doc);
    }
    free(l);
}

void mod_ptrpatch(const char *corpus) {
    buf b;
    buf_init(&b);

    size_t nd = 0, np = 0;
    loaded *docs = load_dir(corpus, "doc", &nd);
    loaded *pats = load_dir(corpus, "patch", &np);

    do_ptr_null(&b);
    do_patch_null(&b);

    for (size_t i = 0; i < nd; i++) {
        do_ptr_imut(&b, docs[i].name, docs[i].doc);
        do_ptr_mut(&b, docs[i].name, docs[i].doc);
        do_ptr_mod(&b, docs[i].name, docs[i].doc);
        do_ptr_ctx(&b, docs[i].name, docs[i].doc);
    }

    for (size_t i = 0; i < VMS_N; i++) {
        size_t n = strlen(VMS[i].src);
        char *src = (char *)malloc(n + 1);
        if (!src) { fprintf(stderr, "harness: oom\n"); exit(2); }
        memcpy(src, VMS[i].src, n + 1);
        vm_run("B-06", VMS[i].name, src, n);
        free(src);
    }

    for (size_t i = 0; i < nd; i++)
        for (size_t j = 0; j < np; j++)
            do_patch(&b, docs[i].name, docs[i].doc, pats[j].name,
                     pats[j].doc);

    free_dir(pats, np);
    free_dir(docs, nd);
    buf_free(&b);
}
