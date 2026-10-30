#include "s3.h"

/*==============================================================================
 * MARK: - the stage-3 script VM
 *
 * A superset of `../harness/vm.c`: it keeps that script dialect's shape (one
 * op per line, `rN` registers, `%XX` byte escapes, `~` for the empty string,
 * `!` for NULL) and adds the four things X-04 and X-05 need and stage 2 does
 * not have:
 *
 *   - an immutable side: two `yyjson_doc *` slots, so a document can be read,
 *     copied to mutable, mutated, copied back and compared, all in one
 *     scripted sequence;
 *   - RFC 6902 patch and RFC 7386 merge patch, through all four entry points;
 *   - an allocator parameter threaded into every allocating call, so the whole
 *     sequence can be replayed with the Nth allocation refused;
 *   - a quiet mode that folds the per-op results into one capped field, so an
 *     allocation-failure sweep costs one record line per N instead of one per
 *     (N, op).
 *
 * `vm3` is deliberately *not* a copy of `vm.c` with additions: the op names it
 * shares with `vm.c` are spelled the same way on purpose, so a script reads
 * the same, but the interpreter is independent and `vm.c` is untouched.
 *
 * Grammar: one op per line, single-space separated tokens.  Blank lines and
 * lines starting with '#' are ignored and do not consume an op index.
 *
 *   rN     register 0..9 holding a `yyjson_mut_val *`
 *   -      discard a result
 *   !      a NULL pointer
 *   ~      the empty string
 *   TEXT   byte string; %XX decodes to an arbitrary byte
 *   N      unsigned integer (base 0); `max` is SIZE_MAX
 *   S      immutable document slot, 0 or 1
 *============================================================================*/

#define VM3_NREG  10
#define VM3_NDOC  2
#define VM3_MAXTOK 8

typedef struct {
    yyjson_mut_doc *md;              /* the working mutable document */
    yyjson_doc     *id[VM3_NDOC];    /* immutable document slots */
    yyjson_mut_val *r[VM3_NREG];
    yyjson_ptr_ctx  ctx;
    yyjson_ptr_err  perr;

    const yyjson_alc *alc;

    /* interned byte strings, freed after every document */
    char  **str;
    size_t  nstr, cstr;
    /* scratch immutable docs created while parsing patches */
    yyjson_doc **sdoc;
    size_t       nsdoc, csdoc;

    int         verbose;
    const char *mod;
    const char *name;
    buf         acc;      /* quiet-mode accumulator */
    size_t      acc_n;
    int         last_code;
    const char *last_msg;
    vm3_stat    st;
} vm3;

static void vm3_die(const char *m) {
    fprintf(stderr, "harness3: vm3: %s\n", m);
    exit(2);
}

/*============================================================== token decode */

static int vm3_hexv(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

char *s3_unq(const char *src, size_t len, size_t *out_len) {
    char *d = (char *)malloc(len + 1);
    if (!d) vm3_die("oom");
    size_t w = 0;
    /* A lone `~` is the empty payload.  The one-payload-per-line corpora
       cannot spell a zero-length line (a blank line is a separator), and the
       empty string is a case the writer must be tested on, so it gets the
       same sentinel the script grammar already uses.  The generator never
       emits a bare `~` for anything else: it escapes a literal tilde. */
    if (len == 1 && src[0] == '~') { d[0] = '\0'; *out_len = 0; return d; }
    for (size_t i = 0; i < len;) {
        if (src[i] == '%' && i + 2 < len && vm3_hexv(src[i + 1]) >= 0 &&
            vm3_hexv(src[i + 2]) >= 0) {
            d[w++] = (char)((vm3_hexv(src[i + 1]) << 4) |
                            vm3_hexv(src[i + 2]));
            i += 3;
        } else {
            d[w++] = src[i++];
        }
    }
    d[w] = '\0';
    *out_len = w;
    return d;
}

int s3_next_line(const char *dat, size_t len, size_t *pos, size_t *lineno,
                 const char **out, size_t *out_len) {
    while (*pos <= len) {
        size_t p = *pos, q = p;
        while (q < len && dat[q] != '\n') q++;
        size_t n = q - p;
        if (n && dat[p + n - 1] == '\r') n--;
        (*lineno)++;
        *pos = (q >= len) ? len + 1 : q + 1;
        if (n != 0 && dat[p] != '#') {
            *out = dat + p;
            *out_len = n;
            return 1;
        }
        if (q >= len) break;
    }
    return 0;
}

/** Intern a decoded TEXT token.  `!` is NULL, `~` is the empty string. */
static const char *vm3_text(vm3 *v, const char *tok, size_t *out_len) {
    if (strcmp(tok, "!") == 0) { *out_len = 0; return NULL; }
    size_t n = strlen(tok);
    char *d;
    if (strcmp(tok, "~") == 0) {
        d = (char *)calloc(1, 1);
        if (!d) vm3_die("oom");
        *out_len = 0;
    } else {
        d = s3_unq(tok, n, out_len);
    }
    if (v->nstr == v->cstr) {
        v->cstr = v->cstr ? v->cstr * 2 : 32;
        v->str = (char **)realloc(v->str, v->cstr * sizeof(char *));
        if (!v->str) vm3_die("oom");
    }
    v->str[v->nstr++] = d;
    return d;
}

static size_t vm3_num(const char *tok) {
    if (strcmp(tok, "max") == 0) return (size_t)-1;
    return (size_t)strtoull(tok, NULL, 0);
}

static int vm3_reg(const char *tok) {
    if (tok[0] == '-' && tok[1] == '\0') return -1;
    if (tok[0] == '!' && tok[1] == '\0') return -1;
    if (tok[0] != 'r' || tok[1] < '0' || tok[1] > '9' || tok[2] != '\0')
        vm3_die("bad register token");
    return tok[1] - '0';
}

static yyjson_mut_val *vm3_get(vm3 *v, const char *tok) {
    if (strcmp(tok, "!") == 0) return NULL;
    int i = vm3_reg(tok);
    return (i < 0) ? NULL : v->r[i];
}

static void vm3_put(vm3 *v, const char *tok, yyjson_mut_val *val) {
    int i = vm3_reg(tok);
    if (i >= 0) v->r[i] = val;
}

static int vm3_slot(const char *tok) {
    long s = strtol(tok, NULL, 10);
    if (s < 0 || s >= VM3_NDOC) vm3_die("bad document slot");
    return (int)s;
}

/*=============================================================== field groups */

static void vm3_note_err(vm3 *v, int code, const char *msg) {
    if (code) {
        v->last_code = code;
        v->last_msg = msg;
        v->st.n_fail++;
    }
}

static void f3_ptr_err(vm3 *v, buf *b, const yyjson_ptr_err *e) {
    f_u(b, "pcode", (unsigned long long)e->code);
    f_cstr(b, "pmsg", e->msg);
    f_u(b, "ppos", (unsigned long long)e->pos);
    vm3_note_err(v, (int)e->code, e->msg);
}

static void f3_ctx(buf *b, const yyjson_ptr_ctx *c) {
    f_tree_mut(b, "ctn", c->ctn);
    f_tree_mut(b, "pre", c->pre);
    f_tree_mut(b, "old", c->old);
}

static void f3_patch_err(vm3 *v, buf *b, const yyjson_patch_err *e) {
    f_u(b, "xcode", (unsigned long long)e->code);
    f_u(b, "xidx", (unsigned long long)e->idx);
    f_cstr(b, "xmsg", e->msg);
    f_u(b, "xpcode", (unsigned long long)e->ptr.code);
    f_cstr(b, "xpmsg", e->ptr.msg);
    f_u(b, "xppos", (unsigned long long)e->ptr.pos);
    vm3_note_err(v, (int)e->code, e->msg);
}

/** Track a scratch immutable document so it outlives the values copied out
    of it and is freed exactly once, at teardown. */
static void vm3_keep(vm3 *v, yyjson_doc *d) {
    if (!d) return;
    if (v->nsdoc == v->csdoc) {
        v->csdoc = v->csdoc ? v->csdoc * 2 : 16;
        v->sdoc = (yyjson_doc **)realloc(v->sdoc,
                                         v->csdoc * sizeof(yyjson_doc *));
        if (!v->sdoc) vm3_die("oom");
    }
    v->sdoc[v->nsdoc++] = d;
}

/** The read flags every `iparse`/patch-parse in a script uses.  Fixed, so a
    script never has to carry flags and a record never depends on them. */
#define VM3_RFLAG (YYJSON_READ_ALLOW_INF_AND_NAN | \
                   YYJSON_READ_ALLOW_EXT_NUMBER)

/** Parse `s[0..n)` into a scratch immutable document on this VM's allocator.
    Padding is required by nothing here (no INSITU), but a padded copy keeps
    the reader's 4-byte loads inside allocated memory under every build. */
static yyjson_doc *vm3_parse(vm3 *v, const char *s, size_t n,
                             yyjson_read_err *err) {
    char *in = pad_copy(s ? s : "", n);
    yyjson_doc *d = yyjson_read_opts(in, n, VM3_RFLAG, v->alc, err);
    free(in);
    vm3_keep(v, d);
    return d;
}

/*==============================================================================
 * MARK: - the interpreter
 *============================================================================*/

void vm3_run(const char *mod, const char *name, const char *src, size_t len,
             const yyjson_alc *alc, int verbose, vm3_stat *out) {
    vm3 v;
    memset(&v, 0, sizeof(v));
    v.mod = mod;
    v.name = name;
    v.alc = alc;
    v.verbose = verbose;
    buf_init(&v.acc);

    buf b;
    buf_init(&b);

    size_t p = 0, idx = 0;
    char tokbuf[2048];
    char *tok[VM3_MAXTOK];

    while (p <= len) {
        size_t q = p;
        while (q < len && src[q] != '\n') q++;
        size_t n = q - p;
        if (n && src[p + n - 1] == '\r') n--;

        if (n == 0 || src[p] == '#') {
            if (q >= len) break;
            p = q + 1;
            continue;
        }
        if (n >= sizeof(tokbuf)) vm3_die("script line too long");
        memcpy(tokbuf, src + p, n);
        tokbuf[n] = '\0';

        int nt = 0;
        char *s = tokbuf;
        while (*s && nt < VM3_MAXTOK) {
            tok[nt++] = s;
            char *sp = strchr(s, ' ');
            if (!sp) break;
            *sp = '\0';
            s = sp + 1;
        }
        for (int i = nt; i < VM3_MAXTOK; i++) tok[i] = (char *)"-";

        const char *op = tok[0];
        size_t head = 0;
        if (verbose) {
            buf_adds(&b, mod);
            buf_adds(&b, " op");
            f_cstr(&b, "case", name);
            f_u(&b, "i", (unsigned long long)idx);
            f_cstr(&b, "op", op);
            head = b.len;
        }
        (void)head;

        int handled = 1;
        yyjson_mut_doc *md = v.md;

        /* ------------------------------------------------- document / slots */
        if (!strcmp(op, "new")) {
            yyjson_mut_doc_free(v.md);
            memset(v.r, 0, sizeof(v.r));
            v.md = yyjson_mut_doc_new(alc);
            f_bool(&b, "ret", v.md != NULL);
        } else if (!strcmp(op, "freedoc")) {
            yyjson_mut_doc_free(v.md);
            v.md = NULL;
            memset(v.r, 0, sizeof(v.r));
            buf_adds(&b, " ret=1");
        } else if (!strcmp(op, "iparse")) {
            int sl = vm3_slot(tok[1]);
            size_t tl = 0;
            const char *t = vm3_text(&v, tok[2], &tl);
            yyjson_read_err e;
            memset(&e, 0, sizeof(e));
            /* the slot's previous document is kept alive by `sdoc`, so an
               overwrite never dangles a value copied out of it earlier */
            yyjson_doc *d = vm3_parse(&v, t, tl, &e);
            v.id[sl] = d;
            f_u(&b, "slot", (unsigned long long)sl);
            f_bool(&b, "ret", d != NULL);
            f_u(&b, "code", (unsigned long long)e.code);
            f_cstr(&b, "msg", e.msg);
            f_u(&b, "pos", (unsigned long long)e.pos);
            vm3_note_err(&v, (int)e.code, e.msg);
            f_tree_imut(&b, "itree", d ? yyjson_doc_get_root(d) : NULL);
        } else if (!strcmp(op, "mutcopy")) {
            int sl = vm3_slot(tok[1]);
            yyjson_mut_doc *m = yyjson_doc_mut_copy(v.id[sl], alc);
            f_u(&b, "slot", (unsigned long long)sl);
            f_bool(&b, "ret", m != NULL);
            if (m) {
                yyjson_mut_doc_free(v.md);
                memset(v.r, 0, sizeof(v.r));
                v.md = m;
            }
            f_tree_mut(&b, "new", m ? yyjson_mut_doc_get_root(m) : NULL);
        } else if (!strcmp(op, "mmutcopy")) {
            yyjson_mut_doc *m = yyjson_mut_doc_mut_copy(v.md, alc);
            f_bool(&b, "ret", m != NULL);
            f_tree_mut(&b, "new", m ? yyjson_mut_doc_get_root(m) : NULL);
            /* a real round trip: the copy replaces the original, so a port
               that copies shallowly diverges on the next mutation, not here */
            if (m) {
                yyjson_mut_doc_free(v.md);
                memset(v.r, 0, sizeof(v.r));
                v.md = m;
            }
        } else if (!strcmp(op, "imutcopy")) {
            int sl = vm3_slot(tok[1]);
            yyjson_doc *d = yyjson_mut_doc_imut_copy(v.md, alc);
            vm3_keep(&v, d);
            v.id[sl] = d;
            f_u(&b, "slot", (unsigned long long)sl);
            f_bool(&b, "ret", d != NULL);
            f_tree_imut(&b, "itree", d ? yyjson_doc_get_root(d) : NULL);
        } else if (!strcmp(op, "vimutcopy")) {
            int sl = vm3_slot(tok[1]);
            yyjson_doc *d = yyjson_mut_val_imut_copy(vm3_get(&v, tok[2]), alc);
            vm3_keep(&v, d);
            v.id[sl] = d;
            f_u(&b, "slot", (unsigned long long)sl);
            f_bool(&b, "ret", d != NULL);
            f_tree_imut(&b, "itree", d ? yyjson_doc_get_root(d) : NULL);
        } else if (!strcmp(op, "valmutcopy")) {
            int sl = vm3_slot(tok[2]);
            size_t tl = 0;
            const char *t = vm3_text(&v, tok[3], &tl);
            yyjson_val *sv = v.id[sl]
                ? yyjson_doc_ptr_getn(v.id[sl], t, tl) : NULL;
            f_str(&b, "ptr", t, tl);
            f_tree_imut(&b, "src", sv);
            yyjson_mut_val *x = yyjson_val_mut_copy(v.md, sv);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "mvalcopy")) {
            yyjson_mut_val *x = yyjson_mut_val_mut_copy(v.md,
                                                        vm3_get(&v, tok[2]));
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "root")) {
            yyjson_mut_doc_set_root(v.md, vm3_get(&v, tok[1]));
            buf_adds(&b, " ret=1");
        } else if (!strcmp(op, "getroot")) {
            yyjson_mut_val *x = yyjson_mut_doc_get_root(v.md);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);

        /* ------------------------------------------------------ constructors */
        } else if (!strcmp(op, "null")) {
            yyjson_mut_val *x = yyjson_mut_null(md);
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "true")) {
            yyjson_mut_val *x = yyjson_mut_true(md);
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "false")) {
            yyjson_mut_val *x = yyjson_mut_false(md);
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "uint")) {
            yyjson_mut_val *x = yyjson_mut_uint(md,
                                   (uint64_t)strtoull(tok[2], NULL, 0));
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "sint")) {
            yyjson_mut_val *x = yyjson_mut_sint(md,
                                   (int64_t)strtoll(tok[2], NULL, 0));
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "realbits")) {
            /* reals travel as their exact IEEE-754 bit pattern, never as a
               decimal literal, so a script can never depend on strtod */
            uint64_t bits = (uint64_t)strtoull(tok[2], NULL, 16);
            double d;
            memcpy(&d, &bits, 8);
            yyjson_mut_val *x = yyjson_mut_real(md, d);
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "arr")) {
            yyjson_mut_val *x = yyjson_mut_arr(md);
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "obj")) {
            yyjson_mut_val *x = yyjson_mut_obj(md);
            vm3_put(&v, tok[1], x);
            f_bool(&b, "ret", x != NULL);
        } else if (!strcmp(op, "strcpy") || !strcmp(op, "strncpy") ||
                   !strcmp(op, "rawcpy") || !strcmp(op, "rawncpy") ||
                   !strcmp(op, "str") || !strcmp(op, "strn") ||
                   !strcmp(op, "raw") || !strcmp(op, "rawn")) {
            /* The non-copying constructors keep the value pointing into this
               VM's string arena, which is freed only after the document, as
               those entry points require.  They are the only way a public
               caller can present a string whose byte after `len` is not a
               NUL, which is exactly what the writer's short-tail branch
               looks at. */
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[2], &sl);
            size_t use = (tok[3][0] == '-' && tok[3][1] == 0) ? sl
                                                              : vm3_num(tok[3]);
            f_str(&b, "s", s2, sl);
            f_u(&b, "n", (unsigned long long)use);
            yyjson_mut_val *x;
            if (!strcmp(op, "strcpy"))       x = yyjson_mut_strcpy(md, s2);
            else if (!strcmp(op, "strncpy")) x = yyjson_mut_strncpy(md, s2,
                                                                    use);
            else if (!strcmp(op, "rawcpy"))  x = yyjson_mut_rawcpy(md, s2);
            else if (!strcmp(op, "rawncpy")) x = yyjson_mut_rawncpy(md, s2,
                                                                    use);
            else if (!strcmp(op, "str"))     x = yyjson_mut_str(md, s2);
            else if (!strcmp(op, "strn"))    x = yyjson_mut_strn(md, s2, use);
            else if (!strcmp(op, "raw"))     x = yyjson_mut_raw(md, s2);
            else                             x = yyjson_mut_rawn(md, s2, use);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);

        /* --------------------------------------------------- array mutation */
        } else if (!strcmp(op, "aapp")) {
            f_bool(&b, "ret", yyjson_mut_arr_append(vm3_get(&v, tok[1]),
                                                    vm3_get(&v, tok[2])));
        } else if (!strcmp(op, "apre")) {
            f_bool(&b, "ret", yyjson_mut_arr_prepend(vm3_get(&v, tok[1]),
                                                     vm3_get(&v, tok[2])));
        } else if (!strcmp(op, "ains")) {
            size_t i = vm3_num(tok[2]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_arr_insert(vm3_get(&v, tok[1]),
                                                    vm3_get(&v, tok[3]), i));
        } else if (!strcmp(op, "arem")) {
            size_t i = vm3_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            yyjson_mut_val *x = yyjson_mut_arr_remove(vm3_get(&v, tok[2]), i);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "aremf")) {
            yyjson_mut_val *x =
                yyjson_mut_arr_remove_first(vm3_get(&v, tok[2]));
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "areml")) {
            yyjson_mut_val *x =
                yyjson_mut_arr_remove_last(vm3_get(&v, tok[2]));
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "aget")) {
            size_t i = vm3_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            yyjson_mut_val *x = yyjson_mut_arr_get(vm3_get(&v, tok[2]), i);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "agetf")) {
            yyjson_mut_val *x = yyjson_mut_arr_get_first(vm3_get(&v, tok[2]));
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "agetl")) {
            yyjson_mut_val *x = yyjson_mut_arr_get_last(vm3_get(&v, tok[2]));
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "aremr")) {
            size_t i = vm3_num(tok[2]), c = vm3_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            f_u(&b, "cnt", (unsigned long long)c);
            f_bool(&b, "ret",
                   yyjson_mut_arr_remove_range(vm3_get(&v, tok[1]), i, c));
        } else if (!strcmp(op, "arep")) {
            size_t i = vm3_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            yyjson_mut_val *x = yyjson_mut_arr_replace(vm3_get(&v, tok[2]), i,
                                                       vm3_get(&v, tok[4]));
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "aclr")) {
            f_bool(&b, "ret", yyjson_mut_arr_clear(vm3_get(&v, tok[1])));
        } else if (!strcmp(op, "arot")) {
            size_t i = vm3_num(tok[2]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_arr_rotate(vm3_get(&v, tok[1]), i));
        } else if (!strcmp(op, "asize")) {
            f_u(&b, "ret",
                (unsigned long long)yyjson_mut_arr_size(vm3_get(&v, tok[1])));

        /* -------------------------------------------------- object mutation */
        } else if (!strcmp(op, "oadd")) {
            f_bool(&b, "ret", yyjson_mut_obj_add(vm3_get(&v, tok[1]),
                                                 vm3_get(&v, tok[2]),
                                                 vm3_get(&v, tok[3])));
        } else if (!strcmp(op, "oput")) {
            f_bool(&b, "ret", yyjson_mut_obj_put(vm3_get(&v, tok[1]),
                                                 vm3_get(&v, tok[2]),
                                                 vm3_get(&v, tok[3])));
        } else if (!strcmp(op, "oins")) {
            size_t i = vm3_num(tok[4]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_obj_insert(vm3_get(&v, tok[1]),
                                                    vm3_get(&v, tok[2]),
                                                    vm3_get(&v, tok[3]), i));
        } else if (!strcmp(op, "orep")) {
            f_bool(&b, "ret", yyjson_mut_obj_replace(vm3_get(&v, tok[1]),
                                                     vm3_get(&v, tok[2]),
                                                     vm3_get(&v, tok[3])));
        } else if (!strcmp(op, "oremk")) {
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[3], &sl);
            f_str(&b, "k", s2, sl);
            yyjson_mut_val *x =
                yyjson_mut_obj_remove_keyn(vm3_get(&v, tok[2]), s2, sl);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "oget")) {
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[3], &sl);
            f_str(&b, "k", s2, sl);
            yyjson_mut_val *x =
                yyjson_mut_obj_getn(vm3_get(&v, tok[2]), s2, sl);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (!strcmp(op, "oclr")) {
            f_bool(&b, "ret", yyjson_mut_obj_clear(vm3_get(&v, tok[1])));
        } else if (!strcmp(op, "orot")) {
            size_t i = vm3_num(tok[2]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_obj_rotate(vm3_get(&v, tok[1]), i));
        } else if (!strcmp(op, "osize")) {
            f_u(&b, "ret",
                (unsigned long long)yyjson_mut_obj_size(vm3_get(&v, tok[1])));
        } else if (!strcmp(op, "oren")) {
            size_t al = 0, bl = 0;
            const char *ka = vm3_text(&v, tok[2], &al);
            const char *kb = vm3_text(&v, tok[3], &bl);
            f_str(&b, "k", ka, al);
            f_str(&b, "k2", kb, bl);
            f_bool(&b, "ret",
                   yyjson_mut_obj_rename_keyn(v.md, vm3_get(&v, tok[1]),
                                              ka, al, kb, bl));

        /* ------------------------------------------------------ JSON pointer */
        } else if (!strcmp(op, "pget")) {
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[2], &sl);
            f_str(&b, "ptr", s2, sl);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            yyjson_mut_val *x = yyjson_mut_doc_ptr_getx(v.md, s2, sl, &v.ctx,
                                                        &v.perr);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
            f3_ptr_err(&v, &b, &v.perr);
            f3_ctx(&b, &v.ctx);
        } else if (!strcmp(op, "ipget")) {
            int slo = vm3_slot(tok[1]);
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[2], &sl);
            yyjson_ptr_err e;
            memset(&e, 0, sizeof(e));
            f_u(&b, "slot", (unsigned long long)slo);
            f_str(&b, "ptr", s2, sl);
            yyjson_val *x = yyjson_doc_ptr_getx(v.id[slo], s2, sl, &e);
            f_tree_imut(&b, "ret", x);
            f3_ptr_err(&v, &b, &e);
        } else if (!strcmp(op, "padd") || !strcmp(op, "pset")) {
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[1], &sl);
            int cp = (int)vm3_num(tok[3]);
            f_str(&b, "ptr", s2, sl);
            f_bool(&b, "cp", cp);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            bool ok = (!strcmp(op, "padd"))
                ? yyjson_mut_doc_ptr_addx(v.md, s2, sl, vm3_get(&v, tok[2]),
                                          cp != 0, &v.ctx, &v.perr)
                : yyjson_mut_doc_ptr_setx(v.md, s2, sl, vm3_get(&v, tok[2]),
                                          cp != 0, &v.ctx, &v.perr);
            f_bool(&b, "ret", ok);
            f3_ptr_err(&v, &b, &v.perr);
            f3_ctx(&b, &v.ctx);
        } else if (!strcmp(op, "prep")) {
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[2], &sl);
            f_str(&b, "ptr", s2, sl);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            yyjson_mut_val *x = yyjson_mut_doc_ptr_replacex(
                v.md, s2, sl, vm3_get(&v, tok[3]), &v.ctx, &v.perr);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
            f3_ptr_err(&v, &b, &v.perr);
            f3_ctx(&b, &v.ctx);
        } else if (!strcmp(op, "pdel")) {
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[2], &sl);
            f_str(&b, "ptr", s2, sl);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            yyjson_mut_val *x = yyjson_mut_doc_ptr_removex(v.md, s2, sl,
                                                           &v.ctx, &v.perr);
            vm3_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
            f3_ptr_err(&v, &b, &v.perr);
            f3_ctx(&b, &v.ctx);
        } else if (!strcmp(op, "pctx")) {
            f3_ctx(&b, &v.ctx);
        } else if (!strcmp(op, "cappend")) {
            f_bool(&b, "ret", yyjson_ptr_ctx_append(&v.ctx,
                                                    vm3_get(&v, tok[1]),
                                                    vm3_get(&v, tok[2])));
            f3_ctx(&b, &v.ctx);
        } else if (!strcmp(op, "creplace")) {
            f_bool(&b, "ret", yyjson_ptr_ctx_replace(&v.ctx,
                                                     vm3_get(&v, tok[1])));
            f3_ctx(&b, &v.ctx);
        } else if (!strcmp(op, "cremove")) {
            f_bool(&b, "ret", yyjson_ptr_ctx_remove(&v.ctx));
            f3_ctx(&b, &v.ctx);

        /* ---------------------------------------------- RFC 6902 / RFC 7386 */
        } else if (!strcmp(op, "patch") || !strcmp(op, "mpatch")) {
            /* immutable original (slot) + immutable patch -> fresh mutable
               output document.  On success the output replaces the working
               document, so a later op sees exactly the state C left; on
               failure the working document must be untouched, which the
               `root=` field on this very line proves. */
            int slo = vm3_slot(tok[1]);
            size_t tl = 0;
            const char *t = vm3_text(&v, tok[2], &tl);
            yyjson_doc *pd = vm3_parse(&v, t, tl, NULL);
            yyjson_mut_doc *o = yyjson_mut_doc_new(alc);
            yyjson_patch_err e;
            memset(&e, 0, sizeof(e));
            f_u(&b, "slot", (unsigned long long)slo);
            f_bool(&b, "pok", pd != NULL);
            f_bool(&b, "onew", o != NULL);
            yyjson_mut_val *r = NULL;
            if (!strcmp(op, "patch")) {
                r = yyjson_patch(o, v.id[slo] ?
                                 yyjson_doc_get_root(v.id[slo]) : NULL,
                                 pd ? yyjson_doc_get_root(pd) : NULL, &e);
            } else {
                r = yyjson_merge_patch(o, v.id[slo] ?
                                       yyjson_doc_get_root(v.id[slo]) : NULL,
                                       pd ? yyjson_doc_get_root(pd) : NULL);
            }
            yyjson_mut_doc_set_root(o, r);
            f_bool(&b, "ret", r != NULL);
            f3_patch_err(&v, &b, &e);
            f_tree_mut(&b, "out", r);
            if (r) {
                yyjson_mut_doc_free(v.md);
                memset(v.r, 0, sizeof(v.r));
                v.md = o;
            } else {
                yyjson_mut_doc_free(o);
            }
        } else if (!strcmp(op, "mutpatch") || !strcmp(op, "mutmpatch")) {
            /* the all-mutable entry points: the patch is parsed and copied
               into the output document, so every allocation the patch itself
               needs also goes through this VM's allocator */
            size_t tl = 0;
            const char *t = vm3_text(&v, tok[1], &tl);
            yyjson_doc *pd = vm3_parse(&v, t, tl, NULL);
            yyjson_mut_doc *o = yyjson_mut_doc_new(alc);
            yyjson_mut_val *pv = (o && pd)
                ? yyjson_val_mut_copy(o, yyjson_doc_get_root(pd)) : NULL;
            yyjson_patch_err e;
            memset(&e, 0, sizeof(e));
            f_bool(&b, "pok", pd != NULL);
            f_bool(&b, "onew", o != NULL);
            f_bool(&b, "pcopy", pv != NULL);
            yyjson_mut_val *r;
            if (!strcmp(op, "mutpatch")) {
                r = yyjson_mut_patch(o, yyjson_mut_doc_get_root(v.md), pv, &e);
            } else {
                r = yyjson_mut_merge_patch(o, yyjson_mut_doc_get_root(v.md),
                                           pv);
            }
            yyjson_mut_doc_set_root(o, r);
            f_bool(&b, "ret", r != NULL);
            f3_patch_err(&v, &b, &e);
            f_tree_mut(&b, "out", r);
            if (r) {
                yyjson_mut_doc_free(v.md);
                memset(v.r, 0, sizeof(v.r));
                v.md = o;
            } else {
                yyjson_mut_doc_free(o);
            }

        /* ------------------------------------------------------- comparison */
        } else if (!strcmp(op, "eq")) {
            f_bool(&b, "ret", yyjson_mut_equals(vm3_get(&v, tok[1]),
                                                vm3_get(&v, tok[2])));
        } else if (!strcmp(op, "eqroot")) {
            f_bool(&b, "ret",
                   yyjson_mut_equals(yyjson_mut_doc_get_root(v.md),
                                     vm3_get(&v, tok[1])));
        } else if (!strcmp(op, "ieq")) {
            int a = vm3_slot(tok[1]), c = vm3_slot(tok[2]);
            f_bool(&b, "ret",
                   yyjson_equals(v.id[a] ? yyjson_doc_get_root(v.id[a]) : NULL,
                                 v.id[c] ? yyjson_doc_get_root(v.id[c])
                                         : NULL));
        } else if (!strcmp(op, "eqstr")) {
            size_t sl = 0;
            const char *s2 = vm3_text(&v, tok[2], &sl);
            f_str(&b, "s", s2, sl);
            f_bool(&b, "ret",
                   yyjson_mut_equals_strn(vm3_get(&v, tok[1]), s2, sl));
        } else if (!strcmp(op, "dump")) {
            f_tree_mut(&b, "ret", vm3_get(&v, tok[1]));

        /* ----------------------------------------------------------- output */
        } else if (!strcmp(op, "write")) {
            size_t ol = 0;
            yyjson_write_err we;
            memset(&we, 0, sizeof(we));
            char *o = yyjson_mut_write_opts(v.md,
                          YYJSON_WRITE_ALLOW_INF_AND_NAN, alc, &ol, &we);
            f_u(&b, "code", (unsigned long long)we.code);
            f_cstr(&b, "msg", we.msg);
            vm3_note_err(&v, (int)we.code, we.msg);
            f_bytes(&b, "ret", o, o ? ol : 0);
            if (o) { if (alc) alc->free(alc->ctx, o); else free(o); }
        } else if (!strcmp(op, "iwrite")) {
            int slo = vm3_slot(tok[1]);
            size_t ol = 0;
            yyjson_write_err we;
            memset(&we, 0, sizeof(we));
            char *o = yyjson_write_opts(v.id[slo],
                          YYJSON_WRITE_ALLOW_INF_AND_NAN, alc, &ol, &we);
            f_u(&b, "slot", (unsigned long long)slo);
            f_u(&b, "code", (unsigned long long)we.code);
            f_cstr(&b, "msg", we.msg);
            vm3_note_err(&v, (int)we.code, we.msg);
            f_bytes(&b, "ret", o, o ? ol : 0);
            if (o) { if (alc) alc->free(alc->ctx, o); else free(o); }
        } else {
            handled = 0;
        }

        if (!handled) {
            fprintf(stderr, "harness3: vm3: unknown op '%s' in case %s\n", op,
                    name);
            exit(2);
        }

        f_tree_mut(&b, "root",
                   v.md ? yyjson_mut_doc_get_root(v.md) : NULL);

        if (verbose) {
            rec_line(&b);
            rec_cases++;
        } else {
            /* Fold the op into the accumulator as `<i>.<op>.<fnv>`: the index
               and the name localise a divergence to one op, and the hash
               covers every field the verbose form would have printed.  X-04
               replays the same script verbosely, so the hash is never the
               only evidence available. */
            if (v.acc_n) buf_addc(&v.acc, '+');
            buf_addf(&v.acc, "%zu.%s.%016llX", idx, op,
                     (unsigned long long)fnv1a(b.dat ? b.dat : "", b.len));
            v.acc_n++;
            buf_clear(&b);
        }
        idx++;

        if (q >= len) break;
        p = q + 1;
    }

    v.st.n_ops = (unsigned long long)idx;

    /* ------------------------------------------------------- final state */
    buf_adds(&b, mod);
    buf_adds(&b, " end");
    f_cstr(&b, "case", name);
    f_u(&b, "ops", (unsigned long long)idx);
    f_u(&b, "nfail", v.st.n_fail);
    f_i(&b, "ecode", (long long)v.last_code);
    f_cstr(&b, "emsg", v.last_msg);
    f_bool(&b, "hasdoc", v.md != NULL);
    f_tree_mut(&b, "root", v.md ? yyjson_mut_doc_get_root(v.md) : NULL);
    for (int i = 0; i < VM3_NDOC; i++) {
        char nm[16];
        snprintf(nm, sizeof(nm), "i%d", i);
        f_tree_imut(&b, nm, v.id[i] ? yyjson_doc_get_root(v.id[i]) : NULL);
    }
    {
        size_t ol = 0;
        char *o = v.md ? yyjson_mut_write_opts(v.md,
                             YYJSON_WRITE_ALLOW_INF_AND_NAN, alc, &ol, NULL)
                       : NULL;
        f_bytes(&b, "json", o, o ? ol : 0);
        if (o) { if (alc) alc->free(alc->ctx, o); else free(o); }
    }
    if (!verbose) {
        f_u(&b, "opsn", (unsigned long long)v.acc_n);
        f_bytes(&b, "opseq", v.acc.dat ? v.acc.dat : "", v.acc.len);
    }
    rec_line(&b);
    rec_cases++;

    /* teardown: the mutable document first, then the scratch immutable
       documents its copied values may still point at, then the string arena
       the non-copying constructors reference */
    yyjson_mut_doc_free(v.md);
    for (size_t i = 0; i < v.nsdoc; i++) yyjson_doc_free(v.sdoc[i]);
    free(v.sdoc);
    for (size_t i = 0; i < v.nstr; i++) free(v.str[i]);
    free(v.str);
    buf_free(&v.acc);
    buf_free(&b);

    if (out) *out = v.st;
}
