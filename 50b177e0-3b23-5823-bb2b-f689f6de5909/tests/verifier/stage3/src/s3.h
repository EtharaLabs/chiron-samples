/*==============================================================================
 * Stage 3 differential corpus: harness extension.
 *
 * This file adds five new modules (X-01 .. X-05) on top of the frozen stage-2
 * machinery in ../harness/.  Nothing under ../harness/ is modified: its
 * `common.c`, `flags.c` and `vm.c` are compiled verbatim and linked with the
 * translation units here, and the record format, the percent-encoding, the
 * canonical tree serialization and the tracing allocator are exactly the
 * stage-2 ones.  A stage-3 record line is therefore readable with the same
 * eyes, and diffable with the same `diff`, as a stage-2 one.
 *
 * The CLI is the stage-2 CLI:
 *     harness3 <module> <corpus-dir> <out-file> [tmp-dir]
 * with `module` one of X-01..X-05, `all` or `list`.
 *============================================================================*/
#ifndef STAGE3_S3_H
#define STAGE3_S3_H

#include "common.h"

/*==============================================================================
 * MARK: - module entry points
 *============================================================================*/

void mod_x01(const char *corpus); /* parse and error decision boundaries */
void mod_x02(const char *corpus); /* numeric round trip */
void mod_x03(const char *corpus); /* writer escaping and formatting */
void mod_x04(const char *corpus); /* mutation, pointer and patch sequences */
void mod_x05(const char *corpus); /* allocation failure injection */

/*==============================================================================
 * MARK: - the stage-3 script VM
 *
 * `../harness/vm.c` already interprets a mutation/pointer script, but it has
 * no RFC 6902 patch, no merge patch, no immutable side and no allocator
 * parameter, and it is frozen.  `vm3.c` is a superset interpreter that adds
 * exactly those, built on the same `buf`/`f_*`/`tree_*` primitives so its
 * output is the same dialect of record line.
 *
 * `alc` is threaded through every allocating entry point, which is what lets
 * X-05 replay a whole sequence under a refused allocation.
 *
 * In `verbose` mode one record line is emitted per op plus a final state dump
 * (what X-04 wants: the first differing line names the op that broke).  In
 * quiet mode the per-op text is accumulated and emitted as one capped
 * `ops=` field on the final line (what X-05 wants: one line per N of the
 * sweep, carrying the whole partial state).
 *============================================================================*/

typedef struct {
    unsigned long long n_ops;  /* ops executed */
    unsigned long long n_fail; /* ops that returned a failure indication */
    int                last_code; /* last non-zero error code seen */
} vm3_stat;

/** Run one script.  `src` is not modified.  `out` receives statistics and may
    be NULL.  Returns the number of ops executed. */
void vm3_run(const char *mod, const char *name, const char *src, size_t len,
             const yyjson_alc *alc, int verbose, vm3_stat *out);

/*==============================================================================
 * MARK: - shared helpers
 *============================================================================*/

/** Decode %XX escapes in `src[0..len)` into a fresh malloc'd buffer with a
    trailing NUL (not counted in `*out_len`).  Caller frees.  This is the
    inverse of `buf_q`, so a corpus line can carry any byte sequence including
    NUL, a newline or invalid UTF-8. */
char *s3_unq(const char *src, size_t len, size_t *out_len);

/** Iterate the logical lines of a slurped text corpus file: blank lines and
    lines whose first byte is '#' are skipped.  `*pos` must start at 0.
    Returns 0 when the file is exhausted.  `*lineno` is the 1-based physical
    line number of the returned line. */
int s3_next_line(const char *dat, size_t len, size_t *pos, size_t *lineno,
                 const char **out, size_t *out_len);

#endif /* STAGE3_S3_H */
