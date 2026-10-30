#include "common.h"

/*==============================================================================
 * MARK: - B-05 mutable documents
 *
 * Five passes:
 *
 *   copy    the whole copy lattice over every file in the JSON corpus:
 *           doc_mut_copy -> mut_doc_mut_copy -> val_mut_copy ->
 *           mut_val_mut_copy -> mut_doc_imut_copy / mut_val_imut_copy,
 *           cross-checked with both equality entry points.  A copy that drops
 *           a subtype bit, mis-links a sibling chain or forgets to copy a
 *           string into the destination pool diverges here on the first file
 *           that contains the shape.
 *   direct  the parameter / NULL paths of the copy and equality entry points,
 *           which no corpus file can reach.
 *   ctor    every key and value constructor in every form, including the
 *           non-copying ones, embedded NULs, a length past the NUL, a length
 *           of zero and a NULL string.
 *   setter  every `yyjson_mut_set_*` applied to a value of each type.
 *   pool    set_str_pool_size / set_val_pool_size at sizes that succeed, that
 *           are rejected, and that force the pool to grow mid-build.  The
 *           allocator event list is recorded, so a different growth schedule
 *           is visible rather than silent.
 *   vm      mutation scripts: append / prepend / insert / remove / clear /
 *           rotate / replace and the two iterators, with the document state
 *           recorded after every single op.
 *============================================================================*/

/*==============================================================================
 * MARK: - pass `copy`
 *============================================================================*/

/* The copy lattice is run under several read flags, not just the default.
   NOFLAG alone never produces a RAW value and never admits invalid UTF-8 or
   inf/nan, so the copy and equality paths for those three would otherwise go
   completely untested: `yyjson_val_mut_copy` has a dedicated branch for
   STR/RAW, and `yyjson_mut_val_imut_copy` has to re-measure their lengths. */
static const rflag_case COPY_FLAGS[] = {
    { "none",   YYJSON_READ_NOFLAG },
    { "numraw", YYJSON_READ_NUMBER_AS_RAW },
    { "bigraw", YYJSON_READ_BIGNUM_AS_RAW },
    { "perm",   YYJSON_READ_ALLOW_INF_AND_NAN |
                YYJSON_READ_ALLOW_INVALID_UNICODE |
                YYJSON_READ_ALLOW_COMMENTS |
                YYJSON_READ_ALLOW_TRAILING_COMMAS |
                YYJSON_READ_ALLOW_EXT_NUMBER |
                YYJSON_READ_ALLOW_EXT_ESCAPE |
                YYJSON_READ_ALLOW_EXT_WHITESPACE |
                YYJSON_READ_ALLOW_SINGLE_QUOTED_STR |
                YYJSON_READ_ALLOW_UNQUOTED_KEY |
                YYJSON_READ_ALLOW_BOM },
    { "permraw", YYJSON_READ_ALLOW_INF_AND_NAN |
                 YYJSON_READ_ALLOW_INVALID_UNICODE |
                 YYJSON_READ_NUMBER_AS_RAW },
};
static const size_t COPY_FLAGS_N = sizeof(COPY_FLAGS) / sizeof(COPY_FLAGS[0]);

static void do_copy_1(buf *b, const char *key, const char *fname,
                      const char *dat, size_t len, yyjson_read_flag rf) {
    char *in = pad_copy(dat, len);
    yyjson_doc *doc = yyjson_read_opts(in, len, rf, NULL, NULL);
    if (!doc) {
        /* Not a parse failure worth a case; B-01/B-02 own the reader. */
        free(in);
        return;
    }
    yyjson_val *root = yyjson_doc_get_root(doc);

    yyjson_mut_doc *m1 = yyjson_doc_mut_copy(doc, NULL);
    yyjson_mut_doc *m2 = yyjson_mut_doc_mut_copy(m1, NULL);
    yyjson_mut_val *r1 = yyjson_mut_doc_get_root(m1);
    yyjson_mut_val *r2 = yyjson_mut_doc_get_root(m2);
    /* a value copy taken into a third, independent document */
    yyjson_mut_doc *m3 = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *v_i = yyjson_val_mut_copy(m3, root);
    yyjson_mut_val *v_m = yyjson_mut_val_mut_copy(m3, r1);
    yyjson_doc *d_doc = yyjson_mut_doc_imut_copy(m1, NULL);
    yyjson_doc *d_val = yyjson_mut_val_imut_copy(r1, NULL);

    buf_adds(b, "B-05 copy");
    f_cstr(b, "case", key);
    f_cstr(b, "f", fname);
    f_u(b, "vals", (unsigned long long)yyjson_doc_get_val_count(doc));
    f_tree_imut(b, "src", root);
    f_tree_mut(b, "m1", r1);
    f_tree_mut(b, "m2", r2);
    f_tree_mut(b, "vi", v_i);
    f_tree_mut(b, "vm", v_m);
    f_tree_imut(b, "d1", d_doc ? yyjson_doc_get_root(d_doc) : NULL);
    f_tree_imut(b, "d2", d_val ? yyjson_doc_get_root(d_val) : NULL);
    /* val_count / read_size of the immutable copies: an imut copy reports the
       value count it actually materialised and a read size of 0 */
    f_u(b, "d1vals", (unsigned long long)
        (d_doc ? yyjson_doc_get_val_count(d_doc) : 0));
    f_u(b, "d1read", (unsigned long long)
        (d_doc ? yyjson_doc_get_read_size(d_doc) : 0));
    f_u(b, "d2vals", (unsigned long long)
        (d_val ? yyjson_doc_get_val_count(d_val) : 0));
    /* equality across every pairing the two entry points can express */
    f_bool(b, "eq_ii", yyjson_equals(root, root));
    f_bool(b, "eq_id1", yyjson_equals(root, d_doc
                                      ? yyjson_doc_get_root(d_doc) : NULL));
    f_bool(b, "eq_id2", yyjson_equals(root, d_val
                                      ? yyjson_doc_get_root(d_val) : NULL));
    f_bool(b, "eq_d12", yyjson_equals(d_doc ? yyjson_doc_get_root(d_doc) : NULL,
                                      d_val ? yyjson_doc_get_root(d_val)
                                            : NULL));
    f_bool(b, "eq_mm", yyjson_mut_equals(r1, r1));
    f_bool(b, "eq_m12", yyjson_mut_equals(r1, r2));
    f_bool(b, "eq_mvi", yyjson_mut_equals(r1, v_i));
    f_bool(b, "eq_mvm", yyjson_mut_equals(r1, v_m));
    f_bool(b, "eq_vivm", yyjson_mut_equals(v_i, v_m));
    rec_line(b);
    rec_cases++;

    /* A round trip through the mutable form must write the same bytes as the
       immutable original did.  That is the cheapest end-to-end check that a
       copy preserved every tag bit, not just the shape. */
    {
        /* inf/nan and invalid UTF-8 are admitted on the write side too, so a
           document the permissive read flags let in can still be compared
           rather than collapsing to "both failed" */
        const yyjson_write_flag wf = YYJSON_WRITE_ALLOW_INF_AND_NAN |
                                     YYJSON_WRITE_ALLOW_INVALID_UNICODE;
        size_t n1 = 0, n2 = 0, n3 = 0;
        char *o1 = yyjson_write(doc, wf, &n1);
        char *o2 = yyjson_mut_write(m1, wf, &n2);
        char *o3 = yyjson_mut_write(m2, wf, &n3);
        buf_adds(b, "B-05 rt");
        f_cstr(b, "case", key);
        f_cstr(b, "f", fname);
        f_bytes(b, "imut", o1, o1 ? n1 : 0);
        f_bool(b, "same_m1", (o1 && o2 && n1 == n2 && memcmp(o1, o2, n1) == 0));
        f_bool(b, "same_m2", (o1 && o3 && n1 == n3 && memcmp(o1, o3, n1) == 0));
        free(o1);
        free(o2);
        free(o3);
        rec_line(b);
        rec_cases++;
    }

    yyjson_doc_free(d_val);
    yyjson_doc_free(d_doc);
    yyjson_mut_doc_free(m3);
    yyjson_mut_doc_free(m2);
    yyjson_mut_doc_free(m1);
    yyjson_doc_free(doc);
    free(in);
}

static void do_copy(buf *b, const char *key, const char *dat, size_t len) {
    for (size_t f = 0; f < COPY_FLAGS_N; f++)
        do_copy_1(b, key, COPY_FLAGS[f].name, dat, len, COPY_FLAGS[f].flg);
}

/*==============================================================================
 * MARK: - pass `direct`
 *============================================================================*/

static void do_direct(buf *b) {
    yyjson_mut_doc *md = yyjson_mut_doc_new(NULL);
    yyjson_doc *doc = yyjson_read("[1,\"a\",{\"k\":2}]", 15,
                                  YYJSON_READ_NOFLAG);
    yyjson_val *root = yyjson_doc_get_root(doc);
    yyjson_mut_val *mv = yyjson_val_mut_copy(md, root);

    buf_adds(b, "B-05 direct case=copy_null");
    f_bool(b, "doc_mut_null", yyjson_doc_mut_copy(NULL, NULL) != NULL);
    f_bool(b, "mdoc_mut_null", yyjson_mut_doc_mut_copy(NULL, NULL) != NULL);
    f_bool(b, "val_mut_doc_null", yyjson_val_mut_copy(NULL, root) != NULL);
    f_bool(b, "val_mut_val_null", yyjson_val_mut_copy(md, NULL) != NULL);
    f_bool(b, "mval_mut_doc_null", yyjson_mut_val_mut_copy(NULL, mv) != NULL);
    f_bool(b, "mval_mut_val_null", yyjson_mut_val_mut_copy(md, NULL) != NULL);
    f_bool(b, "mdoc_imut_null", yyjson_mut_doc_imut_copy(NULL, NULL) != NULL);
    f_bool(b, "mval_imut_null", yyjson_mut_val_imut_copy(NULL, NULL) != NULL);
    rec_line(b);
    rec_cases++;

    /* An empty mutable document: root is NULL, so both imut_copy entry points
       see a NULL root.  The two disagree on purpose in the C (one takes the
       doc, one takes the value), and that difference is worth pinning. */
    {
        yyjson_mut_doc *empty = yyjson_mut_doc_new(NULL);
        yyjson_doc *a = yyjson_mut_doc_imut_copy(empty, NULL);
        yyjson_doc *c = yyjson_mut_val_imut_copy(
            yyjson_mut_doc_get_root(empty), NULL);
        buf_adds(b, "B-05 direct case=imut_copy_empty");
        f_bool(b, "doc_ok", a != NULL);
        if (a) {
            f_bool(b, "doc_root", yyjson_doc_get_root(a) != NULL);
            f_u(b, "doc_vals", (unsigned long long)
                yyjson_doc_get_val_count(a));
            f_tree_imut(b, "doc_tree", yyjson_doc_get_root(a));
        }
        f_bool(b, "val_ok", c != NULL);
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(a);
        yyjson_doc_free(c);
        yyjson_mut_doc_free(empty);
    }

    buf_adds(b, "B-05 direct case=equals_null");
    f_bool(b, "ii_nn", yyjson_equals(NULL, NULL));
    f_bool(b, "ii_ln", yyjson_equals(root, NULL));
    f_bool(b, "ii_nr", yyjson_equals(NULL, root));
    f_bool(b, "mm_nn", yyjson_mut_equals(NULL, NULL));
    f_bool(b, "mm_ln", yyjson_mut_equals(mv, NULL));
    f_bool(b, "mm_nr", yyjson_mut_equals(NULL, mv));
    rec_line(b);
    rec_cases++;

    /* equals_str / equals_strn over a value of every type, with the key
       variants that carry an explicit length */
    {
        static const char *SRC =
            "[null,true,false,0,-1,1.5,\"\",\"abc\",[],{},[1],{\"a\":1}]";
        yyjson_doc *d2 = yyjson_read(SRC, strlen(SRC), YYJSON_READ_NOFLAG);
        yyjson_mut_doc *m2 = yyjson_doc_mut_copy(d2, NULL);
        yyjson_val *a2 = yyjson_doc_get_root(d2);
        yyjson_mut_val *ma2 = yyjson_mut_doc_get_root(m2);
        size_t n = yyjson_arr_size(a2);
        static const char *PROBE[] = { "", "abc", "ab", "abcd", "null",
                                       "true", "0" };
        for (size_t i = 0; i < n; i++) {
            yyjson_val *iv = yyjson_arr_get(a2, i);
            yyjson_mut_val *mvv = yyjson_mut_arr_get(ma2, i);
            buf_adds(b, "B-05 direct");
            buf_addf(b, " case=equals_str/%zu", i);
            f_tree_imut(b, "v", iv);
            for (size_t p = 0; p < sizeof(PROBE) / sizeof(PROBE[0]); p++) {
                size_t pl = strlen(PROBE[p]);
                char nm[32];
                snprintf(nm, sizeof(nm), "i%zu", p);
                f_bool(b, nm, yyjson_equals_str(iv, PROBE[p]));
                snprintf(nm, sizeof(nm), "in%zu", p);
                f_bool(b, nm, yyjson_equals_strn(iv, PROBE[p], pl));
                snprintf(nm, sizeof(nm), "m%zu", p);
                f_bool(b, nm, yyjson_mut_equals_str(mvv, PROBE[p]));
                snprintf(nm, sizeof(nm), "mn%zu", p);
                f_bool(b, nm, yyjson_mut_equals_strn(mvv, PROBE[p], pl));
            }
            /* NULL string arguments */
            f_bool(b, "inull", yyjson_equals_str(iv, NULL));
            f_bool(b, "mnull", yyjson_mut_equals_str(mvv, NULL));
            /* a length that runs past the NUL, and a zero length */
            f_bool(b, "ilong", yyjson_equals_strn(iv, "abc", 4));
            f_bool(b, "izero", yyjson_equals_strn(iv, "abc", 0));
            f_bool(b, "mlong", yyjson_mut_equals_strn(mvv, "abc", 4));
            f_bool(b, "mzero", yyjson_mut_equals_strn(mvv, "abc", 0));
            rec_line(b);
            rec_cases++;
        }
        /* cross-type equality: every ordered pair of the twelve values */
        for (size_t i = 0; i < n; i++) {
            buf_adds(b, "B-05 direct");
            buf_addf(b, " case=equals_cross/%zu", i);
            for (size_t j = 0; j < n; j++) {
                char nm[32];
                snprintf(nm, sizeof(nm), "i%zu", j);
                f_bool(b, nm, yyjson_equals(yyjson_arr_get(a2, i),
                                            yyjson_arr_get(a2, j)));
                snprintf(nm, sizeof(nm), "m%zu", j);
                f_bool(b, nm, yyjson_mut_equals(yyjson_mut_arr_get(ma2, i),
                                                yyjson_mut_arr_get(ma2, j)));
            }
            rec_line(b);
            rec_cases++;
        }
        yyjson_mut_doc_free(m2);
        yyjson_doc_free(d2);
    }

    yyjson_mut_doc_free(md);
    yyjson_doc_free(doc);
}

/*==============================================================================
 * MARK: - pass `ctor`
 *============================================================================*/

/* Strings with an embedded NUL, invalid UTF-8 and a zero length: the cases
   where `_strn` and `_str` must part ways. */
static const struct { const char *name, *dat; size_t len; } CSTR[] = {
    { "empty",  "",                0 },
    { "ascii",  "abc",             3 },
    { "nul",    "a\0b",            3 },
    { "nul0",   "\0ab",            3 },
    { "utf8",   "\xc3\xa9\xe4\xb8\xad", 5 },
    { "bad",    "a\x80\xc3",       3 },
    { "quote",  "a\"b\\c\nd",      7 },
    { "short",  "abcdef",          2 },  /* len < strlen */
    { "long",   "abc\0xy",         6 },  /* len > strlen: the bytes past the NUL are defined,
                                            so the record cannot depend on what the compiler
                                            happens to place after a shorter literal */
};
static const size_t CSTR_N = sizeof(CSTR) / sizeof(CSTR[0]);

static void do_ctor(buf *b) {
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);

    /* ---- scalar constructors ---- */
    buf_adds(b, "B-05 ctor case=scalar");
    f_tree_mut(b, "null", yyjson_mut_null(doc));
    f_tree_mut(b, "true", yyjson_mut_true(doc));
    f_tree_mut(b, "false", yyjson_mut_false(doc));
    f_tree_mut(b, "bool0", yyjson_mut_bool(doc, false));
    f_tree_mut(b, "bool1", yyjson_mut_bool(doc, true));
    f_tree_mut(b, "arr", yyjson_mut_arr(doc));
    f_tree_mut(b, "obj", yyjson_mut_obj(doc));
    /* the NULL-doc path of every one of them */
    f_bool(b, "null_nd", yyjson_mut_null(NULL) != NULL);
    f_bool(b, "true_nd", yyjson_mut_true(NULL) != NULL);
    f_bool(b, "arr_nd", yyjson_mut_arr(NULL) != NULL);
    f_bool(b, "obj_nd", yyjson_mut_obj(NULL) != NULL);
    rec_line(b);
    rec_cases++;

    /* ---- numeric constructors over the interesting magnitudes ---- */
    {
        static const uint64_t U[] = {
            0, 1, 0x7FFFFFFFFFFFFFFFULL, 0x8000000000000000ULL,
            0xFFFFFFFFFFFFFFFFULL, 123456789
        };
        static const int64_t S[] = {
            0, -1, 1, INT64_MAX, INT64_MIN, -123456789
        };
        /* reals as exact bit patterns: +0, -0, 1.0, 0.1, DBL_MAX, DBL_MIN,
           smallest subnormal, inf, -inf, nan */
        static const uint64_t R[] = {
            0x0000000000000000ULL, 0x8000000000000000ULL,
            0x3FF0000000000000ULL, 0x3FB999999999999AULL,
            0x7FEFFFFFFFFFFFFFULL, 0x0010000000000000ULL,
            0x0000000000000001ULL, 0x7FF0000000000000ULL,
            0xFFF0000000000000ULL, 0x7FF8000000000000ULL
        };
        for (size_t i = 0; i < sizeof(U) / sizeof(U[0]); i++) {
            buf_adds(b, "B-05 ctor");
            buf_addf(b, " case=uint/%zu", i);
            f_x64(b, "in", (unsigned long long)U[i]);
            f_tree_mut(b, "uint", yyjson_mut_uint(doc, U[i]));
            rec_line(b);
            rec_cases++;
        }
        for (size_t i = 0; i < sizeof(S) / sizeof(S[0]); i++) {
            buf_adds(b, "B-05 ctor");
            buf_addf(b, " case=sint/%zu", i);
            f_i(b, "in", (long long)S[i]);
            f_tree_mut(b, "sint", yyjson_mut_sint(doc, S[i]));
            f_tree_mut(b, "int", yyjson_mut_int(doc, S[i]));
            rec_line(b);
            rec_cases++;
        }
        for (size_t i = 0; i < sizeof(R) / sizeof(R[0]); i++) {
            double d;
            memcpy(&d, &R[i], 8);
            buf_adds(b, "B-05 ctor");
            buf_addf(b, " case=real/%zu", i);
            f_x64(b, "in", (unsigned long long)R[i]);
            f_tree_mut(b, "real", yyjson_mut_real(doc, d));
            f_tree_mut(b, "double", yyjson_mut_double(doc, d));
            /* float narrows: record the double that comes back out */
            f_tree_mut(b, "float", yyjson_mut_float(doc, (float)d));
            rec_line(b);
            rec_cases++;
        }
    }

    /* ---- string and raw constructors, all eight forms ---- */
    for (size_t i = 0; i < CSTR_N; i++) {
        const char *s = CSTR[i].dat;
        size_t n = CSTR[i].len;
        buf_adds(b, "B-05 ctor");
        buf_addf(b, " case=str/%s", CSTR[i].name);
        f_str(b, "in", s, n);
        f_u(b, "len", (unsigned long long)n);
        f_tree_mut(b, "str", yyjson_mut_str(doc, s));
        f_tree_mut(b, "strn", yyjson_mut_strn(doc, s, n));
        f_tree_mut(b, "strcpy", yyjson_mut_strcpy(doc, s));
        f_tree_mut(b, "strncpy", yyjson_mut_strncpy(doc, s, n));
        f_tree_mut(b, "raw", yyjson_mut_raw(doc, s));
        f_tree_mut(b, "rawn", yyjson_mut_rawn(doc, s, n));
        f_tree_mut(b, "rawcpy", yyjson_mut_rawcpy(doc, s));
        f_tree_mut(b, "rawncpy", yyjson_mut_rawncpy(doc, s, n));
        rec_line(b);
        rec_cases++;
    }
    buf_adds(b, "B-05 ctor case=str/nullarg");
    f_bool(b, "str", yyjson_mut_str(doc, NULL) != NULL);
    f_bool(b, "strn", yyjson_mut_strn(doc, NULL, 0) != NULL);
    f_bool(b, "strcpy", yyjson_mut_strcpy(doc, NULL) != NULL);
    f_bool(b, "strncpy", yyjson_mut_strncpy(doc, NULL, 0) != NULL);
    f_bool(b, "raw", yyjson_mut_raw(doc, NULL) != NULL);
    f_bool(b, "rawn", yyjson_mut_rawn(doc, NULL, 0) != NULL);
    f_bool(b, "rawcpy", yyjson_mut_rawcpy(doc, NULL) != NULL);
    f_bool(b, "rawncpy", yyjson_mut_rawncpy(doc, NULL, 0) != NULL);
    f_bool(b, "str_nd", yyjson_mut_str(NULL, "a") != NULL);
    f_bool(b, "strcpy_nd", yyjson_mut_strcpy(NULL, "a") != NULL);
    rec_line(b);
    rec_cases++;

    /* ---- the array/object convenience constructors ---- */
    {
        static const uint8_t  U8[]  = { 0, 1, 255 };
        static const uint16_t U16[] = { 0, 1, 65535 };
        static const uint32_t U32[] = { 0, 1, 4294967295u };
        static const uint64_t U64[] = { 0, 1, 18446744073709551615ULL };
        static const int8_t   S8[]  = { 0, -128, 127 };
        static const int16_t  S16[] = { 0, -32768, 32767 };
        static const int32_t  S32[] = { 0, -2147483647 - 1, 2147483647 };
        static const int64_t  S64[] = { 0, INT64_MIN, INT64_MAX };
        static const bool     BL[]  = { true, false, true };
        static const float    FL[]  = { 0.0f, -1.5f, 3.5f };
        static const double   DB[]  = { 0.0, -1.5, 3.5 };
        /* the `_with_str` family takes `const char **`, so the element type
           must be `const char *`, not `const char *const` */
        static const char *SP[] = { "a", "", "bc" };

        buf_adds(b, "B-05 ctor case=arr_with");
        f_tree_mut(b, "u8", yyjson_mut_arr_with_uint8(doc, U8, 3));
        f_tree_mut(b, "u16", yyjson_mut_arr_with_uint16(doc, U16, 3));
        f_tree_mut(b, "u32", yyjson_mut_arr_with_uint32(doc, U32, 3));
        f_tree_mut(b, "u64", yyjson_mut_arr_with_uint64(doc, U64, 3));
        f_tree_mut(b, "s8", yyjson_mut_arr_with_sint8(doc, S8, 3));
        f_tree_mut(b, "s16", yyjson_mut_arr_with_sint16(doc, S16, 3));
        f_tree_mut(b, "s32", yyjson_mut_arr_with_sint32(doc, S32, 3));
        f_tree_mut(b, "s64", yyjson_mut_arr_with_sint64(doc, S64, 3));
        f_tree_mut(b, "uint", yyjson_mut_arr_with_uint(doc, U64, 3));
        f_tree_mut(b, "sint", yyjson_mut_arr_with_sint(doc, S64, 3));
        f_tree_mut(b, "bool", yyjson_mut_arr_with_bool(doc, BL, 3));
        f_tree_mut(b, "float", yyjson_mut_arr_with_float(doc, FL, 3));
        f_tree_mut(b, "double", yyjson_mut_arr_with_double(doc, DB, 3));
        f_tree_mut(b, "real", yyjson_mut_arr_with_real(doc, DB, 3));
        f_tree_mut(b, "str", yyjson_mut_arr_with_str(doc, SP, 3));
        f_tree_mut(b, "strcpy", yyjson_mut_arr_with_strcpy(doc, SP, 3));
        rec_line(b);
        rec_cases++;

        /* count 0 and a NULL vector: the documented early-outs */
        buf_adds(b, "B-05 ctor case=arr_with_edge");
        f_tree_mut(b, "zero", yyjson_mut_arr_with_uint8(doc, U8, 0));
        f_bool(b, "nullvec",
               yyjson_mut_arr_with_uint8(doc, NULL, 3) != NULL);
        f_bool(b, "nulldoc",
               yyjson_mut_arr_with_uint8(NULL, U8, 3) != NULL);
        f_tree_mut(b, "zero_str", yyjson_mut_arr_with_str(doc, SP, 0));
        rec_line(b);
        rec_cases++;

        /* strn / strncpy forms take a parallel length vector */
        {
            static const char *S2[] = { "abc", "a\0c", "" };
            static const size_t L2[] = { 3, 3, 0 };
            buf_adds(b, "B-05 ctor case=arr_with_strn");
            f_tree_mut(b, "strn", yyjson_mut_arr_with_strn(doc, S2, L2, 3));
            f_tree_mut(b, "strncpy",
                       yyjson_mut_arr_with_strncpy(doc, S2, L2, 3));
            f_bool(b, "nulllen",
                   yyjson_mut_arr_with_strn(doc, S2, NULL, 3) != NULL);
            rec_line(b);
            rec_cases++;
        }

        /* object bulk constructors */
        {
            static const char *K[] = { "a", "b", "" };
            static const char *V[] = { "1", "", "3" };
            static const char *KV[] = { "k1", "v1", "k2", "v2" };
            buf_adds(b, "B-05 ctor case=obj_with");
            f_tree_mut(b, "str", yyjson_mut_obj_with_str(doc, K, V, 3));
            f_tree_mut(b, "kv", yyjson_mut_obj_with_kv(doc, KV, 2));
            f_tree_mut(b, "zero", yyjson_mut_obj_with_str(doc, K, V, 0));
            f_bool(b, "nullk",
                   yyjson_mut_obj_with_str(doc, NULL, V, 3) != NULL);
            f_bool(b, "nullv",
                   yyjson_mut_obj_with_str(doc, K, NULL, 3) != NULL);
            rec_line(b);
            rec_cases++;
        }
    }

    /* ---- the arr_add_* / obj_add_* convenience families ---- */
    {
        yyjson_mut_val *arr = yyjson_mut_arr(doc);
        buf_adds(b, "B-05 ctor case=arr_add");
        f_bool(b, "val", yyjson_mut_arr_add_val(arr, yyjson_mut_uint(doc, 7)));
        f_bool(b, "null", yyjson_mut_arr_add_null(doc, arr));
        f_bool(b, "true", yyjson_mut_arr_add_true(doc, arr));
        f_bool(b, "false", yyjson_mut_arr_add_false(doc, arr));
        f_bool(b, "bool", yyjson_mut_arr_add_bool(doc, arr, true));
        f_bool(b, "uint", yyjson_mut_arr_add_uint(doc, arr, 1));
        f_bool(b, "sint", yyjson_mut_arr_add_sint(doc, arr, -1));
        f_bool(b, "int", yyjson_mut_arr_add_int(doc, arr, -2));
        f_bool(b, "float", yyjson_mut_arr_add_float(doc, arr, 1.5f));
        f_bool(b, "double", yyjson_mut_arr_add_double(doc, arr, 2.5));
        f_bool(b, "real", yyjson_mut_arr_add_real(doc, arr, 3.5));
        f_bool(b, "str", yyjson_mut_arr_add_str(doc, arr, "s"));
        f_bool(b, "strn", yyjson_mut_arr_add_strn(doc, arr, "s\0t", 3));
        f_bool(b, "strcpy", yyjson_mut_arr_add_strcpy(doc, arr, "c"));
        f_bool(b, "strncpy", yyjson_mut_arr_add_strncpy(doc, arr, "c\0d", 3));
        f_bool(b, "subarr", yyjson_mut_arr_add_arr(doc, arr) != NULL);
        f_bool(b, "subobj", yyjson_mut_arr_add_obj(doc, arr) != NULL);
        /* the not-an-array and NULL paths */
        f_bool(b, "notarr", yyjson_mut_arr_add_null(doc,
                                                    yyjson_mut_obj(doc)));
        f_bool(b, "nullarr", yyjson_mut_arr_add_null(doc, NULL));
        f_bool(b, "nullval", yyjson_mut_arr_add_val(arr, NULL));
        f_u(b, "size", (unsigned long long)yyjson_mut_arr_size(arr));
        f_tree_mut(b, "tree", arr);
        rec_line(b);
        rec_cases++;

        yyjson_mut_val *obj = yyjson_mut_obj(doc);
        buf_adds(b, "B-05 ctor case=obj_add");
        f_bool(b, "null", yyjson_mut_obj_add_null(doc, obj, "a"));
        f_bool(b, "true", yyjson_mut_obj_add_true(doc, obj, "b"));
        f_bool(b, "false", yyjson_mut_obj_add_false(doc, obj, "c"));
        f_bool(b, "bool", yyjson_mut_obj_add_bool(doc, obj, "d", false));
        f_bool(b, "uint", yyjson_mut_obj_add_uint(doc, obj, "e", 1));
        f_bool(b, "sint", yyjson_mut_obj_add_sint(doc, obj, "f", -1));
        f_bool(b, "int", yyjson_mut_obj_add_int(doc, obj, "g", -2));
        f_bool(b, "float", yyjson_mut_obj_add_float(doc, obj, "h", 1.5f));
        f_bool(b, "double", yyjson_mut_obj_add_double(doc, obj, "i", 2.5));
        f_bool(b, "real", yyjson_mut_obj_add_real(doc, obj, "j", 3.5));
        f_bool(b, "str", yyjson_mut_obj_add_str(doc, obj, "k", "s"));
        f_bool(b, "strn", yyjson_mut_obj_add_strn(doc, obj, "l", "s\0t", 3));
        f_bool(b, "strcpy", yyjson_mut_obj_add_strcpy(doc, obj, "m", "c"));
        f_bool(b, "strncpy",
               yyjson_mut_obj_add_strncpy(doc, obj, "n", "c\0d", 3));
        f_bool(b, "subarr", yyjson_mut_obj_add_arr(doc, obj, "o") != NULL);
        f_bool(b, "subobj", yyjson_mut_obj_add_obj(doc, obj, "p") != NULL);
        f_bool(b, "val", yyjson_mut_obj_add_val(doc, obj, "q",
                                                yyjson_mut_uint(doc, 9)));
        /* duplicate key: documented to be allowed by the add_* family */
        f_bool(b, "dup", yyjson_mut_obj_add_uint(doc, obj, "a", 2));
        f_bool(b, "nullkey", yyjson_mut_obj_add_null(doc, obj, NULL));
        f_bool(b, "nullobj", yyjson_mut_obj_add_null(doc, NULL, "z"));
        f_bool(b, "notobj", yyjson_mut_obj_add_null(doc,
                                                    yyjson_mut_arr(doc), "z"));
        f_u(b, "size", (unsigned long long)yyjson_mut_obj_size(obj));
        f_tree_mut(b, "tree", obj);
        rec_line(b);
        rec_cases++;
    }

    yyjson_mut_doc_free(doc);
}

/*==============================================================================
 * MARK: - pass `setter`
 *============================================================================*/

static void do_setter(buf *b) {
    static const char *SRC =
        "[null,true,0,-1,1.5,\"abc\",[1],{\"a\":1}]";
    static const char *SETTER[] = {
        "null", "bool", "uint", "sint", "int", "float", "double", "real",
        "str", "strn", "str_noesc", "raw", "fp_to_fixed", "fp_to_float"
    };
    static const size_t SETTER_N = sizeof(SETTER) / sizeof(SETTER[0]);

    for (size_t s = 0; s < SETTER_N; s++) {
        /* a fresh document per setter, so one setter cannot colour the next */
        yyjson_doc *d = yyjson_read(SRC, strlen(SRC), YYJSON_READ_NOFLAG);
        yyjson_mut_doc *m = yyjson_doc_mut_copy(d, NULL);
        yyjson_mut_val *root = yyjson_mut_doc_get_root(m);
        size_t n = yyjson_mut_arr_size(root);
        buf_adds(b, "B-05 setter");
        f_cstr(b, "case", SETTER[s]);
        for (size_t i = 0; i < n; i++) {
            yyjson_mut_val *v = yyjson_mut_arr_get(root, i);
            char nm[32];
            snprintf(nm, sizeof(nm), "b%zu", i);
            f_tree_mut(b, nm, v);
            switch (s) {
                case 0: yyjson_mut_set_null(v); break;
                case 1: yyjson_mut_set_bool(v, true); break;
                case 2: yyjson_mut_set_uint(v, 42); break;
                case 3: yyjson_mut_set_sint(v, -42); break;
                case 4: yyjson_mut_set_int(v, -43); break;
                case 5: yyjson_mut_set_float(v, 1.25f); break;
                case 6: yyjson_mut_set_double(v, 1.25); break;
                case 7: yyjson_mut_set_real(v, 2.25); break;
                case 8: yyjson_mut_set_str(v, "zz"); break;
                case 9: yyjson_mut_set_strn(v, "z\0z", 3); break;
                case 10: yyjson_mut_set_str_noesc(v, true); break;
                case 11: yyjson_mut_set_raw(v, "1e3", 3); break;
                case 12: yyjson_mut_set_fp_to_fixed(v, 3); break;
                default: yyjson_mut_set_fp_to_float(v, true); break;
            }
            snprintf(nm, sizeof(nm), "a%zu", i);
            f_tree_mut(b, nm, v);
            /* the write is where a changed subtype actually shows */
            snprintf(nm, sizeof(nm), "w%zu", i);
            size_t ol = 0;
            char *o = yyjson_mut_val_write(v, YYJSON_WRITE_ALLOW_INF_AND_NAN,
                                           &ol);
            f_bytes(b, nm, o, o ? ol : 0);
            free(o);
        }
        /* NULL target */
        switch (s) {
            case 0: yyjson_mut_set_null(NULL); break;
            case 8: yyjson_mut_set_str(NULL, "z"); break;
            default: break;
        }
        f_tree_mut(b, "root", root);
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(m);
        yyjson_doc_free(d);
    }

    /* set_str/set_strn/set_raw return a bool on some entry points; record the
       no-escape and fp-format flags interacting with the writer */
    {
        yyjson_mut_doc *m = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *arr = yyjson_mut_arr(m);
        yyjson_mut_doc_set_root(m, arr);
        /* a string that needs escaping, marked no-escape: the writer is then
           allowed to emit it verbatim */
        yyjson_mut_val *s1 = yyjson_mut_strcpy(m, "a\"b");
        yyjson_mut_set_str_noesc(s1, true);
        yyjson_mut_arr_append(arr, s1);
        yyjson_mut_val *s2 = yyjson_mut_strcpy(m, "a\"b");
        yyjson_mut_set_str_noesc(s2, false);
        yyjson_mut_arr_append(arr, s2);
        for (int p = 0; p <= 16; p++) {
            yyjson_mut_val *r = yyjson_mut_real(m, 1.0 / 3.0);
            yyjson_mut_set_fp_to_fixed(r, p);
            yyjson_mut_arr_append(arr, r);
        }
        yyjson_mut_val *f1 = yyjson_mut_real(m, 1.0 / 3.0);
        yyjson_mut_set_fp_to_float(f1, true);
        yyjson_mut_arr_append(arr, f1);
        buf_adds(b, "B-05 setter case=flags");
        f_tree_mut(b, "tree", arr);
        for (size_t w = 0; w < WFLAGS_CORE_N; w++) {
            size_t ol = 0;
            char *o = yyjson_mut_write(m, WFLAGS_CORE[w].flg, &ol);
            char nm[32];
            snprintf(nm, sizeof(nm), "w_%s", WFLAGS_CORE[w].name);
            f_bytes(b, nm, o, o ? ol : 0);
            free(o);
        }
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(m);
    }
}

/*==============================================================================
 * MARK: - pass `pool`
 *============================================================================*/

static void do_pool(buf *b) {
    static const size_t SZ[] = {
        0, 1, 2, 7, 8, 15, 16, 17, 31, 32, 63, 64, 128, 256, 1024, 65536,
        (size_t)-1, ((size_t)-1) / 2
    };
    static const size_t SZ_N = sizeof(SZ) / sizeof(SZ[0]);

    for (size_t i = 0; i < SZ_N; i++) {
        for (int which = 0; which < 2; which++) {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            yyjson_mut_doc *doc = yyjson_mut_doc_new(&alc);
            bool ok = which == 0
                ? yyjson_mut_doc_set_str_pool_size(doc, SZ[i])
                : yyjson_mut_doc_set_val_pool_size(doc, SZ[i]);

            buf_adds(b, "B-05 pool");
            buf_addf(b, " case=%s/%zu", which == 0 ? "str" : "val", i);
            f_u(b, "sz", (unsigned long long)SZ[i]);
            f_bool(b, "set", ok);

            /* Build a document that needs many values and many string bytes,
               so the pool has to grow several times.  The allocator event list
               makes the growth schedule itself part of the record. */
            yyjson_mut_val *arr = yyjson_mut_arr(doc);
            yyjson_mut_doc_set_root(doc, arr);
            int built = 1;
            for (int k = 0; k < 40; k++) {
                char key[32];
                snprintf(key, sizeof(key), "key_%08d", k);
                yyjson_mut_val *o = yyjson_mut_obj(doc);
                if (!o || !yyjson_mut_obj_add_strcpy(doc, o, "k", key) ||
                    !yyjson_mut_obj_add_uint(doc, o, "n", (uint64_t)k) ||
                    !yyjson_mut_arr_append(arr, o)) {
                    built = 0;
                    break;
                }
            }
            f_bool(b, "built", built);
            f_u(b, "size", (unsigned long long)yyjson_mut_arr_size(arr));
            {
                size_t ol = 0;
                char *o = yyjson_mut_write(doc, YYJSON_WRITE_NOFLAG, &ol);
                f_bytes(b, "json", o, o ? ol : 0);
                free(o);
            }
            /* changing the pool size after the pool is in use */
            f_bool(b, "reset", which == 0
                   ? yyjson_mut_doc_set_str_pool_size(doc, 8)
                   : yyjson_mut_doc_set_val_pool_size(doc, 8));
            f_bool(b, "set_nulldoc", which == 0
                   ? yyjson_mut_doc_set_str_pool_size(NULL, 16)
                   : yyjson_mut_doc_set_val_pool_size(NULL, 16));
            yyjson_mut_doc_free(doc);
            f_u(b, "live", (unsigned long long)t.live);
            f_alc_trace(b, "alc", &t);
            rec_line(b);
            rec_cases++;
            alc_trace_free(&t);
        }
    }
}

/*==============================================================================
 * MARK: - pass `vm`: mutation scripts
 *============================================================================*/

/* Each script is a complete case.  `max` is SIZE_MAX, which is the
   out-of-range probe for every index parameter. */
static const struct { const char *name; const char *src; } VMS[] = {

{ "arr_build",
  "new\n"
  "arr r1\n"
  "root r1\n"
  "uint r2 1\n"
  "uint r3 2\n"
  "uint r4 3\n"
  "aapp r1 r2\n"
  "aapp r1 r3\n"
  "apre r1 r4\n"
  "asize r1\n"
  "agetf - r1\n"
  "agetl - r1\n"
  "aget - r1 0\n"
  "aget - r1 2\n"
  "aget - r1 3\n"
  "aget - r1 max\n"
},

{ "arr_insert",
  "new\n"
  "arr r1\n"
  "root r1\n"
  "uint r2 10\n"
  "ains r1 0 r2\n"
  "uint r2 11\n"
  "ains r1 0 r2\n"
  "uint r2 12\n"
  "ains r1 1 r2\n"
  "uint r2 13\n"
  "ains r1 3 r2\n"
  "uint r2 14\n"
  "ains r1 5 r2\n"
  "uint r2 15\n"
  "ains r1 max r2\n"
  "ains r1 0 !\n"
  "asize r1\n"
},

{ "arr_remove",
  "new\n"
  "parse r1 [0,1,2,3,4,5]\n"
  "root r1\n"
  "arem r2 r1 0\n"
  "arem r2 r1 3\n"
  "arem r2 r1 max\n"
  "aremf r2 r1\n"
  "areml r2 r1\n"
  "asize r1\n"
  "aremf r2 r1\n"
  "aremf r2 r1\n"
  "areml r2 r1\n"
},

{ "arr_remove_range",
  "new\n"
  "parse r1 [0,1,2,3,4,5,6,7]\n"
  "root r1\n"
  "aremr r1 0 0\n"
  "aremr r1 2 3\n"
  "aremr r1 max 1\n"
  "aremr r1 0 max\n"
  "aremr r1 1 100\n"
  "asize r1\n"
  "aremr r1 0 5\n"
  "asize r1\n"
  "aremr r1 0 0\n"
},

{ "arr_rotate",
  "new\n"
  "parse r1 [1,2,3,4,5]\n"
  "root r1\n"
  "arot r1 0\n"
  "arot r1 2\n"
  "arot r1 4\n"
  "arot r1 5\n"
  "arot r1 max\n"
  "aclr r1\n"
  "arot r1 0\n"
  "asize r1\n"
},

{ "arr_replace",
  "new\n"
  "parse r1 [1,2,3]\n"
  "root r1\n"
  "str r5 X\n"
  "arep r2 r1 0 r5\n"
  "str r5 Y\n"
  "arep r2 r1 2 r5\n"
  "str r5 Z\n"
  "arep r2 r1 3 r5\n"
  "arep r2 r1 max r5\n"
  "arep r2 r1 1 !\n"
  "dump r1\n"
},

{ "arr_clear_nested",
  "new\n"
  "parse r1 [[1,2],{\"a\":1},3]\n"
  "root r1\n"
  "aget r2 r1 0\n"
  "aclr r2\n"
  "aget r3 r1 1\n"
  "aclr r3\n"
  "oclr r3\n"
  "aclr r1\n"
  "asize r1\n"
},

{ "arr_nullarg",
  "new\n"
  "arr r1\n"
  "root r1\n"
  "aapp ! r1\n"
  "apre ! r1\n"
  "ains ! 0 r1\n"
  "arem r2 ! 0\n"
  "aremf r2 !\n"
  "areml r2 !\n"
  "aremr ! 0 1\n"
  "aclr !\n"
  "arot ! 0\n"
  "arep r2 ! 0 r1\n"
  "aget r2 ! 0\n"
  "agetf r2 !\n"
  "agetl r2 !\n"
  "asize !\n"
},

{ "arr_wrongtype",
  "new\n"
  "obj r1\n"
  "root r1\n"
  "uint r2 1\n"
  "aapp r1 r2\n"
  "apre r1 r2\n"
  "ains r1 0 r2\n"
  "arem r3 r1 0\n"
  "aclr r1\n"
  "arot r1 0\n"
  "asize r1\n"
  "oadd r1 ! r2\n"
},

{ "arr_iter",
  "new\n"
  "parse r1 [0,1,2,3,4]\n"
  "root r1\n"
  "aiter r1 max\n"
  "aiter r1 0\n"
  "aiter r1 max\n"
  "aiter r1 2\n"
  "aiter r1 max\n"
  "aiter r1 max\n"
  "aclr r1\n"
  "aiter r1 max\n"
  "aiter ! max\n"
},

{ "arr_iter_removeall",
  "new\n"
  "parse r1 [0,1,2]\n"
  "root r1\n"
  "aiter r1 0\n"
  "aiter r1 0\n"
  "aiter r1 0\n"
  "asize r1\n"
},

{ "obj_build",
  "new\n"
  "obj r1\n"
  "root r1\n"
  "str r2 a\n"
  "uint r3 1\n"
  "oadd r1 r2 r3\n"
  "str r2 b\n"
  "uint r3 2\n"
  "oadd r1 r2 r3\n"
  "osize r1\n"
  "oget - r1 a\n"
  "oget - r1 b\n"
  "oget - r1 c\n"
  "oget - r1 ~\n"
  "ogetn - r1 abc 1\n"
  "ogetn - r1 a 0\n"
},

{ "obj_dupkey",
  "new\n"
  "obj r1\n"
  "root r1\n"
  "str r2 k\n"
  "uint r3 1\n"
  "oadd r1 r2 r3\n"
  "str r2 k\n"
  "uint r3 2\n"
  "oadd r1 r2 r3\n"
  "osize r1\n"
  "oget - r1 k\n"
  "str r2 k\n"
  "uint r3 9\n"
  "oput r1 r2 r3\n"
  "osize r1\n"
  "oget - r1 k\n"
},

{ "obj_put_insert",
  "new\n"
  "parse r1 {\"a\":1,\"b\":2,\"c\":3}\n"
  "root r1\n"
  "str r2 d\n"
  "uint r3 4\n"
  "oins r1 r2 r3 0\n"
  "str r2 e\n"
  "uint r3 5\n"
  "oins r1 r2 r3 2\n"
  "str r2 f\n"
  "uint r3 6\n"
  "oins r1 r2 r3 max\n"
  "str r2 g\n"
  "uint r3 7\n"
  "oins r1 r2 r3 5\n"
  "osize r1\n"
},

{ "obj_replace_remove",
  "new\n"
  "parse r1 {\"a\":1,\"b\":2,\"a\":3,\"c\":4}\n"
  "root r1\n"
  "str r2 a\n"
  "uint r3 99\n"
  "orep r1 r2 r3\n"
  "osize r1\n"
  "oremk r4 r1 b -\n"
  "oremk r4 r1 zz -\n"
  "orems r4 r1 a -\n"
  "osize r1\n"
  "oremkn r4 r1 cxx 1\n"
  "osize r1\n"
},

{ "obj_remove_key_variants",
  "new\n"
  "parse r1 {\"\":0,\"a\":1,\"ab\":2,\"a%00b\":3}\n"
  "root r1\n"
  "oremk r2 r1 ~ -\n"
  "oremkn r2 r1 ab 1\n"
  "oremk r2 r1 ab -\n"
  "oremsn r2 r1 a%00b 3\n"
  "oremk r2 r1 ! -\n"
  "osize r1\n"
  "dump r1\n"
},

{ "obj_rename",
  "new\n"
  "parse r1 {\"a\":1,\"b\":2,\"a\":3}\n"
  "root r1\n"
  "oren a z -\n"
  "osize r1\n"
  "oren zz q -\n"
  "orenn b %20 -\n"
  "oren ! x -\n"
  "oren z ! -\n"
  "dump r1\n"
},

{ "obj_rotate",
  "new\n"
  "parse r1 {\"a\":1,\"b\":2,\"c\":3,\"d\":4}\n"
  "root r1\n"
  "orot r1 0\n"
  "orot r1 1\n"
  "orot r1 3\n"
  "orot r1 4\n"
  "orot r1 max\n"
  "oclr r1\n"
  "orot r1 0\n"
  "osize r1\n"
},

{ "obj_iter",
  "new\n"
  "parse r1 {\"a\":1,\"b\":2,\"c\":3,\"d\":4}\n"
  "root r1\n"
  "oiter r1 max\n"
  "oiter r1 0\n"
  "oiter r1 max\n"
  "oiter r1 2\n"
  "oiter r1 max\n"
  "oclr r1\n"
  "oiter r1 max\n"
  "oiter ! max\n"
},

{ "obj_badkey",
  "new\n"
  "obj r1\n"
  "root r1\n"
  "uint r2 1\n"
  "uint r3 2\n"
  "oadd r1 r2 r3\n"
  "null r2\n"
  "oadd r1 r2 r3\n"
  "arr r2\n"
  "oadd r1 r2 r3\n"
  "str r2 ok\n"
  "oadd r1 r2 !\n"
  "oadd r1 ! r3\n"
  "osize r1\n"
},

{ "nocopy_strings",
  "# the non-copying constructors keep a borrowed pointer; the VM interns the\n"
  "# bytes for the document's whole lifetime, which is the documented contract\n"
  "new\n"
  "arr r1\n"
  "root r1\n"
  "str r2 plain\n"
  "aapp r1 r2\n"
  "strn r2 a%00b 3\n"
  "aapp r1 r2\n"
  "strn r2 abcdef 2\n"
  "aapp r1 r2\n"
  "strcpy r2 copied\n"
  "aapp r1 r2\n"
  "strncpy r2 x%00y 3\n"
  "aapp r1 r2\n"
  "raw r2 1e3\n"
  "aapp r1 r2\n"
  "rawn r2 %5B1%2C2%5D 5\n"
  "aapp r1 r2\n"
  "rawcpy r2 true\n"
  "aapp r1 r2\n"
  "rawncpy r2 nullX 4\n"
  "aapp r1 r2\n"
  "write\n"
},

{ "pool_growth",
  "# a tiny value pool forces growth on nearly every constructor\n"
  "new\n"
  "valpool 1\n"
  "strpool 1\n"
  "arr r1\n"
  "root r1\n"
  "strcpy r2 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n"
  "aapp r1 r2\n"
  "strcpy r2 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n"
  "aapp r1 r2\n"
  "obj r3\n"
  "aapp r1 r3\n"
  "str r4 k\n"
  "strcpy r5 cccccccccccccccccccccccccccccccc\n"
  "oadd r3 r4 r5\n"
  "asize r1\n"
  "write\n"
},

{ "equality",
  "new\n"
  "parse r1 [1,2]\n"
  "parse r2 [1,2]\n"
  "parse r3 [1,2,3]\n"
  "parse r4 {\"a\":1}\n"
  "parse r5 {\"a\":1}\n"
  "parse r6 {\"a\":2}\n"
  "root r1\n"
  "eq r1 r2\n"
  "eq r1 r3\n"
  "eq r4 r5\n"
  "eq r4 r6\n"
  "eq r1 r4\n"
  "eq r1 !\n"
  "eq ! !\n"
  "strcpy r7 abc\n"
  "eqstr r7 abc\n"
  "eqstr r7 ab\n"
  "eqstr r7 ~\n"
  "eqstr r7 !\n"
  "eqstr r1 abc\n"
},

{ "root_churn",
  "new\n"
  "getroot r9\n"
  "uint r1 1\n"
  "root r1\n"
  "getroot r9\n"
  "arr r2\n"
  "root r2\n"
  "aapp r2 r1\n"
  "getroot r9\n"
  "root !\n"
  "getroot r9\n"
  "write\n"
  "root r2\n"
  "write\n"
  "freedoc\n"
},
};
static const size_t VMS_N = sizeof(VMS) / sizeof(VMS[0]);

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

void mod_mutable(const char *corpus) {
    buf b;
    buf_init(&b);
    char path[4096];

    do_direct(&b);
    do_ctor(&b);
    do_setter(&b);
    do_pool(&b);

    for (size_t i = 0; i < VMS_N; i++) {
        size_t n = strlen(VMS[i].src);
        char *src = (char *)malloc(n + 1);
        if (!src) { fprintf(stderr, "harness: oom\n"); exit(2); }
        memcpy(src, VMS[i].src, n + 1);
        vm_run("B-05", VMS[i].name, src, n);
        free(src);
    }

    /* the copy lattice over the whole corpus, plus the hand-written docs */
    static const char *SUBS[2] = { "doc", "json" };
    for (int s = 0; s < 2; s++) {
        flist fl = corpus_list(corpus, SUBS[s]);
        for (size_t i = 0; i < fl.n; i++) {
            char *dat = NULL;
            size_t len = 0;
            snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
            if (!file_slurp(path, &dat, &len)) continue;
            do_copy(&b, fl.name[i], dat, len);
            free(dat);
        }
        flist_free(&fl);
    }

    buf_free(&b);
}
