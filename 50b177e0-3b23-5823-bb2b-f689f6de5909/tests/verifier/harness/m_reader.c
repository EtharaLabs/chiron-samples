#include "common.h"

/*==============================================================================
 * MARK: - B-01 reader
 *
 * `yyjson_read_opts` over the whole JSON corpus across a curated read-flag
 * sweep.  Per case: result null-ness, err.code, the exact err.msg bytes,
 * err.pos, yyjson_doc_get_read_size, yyjson_doc_get_val_count and a canonical
 * depth-first serialization of the resulting tree.
 *============================================================================*/

/** Make a writable copy padded with YYJSON_PADDING_SIZE zero bytes, as
    YYJSON_READ_INSITU requires.  A fresh copy per case means an in-situ read
    can never contaminate a later case. */
char *pad_copy(const char *dat, size_t len) {
    char *p = (char *)malloc(len + YYJSON_PADDING_SIZE);
    if (!p) { fprintf(stderr, "harness: oom\n"); exit(2); }
    if (len) memcpy(p, dat, len);
    memset(p + len, 0, YYJSON_PADDING_SIZE);
    return p;
}

/** Emit the standard read-result field group. */
void f_read_result(buf *b, yyjson_doc *doc, const yyjson_read_err *err) {
    f_bool(b, "ok", doc != NULL);
    f_u(b, "code", (unsigned long long)err->code);
    f_cstr(b, "msg", err->msg);
    f_u(b, "pos", (unsigned long long)err->pos);
    if (doc) {
        yyjson_val *root = yyjson_doc_get_root(doc);
        f_bool(b, "root", root != NULL);
        f_u(b, "read", (unsigned long long)yyjson_doc_get_read_size(doc));
        f_u(b, "vals", (unsigned long long)yyjson_doc_get_val_count(doc));
        f_tree_imut(b, "tree", root);
    }
}

void mod_reader(const char *corpus) {
    flist fl = corpus_list(corpus, "json");
    buf b;
    buf_init(&b);
    char path[4096];

    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) {
            buf_adds(&b, "B-01");
            f_cstr(&b, "case", fl.name[i]);
            buf_adds(&b, " !slurp_failed");
            rec_line(&b);
            continue;
        }

        for (size_t f = 0; f < RFLAGS_N; f++) {
            yyjson_read_flag flg = RFLAGS[f].flg;
            char *in = pad_copy(dat, len);
            yyjson_read_err err;
            memset(&err, 0, sizeof(err));
            yyjson_doc *doc = yyjson_read_opts(in, len, flg, NULL, &err);

            buf_adds(&b, "B-01 read");
            f_cstr(&b, "case", fl.name[i]);
            f_cstr(&b, "f", RFLAGS[f].name);
            buf_addf(&b, " flg=0x%08X", (unsigned)flg);
            f_u(&b, "inlen", (unsigned long long)len);
            f_u(&b, "maxmem", (unsigned long long)
                yyjson_read_max_memory_usage(len, flg));
            f_read_result(&b, doc, &err);
            rec_line(&b);
            rec_cases++;

            yyjson_doc_free(doc);
            free(in);
        }

        /* the `yyjson_read` inline wrapper: default flags, no err out */
        {
            char *in = pad_copy(dat, len);
            yyjson_doc *doc = yyjson_read(in, len, YYJSON_READ_NOFLAG);
            buf_adds(&b, "B-01 read_inline");
            f_cstr(&b, "case", fl.name[i]);
            f_bool(&b, "ok", doc != NULL);
            if (doc) {
                f_u(&b, "read",
                    (unsigned long long)yyjson_doc_get_read_size(doc));
                f_u(&b, "vals",
                    (unsigned long long)yyjson_doc_get_val_count(doc));
                f_tree_imut(&b, "tree", yyjson_doc_get_root(doc));
            }
            rec_line(&b);
            rec_cases++;
            yyjson_doc_free(doc);
            free(in);
        }

        free(dat);
    }
    flist_free(&fl);
    buf_free(&b);
}
