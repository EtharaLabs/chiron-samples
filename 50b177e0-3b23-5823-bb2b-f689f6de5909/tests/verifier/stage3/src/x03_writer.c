#include "s3.h"

/*==============================================================================
 * MARK: - X-03 writer escaping and formatting
 *
 * The writer's string path is a 256-entry encode-type table selected by two
 * flags (ESCAPE_UNICODE x ESCAPE_SLASHES), a 512-byte single-character escape
 * table, and a hex table selected by LOWERCASE_HEX.  Each table entry is a
 * separate branch: CPY_1, ERR_1, ESC_A, ESC_1, CPY_2, ESC_2, CPY_3, ESC_3,
 * CPY_4, ESC_4, plus the three error exits (err_one / err_cpy / err_esc) that
 * ALLOW_INVALID_UNICODE turns into a U+FFFD substitution.  A port that gets
 * one table cell wrong is correct on almost every string in existence.
 *
 * corpus/X-03/str/ therefore holds, one percent-encoded payload per line,
 * strings that hit each cell in isolation and at the boundaries of the
 * writer's unrolled copy loop (the 16-byte and 4-byte blocks, and the
 * `src + 4 > end` short-tail branch), truncated multi-byte sequences at every
 * cut point, and the overlong / surrogate / out-of-range forms that
 * `is_utf8_seq2/3/4` must reject.
 *
 * corpus/X-03/num/ holds the reals whose rendering depends on the formatting
 * flags: FP_TO_FIXED(0..16) across its whole range, FP_TO_FLOAT, and the
 * values that straddle the 1e21 notation switch.
 *
 * Every payload is written through five distinct call sites, because the
 * writer does not share one string routine between them:
 *   - the mutable document writer, with the payload as an array element;
 *   - the mutable document writer, with the payload as an OBJECT KEY (keys
 *     take a different branch and a different buffer estimate);
 *   - the mutable single-value writer, which emits a bare string;
 *   - the immutable document writer, on a copy of the same document;
 *   - the mutable document writer again with YYJSON_SUBTYPE_NOESC set, which
 *     is the branch that skips escaping entirely.
 *============================================================================*/

static void f_wr(buf *b, const char *nm, char *o, size_t ol,
                 const yyjson_write_err *e) {
    char tmp[32];
    snprintf(tmp, sizeof(tmp), "%s_code", nm);
    f_u(b, tmp, (unsigned long long)e->code);
    snprintf(tmp, sizeof(tmp), "%s_msg", nm);
    f_cstr(b, tmp, e->msg);
    f_bytes(b, nm, o, o ? ol : 0);
}

/*==============================================================================
 * MARK: - strings
 *============================================================================*/

static void do_str(buf *b, const char *key, const char *s, size_t n) {
    for (size_t f = 0; f < WFLAGS_N; f++) {
        yyjson_write_flag flg = WFLAGS[f].flg;

        buf_adds(b, "X-03 str");
        f_cstr(b, "case", key);
        f_cstr(b, "w", WFLAGS[f].name);
        buf_addf(b, " flg=0x%08X", (unsigned)flg);
        f_u(b, "slen", (unsigned long long)n);

        /* An array holding the payload, and an object keyed by it. */
        yyjson_mut_doc *md = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *arr = yyjson_mut_arr(md);
        yyjson_mut_val *sv = yyjson_mut_strncpy(md, s, n);
        yyjson_mut_arr_append(arr, sv);
        yyjson_mut_doc_set_root(md, arr);

        yyjson_mut_doc *kd = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *obj = yyjson_mut_obj(kd);
        yyjson_mut_obj_add(obj, yyjson_mut_strncpy(kd, s, n),
                           yyjson_mut_strncpy(kd, s, n));
        yyjson_mut_doc_set_root(kd, obj);

        size_t ol;
        yyjson_write_err we;

        ol = 0; memset(&we, 0, sizeof(we));
        char *o1 = yyjson_mut_write_opts(md, flg, NULL, &ol, &we);
        f_wr(b, "arr", o1, ol, &we);
        free(o1);

        ol = 0; memset(&we, 0, sizeof(we));
        char *o2 = yyjson_mut_write_opts(kd, flg, NULL, &ol, &we);
        f_wr(b, "key", o2, ol, &we);
        free(o2);

        ol = 0; memset(&we, 0, sizeof(we));
        char *o3 = yyjson_mut_val_write_opts(sv, flg, NULL, &ol, &we);
        f_wr(b, "bare", o3, ol, &we);
        free(o3);

        yyjson_doc *id = yyjson_mut_doc_imut_copy(md, NULL);
        ol = 0; memset(&we, 0, sizeof(we));
        char *o4 = yyjson_write_opts(id, flg, NULL, &ol, &we);
        f_wr(b, "imut", o4, ol, &we);
        free(o4);
        /* the immutable single-value writer, a fifth distinct call site */
        ol = 0; memset(&we, 0, sizeof(we));
        yyjson_val *iv = id ? yyjson_arr_get_first(yyjson_doc_get_root(id))
                            : NULL;
        char *o5 = yyjson_val_write_opts(iv, flg, NULL, &ol, &we);
        f_wr(b, "ibare", o5, ol, &we);
        free(o5);
        yyjson_doc_free(id);

        f_bool(b, "noesc", yyjson_mut_set_str_noesc(sv, true));
        ol = 0; memset(&we, 0, sizeof(we));
        char *o6 = yyjson_mut_write_opts(md, flg, NULL, &ol, &we);
        f_wr(b, "nesc", o6, ol, &we);
        free(o6);

        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(kd);
        yyjson_mut_doc_free(md);
    }

    /* A raw value carrying the same bytes: the raw path bypasses escaping
       entirely and must emit the payload verbatim, which is the cheapest way
       to notice a port that escapes raw values or validates them. */
    {
        yyjson_mut_doc *md = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *rv = yyjson_mut_rawncpy(md, s, n);
        yyjson_mut_doc_set_root(md, rv);
        buf_adds(b, "X-03 raw");
        f_cstr(b, "case", key);
        for (size_t f = 0; f < WFLAGS_CORE_N; f++) {
            size_t ol = 0;
            yyjson_write_err we;
            memset(&we, 0, sizeof(we));
            char *o = yyjson_mut_write_opts(md, WFLAGS_CORE[f].flg, NULL, &ol,
                                            &we);
            buf_addc(b, ' ');
            buf_adds(b, WFLAGS_CORE[f].name);
            buf_addc(b, '=');
            if (!o) buf_addf(b, "E%u", (unsigned)we.code);
            else { buf_addf(b, "%zu/", ol); buf_q(b, o, ol); }
            free(o);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(md);
    }
}

/*==============================================================================
 * MARK: - numbers
 *============================================================================*/

static void do_num(buf *b, const char *key, const char *txt, size_t n) {
    char *in = pad_copy(txt, n);
    yyjson_doc *d = yyjson_read_opts(in, n,
                        YYJSON_READ_ALLOW_INF_AND_NAN |
                        YYJSON_READ_ALLOW_EXT_NUMBER, NULL, NULL);
    free(in);
    if (!d) {
        buf_adds(b, "X-03 num");
        f_cstr(b, "case", key);
        buf_adds(b, " !unreadable");
        rec_line(b);
        rec_cases++;
        return;
    }
    yyjson_val *root = yyjson_doc_get_root(d);
    yyjson_mut_doc *md = yyjson_doc_mut_copy(d, NULL);
    yyjson_mut_val *mroot = yyjson_mut_doc_get_root(md);

    buf_adds(b, "X-03 num");
    f_cstr(b, "case", key);
    f_str(b, "in", txt, n);
    for (size_t f = 0; f < WFLAGS_N; f++) {
        size_t ol = 0;
        yyjson_write_err we;
        memset(&we, 0, sizeof(we));
        char *o = yyjson_val_write_opts(root, WFLAGS[f].flg, NULL, &ol, &we);
        buf_addc(b, ' ');
        buf_adds(b, WFLAGS[f].name);
        buf_addc(b, '=');
        if (!o) buf_addf(b, "E%u", (unsigned)we.code);
        else { buf_addf(b, "%zu/", ol); buf_q(b, o, ol); }
        free(o);
    }
    rec_line(b);
    rec_cases++;

    /* The per-value formatting overrides, which are a different mechanism
       from the write flags and interact with them: a value carrying
       `fp_to_fixed(k)` written under `FP_TO_FLOAT`, and so on across the
       whole documented precision range. */
    for (int prec = -1; prec <= 16; prec++) {
        yyjson_mut_doc *m2 = yyjson_doc_mut_copy(d, NULL);
        yyjson_mut_val *v2 = yyjson_mut_doc_get_root(m2);
        buf_adds(b, "X-03 numset");
        f_cstr(b, "case", key);
        f_i(b, "prec", prec);
        if (prec < 0) {
            f_bool(b, "set", yyjson_mut_set_fp_to_float(v2, true));
        } else {
            f_bool(b, "set", yyjson_mut_set_fp_to_fixed(v2, prec));
        }
        for (size_t f = 0; f < WFLAGS_NUM_N; f++) {
            size_t ol = 0;
            yyjson_write_err we;
            memset(&we, 0, sizeof(we));
            char *o = yyjson_mut_val_write_opts(v2, WFLAGS_NUM[f].flg, NULL,
                                                &ol, &we);
            buf_addc(b, ' ');
            buf_adds(b, WFLAGS_NUM[f].name);
            buf_addc(b, '=');
            if (!o) buf_addf(b, "E%u", (unsigned)we.code);
            else { buf_addf(b, "%zu/", ol); buf_q(b, o, ol); }
            free(o);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(m2);
    }

    /* `yyjson_mut_set_float` / `set_double` / `set_real` on the same value:
       three entry points that must produce three specific tags. */
    {
        buf_adds(b, "X-03 numsub");
        f_cstr(b, "case", key);
        double dv = yyjson_get_num(root);
        yyjson_mut_doc *m3 = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *a = yyjson_mut_real(m3, dv);
        yyjson_mut_val *c = yyjson_mut_real(m3, dv);
        yyjson_mut_val *e = yyjson_mut_real(m3, dv);
        f_bool(b, "sf", yyjson_mut_set_float(a, (float)dv));
        f_bool(b, "sd", yyjson_mut_set_double(c, dv));
        f_bool(b, "sr", yyjson_mut_set_real(e, dv));
        f_tree_mut(b, "ta", a);
        f_tree_mut(b, "tc", c);
        f_tree_mut(b, "te", e);
        for (size_t f = 0; f < WFLAGS_NUM_N; f++) {
            size_t ol = 0;
            char *o = yyjson_mut_val_write_opts(a, WFLAGS_NUM[f].flg, NULL,
                                                &ol, NULL);
            buf_addc(b, ' ');
            buf_adds(b, WFLAGS_NUM[f].name);
            buf_addc(b, '=');
            if (!o) buf_adds(b, "E");
            else { buf_addf(b, "%zu/", ol); buf_q(b, o, ol); }
            free(o);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(m3);
    }

    (void)mroot;
    yyjson_mut_doc_free(md);
    yyjson_doc_free(d);
}

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

static void run_sub(const char *corpus, const char *sub,
                    void (*fn)(buf *, const char *, const char *, size_t)) {
    flist fl = corpus_list(corpus, sub);
    buf b;
    buf_init(&b);
    char path[4096];
    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;
        size_t pos = 0, lineno = 0;
        const char *ln;
        size_t lnlen;
        while (s3_next_line(dat, len, &pos, &lineno, &ln, &lnlen)) {
            size_t tl = 0;
            char *txt = s3_unq(ln, lnlen, &tl);
            char key[4200];
            snprintf(key, sizeof(key), "%s:%zu", fl.name[i], lineno);
            fn(&b, key, txt, tl);
            free(txt);
        }
        free(dat);
    }
    flist_free(&fl);
    buf_free(&b);
}

void mod_x03(const char *corpus) {
    run_sub(corpus, "X-03/str", do_str);
    run_sub(corpus, "X-03/num", do_num);
}
