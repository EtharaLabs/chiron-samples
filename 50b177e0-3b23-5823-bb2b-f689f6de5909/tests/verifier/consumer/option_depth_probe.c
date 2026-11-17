/* What a consumer observes when it sets a depth limit: module B-13.
 *
 * The exported symbol set cannot tell an honoured depth limit from an ignored one. State A
 * exports all 52 symbols whether a depth limit is compiled in or not, so the relation behind
 * T-23 is satisfied by a submission that accepts `-DYYJSON_READER_DEPTH_LIMIT=8` and then
 * builds a library that reads any depth at all. This program is the observation that does
 * separate them, and it is the only thing module B-13 reads.
 *
 * It asks the installed library, through the unchanged public header alone, which nesting
 * depths it accepts and which it refuses, on the three paths a depth limit reaches: the
 * reader, the immutable writer and the mutable writer. Nothing here knows what a limit was
 * configured; the record is compared against the record State A's own library produced under
 * the same `-D`, frozen into the image at image build time. A library that honours the limit
 * reproduces State A's refusals; a library that ignores it accepts everything and diverges.
 *
 * The record carries only booleans, State A's own error codes and the byte length of a
 * fourteen-byte-at-most output. It reads no clock, measures no rate, allocates nothing whose
 * address it prints, and depends on no word size, alignment or byte order, so the same text
 * is correct on every platform the bundle builds for. It names no module, type or function of
 * the submission: a port laid out differently from the reference produces the same record.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "yyjson.h"

/* Deeper than any limit the harness configures, and shallow enough that every case is an
 * ordinary document rather than a stress test. State A's own records under a limit of 8 turn
 * over inside this range, which is what makes the record discriminating. */
#define PROBE_MAX_DEPTH 12

/* `[[[...1...]]]` at the requested nesting depth. */
static char *nested_text(int depth, size_t *length) {
    size_t span = (size_t)depth * 2u + 1u;
    char *text = (char *)malloc(span + 1u);
    size_t at = 0;
    int i;
    if (!text) return NULL;
    for (i = 0; i < depth; i++) text[at++] = '[';
    text[at++] = '1';
    for (i = 0; i < depth; i++) text[at++] = ']';
    text[at] = '\0';
    *length = at;
    return text;
}

/* An array nested `depth` deep, built through the mutable API and never through the reader,
 * so the mutable writer is observed even when the reader is the limited side. */
static yyjson_mut_doc *nested_mutable(int depth) {
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root, *cursor, *child;
    int i;
    if (!doc) return NULL;
    root = yyjson_mut_arr(doc);
    if (!root) { yyjson_mut_doc_free(doc); return NULL; }
    cursor = root;
    for (i = 1; i < depth; i++) {
        child = yyjson_mut_arr(doc);
        if (!child || !yyjson_mut_arr_append(cursor, child)) {
            yyjson_mut_doc_free(doc);
            return NULL;
        }
        cursor = child;
    }
    yyjson_mut_doc_set_root(doc, root);
    return doc;
}

int main(void) {
    int depth;

    printf("depth-probe max=%d\n", PROBE_MAX_DEPTH);

    for (depth = 1; depth <= PROBE_MAX_DEPTH; depth++) {
        size_t length = 0;
        char *text = nested_text(depth, &length);
        yyjson_read_err read_error;
        yyjson_doc *doc;

        if (!text) { printf("read d=%d unbuilt\n", depth); return 1; }
        memset(&read_error, 0, sizeof read_error);
        doc = yyjson_read_opts(text, length, 0, NULL, &read_error);
        printf("read d=%d ok=%d code=%u\n", depth, doc != NULL, (unsigned)read_error.code);
        if (doc) {
            size_t written = 0;
            yyjson_write_err write_error;
            char *out;
            memset(&write_error, 0, sizeof write_error);
            out = yyjson_write_opts(doc, 0, NULL, &written, &write_error);
            printf("write d=%d ok=%d code=%u len=%zu\n", depth, out != NULL,
                   (unsigned)write_error.code, out ? written : (size_t)0);
            free(out);
            yyjson_doc_free(doc);
        }
        free(text);
    }

    for (depth = 1; depth <= PROBE_MAX_DEPTH; depth++) {
        yyjson_mut_doc *doc = nested_mutable(depth);
        size_t written = 0;
        yyjson_write_err write_error;
        char *out;

        if (!doc) { printf("mutwrite d=%d unbuilt\n", depth); return 1; }
        memset(&write_error, 0, sizeof write_error);
        out = yyjson_mut_write_opts(doc, 0, NULL, &written, &write_error);
        printf("mutwrite d=%d ok=%d code=%u len=%zu\n", depth, out != NULL,
               (unsigned)write_error.code, out ? written : (size_t)0);
        free(out);
        yyjson_mut_doc_free(doc);
    }

    return 0;
}
