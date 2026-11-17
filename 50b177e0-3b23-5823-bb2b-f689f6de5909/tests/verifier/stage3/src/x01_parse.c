#include "s3.h"

/*==============================================================================
 * MARK: - X-01 parse and error decision boundaries
 *
 * Every file under corpus/X-01/ is a raw byte sequence that sits on a reader
 * decision boundary: one byte short of a complete document, one byte into a
 * multi-byte UTF-8 sequence, a BOM or an extended-whitespace code point at a
 * position where it may or may not be trivia, a comment or trailing comma that
 * is legal under one flag and not under another, a JSON5 form, an escape
 * sequence at the edge of the surrogate range.  Many of the files differ from
 * a sibling in one byte and must report a *different* error code or a
 * different `err.pos`.
 *
 * Per file, per flag combination:
 *   - the standard stage-2 read-result group (ok / code / msg / pos /
 *     read size / value count / canonical tree), which is what pins the
 *     diagnostic as well as the tree;
 *   - `yyjson_locate_pos` on `err.pos`, so an off-by-one in a reported byte
 *     offset also shows up as a wrong line/column/character index rather than
 *     only as a wrong integer;
 *   - a write-back of whatever was parsed, so a tree that serialises the same
 *     way but was built differently still has a second chance to diverge.
 *
 * The flag sweep is the frozen stage-2 `RFLAGS` table: it already contains
 * every individual read flag, the JSON5 bundle and the pairs that interact,
 * so a case never has to carry its own flag list and the corpus stays pure
 * bytes.
 *============================================================================*/

/** `yyjson_locate_pos` over the whole input at the reported error offset.
    Recorded for every read, error or not: on a success `err.pos` is 0 and the
    location of offset 0 is itself a fact about the function. */
static void f_locate(buf *b, const char *dat, size_t len, size_t pos) {
    size_t line = 0, col = 0, chr = 0;
    bool ok = yyjson_locate_pos(dat, len, pos, &line, &col, &chr);
    f_bool(b, "loc", ok);
    f_u(b, "line", (unsigned long long)line);
    f_u(b, "col", (unsigned long long)col);
    f_u(b, "chr", (unsigned long long)chr);
    /* the NULL out-parameter paths, which are separate branches in C */
    f_bool(b, "loc_n", yyjson_locate_pos(dat, len, pos, NULL, NULL, NULL));
}

void mod_x01(const char *corpus) {
    flist fl = corpus_list(corpus, "X-01");
    buf b;
    buf_init(&b);
    char path[4096];

    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) {
            buf_adds(&b, "X-01 read");
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

            buf_adds(&b, "X-01 read");
            f_cstr(&b, "case", fl.name[i]);
            f_cstr(&b, "f", RFLAGS[f].name);
            buf_addf(&b, " flg=0x%08X", (unsigned)flg);
            f_u(&b, "inlen", (unsigned long long)len);
            f_u(&b, "maxmem", (unsigned long long)
                yyjson_read_max_memory_usage(len, flg));
            f_read_result(&b, doc, &err);
            /* locate against the ORIGINAL bytes, never the in-situ scratch:
               an in-situ read overwrites its input, so locating in `in` would
               make the record depend on the flag through the buffer */
            f_locate(&b, dat, len, err.pos);

            /* Write the parsed document straight back.  Two trees that
               serialise identically can still have been built differently
               (a wrong string length, a raw value mistaken for a string),
               and the writer is the cheapest way to notice. */
            if (doc) {
                size_t ol = 0;
                yyjson_write_err we;
                memset(&we, 0, sizeof(we));
                char *o = yyjson_write_opts(doc,
                              YYJSON_WRITE_ALLOW_INF_AND_NAN |
                              YYJSON_WRITE_ALLOW_INVALID_UNICODE,
                              NULL, &ol, &we);
                f_u(&b, "wcode", (unsigned long long)we.code);
                f_cstr(&b, "wmsg", we.msg);
                f_bytes(&b, "wout", o, o ? ol : 0);
                free(o);
                /* and once with ESCAPE_UNICODE, which takes the other branch
                   of every string the reader just produced */
                ol = 0;
                memset(&we, 0, sizeof(we));
                o = yyjson_write_opts(doc,
                        YYJSON_WRITE_ESCAPE_UNICODE |
                        YYJSON_WRITE_ALLOW_INF_AND_NAN |
                        YYJSON_WRITE_ALLOW_INVALID_UNICODE,
                        NULL, &ol, &we);
                f_u(&b, "ecode", (unsigned long long)we.code);
                f_bytes(&b, "eout", o, o ? ol : 0);
                free(o);
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
