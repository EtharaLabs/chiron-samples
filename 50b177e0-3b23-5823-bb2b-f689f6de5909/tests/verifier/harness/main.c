#include "common.h"

const char *g_tmpdir = NULL;

typedef struct {
    const char *name;
    mod_fn      fn;
} mod_entry;

static const mod_entry MODS[] = {
    { "B-01", mod_reader },
    { "B-02", mod_errors },
    { "B-03", mod_numbers },
    { "B-04", mod_writer },
    { "B-05", mod_mutable },
    { "B-06", mod_ptrpatch },
    { "B-07", mod_alloc },
    { "B-08", mod_incr },
    { "B-09", mod_hdr },
    { "B-10", mod_files },
};
static const size_t MODS_N = sizeof(MODS) / sizeof(MODS[0]);

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr,
                "usage: harness <module> <corpus-dir> <out-file> [tmp-dir]\n"
                "       module is one of B-01..B-10, or `all`, or `list`\n");
        return 2;
    }
    const char *mod    = argv[1];
    const char *corpus = argv[2];
    const char *out    = argv[3];
    g_tmpdir = (argc > 4) ? argv[4] : getenv("HARNESS_TMPDIR");

    if (strcmp(mod, "list") == 0) {
        for (size_t i = 0; i < MODS_N; i++) printf("%s\n", MODS[i].name);
        return 0;
    }

    rec_open(out);

    int ran = 0;
    for (size_t i = 0; i < MODS_N; i++) {
        if (strcmp(mod, "all") != 0 && strcmp(mod, MODS[i].name) != 0)
            continue;

        /* Each module's byte stream is self-delimiting and independent of the
           modules around it, so running one module alone yields exactly the
           same bytes as its section inside `all`. */
        unsigned long long l0 = rec_lines, c0 = rec_cases;
        buf b;
        buf_init(&b);
        buf_addf(&b, "HDR format=1 module=%s", MODS[i].name);
        f_u(&b, "libver", (unsigned long long)yyjson_version());
        rec_line(&b);
        buf_free(&b);

        MODS[i].fn(corpus);
        ran = 1;

        buf_init(&b);
        buf_addf(&b, "END module=%s lines=%llu cases=%llu", MODS[i].name,
                 rec_lines - l0, rec_cases - c0);
        rec_line(&b);
        buf_free(&b);
    }
    if (!ran) {
        fprintf(stderr, "harness: unknown module '%s'\n", mod);
        rec_close();
        return 2;
    }
    rec_close();
    return 0;
}
