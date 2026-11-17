#include "s3.h"

/*==============================================================================
 * MARK: - X-04 mutation, pointer and patch sequences
 *
 * Each file under corpus/X-04/ is one `vm3` script: a multi-step sequence that
 * builds or reads a document, mutates it, resolves and modifies JSON Pointers,
 * applies an RFC 6902 patch and an RFC 7386 merge patch, copies the document
 * between its mutable and immutable forms, and compares the results for
 * equality.
 *
 * A single-step test cannot catch the interesting failures here.  What matters
 * is the state an operation leaves *for the next operation*: a patch that
 * fails on its third op must leave the output document exactly as C leaves it,
 * a `remove` must unlink without corrupting the sibling chain, a context
 * handed back by `ptr_getx` must still be valid after the container it points
 * into has been mutated, and a mutable-to-immutable round trip must preserve
 * duplicate keys, string lengths and number subtypes.  The VM therefore emits
 * one record line per executed op, each carrying the whole document root, so a
 * divergence is attributed to the op that caused it rather than to the end of
 * the script.
 *============================================================================*/

void mod_x04(const char *corpus) {
    flist fl = corpus_list(corpus, "X-04");
    char path[4096];

    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) {
            buf b;
            buf_init(&b);
            buf_adds(&b, "X-04 op");
            f_cstr(&b, "case", fl.name[i]);
            buf_adds(&b, " !slurp_failed");
            rec_line(&b);
            buf_free(&b);
            continue;
        }
        vm3_run("X-04", fl.name[i], dat, len, NULL, 1, NULL);
        free(dat);
    }
    flist_free(&fl);
}
