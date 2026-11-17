#include "common.h"

/*==============================================================================
 * MARK: - B-08 the incremental reader
 *
 * `yyjson_incr_read`'s `len` is the CUMULATIVE number of bytes available in
 * the buffer handed to `yyjson_incr_new`, not a chunk size: the C sets
 * `end = hdr + len` on every call and rejects `len > buf_len`.  A "chunk size
 * of k" therefore means feeding the cumulative lengths k, 2k, 3k, ... and the
 * reader has to resume from the exact parser state it suspended in.
 *
 * That resumption is the whole point of the module: every suspend/resume label
 * in the C (doc_begin, arr_val_begin/end, obj_key_begin/end, obj_val_begin/end,
 * doc_end) has to be reachable, and feeding one byte at a time over the whole
 * corpus is what reaches all of them, in every order.
 *
 * Passes:
 *   step    every corpus file at chunk sizes 1, 2, 3, 7, 64, 4096, len-1 and
 *           len, recording each step's return, code, message and byte
 *           position, then the final document.
 *   equiv   the incremental result against the one-shot `yyjson_read_opts`
 *           result for the same input and flags: they must agree exactly.
 *   flags   the flag masking `yyjson_incr_new` performs (JSON5, ALLOW_BOM and
 *           ALLOW_INVALID_UNICODE are stripped), and INSITU, which makes the
 *           state borrow the caller's buffer instead of copying it.
 *   misuse  reading past completion, reading after an error, a zero-length
 *           chunk, a cumulative length beyond the buffer, a non-monotonic
 *           length, NULL state, NULL buffer, a zero-length buffer, freeing
 *           mid-stream, and freeing twice via separate states.
 *============================================================================*/

#define STEP_CAP 4096u

static const size_t CHUNKS[] = { 1, 2, 3, 7, 64, 4096 };
static const size_t CHUNKS_N = sizeof(CHUNKS) / sizeof(CHUNKS[0]);

/** Drive one input to completion at one cumulative step size.
    Emits one `step` record holding the whole step sequence, then one `done`
    record holding the final outcome. */
static void do_steps(buf *b, const char *key, const char *dat, size_t len,
                     yyjson_read_flag flg, const char *fname, size_t step) {
    if (step == 0) return;

    /* `yyjson_incr_new` copies the buffer unless INSITU is set, in which case
       it borrows it and requires the padding to already be there. */
    char *in = pad_copy(dat, len);
    yyjson_incr_state *st = yyjson_incr_new(in, len, flg, NULL);

    buf_adds(b, "B-08 step");
    f_cstr(b, "case", key);
    f_cstr(b, "f", fname);
    f_u(b, "step", (unsigned long long)step);
    f_u(b, "inlen", (unsigned long long)len);
    f_bool(b, "new", st != NULL);

    yyjson_doc *doc = NULL;
    size_t nsteps = 0;
    int finished = 0;
    yyjson_read_err last;
    memset(&last, 0, sizeof(last));

    if (st) {
        buf_adds(b, " seq=");
        size_t avail = 0;
        while (!finished) {
            avail += step;
            if (avail > len) avail = len;
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            doc = yyjson_incr_read(st, avail, &e);
            last = e;

            if (nsteps) buf_addc(b, '+');
            /* avail / returned-a-doc / code / pos; the message only when it
               is not the `need more data` one, which would otherwise repeat
               thousands of times for a one-byte sweep */
            buf_addf(b, "%zu.%d.%u.%zu", avail, doc != NULL ? 1 : 0,
                     (unsigned)e.code, e.pos);
            if (e.code != YYJSON_READ_ERROR_MORE && e.msg) {
                buf_addc(b, '.');
                buf_q(b, e.msg, strlen(e.msg));
            }
            nsteps++;

            if (doc) finished = 1;
            else if (e.code != YYJSON_READ_ERROR_MORE) finished = 1;
            else if (avail >= len) finished = 1;
            else if (nsteps > STEP_CAP) finished = 1;
        }
        if (!nsteps) buf_adds(b, "!empty");
    }
    f_u(b, "nsteps", (unsigned long long)nsteps);
    rec_line(b);
    rec_cases++;

    buf_adds(b, "B-08 done");
    f_cstr(b, "case", key);
    f_cstr(b, "f", fname);
    f_u(b, "step", (unsigned long long)step);
    f_bool(b, "ok", doc != NULL);
    f_u(b, "code", (unsigned long long)last.code);
    f_cstr(b, "msg", last.msg);
    f_u(b, "pos", (unsigned long long)last.pos);
    if (doc) {
        f_u(b, "read", (unsigned long long)yyjson_doc_get_read_size(doc));
        f_u(b, "vals", (unsigned long long)yyjson_doc_get_val_count(doc));
        f_tree_imut(b, "tree", yyjson_doc_get_root(doc));
    }
    rec_line(b);
    rec_cases++;

    yyjson_doc_free(doc);
    yyjson_incr_free(st);
    free(in);
}

/** The incremental result against the one-shot result. */
static void do_equiv(buf *b, const char *key, const char *dat, size_t len,
                     yyjson_read_flag flg, const char *fname) {
    char *in1 = pad_copy(dat, len);
    yyjson_read_err e1;
    memset(&e1, 0, sizeof(e1));
    yyjson_doc *one = yyjson_read_opts(in1, len, flg, NULL, &e1);

    /* the whole buffer in a single incremental call */
    char *in2 = pad_copy(dat, len);
    yyjson_incr_state *st = yyjson_incr_new(in2, len, flg, NULL);
    yyjson_read_err e2;
    memset(&e2, 0, sizeof(e2));
    yyjson_doc *inc = st ? yyjson_incr_read(st, len ? len : 1, &e2) : NULL;

    buf_adds(b, "B-08 equiv");
    f_cstr(b, "case", key);
    f_cstr(b, "f", fname);
    f_bool(b, "one_ok", one != NULL);
    f_u(b, "one_code", (unsigned long long)e1.code);
    f_cstr(b, "one_msg", e1.msg);
    f_u(b, "one_pos", (unsigned long long)e1.pos);
    f_bool(b, "inc_ok", inc != NULL);
    f_u(b, "inc_code", (unsigned long long)e2.code);
    f_cstr(b, "inc_msg", e2.msg);
    f_u(b, "inc_pos", (unsigned long long)e2.pos);
    f_bool(b, "same_tree",
           yyjson_equals(one ? yyjson_doc_get_root(one) : NULL,
                         inc ? yyjson_doc_get_root(inc) : NULL));
    f_u(b, "one_vals",
        (unsigned long long)(one ? yyjson_doc_get_val_count(one) : 0));
    f_u(b, "inc_vals",
        (unsigned long long)(inc ? yyjson_doc_get_val_count(inc) : 0));
    f_u(b, "one_read",
        (unsigned long long)(one ? yyjson_doc_get_read_size(one) : 0));
    f_u(b, "inc_read",
        (unsigned long long)(inc ? yyjson_doc_get_read_size(inc) : 0));
    f_tree_imut(b, "inc_tree", inc ? yyjson_doc_get_root(inc) : NULL);
    rec_line(b);
    rec_cases++;

    yyjson_doc_free(inc);
    yyjson_incr_free(st);
    free(in2);
    yyjson_doc_free(one);
    free(in1);
}

/*==============================================================================
 * MARK: - pass `misuse`
 *============================================================================*/

static void do_misuse(buf *b) {
    static const char *SRC = "[1,2,3]";
    size_t len = strlen(SRC);

    /* ---- new() parameter paths ---- */
    {
        char *in = pad_copy(SRC, len);
        buf_adds(b, "B-08 misuse case=new_params");
        yyjson_incr_state *a = yyjson_incr_new(NULL, len, 0, NULL);
        f_bool(b, "buf_null", a != NULL);
        yyjson_incr_free(a);
        yyjson_incr_state *c = yyjson_incr_new(in, 0, 0, NULL);
        f_bool(b, "len_zero", c != NULL);
        /* a zero-length buffer: new() succeeds, but every read is refused
           because `len` must be non-zero and `<= buf_len` */
        if (c) {
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            f_bool(b, "zero_read", yyjson_incr_read(c, 1, &e) != NULL);
            f_u(b, "zero_code", (unsigned long long)e.code);
            f_cstr(b, "zero_msg", e.msg);
            f_u(b, "zero_pos", (unsigned long long)e.pos);
        }
        yyjson_incr_free(c);
        yyjson_incr_state *d = yyjson_incr_new(in, (size_t)-1, 0, NULL);
        f_bool(b, "len_overflow", d != NULL);
        yyjson_incr_free(d);
        /* freeing NULL is a documented no-op */
        yyjson_incr_free(NULL);
        buf_adds(b, " free_null=1");
        rec_line(b);
        rec_cases++;
        free(in);
    }

    /* ---- read() parameter paths ---- */
    {
        buf_adds(b, "B-08 misuse case=read_params");
        yyjson_read_err e;
        memset(&e, 0, sizeof(e));
        f_bool(b, "state_null", yyjson_incr_read(NULL, 1, &e) != NULL);
        f_u(b, "state_null_code", (unsigned long long)e.code);
        f_cstr(b, "state_null_msg", e.msg);
        f_u(b, "state_null_pos", (unsigned long long)e.pos);

        char *in = pad_copy(SRC, len);
        yyjson_incr_state *st = yyjson_incr_new(in, len, 0, NULL);
        memset(&e, 0, sizeof(e));
        f_bool(b, "len_zero", yyjson_incr_read(st, 0, &e) != NULL);
        f_u(b, "len_zero_code", (unsigned long long)e.code);
        f_cstr(b, "len_zero_msg", e.msg);
        memset(&e, 0, sizeof(e));
        f_bool(b, "len_over", yyjson_incr_read(st, len + 1, &e) != NULL);
        f_u(b, "len_over_code", (unsigned long long)e.code);
        f_cstr(b, "len_over_msg", e.msg);
        memset(&e, 0, sizeof(e));
        f_bool(b, "len_max", yyjson_incr_read(st, (size_t)-1, &e) != NULL);
        f_u(b, "len_max_code", (unsigned long long)e.code);
        /* a refused call must not have advanced the state: the next legal
           call still has to parse the whole document */
        memset(&e, 0, sizeof(e));
        yyjson_doc *doc = yyjson_incr_read(st, len, &e);
        f_bool(b, "recover", doc != NULL);
        f_u(b, "recover_code", (unsigned long long)e.code);
        f_tree_imut(b, "recover_tree",
                    doc ? yyjson_doc_get_root(doc) : NULL);
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(doc);
        yyjson_incr_free(st);
        free(in);
    }

    /* ---- no err out-parameter, on a state that has not yet completed ---- */
    {
        char *in = pad_copy(SRC, len);
        yyjson_incr_state *st = yyjson_incr_new(in, len, 0, NULL);
        buf_adds(b, "B-08 misuse case=no_err_out");
        f_bool(b, "partial", st && yyjson_incr_read(st, 2, NULL) != NULL);
        yyjson_doc *d = st ? yyjson_incr_read(st, len, NULL) : NULL;
        f_bool(b, "full", d != NULL);
        f_tree_imut(b, "tree", d ? yyjson_doc_get_root(d) : NULL);
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(d);
        yyjson_incr_free(st);
        free(in);
    }

    /* ---- reading past completion ----
       NOT TESTED, and deliberately so.  On success `yyjson_incr_read` transfers
       ownership by setting `state->hdr = NULL` and `state->val_hdr = NULL`,
       but leaves `state->label` at the completed label.  A further call
       re-enters at `doc_end`, where the first statement is
       `doc = (yyjson_doc *)val_hdr; doc->root = ...` on a NULL `val_hdr`: a
       guaranteed NULL write.  So this is undefined behaviour in the C, not an
       error path with an observable code, and a crash in the harness would
       prove nothing about a submission.  The adjacent, defined misuse cases
       (reading after a HARD ERROR, which does not transfer ownership, and
       reading with a refused length) are covered above and below instead. */

    /* ---- reading after a hard error ---- */
    {
        static const char *BAD = "[1,2,}]";
        size_t bl = strlen(BAD);
        char *in = pad_copy(BAD, bl);
        yyjson_incr_state *st = yyjson_incr_new(in, bl, 0, NULL);
        buf_adds(b, "B-08 misuse case=after_error");
        for (int k = 0; k < 3; k++) {
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *d = yyjson_incr_read(st, bl, &e);
            char nm[32];
            snprintf(nm, sizeof(nm), "r%d", k);
            f_bool(b, nm, d != NULL);
            snprintf(nm, sizeof(nm), "r%d_code", k);
            f_u(b, nm, (unsigned long long)e.code);
            snprintf(nm, sizeof(nm), "r%d_msg", k);
            f_cstr(b, nm, e.msg);
            snprintf(nm, sizeof(nm), "r%d_pos", k);
            f_u(b, nm, (unsigned long long)e.pos);
            yyjson_doc_free(d);
            /* A hard error leaves ownership with the state, so re-reading is
               defined.  A success would transfer it, after which another call
               is the UB described above, so never make one. */
            if (d) break;
        }
        rec_line(b);
        rec_cases++;
        yyjson_incr_free(st);
        free(in);
    }

    /* ---- a cumulative length that goes backwards ---- */
    {
        static const char *SRC2 = "[100,200,300]";
        size_t l2 = strlen(SRC2);
        char *in = pad_copy(SRC2, l2);
        yyjson_incr_state *st = yyjson_incr_new(in, l2, 0, NULL);
        buf_adds(b, "B-08 misuse case=non_monotonic");
        static const size_t SEQ[] = { 6, 3, 6, 9, 5, 13 };
        for (size_t k = 0; k < sizeof(SEQ) / sizeof(SEQ[0]); k++) {
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *d = yyjson_incr_read(st, SEQ[k], &e);
            char nm[32];
            snprintf(nm, sizeof(nm), "a%zu", SEQ[k]);
            f_bool(b, nm, d != NULL);
            snprintf(nm, sizeof(nm), "a%zu_code", SEQ[k]);
            f_u(b, nm, (unsigned long long)e.code);
            snprintf(nm, sizeof(nm), "a%zu_pos", SEQ[k]);
            f_u(b, nm, (unsigned long long)e.pos);
            snprintf(nm, sizeof(nm), "a%zu_tree", SEQ[k]);
            f_tree_imut(b, nm, d ? yyjson_doc_get_root(d) : NULL);
            yyjson_doc_free(d);
            if (d) break;
        }
        rec_line(b);
        rec_cases++;
        yyjson_incr_free(st);
        free(in);
    }

    /* ---- freeing mid-stream, at every suspend point of one document ---- */
    {
        static const char *SRC3 = "{\"key\":[1,{\"k2\":\"v\"},true]}";
        size_t l3 = strlen(SRC3);
        for (size_t cut = 1; cut <= l3; cut++) {
            char *in = pad_copy(SRC3, l3);
            yyjson_incr_state *st = yyjson_incr_new(in, l3, 0, NULL);
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *d = st ? yyjson_incr_read(st, cut, &e) : NULL;
            buf_adds(b, "B-08 misuse");
            buf_addf(b, " case=free_midstream/%zu", cut);
            f_bool(b, "ok", d != NULL);
            f_u(b, "code", (unsigned long long)e.code);
            f_cstr(b, "msg", e.msg);
            f_u(b, "pos", (unsigned long long)e.pos);
            f_tree_imut(b, "tree", d ? yyjson_doc_get_root(d) : NULL);
            rec_line(b);
            rec_cases++;
            /* free the state while it is still suspended: everything it
               allocated has to go, including the partial value buffer */
            yyjson_doc_free(d);
            yyjson_incr_free(st);
            free(in);
        }
    }

    /* ---- mid-stream free with a tracing allocator, so a leak on the
       suspended path is visible as a non-zero outstanding count ---- */
    {
        static const char *SRC4 = "[1,[2,[3,[4,\"abcdefghij\"]]],{\"a\":1}]";
        size_t l4 = strlen(SRC4);
        for (size_t cut = 1; cut <= l4; cut++) {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            char *in = pad_copy(SRC4, l4);
            yyjson_incr_state *st = yyjson_incr_new(in, l4, 0, &alc);
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *d = st ? yyjson_incr_read(st, cut, &e) : NULL;
            yyjson_doc_free(d);
            yyjson_incr_free(st);
            buf_adds(b, "B-08 misuse");
            buf_addf(b, " case=free_leak/%zu", cut);
            f_bool(b, "ok", d != NULL);
            f_u(b, "code", (unsigned long long)e.code);
            f_u(b, "live", (unsigned long long)t.live);
            f_alc_trace(b, "alc", &t);
            rec_line(b);
            rec_cases++;
            alc_trace_free(&t);
            free(in);
        }
    }

    /* ---- a root-level number longer than INCR_NUM_MAX_LEN ----
       The incremental reader gives up on a root-level number once it has
       consumed more than 1024 bytes without reaching the end of the buffer,
       reporting "number too long".  No corpus literal is anywhere near that
       long, so the only way to reach the diagnostic is to build one.  The
       sweep straddles the threshold so the boundary itself is pinned, not just
       one side of it. */
    {
        /* The check is `cur > state->cur + INCR_NUM_MAX_LEN` while the buffer
           is exhausted, and `state->cur` is re-saved on every suspend.  So the
           threshold is crossed only when a SINGLE chunk delivers more than
           1024 new bytes of digits, not when the number is merely longer than
           that in total.  The step sweep therefore includes sizes on both
           sides of 1024. */
        static const size_t NLEN[] = { 1022, 1025, 2048, 4096 };
        static const size_t NSTEP[] = { 256, 1024, 1025, 1100, 3000 };
        for (size_t i = 0; i < sizeof(NLEN) / sizeof(NLEN[0]); i++)
        for (size_t sI = 0; sI < sizeof(NSTEP) / sizeof(NSTEP[0]); sI++) {
            size_t nl = NLEN[i];
            size_t nstep = NSTEP[sI];
            char *num = (char *)malloc(nl + 1);
            if (!num) { fprintf(stderr, "harness: oom\n"); exit(2); }
            /* A long INTEGER is useless here: past ~309 digits `read_num`
               fails with "number is infinity when parsed as double" before the
               length check is ever reached.  `check_maybe_truncated_number` is
               only consulted after `read_num` SUCCEEDS.  A long real `1.ddd…`
               stays finite however many digits follow, so it is the only shape
               that both succeeds and exceeds INCR_NUM_MAX_LEN. */
            num[0] = '1';
            num[1] = '.';
            for (size_t k = 2; k < nl; k++) num[k] = (char)('0' + (k % 10));
            num[nl] = '\0';

            /* The macro has three call sites: a number at the document root,
               a number inside an array (`arr_val_maybe_end`) and a number
               inside an object (`obj_val_maybe_end`).  Wrapping the same digit
               run in `[...,1]` and `{"k":...,"j":1}` reaches the latter two;
               the trailing element guarantees buffer remains after the number,
               which the check requires. */
            buf shaped[3];
            for (int sh = 0; sh < 3; sh++) buf_init(&shaped[sh]);
            buf_add(&shaped[0], num, nl);
            buf_addc(&shaped[1], '[');
            buf_add(&shaped[1], num, nl);
            buf_adds(&shaped[1], ",1]");
            buf_adds(&shaped[2], "{\"k\":");
            buf_add(&shaped[2], num, nl);
            buf_adds(&shaped[2], ",\"j\":1}");
            static const char *const SHN[3] = { "root", "arr", "obj" };

            static const yyjson_read_flag LF[3] = {
                YYJSON_READ_NOFLAG, YYJSON_READ_NUMBER_AS_RAW,
                YYJSON_READ_BIGNUM_AS_RAW
            };
            static const char *const LFN[3] = { "none", "numraw",
                                                "bignumraw" };
            for (int sh = 0; sh < 3; sh++)
            for (int f = 0; f < 3; f++) {
                const char *src = shaped[sh].dat;
                size_t slen = shaped[sh].len;
                char *in = pad_copy(src, slen);
                yyjson_incr_state *st = yyjson_incr_new(in, slen, LF[f], NULL);
                yyjson_doc *d = NULL;
                yyjson_read_err e;
                memset(&e, 0, sizeof(e));
                size_t avail = 0, steps = 0;
                while (st) {
                    avail += nstep;
                    if (avail > slen) avail = slen;
                    memset(&e, 0, sizeof(e));
                    d = yyjson_incr_read(st, avail, &e);
                    steps++;
                    if (d || e.code != YYJSON_READ_ERROR_MORE) break;
                    if (avail >= slen) break;
                }
                buf_adds(b, "B-08 misuse");
                buf_addf(b, " case=long_number/%s/%zu/%zu", SHN[sh], nl,
                         nstep);
                f_cstr(b, "f", LFN[f]);
                f_u(b, "inlen", (unsigned long long)slen);
                f_u(b, "steps", (unsigned long long)steps);
                f_bool(b, "ok", d != NULL);
                f_u(b, "code", (unsigned long long)e.code);
                f_cstr(b, "msg", e.msg);
                f_u(b, "pos", (unsigned long long)e.pos);
                f_tree_imut(b, "tree", d ? yyjson_doc_get_root(d) : NULL);
                /* the one-shot reader has no such limit, so the two are
                   expected to disagree here; recording both pins that */
                char *in2 = pad_copy(src, slen);
                yyjson_read_err e2;
                memset(&e2, 0, sizeof(e2));
                yyjson_doc *one = yyjson_read_opts(in2, slen, LF[f], NULL,
                                                   &e2);
                f_bool(b, "one_ok", one != NULL);
                f_u(b, "one_code", (unsigned long long)e2.code);
                f_cstr(b, "one_msg", e2.msg);
                rec_line(b);
                rec_cases++;
                yyjson_doc_free(one);
                free(in2);
                yyjson_doc_free(d);
                yyjson_incr_free(st);
                free(in);
            }
            for (int sh = 0; sh < 3; sh++) buf_free(&shaped[sh]);
            free(num);
        }
    }

    /* ---- INSITU: the state borrows the caller's buffer ---- */
    {
        char *in = pad_copy(SRC, len);
        yyjson_incr_state *st = yyjson_incr_new(in, len, YYJSON_READ_INSITU,
                                                NULL);
        buf_adds(b, "B-08 misuse case=insitu");
        f_bool(b, "new", st != NULL);
        yyjson_doc *d = NULL;
        for (size_t avail = 1; avail <= len; avail++) {
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            d = yyjson_incr_read(st, avail, &e);
            char nm[32];
            snprintf(nm, sizeof(nm), "a%zu", avail);
            f_bool(b, nm, d != NULL);
            snprintf(nm, sizeof(nm), "a%zu_code", avail);
            f_u(b, nm, (unsigned long long)e.code);
            if (d) break;
        }
        f_tree_imut(b, "tree", d ? yyjson_doc_get_root(d) : NULL);
        rec_line(b);
        rec_cases++;
        yyjson_doc_free(d);
        yyjson_incr_free(st);
        free(in);
    }

    /* ---- allocation failure at every call of an incremental parse ---- */
    {
        static const char *SRC5 = "{\"a\":[1,2,\"xyz\"],\"b\":null}";
        size_t l5 = strlen(SRC5);
        for (size_t n = 1; n <= 12; n++) {
            alc_trace t;
            yyjson_alc alc;
            alc_trace_init(&t, &alc);
            t.fail_after = n - 1;
            char *in = pad_copy(SRC5, l5);
            yyjson_incr_state *st = yyjson_incr_new(in, l5, 0, &alc);
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            yyjson_doc *d = NULL;
            size_t avail = 0;
            int steps = 0;
            while (st && avail < l5) {
                avail += 4;
                if (avail > l5) avail = l5;
                memset(&e, 0, sizeof(e));
                d = yyjson_incr_read(st, avail, &e);
                steps++;
                if (d || e.code != YYJSON_READ_ERROR_MORE) break;
            }
            buf_adds(b, "B-08 misuse");
            buf_addf(b, " case=alloc_fail/%zu", n);
            f_bool(b, "new", st != NULL);
            f_bool(b, "ok", d != NULL);
            f_u(b, "steps", (unsigned long long)steps);
            f_u(b, "code", (unsigned long long)e.code);
            f_cstr(b, "msg", e.msg);
            f_u(b, "pos", (unsigned long long)e.pos);
            yyjson_doc_free(d);
            yyjson_incr_free(st);
            f_u(b, "live", (unsigned long long)t.live);
            f_alc_trace(b, "alc", &t);
            rec_line(b);
            rec_cases++;
            alc_trace_free(&t);
            free(in);
        }
    }
}

/*==============================================================================
 * MARK: - pass `flags`
 *
 * `yyjson_incr_new` strips JSON5, ALLOW_BOM and ALLOW_INVALID_UNICODE, so an
 * input that only the stripped flag would accept must be rejected even when
 * the caller asked for it.  That is a silent behaviour, and the only way to
 * see it is to compare against the one-shot reader with the same flags.
 *============================================================================*/

static void do_flags(buf *b) {
    static const struct { const char *name, *src; } IN[] = {
        { "plain",    "[1,2]" },
        { "bom",      "\xef\xbb\xbf[1]" },
        { "comment",  "[1,/*c*/2]" },
        { "trailing", "[1,2,]" },
        { "infnan",   "[Infinity,NaN]" },
        { "sqstr",    "['a']" },
        { "unqkey",   "{a:1}" },
        { "hex",      "[0x10]" },
        { "badutf8",  "[\"a\x80\"]" },
        { "extesc",   "[\"\\x41\"]" },
        { "extws",    "[\x0b1]" },
        { "bignum",   "[123456789012345678901234567890]" },
        { "numraw",   "[1.5]" },
    };
    static const size_t IN_N = sizeof(IN) / sizeof(IN[0]);

    for (size_t i = 0; i < IN_N; i++) {
        size_t len = strlen(IN[i].src);
        for (size_t f = 0; f < RFLAGS_ALL_N; f++) {
            do_equiv(b, IN[i].name, IN[i].src, len, RFLAGS_ALL[f].flg,
                     RFLAGS_ALL[f].name);
            /* and one byte at a time, which is where resumption actually
               has to carry the flag state across calls */
            do_steps(b, IN[i].name, IN[i].src, len, RFLAGS_ALL[f].flg,
                     RFLAGS_ALL[f].name, 1);
        }
        /* the composite flag sets too */
        for (size_t f = 0; f < RFLAGS_N; f++) {
            do_equiv(b, IN[i].name, IN[i].src, len, RFLAGS[f].flg,
                     RFLAGS[f].name);
        }
    }
}

/*==============================================================================
 * MARK: - entry point
 *============================================================================*/

void mod_incr(const char *corpus) {
    buf b;
    buf_init(&b);
    char path[4096];

    do_misuse(&b);
    do_flags(&b);

    /* Root-level numbers, one literal per line from corpus/num.
       A number at the root has no closing delimiter, so the incremental reader
       cannot know it is finished until the buffer is exhausted.  That is the
       only path that reaches `check_maybe_truncated_number` and its
       INCR_NUM_MAX_LEN "number too long" diagnostic, and it is also where a
       document can be returned while input still remains.  No bracketed
       corpus document can get there. */
    {
        flist nl = corpus_list(corpus, "num");
        for (size_t i = 0; i < nl.n; i++) {
            char *dat = NULL;
            size_t len = 0;
            snprintf(path, sizeof(path), "%s/%s", corpus, nl.name[i]);
            if (!file_slurp(path, &dat, &len)) continue;

            size_t p = 0, lineno = 0, done = 0;
            while (p <= len && done < 60) {
                size_t q = p;
                while (q < len && dat[q] != '\n') q++;
                size_t n = q - p;
                if (n && dat[p + n - 1] == '\r') n--;
                lineno++;
                if (n != 0 && dat[p] != '#' && n < 512) {
                    char key[4200];
                    snprintf(key, sizeof(key), "%s:%zu", nl.name[i], lineno);
                    static const yyjson_read_flag NF[3] = {
                        YYJSON_READ_NOFLAG,
                        YYJSON_READ_NUMBER_AS_RAW,
                        YYJSON_READ_ALLOW_INF_AND_NAN |
                        YYJSON_READ_ALLOW_EXT_NUMBER
                    };
                    static const char *const NFN[3] = { "none", "numraw",
                                                        "extnum" };
                    for (int f = 0; f < 3; f++) {
                        do_steps(&b, key, dat + p, n, NF[f], NFN[f], 1);
                        do_steps(&b, key, dat + p, n, NF[f], NFN[f], 7);
                        do_equiv(&b, key, dat + p, n, NF[f], NFN[f]);
                    }
                    done++;
                }
                if (q >= len) break;
                p = q + 1;
            }
            free(dat);
        }
        flist_free(&nl);
    }

    static const char *SUBS[3] = { "doc", "err", "json" };
    for (int s = 0; s < 3; s++) {
        flist fl = corpus_list(corpus, SUBS[s]);
        for (size_t i = 0; i < fl.n; i++) {
            char *dat = NULL;
            size_t len = 0;
            snprintf(path, sizeof(path), "%s/%s", corpus, fl.name[i]);
            if (!file_slurp(path, &dat, &len)) continue;

            /* the fixed chunk ladder, plus len-1 and len, which are the two
               sizes that exercise "almost there" and "all at once" */
            for (size_t c = 0; c < CHUNKS_N; c++) {
                /* a one-byte sweep over a megabyte-scale corpus file would
                   dominate the record for no extra coverage, so the small
                   step sizes are reserved for inputs short enough that every
                   step is a distinct parser state */
                if (CHUNKS[c] < 64 && len > 4096) continue;
                do_steps(&b, fl.name[i], dat, len, YYJSON_READ_NOFLAG,
                         "none", CHUNKS[c]);
            }
            if (len > 1)
                do_steps(&b, fl.name[i], dat, len, YYJSON_READ_NOFLAG,
                         "none", len - 1);
            if (len > 0)
                do_steps(&b, fl.name[i], dat, len, YYJSON_READ_NOFLAG,
                         "none", len);

            /* Stepwise under NUMBER_AS_RAW / BIGNUM_AS_RAW.
               The incremental reader defers a RAW value's null terminator
               across a suspend (`state->raw_ptr` / `state->raw_end`) and
               commits it at the next `save_incr_state`.  Nothing else in the
               harness touches that: a whole-buffer read never suspends, and a
               stepwise read without a raw flag never creates a RAW value.  So
               the small step sizes are repeated under both raw flags. */
            if (len <= 4096) {
                static const size_t RSTEP[3] = { 1, 3, 7 };
                for (int rs = 0; rs < 3; rs++) {
                    do_steps(&b, fl.name[i], dat, len,
                             YYJSON_READ_NUMBER_AS_RAW, "numraw", RSTEP[rs]);
                    do_steps(&b, fl.name[i], dat, len,
                             YYJSON_READ_BIGNUM_AS_RAW, "bignumraw",
                             RSTEP[rs]);
                }
            }

            do_equiv(&b, fl.name[i], dat, len, YYJSON_READ_NOFLAG, "none");
            do_equiv(&b, fl.name[i], dat, len, YYJSON_READ_INSITU, "insitu");
            do_equiv(&b, fl.name[i], dat, len, YYJSON_READ_STOP_WHEN_DONE,
                     "stop");
            do_equiv(&b, fl.name[i], dat, len, YYJSON_READ_NUMBER_AS_RAW,
                     "numraw");
            do_equiv(&b, fl.name[i], dat, len, YYJSON_READ_BIGNUM_AS_RAW,
                     "bignumraw");

            free(dat);
        }
        flist_free(&fl);
    }

    buf_free(&b);
}
