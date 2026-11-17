#include "s3.h"

const char *g_tmpdir = NULL;

typedef struct {
    const char *name;
    mod_fn      fn;
} mod_entry;

static const mod_entry MODS[] = {
    { "X-01", mod_x01 },
    { "X-02", mod_x02 },
    { "X-03", mod_x03 },
    { "X-04", mod_x04 },
    { "X-05", mod_x05 },
};
static const size_t MODS_N = sizeof(MODS) / sizeof(MODS[0]);

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr,
                "usage: harness3 <module> <corpus-dir> <out-file> [tmp-dir]\n"
                "       module is one of X-01..X-05, or `all`, or `list`\n");
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
        fprintf(stderr, "harness3: unknown module '%s'\n", mod);
        rec_close();
        return 2;
    }
    rec_close();
    return 0;
}
