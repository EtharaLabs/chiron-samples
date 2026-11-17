#include "common.h"

#include <unistd.h>

/*==============================================================================
 * MARK: - B-04 writer
 *
 * Every write entry point across the write-flag sweep.  Entry points:
 *   doc     yyjson_write_opts / _buf / _file / _fp
 *   val     yyjson_val_write_opts / _buf / _file / _fp
 *   mdoc    yyjson_mut_write_opts / _buf / _file / _fp
 *   mval    yyjson_mut_val_write_opts / _buf / _file / _fp
 * plus the `yyjson_write` / `yyjson_mut_write` / `yyjson_val_write` /
 * `yyjson_mut_val_write` inline wrappers.
 *
 * Recorded per case: the exact output bytes, the returned length, err.code and
 * the exact err.msg.  The four families are compared against each other on
 * the same document, so a port that gets one of them wrong diverges here even
 * when the others agree.
 *
 * Pass A covers the whole JSON corpus at WFLAGS_CORE.
 * Pass B covers corpus/wjson (hand-written cases for escapes, invalid UTF-8,
 * inf/nan, raw values, deep nesting, every fp format) at the full WFLAGS.
 *============================================================================*/

static char g_wpath[4096];

static void wpath_init(void) {
    if (g_wpath[0]) return;
    const char *d = g_tmpdir ? g_tmpdir : ".";
    snprintf(g_wpath, sizeof(g_wpath), "%s/b04.out", d);
}

/** Read back what a *_write_file / *_write_fp call put on disk. */
static void f_disk(buf *b, const char *name, bool ok,
                   const yyjson_write_err *err) {
    f_bool(b, name, ok);
    buf_addc(b, ' ');
    buf_adds(b, name);
    buf_addf(b, "_code=%u", (unsigned)err->code);
    buf_addc(b, ' ');
    buf_adds(b, name);
    buf_adds(b, "_msg=");
    if (!err->msg) buf_adds(b, "!null");
    else if (!*err->msg) buf_adds(b, "!empty");
    else buf_q(b, err->msg, strlen(err->msg));

    char *dat = NULL;
    size_t len = 0;
    char nm[64];
    snprintf(nm, sizeof(nm), "%s_disk", name);
    if (ok && file_slurp(g_wpath, &dat, &len)) {
        f_bytes(b, nm, dat, len);
    } else {
        f_str(b, nm, NULL, 0);
    }
    free(dat);
    unlink(g_wpath);
}

static void emit_opts(buf *b, const char *name, char *out, size_t olen,
                      const yyjson_write_err *err) {
    buf_addc(b, ' ');
    buf_adds(b, name);
    buf_addf(b, "_code=%u", (unsigned)err->code);
    buf_addc(b, ' ');
    buf_adds(b, name);
    buf_adds(b, "_msg=");
    if (!err->msg) buf_adds(b, "!null");
    else if (!*err->msg) buf_adds(b, "!empty");
    else buf_q(b, err->msg, strlen(err->msg));
    if (out) {
        f_bytes(b, name, out, olen);
        /* the returned length must agree with the NUL-terminated length the
           header documents */
        buf_addc(b, ' ');
        buf_adds(b, name);
        buf_addf(b, "_strlen=%zu", strlen(out));
    } else {
        f_str(b, name, NULL, 0);
        buf_addc(b, ' ');
        buf_adds(b, name);
        buf_addf(b, "_retlen=%zu", olen);
    }
}

/* `_buf` variants over caller buffer sizes: zero, one, one short of what the
   document needs, exactly enough, and generous.  On failure only the return
   value and the error are recorded, never the buffer contents, because how
   far a failing writer scribbled inside its own scratch space is not part of
   the documented contract. */
static void emit_buf_family(buf *b, yyjson_doc *doc, yyjson_val *val,
                            yyjson_mut_doc *mdoc, yyjson_mut_val *mval,
                            yyjson_write_flag flg, size_t need) {
    static const char *SZ_NAME[5] = { "zero", "one", "short", "exact", "big" };
    size_t szs[5];
    szs[0] = 0;
    szs[1] = 1;
    szs[2] = need > 1 ? need - 1 : 0;
    szs[3] = need;
    szs[4] = need + 64;
    for (int s = 0; s < 5; s++) {
        size_t cap = szs[s];
        for (int k = 0; k < 4; k++) {
            static const char *TAG[4] = { "doc", "val", "mdoc", "mval" };
            char *cb = (char *)calloc(cap + 1, 1);
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            size_t n = 0;
            switch (k) {
                case 0: n = yyjson_write_buf(cb, cap, doc, flg, &e); break;
                case 1: n = yyjson_val_write_buf(cb, cap, val, flg, &e); break;
                case 2: n = yyjson_mut_write_buf(cb, cap, mdoc, flg, &e); break;
                default:
                    n = yyjson_mut_val_write_buf(cb, cap, mval, flg, &e);
                    break;
            }
            buf_addf(b, " %s_%s=%zu/%u/", SZ_NAME[s], TAG[k], n,
                     (unsigned)e.code);
            if (!e.msg) buf_adds(b, "!null");
            else if (!*e.msg) buf_adds(b, "!empty");
            else buf_q(b, e.msg, strlen(e.msg));
            if (n) {
                char nm[64];
                snprintf(nm, sizeof(nm), "%s_%s", SZ_NAME[s], TAG[k]);
                f_bytes(b, nm, cb, n);
            }
            free(cb);
        }
    }
}

static void one(buf *b, const char *key, const char *dat, size_t len,
                const wflag_case *wf, size_t wfn) {
    /* A single parse shared by all write flags: the document under test. */
    char *in = pad_copy(dat, len);
    yyjson_read_err rerr;
    memset(&rerr, 0, sizeof(rerr));
    yyjson_read_flag rf = YYJSON_READ_ALLOW_INF_AND_NAN |
                          YYJSON_READ_ALLOW_INVALID_UNICODE |
                          YYJSON_READ_ALLOW_COMMENTS |
                          YYJSON_READ_ALLOW_TRAILING_COMMAS |
                          YYJSON_READ_ALLOW_EXT_NUMBER |
                          YYJSON_READ_ALLOW_EXT_ESCAPE |
                          YYJSON_READ_ALLOW_EXT_WHITESPACE |
                          YYJSON_READ_ALLOW_SINGLE_QUOTED_STR |
                          YYJSON_READ_ALLOW_UNQUOTED_KEY |
                          YYJSON_READ_ALLOW_BOM;
    yyjson_doc *doc = yyjson_read_opts(in, len, rf, NULL, &rerr);
    if (!doc) {
        buf_adds(b, "B-04 skip");
        f_cstr(b, "case", key);
        f_u(b, "code", (unsigned long long)rerr.code);
        f_cstr(b, "msg", rerr.msg);
        rec_line(b);
        rec_cases++;
        free(in);
        return;
    }
    yyjson_val *root = yyjson_doc_get_root(doc);
    yyjson_mut_doc *mdoc = yyjson_doc_mut_copy(doc, NULL);
    yyjson_mut_val *mroot = mdoc ? yyjson_mut_doc_get_root(mdoc) : NULL;

    wpath_init();

    for (size_t f = 0; f < wfn; f++) {
        yyjson_write_flag flg = wf[f].flg;
        size_t need = 0;

        buf_adds(b, "B-04 opts");
        f_cstr(b, "case", key);
        f_cstr(b, "w", wf[f].name);
        buf_addf(b, " flg=0x%08X", (unsigned)flg);

        {
            size_t n = 0;
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            char *o = yyjson_write_opts(doc, flg, NULL, &n, &e);
            emit_opts(b, "doc", o, n, &e);
            if (o) need = n + 1;
            free(o);
        }
        {
            size_t n = 0;
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            char *o = yyjson_val_write_opts(root, flg, NULL, &n, &e);
            emit_opts(b, "val", o, n, &e);
            free(o);
        }
        {
            size_t n = 0;
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            char *o = yyjson_mut_write_opts(mdoc, flg, NULL, &n, &e);
            emit_opts(b, "mdoc", o, n, &e);
            free(o);
        }
        {
            size_t n = 0;
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            char *o = yyjson_mut_val_write_opts(mroot, flg, NULL, &n, &e);
            emit_opts(b, "mval", o, n, &e);
            free(o);
        }
        /* the inline wrappers, which pass a NULL allocator and no err */
        {
            size_t n = 0;
            char *o = yyjson_write(doc, flg, &n);
            buf_adds(b, " iw_doc=");
            if (!o) buf_adds(b, "!null");
            else buf_addf(b, "%zu", n);
            free(o);
            n = 0;
            o = yyjson_val_write(root, flg, &n);
            buf_adds(b, " iw_val=");
            if (!o) buf_adds(b, "!null"); else buf_addf(b, "%zu", n);
            free(o);
            n = 0;
            o = yyjson_mut_write(mdoc, flg, &n);
            buf_adds(b, " iw_mdoc=");
            if (!o) buf_adds(b, "!null"); else buf_addf(b, "%zu", n);
            free(o);
            n = 0;
            o = yyjson_mut_val_write(mroot, flg, &n);
            buf_adds(b, " iw_mval=");
            if (!o) buf_adds(b, "!null"); else buf_addf(b, "%zu", n);
            free(o);
        }
        rec_line(b);
        rec_cases++;

        /* ---- _buf family ---- */
        buf_adds(b, "B-04 wbuf");
        f_cstr(b, "case", key);
        f_cstr(b, "w", wf[f].name);
        emit_buf_family(b, doc, root, mdoc, mroot, flg, need);
        rec_line(b);
        rec_cases++;

        /* ---- _file and _fp families ---- */
        buf_adds(b, "B-04 wfile");
        f_cstr(b, "case", key);
        f_cstr(b, "w", wf[f].name);
        {
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = yyjson_write_file(g_wpath, doc, flg, NULL, &e);
            f_disk(b, "doc", ok, &e);
        }
        {
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = yyjson_val_write_file(g_wpath, root, flg, NULL, &e);
            f_disk(b, "val", ok, &e);
        }
        {
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = yyjson_mut_write_file(g_wpath, mdoc, flg, NULL, &e);
            f_disk(b, "mdoc", ok, &e);
        }
        {
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = yyjson_mut_val_write_file(g_wpath, mroot, flg, NULL, &e);
            f_disk(b, "mval", ok, &e);
        }
        rec_line(b);
        rec_cases++;

        buf_adds(b, "B-04 wfp");
        f_cstr(b, "case", key);
        f_cstr(b, "w", wf[f].name);
        {
            FILE *fp = fopen(g_wpath, "wb");
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = fp && yyjson_write_fp(fp, doc, flg, NULL, &e);
            if (fp) fclose(fp);
            f_disk(b, "doc", ok, &e);
        }
        {
            FILE *fp = fopen(g_wpath, "wb");
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = fp && yyjson_val_write_fp(fp, root, flg, NULL, &e);
            if (fp) fclose(fp);
            f_disk(b, "val", ok, &e);
        }
        {
            FILE *fp = fopen(g_wpath, "wb");
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = fp && yyjson_mut_write_fp(fp, mdoc, flg, NULL, &e);
            if (fp) fclose(fp);
            f_disk(b, "mdoc", ok, &e);
        }
        {
            FILE *fp = fopen(g_wpath, "wb");
            yyjson_write_err e;
            memset(&e, 0, sizeof(e));
            bool ok = fp && yyjson_mut_val_write_fp(fp, mroot, flg, NULL, &e);
            if (fp) fclose(fp);
            f_disk(b, "mval", ok, &e);
        }
        rec_line(b);
        rec_cases++;
    }

    yyjson_mut_doc_free(mdoc);
    yyjson_doc_free(doc);
    free(in);
}

void mod_writer(const char *corpus) {
    buf b;
    buf_init(&b);
    char path[4096];

    /* ---- pass A: whole corpus, core flags ---- */
    flist fl = corpus_list(corpus, "json");
    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;
        one(&b, fl.name[i], dat, len, WFLAGS_CORE, WFLAGS_CORE_N);
        free(dat);
    }
    flist_free(&fl);

    /* ---- pass B: writer-specific corpus, full flag sweep ---- */
    flist wl = corpus_list(corpus, "wjson");
    for (size_t i = 0; i < wl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, wl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;
        one(&b, wl.name[i], dat, len, WFLAGS, WFLAGS_N);
        free(dat);
    }
    flist_free(&wl);

    buf_free(&b);
}
