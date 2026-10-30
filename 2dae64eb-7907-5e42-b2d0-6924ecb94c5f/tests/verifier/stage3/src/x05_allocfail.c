#include "s3.h"

/*==============================================================================
 * MARK: - X-05 allocation failure injection
 *
 * Each file under corpus/X-05/ is a `vm3` script, replayed once per value of
 * N with the Nth allocator call refused, for N from 1 up to one past the
 * number of calls the successful run makes.  N = 0 is the unrefused run and is
 * recorded too, so the sweep contains its own baseline.
 *
 * Three things must match C on every N:
 *   - the error code and message the refused operation reports;
 *   - the partial state: the document root, the serialised JSON and the two
 *     immutable slots exactly as C left them, plus a per-op digest that
 *     localises the first divergence to one op index;
 *   - the number of outstanding blocks after the failure has been cleaned up.
 *     A non-zero `live` where C has zero is a leak on an error path, and a
 *     zero where C has non-zero means the port freed something C still owns.
 *
 * The full allocator event list is recorded as well.  A port that allocates
 * the same total but on a different growth schedule diverges there even when
 * every value it produces is right, which is exactly the class of difference
 * a behavioural test over valid inputs cannot see.
 *
 * `vm3` runs in quiet mode here: one record line for the state and one for
 * the allocator accounting, per N.  The same scripts in X-04 are replayed
 * verbosely, so the digest is never the only evidence available.
 *============================================================================*/

void mod_x05(const char *corpus) {
    flist fl = corpus_list(corpus, "X-05");
    buf b;
    buf_init(&b);
    char path[4096];
    char key[4300];

    for (size_t i = 0; i < fl.n; i++) {
        char *dat = NULL;
        size_t len = 0;
        snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
        if (!file_slurp(path, &dat, &len)) continue;

        /* N = 0 is the baseline: it both pins the successful replay and
           measures the sweep's length. */
        size_t total = 0;
        for (size_t n = 0;; n++) {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            t.fail_after = (n == 0) ? 0 : n - 1;

            snprintf(key, sizeof(key), "%s#%zu", fl.name[i], n);
            vm3_stat st;
            memset(&st, 0, sizeof(st));
            vm3_run("X-05", key, dat, len, &alc, 0, &st);

            buf_adds(&b, "X-05 alloc");
            f_cstr(&b, "case", key);
            f_u(&b, "fail_at", (unsigned long long)n);
            f_u(&b, "ops", st.n_ops);
            f_u(&b, "nfail", st.n_fail);
            f_u(&b, "live", (unsigned long long)t.live);
            f_u(&b, "ncall", (unsigned long long)t.n_alloc);
            f_alc_trace(&b, "alc", &t);
            rec_line(&b);
            rec_cases++;

            if (n == 0) total = t.n_alloc;
            alc_trace_free(&t);

            if (n >= total + 1) break;
            /* A script whose unrefused replay allocates nothing still gets
               its N = 1 probe, which is what proves it allocates nothing. */
            if (total == 0 && n >= 1) break;
        }
        free(dat);
    }
    flist_free(&fl);
    buf_free(&b);
}
