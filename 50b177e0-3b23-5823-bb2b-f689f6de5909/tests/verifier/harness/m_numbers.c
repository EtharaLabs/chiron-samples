#include "common.h"

/*==============================================================================
 * MARK: - B-03 numbers
 *
 * Every numeric literal in corpus/num (one per line, '#' comments and blank
 * lines skipped, exactly the convention upstream's test_number.c uses).
 *
 * Per literal:
 *   `read`   one field per read-flag combination that changes how a number is
 *            tagged: the full 8-bit tag and the raw 64-bit payload, or
 *            E<code>.<pos> when the literal is rejected.
 *   `rdnum`  yyjson_read_number / yyjson_mut_read_number: the consumed length,
 *            the tag, the payload, and whether the result equals the value the
 *            document reader produced.
 *   `write`  one field per write-flag combination, including
 *            YYJSON_WRITE_FP_TO_FIXED(1..15) and YYJSON_WRITE_FP_TO_FLOAT.
 *   `wrnum`  yyjson_write_number / yyjson_mut_write_number.
 *   `numerr` full code/msg/pos under every individual read flag, for the
 *            literals that are rejected (this is where the number reader's
 *            diagnostics live).
 *============================================================================*/

/** 8-bit tag + raw payload of a scalar, as a single safe token. */
static void tok_val(buf *b, yyjson_val *v) {
    if (!v) { buf_adds(b, "!null"); return; }
    yyjson_type t = yyjson_get_type(v);
    buf_addf(b, "%02X.", (unsigned)yyjson_get_tag(v));
    if (t == YYJSON_TYPE_RAW) {
        const char *s = yyjson_get_raw(v);
        size_t n = yyjson_get_len(v);
        if (!s) buf_adds(b, "!null");
        else if (!n) buf_adds(b, "!empty");
        else buf_q(b, s, n);
    } else if (t == YYJSON_TYPE_NUM) {
        uint64_t bits;
        if (yyjson_get_subtype(v) == YYJSON_SUBTYPE_REAL) {
            double d = yyjson_get_real(v);
            memcpy(&bits, &d, 8);
        } else {
            bits = yyjson_get_uint(v);
        }
        buf_addf(b, "0x%016llX", (unsigned long long)bits);
    } else {
        buf_adds(b, "!nonnum");
    }
}

static void tok_mut_val(buf *b, yyjson_mut_val *v) {
    if (!v) { buf_adds(b, "!null"); return; }
    yyjson_type t = yyjson_mut_get_type(v);
    buf_addf(b, "%02X.", (unsigned)yyjson_mut_get_tag(v));
    if (t == YYJSON_TYPE_RAW) {
        const char *s = yyjson_mut_get_raw(v);
        size_t n = yyjson_mut_get_len(v);
        if (!s) buf_adds(b, "!null");
        else if (!n) buf_adds(b, "!empty");
        else buf_q(b, s, n);
    } else if (t == YYJSON_TYPE_NUM) {
        uint64_t bits;
        if (yyjson_mut_get_subtype(v) == YYJSON_SUBTYPE_REAL) {
            double d = yyjson_mut_get_real(v);
            memcpy(&bits, &d, 8);
        } else {
            bits = yyjson_mut_get_uint(v);
        }
        buf_addf(b, "0x%016llX", (unsigned long long)bits);
    } else {
        buf_adds(b, "!nonnum");
    }
}

static void do_line(buf *b, const char *file, size_t lineno,
                    const char *txt, size_t len) {
    char key[4200];
    snprintf(key, sizeof(key), "%s:%zu", file, lineno);

    /* ---- read ---- */
    buf_adds(b, "B-03 read");
    f_cstr(b, "case", key);
    f_str(b, "in", txt, len);
    for (size_t f = 0; f < RFLAGS_NUM_N; f++) {
        char *in = pad_copy(txt, len);
        yyjson_read_err err;
        memset(&err, 0, sizeof(err));
        yyjson_doc *doc =
            yyjson_read_opts(in, len, RFLAGS_NUM[f].flg, NULL, &err);
        buf_addc(b, ' ');
        buf_adds(b, RFLAGS_NUM[f].name);
        buf_addc(b, '=');
        if (!doc) {
            buf_addf(b, "E%u.%zu", (unsigned)err.code, err.pos);
        } else {
            tok_val(b, yyjson_doc_get_root(doc));
        }
        yyjson_doc_free(doc);
        free(in);
    }
    rec_line(b);
    rec_cases++;

    /* ---- yyjson_read_number / yyjson_mut_read_number ---- */
    buf_adds(b, "B-03 rdnum");
    f_cstr(b, "case", key);
    for (size_t f = 0; f < RFLAGS_NUM_N; f++) {
        yyjson_val v;
        yyjson_read_err err;
        memset(&v, 0, sizeof(v));
        memset(&err, 0, sizeof(err));
        const char *end =
            yyjson_read_number(txt, &v, RFLAGS_NUM[f].flg, NULL, &err);
        buf_addc(b, ' ');
        buf_adds(b, RFLAGS_NUM[f].name);
        buf_addc(b, '=');
        if (!end) {
            buf_addf(b, "E%u.%zu", (unsigned)err.code, err.pos);
        } else {
            buf_addf(b, "%zu/", (size_t)(end - txt));
            if ((size_t)(end - txt) == len) tok_val(b, &v);
            else buf_adds(b, "!short");
        }

        yyjson_mut_val mv;
        memset(&mv, 0, sizeof(mv));
        yyjson_read_err merr;
        memset(&merr, 0, sizeof(merr));
        const char *mend =
            yyjson_mut_read_number(txt, &mv, RFLAGS_NUM[f].flg, NULL, &merr);
        buf_addc(b, '/');
        if (!mend) {
            buf_addf(b, "E%u.%zu", (unsigned)merr.code, merr.pos);
        } else {
            buf_addf(b, "%zu/", (size_t)(mend - txt));
            if ((size_t)(mend - txt) == len) tok_mut_val(b, &mv);
            else buf_adds(b, "!short");
        }
    }
    rec_line(b);
    rec_cases++;

    /* ---- write round trip ----
       Parse with the most permissive numeric flag set, then write the value
       with every write-flag combination.  Also parse as raw, so the raw
       writing path is exercised on the same literal. */
    for (int as_raw = 0; as_raw < 2; as_raw++) {
        yyjson_read_flag rf = YYJSON_READ_ALLOW_INF_AND_NAN |
                              YYJSON_READ_ALLOW_EXT_NUMBER;
        if (as_raw) rf |= YYJSON_READ_NUMBER_AS_RAW;
        char *in = pad_copy(txt, len);
        yyjson_read_err err;
        memset(&err, 0, sizeof(err));
        yyjson_doc *doc = yyjson_read_opts(in, len, rf, NULL, &err);
        if (!doc) { free(in); continue; }
        yyjson_val *root = yyjson_doc_get_root(doc);

        buf_adds(b, "B-03 write");
        f_cstr(b, "case", key);
        f_cstr(b, "mode", as_raw ? "raw" : "num");
        for (size_t f = 0; f < WFLAGS_NUM_N; f++) {
            size_t olen = 0;
            yyjson_write_err werr;
            memset(&werr, 0, sizeof(werr));
            char *o = yyjson_val_write_opts(root, WFLAGS_NUM[f].flg, NULL,
                                            &olen, &werr);
            buf_addc(b, ' ');
            buf_adds(b, WFLAGS_NUM[f].name);
            buf_addc(b, '=');
            if (!o) {
                buf_addf(b, "E%u", (unsigned)werr.code);
            } else {
                buf_addf(b, "%zu/", olen);
                buf_q(b, o, olen);
            }
            free(o);
        }
        rec_line(b);
        rec_cases++;

        /* yyjson_write_number / yyjson_mut_write_number */
        {
            char nbuf[64];
            memset(nbuf, 0, sizeof(nbuf));
            char *e = yyjson_write_number(root, nbuf);
            buf_adds(b, "B-03 wrnum");
            f_cstr(b, "case", key);
            f_cstr(b, "mode", as_raw ? "raw" : "num");
            if (!e) {
                buf_adds(b, " imut=!null");
            } else {
                buf_addf(b, " imut=%zu/", (size_t)(e - nbuf));
                buf_q(b, nbuf, (size_t)(e - nbuf));
            }

            yyjson_mut_doc *md = yyjson_doc_mut_copy(doc, NULL);
            yyjson_mut_val *mroot = md ? yyjson_mut_doc_get_root(md) : NULL;
            char mbuf[64];
            memset(mbuf, 0, sizeof(mbuf));
            char *me = mroot ? yyjson_mut_write_number(mroot, mbuf) : NULL;
            if (!me) {
                buf_adds(b, " mut=!null");
            } else {
                buf_addf(b, " mut=%zu/", (size_t)(me - mbuf));
                buf_q(b, mbuf, (size_t)(me - mbuf));
            }
            yyjson_mut_doc_free(md);
            rec_line(b);
            rec_cases++;
        }

        yyjson_doc_free(doc);
        free(in);
    }

    /* ---- numerr: full diagnostics under every individual read flag,
       recorded whenever the default read rejects the literal ---- */
    {
        char *in = pad_copy(txt, len);
        yyjson_read_err err;
        memset(&err, 0, sizeof(err));
        yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                           &err);
        int rejected = (doc == NULL);
        yyjson_doc_free(doc);
        free(in);
        if (rejected) {
            for (size_t f = 0; f < RFLAGS_ALL_N; f++) {
                char *in2 = pad_copy(txt, len);
                yyjson_read_err e2;
                memset(&e2, 0, sizeof(e2));
                yyjson_doc *d2 =
                    yyjson_read_opts(in2, len, RFLAGS_ALL[f].flg, NULL, &e2);
                buf_adds(b, "B-03 numerr");
                f_cstr(b, "case", key);
                f_cstr(b, "f", RFLAGS_ALL[f].name);
                f_bool(b, "ok", d2 != NULL);
                f_u(b, "code", (unsigned long long)e2.code);
                f_cstr(b, "msg", e2.msg);
                f_u(b, "pos", (unsigned long long)e2.pos);
                rec_line(b);
                rec_cases++;
                yyjson_doc_free(d2);
                free(in2);
            }
        }
    }
}

void mod_numbers(const char *corpus) {
    flist fl = corpus_list(corpus, "num");
    buf b;
    buf_init(&b);
    char path[4096];

    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;

        size_t p = 0, lineno = 0;
        while (p <= len) {
            size_t q = p;
            while (q < len && dat[q] != '\n') q++;
            size_t n = q - p;
            /* strip a trailing CR so CRLF corpora behave identically */
            if (n && dat[p + n - 1] == '\r') n--;
            lineno++;
            if (n != 0 && dat[p] != '#') {
                char *txt = (char *)malloc(n + 1);
                memcpy(txt, dat + p, n);
                txt[n] = '\0';
                do_line(&b, fl.name[i], lineno, txt, n);
                free(txt);
            }
            if (q >= len) break;
            p = q + 1;
        }
        free(dat);
    }
    flist_free(&fl);
    buf_free(&b);
}
