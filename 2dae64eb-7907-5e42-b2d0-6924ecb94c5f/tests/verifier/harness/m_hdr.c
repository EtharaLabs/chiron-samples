#include "common.h"

/*==============================================================================
 * MARK: - B-09 header interop
 *
 * The header's inline functions are compiled into the CALLER, not the library.
 * They reach straight into `yyjson_val`, `yyjson_mut_val`, `yyjson_doc`,
 * `yyjson_mut_doc`, the two pool structs and the four iterator structs, and
 * they decode the type/subtype/length tag themselves.  A port can therefore be
 * perfectly correct in every exported function and still fail here, because
 * the structs it hands back are laid out differently or the tag is encoded
 * differently.  Nothing else in the harness can see that.
 *
 * Every value used below is one the LIBRARY built, never one the harness
 * forged, so what is being recorded is always "the header's reading of the
 * library's bytes".
 *
 * Passes:
 *   probe     every accessor family on every value of a type-coverage document,
 *             on both the immutable and the mutable representation, with the
 *             `unsafe_*` twin of each recorded alongside the checked one.
 *   tag       the tag encoding: tag, type, subtype and length, and the
 *             identities the header relies on between them.
 *   chain     `unsafe_yyjson_get_first` / `get_next` / `arr_is_flat`, which
 *             are the sibling-walk primitives, checked against the public
 *             iterators over the same container.
 *   iter      every iterator entry point, including `_with`, `has_next` and
 *             the keyed object lookups, and their behaviour past the end.
 *   setter    `yyjson_set_*` and `unsafe_yyjson_set_*` applied in place to an
 *             immutable document the library built.
 *   mutpool   `unsafe_yyjson_mut_val`, `unsafe_yyjson_mut_str_alc`,
 *             `unsafe_yyjson_mut_strncpy` and the two pool-grow helpers,
 *             which touch `yyjson_mut_doc`'s pool structs directly.
 *   unsafeptr the `unsafe_*` pointer entry points.
 *   misc      the memcpy/memmove/memset/memcmp/strlen shims,
 *             `unsafe_yyjson_u64_to_f64`, `yyjson_get_type_desc`,
 *             `yyjson_version` and `yyjson_locate_pos`.
 *============================================================================*/

/* One document per distinct value shape the tag encoding can carry. */
static const char *const HSRC =
    "{"
    "\"nul\":null,\"t\":true,\"f\":false,"
    "\"u0\":0,\"u1\":1,\"umax\":18446744073709551615,"
    "\"s1\":-1,\"smin\":-9223372036854775808,"
    "\"r0\":0.0,\"rneg\":-0.0,\"rpi\":3.141592653589793,"
    "\"rbig\":1.7976931348623157e308,\"rsml\":5e-324,"
    "\"se\":\"\",\"sa\":\"abc\",\"sesc\":\"a\\\"b\","
    "\"suni\":\"\\u00e9\\u4e2d\","
    "\"ae\":[],\"af\":[1,2,3],\"an\":[[1],[2,[3]]],"
    "\"oe\":{},\"oo\":{\"k\":1,\"k2\":[2]},"
    "\"deep\":{\"a\":{\"b\":{\"c\":[1,2,{\"d\":null}]}}}"
    "}";

/*==============================================================================
 * MARK: - accessor probes
 *============================================================================*/

static void probe_imut(buf *b, const char *nm, yyjson_val *v) {
    char k[96];
#define K(s) (snprintf(k, sizeof(k), "%s_" s, nm), k)
    f_u(b, K("tag"), (unsigned long long)yyjson_get_tag(v));
    f_u(b, K("type"), (unsigned long long)yyjson_get_type(v));
    f_u(b, K("sub"), (unsigned long long)yyjson_get_subtype(v));
    f_u(b, K("len"), (unsigned long long)yyjson_get_len(v));
    f_cstr(b, K("desc"), yyjson_get_type_desc(v));
    f_bool(b, K("is_raw"), yyjson_is_raw(v));
    f_bool(b, K("is_null"), yyjson_is_null(v));
    f_bool(b, K("is_true"), yyjson_is_true(v));
    f_bool(b, K("is_false"), yyjson_is_false(v));
    f_bool(b, K("is_bool"), yyjson_is_bool(v));
    f_bool(b, K("is_uint"), yyjson_is_uint(v));
    f_bool(b, K("is_sint"), yyjson_is_sint(v));
    f_bool(b, K("is_int"), yyjson_is_int(v));
    f_bool(b, K("is_real"), yyjson_is_real(v));
    f_bool(b, K("is_num"), yyjson_is_num(v));
    f_bool(b, K("is_str"), yyjson_is_str(v));
    f_bool(b, K("is_arr"), yyjson_is_arr(v));
    f_bool(b, K("is_obj"), yyjson_is_obj(v));
    f_bool(b, K("is_ctn"), yyjson_is_ctn(v));
    f_bool(b, K("get_bool"), yyjson_get_bool(v));
    f_u(b, K("get_uint"), (unsigned long long)yyjson_get_uint(v));
    f_i(b, K("get_sint"), (long long)yyjson_get_sint(v));
    f_i(b, K("get_int"), (long long)yyjson_get_int(v));
    {
        /* reals travel as their exact bit pattern: a decimal rendering would
           smuggle the writer's formatting into a value-accessor record */
        double d = yyjson_get_real(v);
        uint64_t bits;
        memcpy(&bits, &d, 8);
        f_x64(b, K("get_real"), (unsigned long long)bits);
        d = yyjson_get_num(v);
        memcpy(&bits, &d, 8);
        f_x64(b, K("get_num"), (unsigned long long)bits);
    }
    f_cstr(b, K("get_str"), yyjson_get_str(v));
    f_cstr(b, K("get_raw"), yyjson_get_raw(v));
    f_u(b, K("arr_size"), (unsigned long long)yyjson_arr_size(v));
    f_u(b, K("obj_size"), (unsigned long long)yyjson_obj_size(v));

    /* The `unsafe_*` twins skip every guard, including the NULL check: they
       dereference unconditionally.  So the whole group is gated on `v`, and
       each type-specific one additionally on its own type. */
    if (!v) return;
    f_u(b, K("u_tag"), (unsigned long long)unsafe_yyjson_get_tag(v));
    f_u(b, K("u_type"), (unsigned long long)unsafe_yyjson_get_type(v));
    f_u(b, K("u_sub"), (unsigned long long)unsafe_yyjson_get_subtype(v));
    f_u(b, K("u_len"), (unsigned long long)unsafe_yyjson_get_len(v));
    f_bool(b, K("u_is_raw"), unsafe_yyjson_is_raw(v));
    f_bool(b, K("u_is_null"), unsafe_yyjson_is_null(v));
    f_bool(b, K("u_is_true"), unsafe_yyjson_is_true(v));
    f_bool(b, K("u_is_false"), unsafe_yyjson_is_false(v));
    f_bool(b, K("u_is_bool"), unsafe_yyjson_is_bool(v));
    f_bool(b, K("u_is_uint"), unsafe_yyjson_is_uint(v));
    f_bool(b, K("u_is_sint"), unsafe_yyjson_is_sint(v));
    f_bool(b, K("u_is_int"), unsafe_yyjson_is_int(v));
    f_bool(b, K("u_is_real"), unsafe_yyjson_is_real(v));
    f_bool(b, K("u_is_num"), unsafe_yyjson_is_num(v));
    f_bool(b, K("u_is_str"), unsafe_yyjson_is_str(v));
    f_bool(b, K("u_is_arr"), unsafe_yyjson_is_arr(v));
    f_bool(b, K("u_is_obj"), unsafe_yyjson_is_obj(v));
    f_bool(b, K("u_is_ctn"), unsafe_yyjson_is_ctn(v));
    /* `unsafe_yyjson_is_str_noesc` classifies STRING BYTES, not a value: it
       answers "could the writer emit these bytes verbatim".  It is the writer's
       fast path, so it is driven here off the value's own bytes. */
    if (yyjson_is_str(v) || yyjson_is_raw(v)) {
        const char *s = yyjson_is_str(v) ? yyjson_get_str(v)
                                         : yyjson_get_raw(v);
        if (s)
            f_bool(b, K("u_noesc"),
                   unsafe_yyjson_is_str_noesc(s, yyjson_get_len(v)));
    }
    if (yyjson_is_bool(v))
        f_bool(b, K("u_get_bool"), unsafe_yyjson_get_bool(v));
    if (yyjson_is_num(v)) {
        f_u(b, K("u_get_uint"), (unsigned long long)unsafe_yyjson_get_uint(v));
        f_i(b, K("u_get_sint"), (long long)unsafe_yyjson_get_sint(v));
        f_i(b, K("u_get_int"), (long long)unsafe_yyjson_get_int(v));
        double d = unsafe_yyjson_get_real(v);
        uint64_t bits;
        memcpy(&bits, &d, 8);
        f_x64(b, K("u_get_real"), (unsigned long long)bits);
        d = unsafe_yyjson_get_num(v);
        memcpy(&bits, &d, 8);
        f_x64(b, K("u_get_num"), (unsigned long long)bits);
    }
    if (yyjson_is_str(v))
        f_cstr(b, K("u_get_str"), unsafe_yyjson_get_str(v));
    if (yyjson_is_raw(v))
        f_cstr(b, K("u_get_raw"), unsafe_yyjson_get_raw(v));
    if (yyjson_is_arr(v))
        f_bool(b, K("u_flat"), unsafe_yyjson_arr_is_flat(v));
#undef K
}

static void probe_mut(buf *b, const char *nm, yyjson_mut_val *v) {
    char k[96];
#define K(s) (snprintf(k, sizeof(k), "%s_" s, nm), k)
    f_u(b, K("tag"), (unsigned long long)yyjson_mut_get_tag(v));
    f_u(b, K("type"), (unsigned long long)yyjson_mut_get_type(v));
    f_u(b, K("sub"), (unsigned long long)yyjson_mut_get_subtype(v));
    f_u(b, K("len"), (unsigned long long)yyjson_mut_get_len(v));
    f_cstr(b, K("desc"), yyjson_mut_get_type_desc(v));
    f_bool(b, K("is_raw"), yyjson_mut_is_raw(v));
    f_bool(b, K("is_null"), yyjson_mut_is_null(v));
    f_bool(b, K("is_true"), yyjson_mut_is_true(v));
    f_bool(b, K("is_false"), yyjson_mut_is_false(v));
    f_bool(b, K("is_bool"), yyjson_mut_is_bool(v));
    f_bool(b, K("is_uint"), yyjson_mut_is_uint(v));
    f_bool(b, K("is_sint"), yyjson_mut_is_sint(v));
    f_bool(b, K("is_int"), yyjson_mut_is_int(v));
    f_bool(b, K("is_real"), yyjson_mut_is_real(v));
    f_bool(b, K("is_num"), yyjson_mut_is_num(v));
    f_bool(b, K("is_str"), yyjson_mut_is_str(v));
    f_bool(b, K("is_arr"), yyjson_mut_is_arr(v));
    f_bool(b, K("is_obj"), yyjson_mut_is_obj(v));
    f_bool(b, K("is_ctn"), yyjson_mut_is_ctn(v));
    f_bool(b, K("get_bool"), yyjson_mut_get_bool(v));
    f_u(b, K("get_uint"), (unsigned long long)yyjson_mut_get_uint(v));
    f_i(b, K("get_sint"), (long long)yyjson_mut_get_sint(v));
    f_i(b, K("get_int"), (long long)yyjson_mut_get_int(v));
    {
        double d = yyjson_mut_get_real(v);
        uint64_t bits;
        memcpy(&bits, &d, 8);
        f_x64(b, K("get_real"), (unsigned long long)bits);
        d = yyjson_mut_get_num(v);
        memcpy(&bits, &d, 8);
        f_x64(b, K("get_num"), (unsigned long long)bits);
    }
    f_cstr(b, K("get_str"), yyjson_mut_get_str(v));
    f_cstr(b, K("get_raw"), yyjson_mut_get_raw(v));
    f_u(b, K("arr_size"), (unsigned long long)yyjson_mut_arr_size(v));
    f_u(b, K("obj_size"), (unsigned long long)yyjson_mut_obj_size(v));
#undef K
}

/*==============================================================================
 * MARK: - pass `probe` and `tag`
 *============================================================================*/

static void do_probe(buf *b) {
    size_t len = strlen(HSRC);
    char *in = pad_copy(HSRC, len);
    yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                       NULL);
    yyjson_val *root = yyjson_doc_get_root(doc);
    yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, NULL);
    yyjson_mut_val *mroot = yyjson_mut_doc_get_root(m);

    buf_adds(b, "B-09 probe case=root");
    probe_imut(b, "i", root);
    probe_mut(b, "m", mroot);
    rec_line(b);
    rec_cases++;

    /* every member of the top-level object, keyed by its own name so the
       record is stable even if the iteration order ever changed */
    yyjson_obj_iter it;
    yyjson_obj_iter_init(root, &it);
    yyjson_mut_obj_iter mit;
    yyjson_mut_obj_iter_init(mroot, &mit);
    yyjson_val *key;
    while ((key = yyjson_obj_iter_next(&it)) != NULL) {
        yyjson_val *val = yyjson_obj_iter_get_val(key);
        const char *kn = yyjson_get_str(key);
        yyjson_mut_val *mval = yyjson_mut_obj_get(mroot, kn);

        buf_adds(b, "B-09 probe");
        f_cstr(b, "case", kn);
        probe_imut(b, "k", key);
        probe_imut(b, "i", val);
        probe_mut(b, "m", mval);
        rec_line(b);
        rec_cases++;

        /* and one level down, for the containers */
        if (yyjson_is_arr(val)) {
            size_t n = yyjson_arr_size(val);
            for (size_t j = 0; j < n; j++) {
                buf_adds(b, "B-09 probe");
                buf_addf(b, " case=%s/%zu", kn, j);
                probe_imut(b, "i", yyjson_arr_get(val, j));
                probe_mut(b, "m", yyjson_mut_arr_get(mval, j));
                rec_line(b);
                rec_cases++;
            }
        } else if (yyjson_is_obj(val)) {
            yyjson_obj_iter it2;
            yyjson_obj_iter_init(val, &it2);
            yyjson_val *k2;
            while ((k2 = yyjson_obj_iter_next(&it2)) != NULL) {
                buf_adds(b, "B-09 probe");
                buf_addf(b, " case=%s/", kn);
                buf_q(b, yyjson_get_str(k2), yyjson_get_len(k2));
                probe_imut(b, "k", k2);
                probe_imut(b, "i", yyjson_obj_iter_get_val(k2));
                probe_mut(b, "m",
                          yyjson_mut_obj_get(mval, yyjson_get_str(k2)));
                rec_line(b);
                rec_cases++;
            }
        }
        (void)mit;
    }

    /* every accessor on NULL, which the checked forms must all survive */
    buf_adds(b, "B-09 probe case=null");
    probe_imut(b, "i", NULL);
    probe_mut(b, "m", NULL);
    rec_line(b);
    rec_cases++;

    /* the doc-level inline accessors, including on NULL */
    buf_adds(b, "B-09 probe case=doc");
    f_bool(b, "root", yyjson_doc_get_root(doc) != NULL);
    f_u(b, "read", (unsigned long long)yyjson_doc_get_read_size(doc));
    f_u(b, "vals", (unsigned long long)yyjson_doc_get_val_count(doc));
    f_bool(b, "null_root", yyjson_doc_get_root(NULL) != NULL);
    f_u(b, "null_read", (unsigned long long)yyjson_doc_get_read_size(NULL));
    f_u(b, "null_vals", (unsigned long long)yyjson_doc_get_val_count(NULL));
    f_bool(b, "mroot", yyjson_mut_doc_get_root(m) != NULL);
    f_bool(b, "mnull_root", yyjson_mut_doc_get_root(NULL) != NULL);
    rec_line(b);
    rec_cases++;

    yyjson_mut_doc_free(m);
    yyjson_doc_free(doc);
    free(in);
}

static void do_tag(buf *b) {
    size_t len = strlen(HSRC);
    char *in = pad_copy(HSRC, len);
    yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                       NULL);
    yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, NULL);

    /* The header documents the tag as `(len << 8) | type | subtype`.  Rather
       than asserting that, the record carries every piece plus the derived
       quantities, so a port that packs the tag differently diverges on the
       exact field that is wrong. */
    buf_adds(b, "B-09 tag case=constants");
    f_u(b, "t_none", (unsigned long long)YYJSON_TYPE_NONE);
    f_u(b, "t_raw", (unsigned long long)YYJSON_TYPE_RAW);
    f_u(b, "t_null", (unsigned long long)YYJSON_TYPE_NULL);
    f_u(b, "t_bool", (unsigned long long)YYJSON_TYPE_BOOL);
    f_u(b, "t_num", (unsigned long long)YYJSON_TYPE_NUM);
    f_u(b, "t_str", (unsigned long long)YYJSON_TYPE_STR);
    f_u(b, "t_arr", (unsigned long long)YYJSON_TYPE_ARR);
    f_u(b, "t_obj", (unsigned long long)YYJSON_TYPE_OBJ);
    f_u(b, "s_none", (unsigned long long)YYJSON_SUBTYPE_NONE);
    f_u(b, "s_false", (unsigned long long)YYJSON_SUBTYPE_FALSE);
    f_u(b, "s_true", (unsigned long long)YYJSON_SUBTYPE_TRUE);
    f_u(b, "s_uint", (unsigned long long)YYJSON_SUBTYPE_UINT);
    f_u(b, "s_sint", (unsigned long long)YYJSON_SUBTYPE_SINT);
    f_u(b, "s_real", (unsigned long long)YYJSON_SUBTYPE_REAL);
    f_u(b, "s_noesc", (unsigned long long)YYJSON_SUBTYPE_NOESC);
    f_u(b, "m_type", (unsigned long long)YYJSON_TYPE_MASK);
    f_u(b, "m_sub", (unsigned long long)YYJSON_SUBTYPE_MASK);
    f_u(b, "b_type", (unsigned long long)YYJSON_TYPE_BIT);
    f_u(b, "b_sub", (unsigned long long)YYJSON_SUBTYPE_BIT);
    f_u(b, "b_resv", (unsigned long long)YYJSON_RESERVED_BIT);
    f_u(b, "b_tag", (unsigned long long)YYJSON_TAG_BIT);
    f_u(b, "pad", (unsigned long long)YYJSON_PADDING_SIZE);
    f_u(b, "ver", (unsigned long long)YYJSON_VERSION_HEX);
    f_u(b, "ver_fn", (unsigned long long)yyjson_version());
    f_cstr(b, "ver_str", YYJSON_VERSION_STRING);
    f_u(b, "sz_val", (unsigned long long)sizeof(yyjson_val));
    f_u(b, "sz_mval", (unsigned long long)sizeof(yyjson_mut_val));
    f_u(b, "sz_alc", (unsigned long long)sizeof(yyjson_alc));
    f_u(b, "sz_rerr", (unsigned long long)sizeof(yyjson_read_err));
    f_u(b, "sz_werr", (unsigned long long)sizeof(yyjson_write_err));
    f_u(b, "sz_perr", (unsigned long long)sizeof(yyjson_ptr_err));
    f_u(b, "sz_pctx", (unsigned long long)sizeof(yyjson_ptr_ctx));
    f_u(b, "sz_patcherr", (unsigned long long)sizeof(yyjson_patch_err));
    f_u(b, "sz_aiter", (unsigned long long)sizeof(yyjson_arr_iter));
    f_u(b, "sz_oiter", (unsigned long long)sizeof(yyjson_obj_iter));
    f_u(b, "sz_maiter", (unsigned long long)sizeof(yyjson_mut_arr_iter));
    f_u(b, "sz_moiter", (unsigned long long)sizeof(yyjson_mut_obj_iter));
    rec_line(b);
    rec_cases++;

    /* per-value tag arithmetic, immutable and mutable side by side */
    {
        yyjson_val *root = yyjson_doc_get_root(doc);
        yyjson_mut_val *mroot = yyjson_mut_doc_get_root(m);
        yyjson_obj_iter it;
        yyjson_obj_iter_init(root, &it);
        yyjson_val *key;
        while ((key = yyjson_obj_iter_next(&it)) != NULL) {
            yyjson_val *v = yyjson_obj_iter_get_val(key);
            const char *kn = yyjson_get_str(key);
            yyjson_mut_val *mv = yyjson_mut_obj_get(mroot, kn);
            uint8_t tg = yyjson_get_tag(v);
            uint8_t mtg = mv ? yyjson_mut_get_tag(mv) : 0;
            buf_adds(b, "B-09 tag");
            f_cstr(b, "case", kn);
            f_u(b, "tag", (unsigned long long)tg);
            f_u(b, "type", (unsigned long long)yyjson_get_type(v));
            f_u(b, "sub", (unsigned long long)yyjson_get_subtype(v));
            f_u(b, "len", (unsigned long long)yyjson_get_len(v));
            f_u(b, "tag_and_type",
                (unsigned long long)(tg & YYJSON_TYPE_MASK));
            f_u(b, "tag_and_sub",
                (unsigned long long)(tg & YYJSON_SUBTYPE_MASK));
            f_bool(b, "type_eq", (tg & YYJSON_TYPE_MASK)
                                 == yyjson_get_type(v));
            f_bool(b, "sub_eq", (tg & YYJSON_SUBTYPE_MASK)
                                == yyjson_get_subtype(v));
            f_u(b, "mtag", (unsigned long long)mtg);
            f_bool(b, "tag_same", tg == mtg);
            rec_line(b);
            rec_cases++;
        }
    }

    yyjson_mut_doc_free(m);
    yyjson_doc_free(doc);
    free(in);
}

/*==============================================================================
 * MARK: - pass `chain`
 *============================================================================*/

static void do_chain(buf *b) {
    static const char *const SRC[] = {
        "[]", "[1]", "[1,2,3]", "[[1],2]", "[{\"a\":1},2]",
        "[\"a\",\"bb\",\"ccc\"]", "[1,[2,[3,[4]]]]",
        "{}", "{\"a\":1}", "{\"a\":1,\"b\":2}", "{\"a\":[1,2],\"b\":{}}",
        "[1.5,2.5]", "[null,true,false]", "[[],[],[]]"
    };
    static const size_t SRC_N = sizeof(SRC) / sizeof(SRC[0]);

    for (size_t i = 0; i < SRC_N; i++) {
        size_t len = strlen(SRC[i]);
        char *in = pad_copy(SRC[i], len);
        yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                           NULL);
        yyjson_val *root = yyjson_doc_get_root(doc);

        buf_adds(b, "B-09 chain");
        f_cstr(b, "case", SRC[i]);
        f_u(b, "len", (unsigned long long)yyjson_get_len(root));
        f_bool(b, "flat", yyjson_is_arr(root)
                          ? unsafe_yyjson_arr_is_flat(root) : false);
        /* `get_next` on the container itself is the "skip the whole subtree"
           primitive the copy routines rely on: its distance from the container
           is the number of values the subtree occupies */
        f_u(b, "span", (unsigned long long)
            (size_t)(unsafe_yyjson_get_next(root) - root));

        /* walk the children with get_first/get_next and compare against the
           public iterator over the same container */
        if (yyjson_is_ctn(root) && yyjson_get_len(root) > 0) {
            yyjson_val *cur = unsafe_yyjson_get_first(root);
            size_t n = yyjson_get_len(root);
            if (yyjson_is_obj(root)) n *= 2;
            buf_adds(b, " walk=");
            for (size_t j = 0; j < n; j++) {
                if (j) buf_addc(b, '+');
                buf_addf(b, "%u.%zu", (unsigned)yyjson_get_tag(cur),
                         (size_t)(unsafe_yyjson_get_next(cur) - cur));
                cur = unsafe_yyjson_get_next(cur);
            }
            /* after the last child, the cursor must land exactly where
               `get_next` on the container points */
            f_bool(b, "closes", cur == unsafe_yyjson_get_next(root));

            if (yyjson_is_arr(root)) {
                yyjson_arr_iter ai;
                yyjson_arr_iter_init(root, &ai);
                yyjson_val *e;
                size_t j = 0;
                int agree = 1;
                yyjson_val *w = unsafe_yyjson_get_first(root);
                while ((e = yyjson_arr_iter_next(&ai)) != NULL) {
                    if (e != w) agree = 0;
                    w = unsafe_yyjson_get_next(w);
                    j++;
                }
                f_bool(b, "iter_agree", agree);
                f_u(b, "iter_n", (unsigned long long)j);
            }
        }
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(doc);
        free(in);
    }
}

/*==============================================================================
 * MARK: - pass `iter`
 *============================================================================*/

static void do_iter(buf *b) {
    static const char *const SRC[] = {
        "[]", "[1]", "[1,2,3,4,5]", "[[1],[2],[3]]",
        "{}", "{\"a\":1}", "{\"a\":1,\"b\":2,\"c\":3}",
        "{\"a\":1,\"a\":2}", "{\"\":1,\"x\":2}", "42", "null", "\"s\""
    };
    static const size_t SRC_N = sizeof(SRC) / sizeof(SRC[0]);

    for (size_t i = 0; i < SRC_N; i++) {
        size_t len = strlen(SRC[i]);
        char *in = pad_copy(SRC[i], len);
        yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                           NULL);
        yyjson_val *root = yyjson_doc_get_root(doc);
        yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, NULL);
        yyjson_mut_val *mroot = yyjson_mut_doc_get_root(m);

        /* ---- immutable array iterator ---- */
        buf_adds(b, "B-09 iter");
        f_cstr(b, "case", SRC[i]);
        f_cstr(b, "kind", "arr");
        {
            yyjson_arr_iter it;
            f_bool(b, "init", yyjson_arr_iter_init(root, &it));
            f_u(b, "idx0", (unsigned long long)it.idx);
            f_u(b, "max", (unsigned long long)it.max);
            buf_adds(b, " seq=");
            yyjson_val *e;
            size_t n = 0;
            while ((e = yyjson_arr_iter_next(&it)) != NULL) {
                if (n) buf_addc(b, '+');
                buf_addf(b, "%zu.%d.", it.idx,
                         yyjson_arr_iter_has_next(&it) ? 1 : 0);
                tree_imut(b, e);
                n++;
            }
            if (!n) buf_adds(b, "!empty");
            /* past the end: the iterator must keep returning NULL */
            f_bool(b, "past1", yyjson_arr_iter_next(&it) != NULL);
            f_bool(b, "past2", yyjson_arr_iter_next(&it) != NULL);
            f_bool(b, "past_has", yyjson_arr_iter_has_next(&it));
            /* the `_with` form, which returns the struct by value */
            yyjson_arr_iter it2 = yyjson_arr_iter_with(root);
            f_bool(b, "with_same", it2.max == it.max);
            size_t n2 = 0;
            while (yyjson_arr_iter_next(&it2) != NULL) n2++;
            f_u(b, "with_n", (unsigned long long)n2);
            /* NULL paths */
            yyjson_arr_iter it3;
            f_bool(b, "init_nullarr", yyjson_arr_iter_init(NULL, &it3));
            f_bool(b, "init_nulliter", yyjson_arr_iter_init(root, NULL));
            f_bool(b, "next_null", yyjson_arr_iter_next(NULL) != NULL);
            f_bool(b, "has_null", yyjson_arr_iter_has_next(NULL));
            yyjson_arr_iter it4 = yyjson_arr_iter_with(NULL);
            f_u(b, "with_null_max", (unsigned long long)it4.max);
        }
        rec_line(b);
        rec_cases++;

        /* ---- immutable object iterator ---- */
        buf_adds(b, "B-09 iter");
        f_cstr(b, "case", SRC[i]);
        f_cstr(b, "kind", "obj");
        {
            yyjson_obj_iter it;
            f_bool(b, "init", yyjson_obj_iter_init(root, &it));
            f_u(b, "idx0", (unsigned long long)it.idx);
            f_u(b, "max", (unsigned long long)it.max);
            buf_adds(b, " seq=");
            yyjson_val *k;
            size_t n = 0;
            while ((k = yyjson_obj_iter_next(&it)) != NULL) {
                if (n) buf_addc(b, '+');
                buf_addf(b, "%zu.%d.", it.idx,
                         yyjson_obj_iter_has_next(&it) ? 1 : 0);
                tree_imut(b, k);
                buf_addc(b, '^');
                tree_imut(b, yyjson_obj_iter_get_val(k));
                n++;
            }
            if (!n) buf_adds(b, "!empty");
            f_bool(b, "past1", yyjson_obj_iter_next(&it) != NULL);
            f_bool(b, "past_has", yyjson_obj_iter_has_next(&it));
            f_bool(b, "getval_null", yyjson_obj_iter_get_val(NULL) != NULL);

            /* the keyed lookups, which the header documents as being able to
               run forward from the iterator's current position and wrap */
            yyjson_obj_iter it2 = yyjson_obj_iter_with(root);
            static const char *const PROBE[5] = { "a", "b", "c", "", "zz" };
            for (int p = 0; p < 5; p++) {
                char nm[32];
                snprintf(nm, sizeof(nm), "get_%d", p);
                f_tree_imut(b, nm, yyjson_obj_iter_get(&it2, PROBE[p]));
                snprintf(nm, sizeof(nm), "getn_%d", p);
                f_tree_imut(b, nm, yyjson_obj_iter_getn(&it2, PROBE[p],
                                                        strlen(PROBE[p])));
                snprintf(nm, sizeof(nm), "idx_%d", p);
                f_u(b, nm, (unsigned long long)it2.idx);
            }
            f_bool(b, "get_nulliter", yyjson_obj_iter_get(NULL, "a") != NULL);
            f_bool(b, "get_nullkey",
                   yyjson_obj_iter_get(&it2, NULL) != NULL);

            /* the direct object lookups */
            f_tree_imut(b, "obj_get_a", yyjson_obj_get(root, "a"));
            f_tree_imut(b, "obj_getn_a", yyjson_obj_getn(root, "a", 1));
            f_tree_imut(b, "obj_getn_a0", yyjson_obj_getn(root, "a", 0));
            f_bool(b, "obj_get_null", yyjson_obj_get(root, NULL) != NULL);
            f_bool(b, "obj_get_nullobj", yyjson_obj_get(NULL, "a") != NULL);
        }
        rec_line(b);
        rec_cases++;

        /* ---- mutable iterators, including the `_with` forms ---- */
        buf_adds(b, "B-09 iter");
        f_cstr(b, "case", SRC[i]);
        f_cstr(b, "kind", "mut");
        {
            yyjson_mut_arr_iter ai;
            f_bool(b, "ainit", yyjson_mut_arr_iter_init(mroot, &ai));
            f_u(b, "aidx0", (unsigned long long)ai.idx);
            f_u(b, "amax", (unsigned long long)ai.max);
            buf_adds(b, " aseq=");
            yyjson_mut_val *e;
            size_t n = 0;
            while ((e = yyjson_mut_arr_iter_next(&ai)) != NULL) {
                if (n) buf_addc(b, '+');
                buf_addf(b, "%zu.%d.", ai.idx,
                         yyjson_mut_arr_iter_has_next(&ai) ? 1 : 0);
                tree_mut(b, e);
                n++;
            }
            if (!n) buf_adds(b, "!empty");
            f_bool(b, "apast", yyjson_mut_arr_iter_next(&ai) != NULL);
            yyjson_mut_arr_iter ai2 = yyjson_mut_arr_iter_with(mroot);
            f_u(b, "awith_max", (unsigned long long)ai2.max);
            f_bool(b, "ainit_null", yyjson_mut_arr_iter_init(NULL, &ai2));
            f_bool(b, "anext_null",
                   yyjson_mut_arr_iter_next(NULL) != NULL);
            f_bool(b, "ahas_null", yyjson_mut_arr_iter_has_next(NULL));
            f_bool(b, "arem_null",
                   yyjson_mut_arr_iter_remove(NULL) != NULL);

            yyjson_mut_obj_iter oi;
            f_bool(b, "oinit", yyjson_mut_obj_iter_init(mroot, &oi));
            f_u(b, "omax", (unsigned long long)oi.max);
            buf_adds(b, " oseq=");
            yyjson_mut_val *k;
            n = 0;
            while ((k = yyjson_mut_obj_iter_next(&oi)) != NULL) {
                if (n) buf_addc(b, '+');
                buf_addf(b, "%zu.%d.", oi.idx,
                         yyjson_mut_obj_iter_has_next(&oi) ? 1 : 0);
                tree_mut(b, k);
                buf_addc(b, '^');
                tree_mut(b, yyjson_mut_obj_iter_get_val(k));
                n++;
            }
            if (!n) buf_adds(b, "!empty");
            yyjson_mut_obj_iter oi2 = yyjson_mut_obj_iter_with(mroot);
            f_tree_mut(b, "oget_a", yyjson_mut_obj_iter_get(&oi2, "a"));
            f_tree_mut(b, "ogetn_a",
                       yyjson_mut_obj_iter_getn(&oi2, "a", 1));
            f_bool(b, "oget_null",
                   yyjson_mut_obj_iter_get(NULL, "a") != NULL);
            f_bool(b, "ogetval_null",
                   yyjson_mut_obj_iter_get_val(NULL) != NULL);
            f_bool(b, "orem_null",
                   yyjson_mut_obj_iter_remove(NULL) != NULL);
            f_tree_mut(b, "mobj_get", yyjson_mut_obj_get(mroot, "a"));
            f_tree_mut(b, "mobj_getn", yyjson_mut_obj_getn(mroot, "a", 1));
            f_tree_mut(b, "marr_get", yyjson_mut_arr_get(mroot, 0));
            f_tree_mut(b, "marr_getf", yyjson_mut_arr_get_first(mroot));
            f_tree_mut(b, "marr_getl", yyjson_mut_arr_get_last(mroot));
        }
        rec_line(b);
        rec_cases++;

        /* ---- the immutable array accessors and the foreach shapes ---- */
        buf_adds(b, "B-09 iter");
        f_cstr(b, "case", SRC[i]);
        f_cstr(b, "kind", "acc");
        f_tree_imut(b, "get0", yyjson_arr_get(root, 0));
        f_tree_imut(b, "get1", yyjson_arr_get(root, 1));
        f_tree_imut(b, "getbig", yyjson_arr_get(root, (size_t)-1));
        f_tree_imut(b, "getf", yyjson_arr_get_first(root));
        f_tree_imut(b, "getl", yyjson_arr_get_last(root));
        f_bool(b, "get_null", yyjson_arr_get(NULL, 0) != NULL);
        f_bool(b, "getf_null", yyjson_arr_get_first(NULL) != NULL);
        f_bool(b, "getl_null", yyjson_arr_get_last(NULL) != NULL);
        f_u(b, "arr_size_null", (unsigned long long)yyjson_arr_size(NULL));
        f_u(b, "obj_size_null", (unsigned long long)yyjson_obj_size(NULL));
        {
            /* yyjson_arr_foreach / yyjson_obj_foreach expand to the index
               bookkeeping the header defines; driving them is the only way to
               pin that expansion against the structs */
            size_t idx, mx;
            yyjson_val *v;
            buf_adds(b, " fe_arr=");
            int first = 1;
            yyjson_arr_foreach(root, idx, mx, v) {
                if (!first) buf_addc(b, '+');
                first = 0;
                buf_addf(b, "%zu.%zu.", idx, mx);
                tree_imut(b, v);
            }
            if (first) buf_adds(b, "!empty");

            yyjson_val *k2, *v2;
            buf_adds(b, " fe_obj=");
            first = 1;
            yyjson_obj_foreach(root, idx, mx, k2, v2) {
                if (!first) buf_addc(b, '+');
                first = 0;
                buf_addf(b, "%zu.%zu.", idx, mx);
                tree_imut(b, k2);
                buf_addc(b, '^');
                tree_imut(b, v2);
            }
            if (first) buf_adds(b, "!empty");

            yyjson_mut_val *mv;
            buf_adds(b, " fe_marr=");
            first = 1;
            yyjson_mut_arr_foreach(mroot, idx, mx, mv) {
                if (!first) buf_addc(b, '+');
                first = 0;
                buf_addf(b, "%zu.%zu.", idx, mx);
                tree_mut(b, mv);
            }
            if (first) buf_adds(b, "!empty");

            yyjson_mut_val *mk, *mv2;
            buf_adds(b, " fe_mobj=");
            first = 1;
            yyjson_mut_obj_foreach(mroot, idx, mx, mk, mv2) {
                if (!first) buf_addc(b, '+');
                first = 0;
                buf_addf(b, "%zu.%zu.", idx, mx);
                tree_mut(b, mk);
                buf_addc(b, '^');
                tree_mut(b, mv2);
            }
            if (first) buf_adds(b, "!empty");
        }
        rec_line(b);
        rec_cases++;

        yyjson_mut_doc_free(m);
        yyjson_doc_free(doc);
        free(in);
    }
}

/*==============================================================================
 * MARK: - pass `setter`
 *
 * `yyjson_set_*` writes through to a value inside an immutable document the
 * library allocated, so it is a direct test of the header's agreement with the
 * library's layout.  A fresh document per setter keeps the cases independent.
 *============================================================================*/

static void do_setter(buf *b) {
    static const char *const SRC =
        "[null,true,0,-1,1.5,\"abc\",[1,2],{\"a\":1}]";
    static const char *const NM[] = {
        "null", "bool", "uint", "sint", "int", "float", "double", "real",
        "str", "strn", "raw", "noesc", "fixed", "tofloat",
        "u_null", "u_bool", "u_uint", "u_sint", "u_real", "u_float",
        "u_double", "u_str", "u_strn", "u_raw", "u_noesc", "u_fixed",
        "u_tofloat", "u_arr", "u_obj", "u_type", "u_tag", "u_len", "u_inclen"
    };
    static const size_t NM_N = sizeof(NM) / sizeof(NM[0]);

    for (size_t s = 0; s < NM_N; s++) {
        size_t len = strlen(SRC);
        char *in = pad_copy(SRC, len);
        yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                           NULL);
        yyjson_val *root = yyjson_doc_get_root(doc);
        size_t n = yyjson_arr_size(root);

        /* The structural `unsafe_*` setters (set_arr, set_obj, set_type,
           set_tag, set_len, inc_len) rewrite a value's tag WITHOUT fixing the
           `uni.ofs` that `unsafe_yyjson_get_next` reads for containers.  Once
           one of them has run on an element, walking the parent array lands on
           a wild pointer.  So every child pointer is captured before the first
           mutation and addressed directly afterwards, never re-walked. */
        yyjson_val **kid = (yyjson_val **)calloc(n ? n : 1, sizeof(*kid));
        if (!kid) { fprintf(stderr, "harness: oom\n"); exit(2); }
        for (size_t i = 0; i < n; i++) kid[i] = yyjson_arr_get(root, i);

        buf_adds(b, "B-09 setter");
        f_cstr(b, "case", NM[s]);
        for (size_t i = 0; i < n; i++) {
            yyjson_val *v = kid[i];
            char nm[32];
            snprintf(nm, sizeof(nm), "b%zu", i);
            f_u(b, nm, (unsigned long long)yyjson_get_tag(v));
            int ret = 1;
            switch (s) {
                case 0:  ret = yyjson_set_null(v); break;
                case 1:  ret = yyjson_set_bool(v, true); break;
                case 2:  ret = yyjson_set_uint(v, 42); break;
                case 3:  ret = yyjson_set_sint(v, -42); break;
                case 4:  ret = yyjson_set_int(v, -43); break;
                case 5:  ret = yyjson_set_float(v, 1.25f); break;
                case 6:  ret = yyjson_set_double(v, 1.25); break;
                case 7:  ret = yyjson_set_real(v, 2.25); break;
                case 8:  ret = yyjson_set_str(v, "zz"); break;
                case 9:  ret = yyjson_set_strn(v, "z\0z", 3); break;
                case 10: ret = yyjson_set_raw(v, "1e3", 3); break;
                case 11: ret = yyjson_set_str_noesc(v, true); break;
                case 12: ret = yyjson_set_fp_to_fixed(v, 3); break;
                case 13: ret = yyjson_set_fp_to_float(v, true); break;
                /* the `unsafe_*` twins return void */
                case 14: unsafe_yyjson_set_null(v); break;
                case 15: unsafe_yyjson_set_bool(v, true); break;
                case 16: unsafe_yyjson_set_uint(v, 42); break;
                case 17: unsafe_yyjson_set_sint(v, -42); break;
                case 18: unsafe_yyjson_set_real(v, 2.25); break;
                case 19: unsafe_yyjson_set_float(v, 1.25f); break;
                case 20: unsafe_yyjson_set_double(v, 1.25); break;
                case 21: unsafe_yyjson_set_str(v, "zz"); break;
                case 22: unsafe_yyjson_set_strn(v, "z\0z", 3); break;
                case 23: unsafe_yyjson_set_raw(v, "1e3", 3); break;
                case 24: unsafe_yyjson_set_str_noesc(v, true); break;
                case 25: unsafe_yyjson_set_fp_to_fixed(v, 3); break;
                case 26: unsafe_yyjson_set_fp_to_float(v, true); break;
                case 27: unsafe_yyjson_set_arr(v, 0); break;
                case 28: unsafe_yyjson_set_obj(v, 0); break;
                case 29: unsafe_yyjson_set_type(v, YYJSON_TYPE_NUM,
                                                YYJSON_SUBTYPE_SINT); break;
                case 30: unsafe_yyjson_set_tag(v, YYJSON_TYPE_STR,
                                               YYJSON_SUBTYPE_NONE, 2); break;
                case 31: unsafe_yyjson_set_len(v, 2); break;
                default: unsafe_yyjson_inc_len(v); break;
            }
            snprintf(nm, sizeof(nm), "r%zu", i);
            f_bool(b, nm, ret);
            snprintf(nm, sizeof(nm), "a%zu", i);
            f_u(b, nm, (unsigned long long)yyjson_get_tag(v));
            snprintf(nm, sizeof(nm), "l%zu", i);
            f_u(b, nm, (unsigned long long)yyjson_get_len(v));
            snprintf(nm, sizeof(nm), "t%zu", i);
            f_u(b, nm, (unsigned long long)yyjson_get_type(v));
            snprintf(nm, sizeof(nm), "s%zu", i);
            f_u(b, nm, (unsigned long long)yyjson_get_subtype(v));
        }
        /* the checked setters on NULL */
        if (s < 14) {
            f_bool(b, "null_null", yyjson_set_null(NULL));
            f_bool(b, "null_uint", yyjson_set_uint(NULL, 1));
            f_bool(b, "null_str", yyjson_set_str(NULL, "x"));
            f_bool(b, "null_raw", yyjson_set_raw(NULL, "x", 1));
            f_bool(b, "null_noesc", yyjson_set_str_noesc(NULL, true));
            f_bool(b, "null_fixed", yyjson_set_fp_to_fixed(NULL, 1));
            /* a NULL string argument to the string setters */
            yyjson_val *v0 = (n > 5) ? kid[5] : NULL;
            f_bool(b, "str_nullarg", yyjson_set_str(v0, NULL));
            f_bool(b, "strn_nullarg", yyjson_set_strn(v0, NULL, 0));
            f_bool(b, "raw_nullarg", yyjson_set_raw(v0, NULL, 0));
        }
        rec_line(b);
        rec_cases++;
        free(kid);
        yyjson_doc_free(doc);
        free(in);
    }
}

/*==============================================================================
 * MARK: - pass `mutpool`
 *============================================================================*/

static void do_mutpool(buf *b) {
    static const size_t CNT[] = { 0, 1, 2, 7, 16, 64, 1024, 65536 };
    static const size_t CNT_N = sizeof(CNT) / sizeof(CNT[0]);

    for (size_t i = 0; i < CNT_N; i++) {
        alc_trace t;
        yyjson_alc alc;
        alc_trace_init(&t, &alc);
        yyjson_mut_doc *doc = yyjson_mut_doc_new(&alc);

        buf_adds(b, "B-09 mutpool");
        buf_addf(b, " case=val/%zu", CNT[i]);
        yyjson_mut_val *v = unsafe_yyjson_mut_val(doc, CNT[i]);
        f_bool(b, "val", v != NULL);
        if (v && CNT[i] > 0) {
            /* a freshly carved run of values must be zeroed, which the tag of
               the first and last one shows */
            f_u(b, "tag0", (unsigned long long)yyjson_mut_get_tag(v));
            f_u(b, "tagN", (unsigned long long)
                yyjson_mut_get_tag(v + (CNT[i] - 1)));
        }
        /* and the string pool helpers */
        char *s = unsafe_yyjson_mut_str_alc(doc, CNT[i]);
        f_bool(b, "str_alc", s != NULL);
        char *c = unsafe_yyjson_mut_strncpy(doc, "abcdefgh", CNT[i] > 8 ? 8
                                                                        : CNT[i]);
        f_bool(b, "strncpy", c != NULL);
        if (c) f_str(b, "strncpy_v", c, CNT[i] > 8 ? 8 : CNT[i]);
        /* `unsafe_yyjson_mut_str_alc(NULL, n)` dereferences `doc->alc` and
           `unsafe_yyjson_mut_strncpy(doc, NULL, n)` memcpys from NULL, both
           unguarded.  Those are not error paths to record, they are undefined
           behaviour, so the harness does not call them: a crash in the
           instrument proves nothing about the library.  The one parameter edge
           that IS defined is `len == SIZE_MAX`, which str_alc rejects to stop
           `len + 1` wrapping. */
        f_bool(b, "str_alc_lenmax",
               unsafe_yyjson_mut_str_alc(doc, (size_t)-1) != NULL);
        /* the pool-grow helpers, reaching into the document's pool structs */
        f_bool(b, "vgrow", unsafe_yyjson_val_pool_grow(&doc->val_pool,
                                                       &doc->alc, CNT[i]));
        f_bool(b, "sgrow", unsafe_yyjson_str_pool_grow(&doc->str_pool,
                                                       &doc->alc, CNT[i]));
        /* the document has to remain usable afterwards */
        yyjson_mut_val *arr = yyjson_mut_arr(doc);
        yyjson_mut_doc_set_root(doc, arr);
        int ok = arr != NULL;
        for (int k = 0; k < 8 && ok; k++)
            ok = yyjson_mut_arr_add_strcpy(doc, arr, "tail");
        f_bool(b, "usable", ok);
        {
            size_t ol = 0;
            char *o = yyjson_mut_write(doc, YYJSON_WRITE_NOFLAG, &ol);
            f_bytes(b, "json", o, o ? ol : 0);
            free(o);
        }
        yyjson_mut_doc_free(doc);
        f_u(b, "live", (unsigned long long)t.live);
        f_alc_trace(b, "alc", &t);
        rec_line(b);
        rec_cases++;
        alc_trace_free(&t);
    }

    /* `unsafe_yyjson_mut_obj_*`, which the public object API is built on */
    {
        yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *obj = yyjson_mut_obj(doc);
        yyjson_mut_doc_set_root(doc, obj);
        buf_adds(b, "B-09 mutpool case=obj_unsafe");
        for (int k = 0; k < 4; k++) {
            char key[8];
            snprintf(key, sizeof(key), "k%d", k);
            unsafe_yyjson_mut_obj_add(obj, yyjson_mut_strcpy(doc, key),
                                      yyjson_mut_uint(doc, (uint64_t)k),
                                      yyjson_mut_obj_size(obj));
        }
        f_u(b, "size", (unsigned long long)yyjson_mut_obj_size(obj));
        f_tree_mut(b, "built", obj);
        f_bool(b, "replace",
               unsafe_yyjson_mut_obj_replace(obj, yyjson_mut_strcpy(doc, "k1"),
                                             yyjson_mut_uint(doc, 99)));
        f_tree_mut(b, "replaced", obj);
        f_tree_mut(b, "removed",
                   unsafe_yyjson_mut_obj_remove(obj, "k2", 2));
        f_tree_mut(b, "after_remove", obj);
        unsafe_yyjson_mut_obj_rotate(obj, 1);
        f_tree_mut(b, "rotated", obj);
        unsafe_yyjson_mut_obj_rotate(obj, 0);
        f_tree_mut(b, "rotated0", obj);
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(doc);
    }
}

/*==============================================================================
 * MARK: - pass `unsafeptr`
 *============================================================================*/

static void do_unsafeptr(buf *b) {
    static const char *const SRC =
        "{\"a\":{\"b\":[1,2]},\"c\":3,\"a~1b\":4}";
    static const char *const PTR[] = {
        "", "/a", "/a/b", "/a/b/0", "/a/b/-", "/c", "/zz", "nope", "/a~1b"
    };
    static const size_t PTR_N = sizeof(PTR) / sizeof(PTR[0]);

    for (size_t i = 0; i < PTR_N; i++) {
        size_t len = strlen(SRC);
        char *in = pad_copy(SRC, len);
        yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                           NULL);
        yyjson_val *root = yyjson_doc_get_root(doc);
        yyjson_mut_doc *m = yyjson_doc_mut_copy(doc, NULL);
        yyjson_mut_val *mroot = yyjson_mut_doc_get_root(m);
        const char *p = PTR[i];
        size_t pl = strlen(p);

        buf_adds(b, "B-09 unsafeptr");
        f_cstr(b, "ptr", p);
        /* `unsafe_yyjson_get_pointer` requires a non-empty pointer, which is
           why the empty case is recorded separately rather than skipped */
        if (pl > 0) {
/* The deprecated spellings are still exported and still part of the ABI a
   consumer may be linked against, so they are covered on purpose.  The
   diagnostic is suppressed only across these calls. */
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-declarations"
            f_tree_imut(b, "u_get",
                        unsafe_yyjson_get_pointer(root, p, pl));
            f_tree_mut(b, "u_mget",
                       unsafe_yyjson_mut_get_pointer(mroot, p, pl));
#pragma GCC diagnostic pop
        }
        {
            yyjson_ptr_err e;
            memset(&e, 0, sizeof(e));
            f_tree_imut(b, "u_getx",
                        unsafe_yyjson_ptr_getx(root, p, pl, &e));
            f_u(b, "u_getx_code", (unsigned long long)e.code);
            f_cstr(b, "u_getx_msg", e.msg);
            f_u(b, "u_getx_pos", (unsigned long long)e.pos);
        }
        {
            yyjson_ptr_ctx c;
            yyjson_ptr_err e;
            memset(&c, 0, sizeof(c));
            memset(&e, 0, sizeof(e));
            f_tree_mut(b, "u_mgetx",
                       unsafe_yyjson_mut_ptr_getx(mroot, p, pl, &c, &e));
            f_u(b, "u_mgetx_code", (unsigned long long)e.code);
            f_cstr(b, "u_mgetx_msg", e.msg);
            f_tree_mut(b, "u_mgetx_ctn", c.ctn);
            f_tree_mut(b, "u_mgetx_pre", c.pre);
            f_tree_mut(b, "u_mgetx_old", c.old);
        }
        {
            yyjson_ptr_ctx c;
            yyjson_ptr_err e;
            memset(&c, 0, sizeof(c));
            memset(&e, 0, sizeof(e));
            f_bool(b, "u_putx",
                   unsafe_yyjson_mut_ptr_putx(mroot, p, pl,
                       yyjson_mut_strcpy(m, "P"), m, false, true, &c, &e));
            f_u(b, "u_putx_code", (unsigned long long)e.code);
            f_tree_mut(b, "u_putx_root", mroot);
        }
        {
            yyjson_ptr_ctx c;
            yyjson_ptr_err e;
            memset(&c, 0, sizeof(c));
            memset(&e, 0, sizeof(e));
            f_tree_mut(b, "u_repx",
                       unsafe_yyjson_mut_ptr_replacex(mroot, p, pl,
                           yyjson_mut_strcpy(m, "R"), &c, &e));
            f_u(b, "u_repx_code", (unsigned long long)e.code);
            f_tree_mut(b, "u_repx_root", mroot);
        }
        {
            yyjson_ptr_ctx c;
            yyjson_ptr_err e;
            memset(&c, 0, sizeof(c));
            memset(&e, 0, sizeof(e));
            f_tree_mut(b, "u_remx",
                       unsafe_yyjson_mut_ptr_removex(mroot, p, pl, &c, &e));
            f_u(b, "u_remx_code", (unsigned long long)e.code);
            f_tree_mut(b, "u_remx_root", mroot);
        }
        f_bool(b, "u_equals", unsafe_yyjson_equals(root, root));
        f_bool(b, "u_mequals", unsafe_yyjson_mut_equals(mroot, mroot));
        rec_line(b);
        rec_cases++;
        yyjson_mut_doc_free(m);
        yyjson_doc_free(doc);
        free(in);
    }
}

/*==============================================================================
 * MARK: - pass `misc`
 *============================================================================*/

static void do_misc(buf *b) {
    /* `yyjson_memcpy` / `memmove` / `memset` / `memcmp` / `strlen` appear in
       api-decls.txt but are internal to yyjson.c, not declared in yyjson.h, so
       no consumer can reach them and this module deliberately does not try. */

    /* unsafe_yyjson_u64_to_f64 over the values where a naive conversion and a
       correctly-rounded one part ways */
    {
        static const uint64_t U[] = {
            0, 1, 2, 0x7FFFFFFFFFFFFFFFULL, 0x8000000000000000ULL,
            0x8000000000000001ULL, 0xFFFFFFFFFFFFFFFFULL,
            0xFFFFFFFFFFFFF800ULL, 0xFFFFFFFFFFFFFC00ULL,
            0x20000000000000ULL, 0x20000000000001ULL, 0x20000000000003ULL,
            9007199254740992ULL, 9007199254740993ULL, 9007199254740995ULL
        };
        for (size_t i = 0; i < sizeof(U) / sizeof(U[0]); i++) {
            double d = unsafe_yyjson_u64_to_f64(U[i]);
            uint64_t bits;
            memcpy(&bits, &d, 8);
            buf_adds(b, "B-09 misc");
            buf_addf(b, " case=u64_to_f64/%zu", i);
            f_x64(b, "in", (unsigned long long)U[i]);
            f_x64(b, "out", (unsigned long long)bits);
            rec_line(b);
            rec_cases++;
        }
    }

    /* yyjson_locate_pos over inputs with CR, LF, CRLF, tabs and multi-byte
       characters, at every byte position */
    {
        static const struct { const char *name, *dat; size_t len; } IN[] = {
            { "empty",  "",                      0 },
            { "oneline","abc",                   3 },
            { "lf",     "ab\ncd\nef",            8 },
            { "crlf",   "ab\r\ncd",              6 },
            { "cr",     "ab\rcd",                5 },
            { "tab",    "a\tb\nc",               5 },
            { "utf8",   "a\xc3\xa9\xe4\xb8\xad\nb", 8 },
            { "badutf8","a\x80\xc3\nb",          5 },
            { "nul",    "a\0b\nc",               5 },
            { "trailnl","ab\n",                  3 },
        };
        for (size_t i = 0; i < sizeof(IN) / sizeof(IN[0]); i++) {
            buf_adds(b, "B-09 misc");
            buf_addf(b, " case=locate/%s", IN[i].name);
            f_str(b, "in", IN[i].dat, IN[i].len);
            for (size_t pos = 0; pos <= IN[i].len + 1; pos++) {
                size_t line = (size_t)-1, col = (size_t)-1, chr = (size_t)-1;
                bool ok = yyjson_locate_pos(IN[i].dat, IN[i].len, pos, &line,
                                            &col, &chr);
                buf_addf(b, " p%zu=%d/%zu/%zu/%zu", pos, ok ? 1 : 0, line,
                         col, chr);
            }
            /* NULL out-parameters, each on its own */
            {
                size_t line = 0, col = 0, chr = 0;
                f_bool(b, "no_line", yyjson_locate_pos(IN[i].dat, IN[i].len,
                                     0, NULL, &col, &chr));
                f_bool(b, "no_col", yyjson_locate_pos(IN[i].dat, IN[i].len,
                                    0, &line, NULL, &chr));
                f_bool(b, "no_chr", yyjson_locate_pos(IN[i].dat, IN[i].len,
                                    0, &line, &col, NULL));
                f_bool(b, "no_any", yyjson_locate_pos(IN[i].dat, IN[i].len,
                                    0, NULL, NULL, NULL));
                f_bool(b, "str_null", yyjson_locate_pos(NULL, IN[i].len, 0,
                                      &line, &col, &chr));
            }
            rec_line(b);
            rec_cases++;
        }
    }

    /* locate_pos against the reader's own err.pos, which is the only reason
       the function exists */
    {
        static const char *const BAD[] = {
            "[1,\n2,\n}]", "{\n\"a\"\n1}", "[\"\\q\"]", "[\xc3]",
            "[1,2\n", "\n\n\n@"
        };
        for (size_t i = 0; i < sizeof(BAD) / sizeof(BAD[0]); i++) {
            size_t len = strlen(BAD[i]);
            char *in = pad_copy(BAD[i], len);
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *d = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG,
                                             NULL, &e);
            size_t line = (size_t)-1, col = (size_t)-1, chr = (size_t)-1;
            bool ok = yyjson_locate_pos(BAD[i], len, e.pos, &line, &col,
                                        &chr);
            buf_adds(b, "B-09 misc");
            buf_addf(b, " case=locate_err/%zu", i);
            f_str(b, "in", BAD[i], len);
            f_bool(b, "parsed", d != NULL);
            f_u(b, "code", (unsigned long long)e.code);
            f_u(b, "pos", (unsigned long long)e.pos);
            f_bool(b, "ok", ok);
            f_u(b, "line", (unsigned long long)line);
            f_u(b, "col", (unsigned long long)col);
            f_u(b, "chr", (unsigned long long)chr);
            rec_line(b);
            rec_cases++;
            yyjson_doc_free(d);
            free(in);
        }
    }
}

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

void mod_hdr(const char *corpus) {
    buf b;
    buf_init(&b);
    char path[4096];

    do_probe(&b);
    do_tag(&b);
    do_chain(&b);
    do_iter(&b);
    do_setter(&b);
    do_mutpool(&b);
    do_unsafeptr(&b);
    do_misc(&b);

    /* A corpus sweep of the tag encoding: for every value in every corpus
       document, the header's own reading of the tag.  This is the pass that
       catches a subtype the reader sets correctly in the library but that the
       header decodes differently. */
    flist fl = corpus_list(corpus, "json");
    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;
        char *in = pad_copy(dat, len);
        yyjson_doc *doc = yyjson_read_opts(in, len, YYJSON_READ_NOFLAG, NULL,
                                           NULL);
        if (doc) {
            /* walk the whole value block in allocation order, which is the
               order `unsafe_yyjson_get_next` defines */
            yyjson_val *root = yyjson_doc_get_root(doc);
            size_t total = yyjson_doc_get_val_count(doc);
            buf_adds(&b, "B-09 tagsweep");
            f_cstr(&b, "case", fl.name[i]);
            f_u(&b, "vals", (unsigned long long)total);
            buf_adds(&b, " tags=");
            uint64_t h = 0xCBF29CE484222325ULL;
            size_t shown = 0;
            yyjson_val *cur = root;
            yyjson_val *end = root + total;
            while (cur < end) {
                unsigned char rec[10];
                rec[0] = (unsigned char)yyjson_get_tag(cur);
                size_t l = yyjson_get_len(cur);
                for (int k = 0; k < 8; k++)
                    rec[1 + k] = (unsigned char)((l >> (k * 8)) & 0xFF);
                rec[9] = (unsigned char)
                    (size_t)(unsafe_yyjson_get_next(cur) - cur);
                for (size_t k = 0; k < sizeof(rec); k++) {
                    h ^= rec[k];
                    h *= 0x100000001B3ULL;
                }
                if (shown < 48) {
                    if (shown) buf_addc(&b, '+');
                    buf_addf(&b, "%u.%zu.%u", (unsigned)rec[0], l,
                             (unsigned)rec[9]);
                    shown++;
                }
                cur++;
            }
            if (!shown) buf_adds(&b, "!empty");
            f_x64(&b, "tagfnv", (unsigned long long)h);
            rec_line(&b);
            rec_cases++;
        }
        yyjson_doc_free(doc);
        free(in);
        free(dat);
    }
    flist_free(&fl);

    buf_free(&b);
}
