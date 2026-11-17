#include "common.h"

#include <sys/stat.h>
#include <unistd.h>

/*==============================================================================
 * MARK: - B-10 the file entry points
 *
 * `yyjson_read_file`, `yyjson_read_fp` and the eight write-to-file / write-to-fp
 * entry points, over a temporary directory.
 *
 * The tmpdir path itself NEVER reaches a record: it is an environment path and
 * would differ between the two runs being compared.  Only the case label, the
 * returned code, the exact message bytes and the bytes that landed on disk are
 * recorded.  For the same reason the records carry no `errno`, no stat data and
 * no file descriptor numbers.
 *
 * Passes:
 *   rd       `read_file` / `read_fp` over the whole JSON corpus, so the file
 *            readers are held to the same results as the buffer reader.
 *   rd_edge  a missing file, an unreadable file, a directory, an empty file, a
 *            file of only whitespace, the empty path, a path whose parent does
 *            not exist, NULL arguments, a closed stream and a write-only
 *            stream.
 *   wr       all eight write entry points over the corpus at the core write
 *            flags, with the bytes read back off disk and compared against the
 *            in-memory writer's output.
 *   wr_edge  a directory as the destination, a path whose parent does not
 *            exist, an unwritable directory, the empty path, NULL arguments, a
 *            NULL document, an empty mutable document and a read-only stream.
 *   rt       write then read back then write again: the second output must
 *            equal the first, which is what makes the pair a round trip rather
 *            than two unrelated operations.
 *============================================================================*/

/* `g_dir` is deliberately half the size of every path derived from it, so each
   concatenation below is provably non-truncating. */
#define DIRCAP 2048
#define PATHCAP 4096
static char g_dir[DIRCAP];
static char g_scratch_in[PATHCAP];
static char g_scratch_out[PATHCAP];
static char g_a_directory[PATHCAP];
static char g_missing_parent[PATHCAP];
static int  g_ready = 0;

static void paths_init(void) {
    if (g_ready) return;
    const char *d = g_tmpdir ? g_tmpdir : ".";
    snprintf(g_dir, sizeof(g_dir), "%s/b10", d);
    mkdir(g_dir, 0755);
    snprintf(g_scratch_in, sizeof(g_scratch_in), "%s/f1", g_dir);
    snprintf(g_scratch_out, sizeof(g_scratch_out), "%s/f2", g_dir);
    snprintf(g_a_directory, sizeof(g_a_directory), "%s/adir", g_dir);
    mkdir(g_a_directory, 0755);
    snprintf(g_missing_parent, sizeof(g_missing_parent), "%s/nosuchdir/f", g_dir);
    g_ready = 1;
}

static void write_bytes(const char *path, const char *dat, size_t len) {
    FILE *fp = fopen(path, "wb");
    if (!fp) return;
    if (len) fwrite(dat, 1, len, fp);
    fclose(fp);
}

/** The standard read-result group for the file readers. */
static void f_rd(buf *b, const char *nm, yyjson_doc *doc,
                 const yyjson_read_err *e) {
    char k[64];
    snprintf(k, sizeof(k), "%s_ok", nm);
    f_bool(b, k, doc != NULL);
    snprintf(k, sizeof(k), "%s_code", nm);
    f_u(b, k, (unsigned long long)e->code);
    snprintf(k, sizeof(k), "%s_msg", nm);
    f_cstr(b, k, e->msg);
    snprintf(k, sizeof(k), "%s_pos", nm);
    f_u(b, k, (unsigned long long)e->pos);
    if (doc) {
        snprintf(k, sizeof(k), "%s_read", nm);
        f_u(b, k, (unsigned long long)yyjson_doc_get_read_size(doc));
        snprintf(k, sizeof(k), "%s_vals", nm);
        f_u(b, k, (unsigned long long)yyjson_doc_get_val_count(doc));
        snprintf(k, sizeof(k), "%s_tree", nm);
        f_tree_imut(b, k, yyjson_doc_get_root(doc));
    }
}

/** The standard write-result group: the error, then what is on disk. */
static void f_wr(buf *b, const char *nm, bool ok, const yyjson_write_err *e,
                 const char *path) {
    char k[64];
    snprintf(k, sizeof(k), "%s_ok", nm);
    f_bool(b, k, ok);
    snprintf(k, sizeof(k), "%s_code", nm);
    f_u(b, k, (unsigned long long)e->code);
    snprintf(k, sizeof(k), "%s_msg", nm);
    f_cstr(b, k, e->msg);
    char *dat = NULL;
    size_t len = 0;
    snprintf(k, sizeof(k), "%s_disk", nm);
    if (path && file_slurp(path, &dat, &len)) f_bytes(b, k, dat, len);
    else f_str(b, k, NULL, 0);
    free(dat);
}

/*==============================================================================
 * MARK: - pass `rd`
 *============================================================================*/

static void do_rd(buf *b, const char *key, const char *dat, size_t len) {
    write_bytes(g_scratch_in, dat, len);

    for (size_t f = 0; f < RFLAGS_ALL_N; f++) {
        yyjson_read_flag flg = RFLAGS_ALL[f].flg;
        yyjson_read_err e1, e2, e3;
        memset(&e1, 0, sizeof(e1));
        memset(&e2, 0, sizeof(e2));
        memset(&e3, 0, sizeof(e3));

        yyjson_doc *a = yyjson_read_file(g_scratch_in, flg, NULL, &e1);

        FILE *fp = fopen(g_scratch_in, "rb");
        yyjson_doc *c = fp ? yyjson_read_fp(fp, flg, NULL, &e2) : NULL;
        if (fp) fclose(fp);

        /* and the buffer reader on the same bytes, so a file-only divergence
           is attributable to the file layer rather than to the parser */
        char *in = pad_copy(dat, len);
        yyjson_doc *d = yyjson_read_opts(in, len, flg, NULL, &e3);

        buf_adds(b, "B-10 rd");
        f_cstr(b, "case", key);
        f_cstr(b, "f", RFLAGS_ALL[f].name);
        f_u(b, "inlen", (unsigned long long)len);
        f_rd(b, "file", a, &e1);
        f_rd(b, "fp", c, &e2);
        f_bool(b, "same_fp",
               yyjson_equals(a ? yyjson_doc_get_root(a) : NULL,
                             c ? yyjson_doc_get_root(c) : NULL));
        f_bool(b, "same_buf",
               yyjson_equals(a ? yyjson_doc_get_root(a) : NULL,
                             d ? yyjson_doc_get_root(d) : NULL));
        f_u(b, "buf_code", (unsigned long long)e3.code);
        rec_line(b);
        rec_cases++;

        yyjson_doc_free(d);
        free(in);
        yyjson_doc_free(c);
        yyjson_doc_free(a);
    }
}

/*==============================================================================
 * MARK: - pass `wr`
 *============================================================================*/

static void do_wr(buf *b, const char *key, const char *dat, size_t len) {
    char *in = pad_copy(dat, len);
    yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                       NULL);
    if (!doc) { free(in); return; }
    yyjson_val *val = yyjson_doc_get_root(doc);
    yyjson_mut_doc *mdoc = yyjson_doc_mut_copy(doc, NULL);
    yyjson_mut_val *mval = yyjson_mut_doc_get_root(mdoc);

    for (size_t w = 0; w < WFLAGS_CORE_N; w++) {
        yyjson_write_flag flg = WFLAGS_CORE[w].flg;
        buf_adds(b, "B-10 wr");
        f_cstr(b, "case", key);
        f_cstr(b, "w", WFLAGS_CORE[w].name);

        /* the in-memory output is the reference the eight entry points are
           each held against */
        size_t ref_len = 0;
        char *ref = yyjson_write_opts(doc, flg, NULL, &ref_len, NULL);
        f_bytes(b, "ref", ref, ref ? ref_len : 0);

        static const char *const NM[8] = {
            "doc_file", "val_file", "mdoc_file", "mval_file",
            "doc_fp",   "val_fp",   "mdoc_fp",   "mval_fp"
        };
        for (int k = 0; k < 8; k++) {
            unlink(g_scratch_out);
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok;
            if (k < 4) {
                switch (k) {
                    case 0: ok = yyjson_write_file(g_scratch_out, doc, flg, NULL, &e);
                            break;
                    case 1: ok = yyjson_val_write_file(g_scratch_out, val, flg, NULL,
                                                       &e);
                            break;
                    case 2: ok = yyjson_mut_write_file(g_scratch_out, mdoc, flg, NULL,
                                                       &e);
                            break;
                    default: ok = yyjson_mut_val_write_file(g_scratch_out, mval, flg,
                                                            NULL, &e);
                            break;
                }
            } else {
                FILE *fp = fopen(g_scratch_out, "wb");
                switch (k) {
                    case 4: ok = fp && yyjson_write_fp(fp, doc, flg, NULL, &e);
                            break;
                    case 5: ok = fp && yyjson_val_write_fp(fp, val, flg, NULL,
                                                           &e);
                            break;
                    case 6: ok = fp && yyjson_mut_write_fp(fp, mdoc, flg, NULL,
                                                           &e);
                            break;
                    default: ok = fp && yyjson_mut_val_write_fp(fp, mval, flg,
                                                                NULL, &e);
                            break;
                }
                if (fp) fclose(fp);
            }
            f_wr(b, NM[k], ok, &e, g_scratch_out);
            /* and whether the bytes on disk match the in-memory reference */
            char *disk = NULL;
            size_t dlen = 0;
            int same = 0;
            if (ok && ref && file_slurp(g_scratch_out, &disk, &dlen))
                same = (dlen == ref_len && memcmp(disk, ref, dlen) == 0);
            free(disk);
            char nm[64];
            snprintf(nm, sizeof(nm), "%s_same", NM[k]);
            f_bool(b, nm, same);
        }
        free(ref);
        rec_line(b);
        rec_cases++;
    }

    /* ---- round trip: write, read back, write again ---- */
    for (size_t w = 0; w < WFLAGS_CORE_N; w++) {
        unlink(g_scratch_out);
        yyjson_write_err we;
        memset(&we, 0, sizeof(we));
        bool ok = yyjson_write_file(g_scratch_out, doc, WFLAGS_CORE[w].flg, NULL, &we);
        yyjson_read_err re;
        memset(&re, 0, sizeof(re));
        yyjson_doc *back = ok ? yyjson_read_file(g_scratch_out,
            YYJSON_READ_ALLOW_INF_AND_NAN | YYJSON_READ_ALLOW_INVALID_UNICODE,
            NULL, &re) : NULL;
        size_t l2 = 0;
        char *o2 = back ? yyjson_write(back, WFLAGS_CORE[w].flg, &l2) : NULL;
        size_t l1 = 0;
        char *o1 = yyjson_write(doc, WFLAGS_CORE[w].flg, &l1);

        buf_adds(b, "B-10 rt");
        f_cstr(b, "case", key);
        f_cstr(b, "w", WFLAGS_CORE[w].name);
        f_bool(b, "wrote", ok);
        f_u(b, "wr_code", (unsigned long long)we.code);
        f_bool(b, "read_back", back != NULL);
        f_u(b, "rd_code", (unsigned long long)re.code);
        f_cstr(b, "rd_msg", re.msg);
        f_bool(b, "stable", (o1 && o2 && l1 == l2 &&
                             memcmp(o1, o2, l1) == 0));
        f_bool(b, "eq", yyjson_equals(val,
               back ? yyjson_doc_get_root(back) : NULL));
        f_bytes(b, "again", o2, o2 ? l2 : 0);
        rec_line(b);
        rec_cases++;

        free(o1);
        free(o2);
        yyjson_doc_free(back);
    }

    yyjson_mut_doc_free(mdoc);
    yyjson_doc_free(doc);
    free(in);
}

/*==============================================================================
 * MARK: - pass `rd_edge`
 *============================================================================*/

static void do_rd_edge(buf *b) {
    static const struct { const char *name, *dat; size_t len; } FILES[] = {
        { "empty",      "",            0 },
        { "ws_only",    " \t\r\n",     4 },
        { "valid",      "[1,2,3]",     7 },
        { "nul_byte",   "[1,\0,2]",    7 },
        { "truncated",  "[1,2",        4 },
        { "trailing_nl","[1,2]\n",     6 },
        { "bom",        "\xef\xbb\xbf[1]", 6 },
    };
    static const size_t FILES_N = sizeof(FILES) / sizeof(FILES[0]);

    for (size_t i = 0; i < FILES_N; i++) {
        write_bytes(g_scratch_in, FILES[i].dat, FILES[i].len);
        yyjson_read_err e1, e2;
        memset(&e1, 0, sizeof(e1));
        memset(&e2, 0, sizeof(e2));
        yyjson_doc *a = yyjson_read_file(g_scratch_in, YYJSON_READ_NOFLAG, NULL, &e1);
        FILE *fp = fopen(g_scratch_in, "rb");
        yyjson_doc *c = fp ? yyjson_read_fp(fp, YYJSON_READ_NOFLAG, NULL, &e2)
                           : NULL;
        if (fp) fclose(fp);
        buf_adds(b, "B-10 rd_edge");
        f_cstr(b, "case", FILES[i].name);
        f_u(b, "len", (unsigned long long)FILES[i].len);
        f_rd(b, "file", a, &e1);
        f_rd(b, "fp", c, &e2);
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(a);
        yyjson_doc_free(c);
    }

    /* ---- paths that cannot be read ----
       A mode-000 file is deliberately NOT among them.  Whether opening it
       fails depends on the effective uid, and under uid 0 the kernel ignores
       the mode entirely, so such a case records "opened fine" here and
       "FILE_OPEN" under any other user.  Since the reference records are
       frozen at image build time and replayed later, a uid-dependent record is
       a false grading failure waiting to happen, so the permission-denied path
       is left uncovered on purpose.  FILE_OPEN is still reached by `missing`,
       `empty_path` and `parent_missing`, and FILE_READ by `directory` and
       `dot`, all of which are uid-independent. */
    {
        char missing[PATHCAP];
        snprintf(missing, sizeof(missing), "%s/no_such_file", g_dir);
        unlink(missing);

        static const char *const LBL[5] = {
            "missing", "directory", "empty_path", "parent_missing", "dot"
        };
        const char *paths[5];
        paths[0] = missing;
        paths[1] = g_a_directory;
        paths[2] = "";
        paths[3] = g_missing_parent;
        paths[4] = ".";

        for (int i = 0; i < 5; i++) {
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *d = yyjson_read_file(paths[i], YYJSON_READ_NOFLAG,
                                             NULL, &e);
            buf_adds(b, "B-10 rd_edge");
            f_cstr(b, "case", LBL[i]);
            f_rd(b, "file", d, &e);
            rec_line(b);
            rec_cases++;
            yyjson_doc_free(d);
        }
    }

    /* ---- NULL and degenerate arguments ---- */
    {
        yyjson_read_err e;
        buf_adds(b, "B-10 rd_edge case=null_args");
        memset(&e, 0, sizeof(e));
        yyjson_doc *a = yyjson_read_file(NULL, YYJSON_READ_NOFLAG, NULL, &e);
        f_rd(b, "path_null", a, &e);
        yyjson_doc_free(a);
        memset(&e, 0, sizeof(e));
        yyjson_doc *c = yyjson_read_fp(NULL, YYJSON_READ_NOFLAG, NULL, &e);
        f_rd(b, "fp_null", c, &e);
        yyjson_doc_free(c);
        /* no err out-parameter */
        write_bytes(g_scratch_in, "[1]", 3);
        yyjson_doc *d = yyjson_read_file(g_scratch_in, YYJSON_READ_NOFLAG, NULL, NULL);
        f_bool(b, "noerr_ok", d != NULL);
        yyjson_doc_free(d);
        rec_line(b);
        rec_cases++;
    }

    /* ---- a stream that is not readable ---- */
    {
        buf_adds(b, "B-10 rd_edge case=bad_stream");
        FILE *wo = fopen(g_scratch_in, "wb");
        yyjson_read_err e;
        memset(&e, 0, sizeof(e));
        yyjson_doc *a = wo ? yyjson_read_fp(wo, YYJSON_READ_NOFLAG, NULL, &e)
                           : NULL;
        f_rd(b, "write_only", a, &e);
        yyjson_doc_free(a);
        if (wo) fclose(wo);

        /* a stream positioned past the end: read_fp reads from the current
           offset, so this must behave like an empty input */
        write_bytes(g_scratch_in, "[1,2,3]", 7);
        FILE *fp = fopen(g_scratch_in, "rb");
        if (fp) fseek(fp, 7, SEEK_SET);
        memset(&e, 0, sizeof(e));
        yyjson_doc *c = fp ? yyjson_read_fp(fp, YYJSON_READ_NOFLAG, NULL, &e)
                           : NULL;
        f_rd(b, "at_eof", c, &e);
        yyjson_doc_free(c);
        if (fp) fclose(fp);

        /* a stream positioned in the middle */
        fp = fopen(g_scratch_in, "rb");
        if (fp) fseek(fp, 2, SEEK_SET);
        memset(&e, 0, sizeof(e));
        yyjson_doc *d = fp ? yyjson_read_fp(fp, YYJSON_READ_NOFLAG, NULL, &e)
                           : NULL;
        f_rd(b, "mid", d, &e);
        yyjson_doc_free(d);
        if (fp) fclose(fp);
        rec_line(b);
        rec_cases++;
    }

    /* ---- the file readers on a pool allocator, including one too small ---- */
    {
        write_bytes(g_scratch_in, "{\"a\":[1,2,3],\"b\":\"xyz\"}", 23);
        static const size_t SZ[] = { 0, 32, 64, 128, 256, 512, 4096 };
        for (size_t i = 0; i < sizeof(SZ) / sizeof(SZ[0]); i++) {
            char pool[4096];
            yyjson_alc alc;
            memset(&alc, 0, sizeof(alc));
            bool pok = yyjson_alc_pool_init(&alc, pool, SZ[i]);
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *a = pok ? yyjson_read_file(g_scratch_in, YYJSON_READ_NOFLAG,
                                                   &alc, &e) : NULL;
            buf_adds(b, "B-10 rd_edge");
            buf_addf(b, " case=pool/%zu", SZ[i]);
            f_bool(b, "init", pok);
            f_rd(b, "file", a, &e);
            rec_line(b);
            rec_cases++;
            yyjson_doc_free(a);
        }
    }
}

/*==============================================================================
 * MARK: - pass `wr_edge`
 *============================================================================*/

static void do_wr_edge(buf *b) {
    yyjson_doc *doc = yyjson_read("[1,2,3]", 7, YYJSON_READ_NOFLAG);
    yyjson_val *val = yyjson_doc_get_root(doc);
    yyjson_mut_doc *mdoc = yyjson_doc_mut_copy(doc, NULL);
    yyjson_mut_val *mval = yyjson_mut_doc_get_root(mdoc);

    /* ---- destinations that cannot be written ----
       A mode-555 directory is excluded for the same uid-dependence reason as
       the unreadable file in `do_rd_edge`: under uid 0 the write succeeds. */
    {
        static const char *const LBL[4] = {
            "directory", "parent_missing", "empty_path", "dot"
        };
        const char *paths[4];
        paths[0] = g_a_directory;
        paths[1] = g_missing_parent;
        paths[2] = "";
        paths[3] = ".";

        for (int i = 0; i < 4; i++) {
            yyjson_write_err e1, e2, e3, e4;
            memset(&e1, 0, sizeof(e1));
            memset(&e2, 0, sizeof(e2));
            memset(&e3, 0, sizeof(e3));
            memset(&e4, 0, sizeof(e4));
            buf_adds(b, "B-10 wr_edge");
            f_cstr(b, "case", LBL[i]);
            f_wr(b, "doc", yyjson_write_file(paths[i], doc, 0, NULL, &e1),
                 &e1, NULL);
            f_wr(b, "val", yyjson_val_write_file(paths[i], val, 0, NULL, &e2),
                 &e2, NULL);
            f_wr(b, "mdoc", yyjson_mut_write_file(paths[i], mdoc, 0, NULL,
                                                  &e3), &e3, NULL);
            f_wr(b, "mval", yyjson_mut_val_write_file(paths[i], mval, 0, NULL,
                                                      &e4), &e4, NULL);
            rec_line(b);
            rec_cases++;
        }
    }

    /* ---- NULL arguments ---- */
    {
        yyjson_write_err e;
        buf_adds(b, "B-10 wr_edge case=null_args");
        memset(&e, 0, sizeof(e));
        f_wr(b, "path_null", yyjson_write_file(NULL, doc, 0, NULL, &e), &e,
             NULL);
        memset(&e, 0, sizeof(e));
        f_wr(b, "doc_null", yyjson_write_file(g_scratch_in, NULL, 0, NULL, &e), &e,
             NULL);
        memset(&e, 0, sizeof(e));
        f_wr(b, "val_null", yyjson_val_write_file(g_scratch_in, NULL, 0, NULL, &e),
             &e, NULL);
        memset(&e, 0, sizeof(e));
        f_wr(b, "mdoc_null", yyjson_mut_write_file(g_scratch_in, NULL, 0, NULL, &e),
             &e, NULL);
        memset(&e, 0, sizeof(e));
        f_wr(b, "mval_null", yyjson_mut_val_write_file(g_scratch_in, NULL, 0, NULL,
                                                       &e), &e, NULL);
        memset(&e, 0, sizeof(e));
        f_wr(b, "fp_null", yyjson_write_fp(NULL, doc, 0, NULL, &e), &e, NULL);
        memset(&e, 0, sizeof(e));
        f_wr(b, "fp_doc_null", yyjson_write_fp(NULL, NULL, 0, NULL, &e), &e,
             NULL);
        /* no err out-parameter */
        unlink(g_scratch_in);
        f_bool(b, "noerr", yyjson_write_file(g_scratch_in, doc, 0, NULL, NULL));
        rec_line(b);
        rec_cases++;
    }

    /* ---- an empty mutable document, whose root is NULL ---- */
    {
        yyjson_mut_doc *empty = yyjson_mut_doc_new(NULL);
        yyjson_write_err e1, e2;
        memset(&e1, 0, sizeof(e1));
        memset(&e2, 0, sizeof(e2));
        unlink(g_scratch_out);
        buf_adds(b, "B-10 wr_edge case=empty_mdoc");
        f_wr(b, "mdoc", yyjson_mut_write_file(g_scratch_out, empty, 0, NULL, &e1), &e1,
             g_scratch_out);
        unlink(g_scratch_out);
        f_wr(b, "mval", yyjson_mut_val_write_file(g_scratch_out,
             yyjson_mut_doc_get_root(empty), 0, NULL, &e2), &e2, g_scratch_out);
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(empty);
    }

    /* ---- a read-only stream as the write destination ---- */
    {
        write_bytes(g_scratch_in, "x", 1);
        FILE *ro = fopen(g_scratch_in, "rb");
        yyjson_write_err e;
        memset(&e, 0, sizeof(e));
        buf_adds(b, "B-10 wr_edge case=read_only_stream");
        bool ok = ro && yyjson_write_fp(ro, doc, 0, NULL, &e);
        f_bool(b, "ok", ok);
        f_u(b, "code", (unsigned long long)e.code);
        f_cstr(b, "msg", e.msg);
        if (ro) fclose(ro);
        rec_line(b);
        rec_cases++;
    }

    /* ---- a destination that already holds longer content: the writers must
       truncate, not overwrite in place ---- */
    {
        write_bytes(g_scratch_out, "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX", 40);
        yyjson_write_err e;
        memset(&e, 0, sizeof(e));
        buf_adds(b, "B-10 wr_edge case=truncate");
        f_wr(b, "file", yyjson_write_file(g_scratch_out, doc, 0, NULL, &e), &e, g_scratch_out);
        write_bytes(g_scratch_out, "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX", 40);
        FILE *fp = fopen(g_scratch_out, "wb");
        memset(&e, 0, sizeof(e));
        bool ok = fp && yyjson_write_fp(fp, doc, 0, NULL, &e);
        if (fp) fclose(fp);
        f_wr(b, "fp", ok, &e, g_scratch_out);
        rec_line(b);
        rec_cases++;
    }

    /* ---- a writer that must fail on content: inf/nan without the flag ---- */
    {
        yyjson_mut_doc *m = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *arr = yyjson_mut_arr(m);
        yyjson_mut_doc_set_root(m, arr);
        double inf = 1.0, nan = 0.0;
        {
            uint64_t bi = 0x7FF0000000000000ULL, bn = 0x7FF8000000000000ULL;
            memcpy(&inf, &bi, 8);
            memcpy(&nan, &bn, 8);
        }
        yyjson_mut_arr_add_real(m, arr, inf);
        yyjson_mut_arr_add_real(m, arr, nan);
        static const char *const WN[3] = { "none", "allow", "asnull" };
        static const yyjson_write_flag WF[3] = {
            YYJSON_WRITE_NOFLAG,
            YYJSON_WRITE_ALLOW_INF_AND_NAN,
            YYJSON_WRITE_INF_AND_NAN_AS_NULL
        };
        for (int i = 0; i < 3; i++) {
            unlink(g_scratch_out);
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            buf_adds(b, "B-10 wr_edge");
            buf_addf(b, " case=infnan/%s", WN[i]);
            f_wr(b, "file", yyjson_mut_write_file(g_scratch_out, m, WF[i], NULL, &e),
                 &e, g_scratch_out);
            unlink(g_scratch_out);
            FILE *fp = fopen(g_scratch_out, "wb");
            memset(&e, 0, sizeof(e));
            bool ok = fp && yyjson_mut_write_fp(fp, m, WF[i], NULL, &e);
            if (fp) fclose(fp);
            f_wr(b, "fp", ok, &e, g_scratch_out);
            rec_line(b);
            rec_cases++;
        }
        yyjson_mut_doc_free(m);
    }

    /* ---- the writers on a pool allocator, including one too small ---- */
    {
        static const size_t SZ[] = { 0, 16, 32, 64, 128, 512, 4096 };
        for (size_t i = 0; i < sizeof(SZ) / sizeof(SZ[0]); i++) {
            char pool[4096];
            yyjson_alc alc;
            memset(&alc, 0, sizeof(alc));
            bool pok = yyjson_alc_pool_init(&alc, pool, SZ[i]);
            unlink(g_scratch_out);
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = pok && yyjson_write_file(g_scratch_out, doc, 0, &alc, &e);
            buf_adds(b, "B-10 wr_edge");
            buf_addf(b, " case=wpool/%zu", SZ[i]);
            f_bool(b, "init", pok);
            f_wr(b, "file", ok, &e, g_scratch_out);
            rec_line(b);
            rec_cases++;
        }
    }

    yyjson_mut_doc_free(mdoc);
    yyjson_doc_free(doc);
}

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

void mod_files(const char *corpus) {
    buf b;
    buf_init(&b);
    char path[4096];

    paths_init();

    do_rd_edge(&b);
    do_wr_edge(&b);

    static const char *SUBS[2] = { "doc", "json" };
    for (int s = 0; s < 2; s++) {
        flist fl = corpus_list(corpus, SUBS[s]);
        for (size_t i = 0; i < fl.n; i++) {
            char *dat = NULL;
            size_t len = 0;
            snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
            if (!file_slurp(path, &dat, &len)) continue;
            do_rd(&b, fl.name[i], dat, len);
            do_wr(&b, fl.name[i], dat, len);
            free(dat);
        }
        flist_free(&fl);
    }

    unlink(g_scratch_in);
    unlink(g_scratch_out);
    rmdir(g_a_directory);
    rmdir(g_dir);
    buf_free(&b);
}
