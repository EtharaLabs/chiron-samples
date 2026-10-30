#include "s3.h"

/*==============================================================================
 * MARK: - X-02 numeric round trip
 *
 * Each line of each file under corpus/X-02/ is one decimal (or hex, or
 * Inf/NaN) literal whose correct `double` the fast paths cannot settle:
 * exact halfway cases between two adjacent doubles, 17-digit and 768-digit
 * significands, subnormals and the smallest denormal, the exponent extremes
 * where the result is 0 or infinity, values that straddle 1e21 (where the
 * writer switches notation), -0.0, and the integers at the u64 / i64
 * boundaries where the reader must choose between uint, sint and real.
 *
 * The discriminating measurement is a closed loop:
 *
 *     text -> read -> 64 raw bits -> write -> text' -> read -> 64 raw bits'
 *
 * and the record carries all four of `bits`, `text'`, `bits'` and whether
 * `bits == bits'`.  A port whose reader is one ulp out produces a different
 * `bits`; a port whose writer picks the wrong shortest representation
 * produces a different `text'`; a port that is wrong in both directions by
 * compensating amounts still fails, because `text'` is compared byte for byte
 * against C's and not merely re-read.
 *
 * On top of the loop: the stage-2 read-flag sweep (which changes how a
 * literal is tagged: uint / sint / real / raw / rejected), the stage-2
 * write-flag sweep (which includes FP_TO_FLOAT and FP_TO_FIXED(1..15)), and
 * the single-value `yyjson_read_number` / `yyjson_write_number` entry points,
 * whose consumed-length and buffer-end results are observable and are a
 * different code path from the document reader.
 *============================================================================*/

/** 8-bit tag plus the raw 64-bit payload of a scalar, as one safe token. */
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

/** The raw 64 bits of a scalar, or 0 and a `kind` discriminator. */
static uint64_t val_bits(yyjson_val *v, char *kind) {
    if (!v) { *kind = '?'; return 0; }
    if (yyjson_is_raw(v)) { *kind = 'R'; return 0; }
    if (!yyjson_is_num(v)) { *kind = 'X'; return 0; }
    if (yyjson_get_subtype(v) == YYJSON_SUBTYPE_REAL) {
        double d = yyjson_get_real(v);
        uint64_t bits;
        memcpy(&bits, &d, 8);
        *kind = 'D';
        return bits;
    }
    *kind = (yyjson_get_subtype(v) == YYJSON_SUBTYPE_SINT) ? 'S' : 'U';
    return yyjson_get_uint(v);
}

/*==============================================================================
 * MARK: - the round trip
 *============================================================================*/

/** Read `txt`, write it back under `wflg`, read the result, and record every
    intermediate.  `rflg` selects how the literal is tagged on the way in. */
static void do_roundtrip(buf *b, const char *key, const char *txt, size_t len,
                         const char *wname, yyjson_write_flag wflg) {
    const yyjson_read_flag RF = YYJSON_READ_ALLOW_INF_AND_NAN |
                                YYJSON_READ_ALLOW_EXT_NUMBER;
    buf_adds(b, "X-02 rt");
    f_cstr(b, "case", key);
    f_cstr(b, "w", wname);

    char *in = pad_copy(txt, len);
    yyjson_read_err e1;
    memset(&e1, 0, sizeof(e1));
    yyjson_doc *d1 = yyjson_read_opts(in, len, RF, NULL, &e1);
    f_bool(b, "r1", d1 != NULL);
    f_u(b, "r1code", (unsigned long long)e1.code);
    f_u(b, "r1pos", (unsigned long long)e1.pos);

    if (!d1) {
        f_cstr(b, "r1msg", e1.msg);
        rec_line(b);
        rec_cases++;
        free(in);
        return;
    }

    yyjson_val *v1 = yyjson_doc_get_root(d1);
    char k1 = 0;
    uint64_t bits1 = val_bits(v1, &k1);
    buf_addf(b, " k1=%c", k1);
    f_x64(b, "bits1", (unsigned long long)bits1);

    size_t ol = 0;
    yyjson_write_err we;
    memset(&we, 0, sizeof(we));
    char *o = yyjson_val_write_opts(v1, wflg, NULL, &ol, &we);
    f_u(b, "wcode", (unsigned long long)we.code);
    f_cstr(b, "wmsg", we.msg);
    f_bytes(b, "out", o, o ? ol : 0);

    if (o) {
        /* Re-read the exact bytes the writer produced.  The second read uses
           the same flags, so a literal that round-trips must come back with
           the same tag as well as the same payload. */
        char *in2 = pad_copy(o, ol);
        yyjson_read_err e2;
        memset(&e2, 0, sizeof(e2));
        yyjson_doc *d2 = yyjson_read_opts(in2, ol, RF, NULL, &e2);
        f_bool(b, "r2", d2 != NULL);
        f_u(b, "r2code", (unsigned long long)e2.code);
        if (d2) {
            yyjson_val *v2 = yyjson_doc_get_root(d2);
            char k2 = 0;
            uint64_t bits2 = val_bits(v2, &k2);
            buf_addf(b, " k2=%c", k2);
            f_x64(b, "bits2", (unsigned long long)bits2);
            f_bool(b, "same", k1 == k2 && bits1 == bits2);
            f_bool(b, "eq", yyjson_equals(v1, v2));
        }
        yyjson_doc_free(d2);
        free(in2);
    }
    free(o);
    yyjson_doc_free(d1);
    free(in);
    rec_line(b);
    rec_cases++;
}

/*==============================================================================
 * MARK: - one literal
 *============================================================================*/

static void do_literal(buf *b, const char *key, const char *txt, size_t len) {
    /* ---- tagging under every numeric read flag ---- */
    buf_adds(b, "X-02 read");
    f_cstr(b, "case", key);
    f_str(b, "in", txt, len);
    f_u(b, "inlen", (unsigned long long)len);
    for (size_t f = 0; f < RFLAGS_NUM_N; f++) {
        char *in = pad_copy(txt, len);
        yyjson_read_err err;
        memset(&err, 0, sizeof(err));
        yyjson_doc *doc = yyjson_read_opts(in, len, RFLAGS_NUM[f].flg, NULL,
                                           &err);
        buf_addc(b, ' ');
        buf_adds(b, RFLAGS_NUM[f].name);
        buf_addc(b, '=');
        if (!doc) buf_addf(b, "E%u.%zu", (unsigned)err.code, err.pos);
        else      tok_val(b, yyjson_doc_get_root(doc));
        yyjson_doc_free(doc);
        free(in);
    }
    rec_line(b);
    rec_cases++;

    /* ---- the diagnostics, under every individual read flag ---- */
    buf_adds(b, "X-02 err");
    f_cstr(b, "case", key);
    for (size_t f = 0; f < RFLAGS_ALL_N; f++) {
        char *in = pad_copy(txt, len);
        yyjson_read_err err;
        memset(&err, 0, sizeof(err));
        yyjson_doc *doc = yyjson_read_opts(in, len, RFLAGS_ALL[f].flg, NULL,
                                           &err);
        buf_addc(b, ' ');
        buf_adds(b, RFLAGS_ALL[f].name);
        buf_addc(b, '=');
        buf_addf(b, "%u.%zu.", (unsigned)err.code, err.pos);
        if (err.msg) buf_q(b, err.msg, strlen(err.msg));
        else buf_adds(b, "!null");
        yyjson_doc_free(doc);
        free(in);
    }
    rec_line(b);
    rec_cases++;

    /* ---- the single-value entry points ---- */
    buf_adds(b, "X-02 rdnum");
    f_cstr(b, "case", key);
    for (size_t f = 0; f < RFLAGS_NUM_N; f++) {
        yyjson_val v;
        yyjson_read_err err;
        memset(&v, 0, sizeof(v));
        memset(&err, 0, sizeof(err));
        const char *end = yyjson_read_number(txt, &v, RFLAGS_NUM[f].flg, NULL,
                                             &err);
        buf_addc(b, ' ');
        buf_adds(b, RFLAGS_NUM[f].name);
        buf_addc(b, '=');
        if (!end) {
            buf_addf(b, "E%u.%zu", (unsigned)err.code, err.pos);
        } else {
            buf_addf(b, "%zu/", (size_t)(end - txt));
            tok_val(b, &v);
        }

        yyjson_mut_val mv;
        yyjson_read_err merr;
        memset(&mv, 0, sizeof(mv));
        memset(&merr, 0, sizeof(merr));
        const char *mend = yyjson_mut_read_number(txt, &mv,
                               RFLAGS_NUM[f].flg, NULL, &merr);
        buf_addc(b, '/');
        if (!mend) {
            buf_addf(b, "E%u.%zu", (unsigned)merr.code, merr.pos);
        } else {
            buf_addf(b, "%zu/", (size_t)(mend - txt));
            tok_mut_val(b, &mv);
        }
    }
    rec_line(b);
    rec_cases++;

    /* ---- the full write sweep, for the num and the raw tagging ---- */
    for (int as_raw = 0; as_raw < 2; as_raw++) {
        yyjson_read_flag rf = YYJSON_READ_ALLOW_INF_AND_NAN |
                              YYJSON_READ_ALLOW_EXT_NUMBER;
        if (as_raw) rf |= YYJSON_READ_NUMBER_AS_RAW;
        char *in = pad_copy(txt, len);
        yyjson_doc *doc = yyjson_read_opts(in, len, rf, NULL, NULL);
        if (!doc) { free(in); continue; }
        yyjson_val *root = yyjson_doc_get_root(doc);

        buf_adds(b, "X-02 write");
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
            if (!o) buf_addf(b, "E%u", (unsigned)werr.code);
            else { buf_addf(b, "%zu/", olen); buf_q(b, o, olen); }
            free(o);
        }
        rec_line(b);
        rec_cases++;

        /* `yyjson_write_number` writes into a caller buffer and returns the
           end pointer; the returned length is part of the contract. */
        {
            /* The header promises "at least 40 bytes" for a floating-point
               value, so bytes 0..40 may legitimately be used as scratch and
               are not pinned.  Bytes 40..96 must never be touched, and that
               *is* pinned: a port that writes further than C has overrun a
               caller's buffer, which is observable and is a bug. */
            char nbuf[96];
            memset(nbuf, 0, sizeof(nbuf));
            char *e = yyjson_write_number(root, nbuf);
            buf_adds(b, "X-02 wrnum");
            f_cstr(b, "case", key);
            f_cstr(b, "mode", as_raw ? "raw" : "num");
            if (!e) {
                buf_adds(b, " imut=!null");
            } else {
                buf_addf(b, " imut=%zu/", (size_t)(e - nbuf));
                buf_q(b, nbuf, (size_t)(e - nbuf));
                int clean = 1;
                for (size_t z = 40; z < sizeof(nbuf); z++)
                    if (nbuf[z]) clean = 0;
                f_bool(b, "imut_clean", clean);
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

    /* ---- the closed loop, under the notations that can change it ---- */
    static const struct { const char *name; yyjson_write_flag flg; } RT[] = {
        { "none",  YYJSON_WRITE_NOFLAG },
        { "infnan", YYJSON_WRITE_ALLOW_INF_AND_NAN },
        { "flt",   YYJSON_WRITE_FP_TO_FLOAT | YYJSON_WRITE_ALLOW_INF_AND_NAN },
        { "fix1",  YYJSON_WRITE_FP_TO_FIXED(1) |
                   YYJSON_WRITE_ALLOW_INF_AND_NAN },
        { "fix6",  YYJSON_WRITE_FP_TO_FIXED(6) |
                   YYJSON_WRITE_ALLOW_INF_AND_NAN },
        { "fix15", YYJSON_WRITE_FP_TO_FIXED(15) |
                   YYJSON_WRITE_ALLOW_INF_AND_NAN },
    };
    for (size_t f = 0; f < sizeof(RT) / sizeof(RT[0]); f++)
        do_roundtrip(b, key, txt, len, RT[f].name, RT[f].flg);
}

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

void mod_x02(const char *corpus) {
    flist fl = corpus_list(corpus, "X-02");
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
            /* %XX decoding, so a literal may carry a byte that is not legal
               in a text file (the reader's behaviour on `1\0002` is a fact
               about the reader, not about the corpus format) */
            size_t tl = 0;
            char *txt = s3_unq(ln, lnlen, &tl);
            char key[4200];
            snprintf(key, sizeof(key), "%s:%zu", fl.name[i], lineno);
            do_literal(&b, key, txt, tl);
            free(txt);
        }
        free(dat);
    }
    flist_free(&fl);
    buf_free(&b);
}
