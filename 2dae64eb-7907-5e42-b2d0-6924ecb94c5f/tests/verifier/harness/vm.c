#include "common.h"

/*==============================================================================
 * MARK: - mutation script VM  (shared by B-05 and B-06)
 *
 * A mutation sequence is far more legible as a script than as a wall of C, and
 * a script is also what lets one case cover `append; insert; rotate; remove`
 * *in that order* with the document state recorded after every single step.
 * Localising a port bug then costs one `diff` and one glance: the first record
 * line that differs names the op that broke.
 *
 * Grammar: one op per line, tokens separated by single spaces.  A line that is
 * empty or starts with '#' is ignored and does not consume an op index.
 *
 *   TOKEN kinds
 *     rN        register 0..9 holding a `yyjson_mut_val *`
 *     -         "discard the result" in a destination position
 *     !         a NULL `yyjson_mut_val *` / NULL `const char *`
 *     ~         the empty string
 *     TEXT      a byte string; %XX hex escapes decode to arbitrary bytes, so a
 *               token can carry a space, a NUL or invalid UTF-8
 *     N         an unsigned integer (strtoull, base 0)
 *     I         an index; `max` is SIZE_MAX, which is the out-of-range probe
 *
 * Every executed op emits exactly one record line:
 *
 *   <mod> vm case=<name> i=<idx> op=<op> [args...] [ret=...] root=<tree>
 *
 * `root` is the canonical tree of the document root after the op, so a
 * divergence shows up on the op that caused it rather than at the end.
 *
 * Strings handed to the NON-copying constructors (`yyjson_mut_str`,
 * `yyjson_mut_raw`, ...) are interned in a VM-owned arena that is freed only
 * after the document, which is exactly the lifetime those entry points require.
 *============================================================================*/

#define VM_NREG 10
#define VM_MAXTOK 8

typedef struct {
    yyjson_mut_doc *doc;
    yyjson_mut_val *r[VM_NREG];
    yyjson_ptr_ctx  ctx;
    yyjson_ptr_err  perr;
    /* interned byte strings; outlive `doc` */
    char  **str;
    size_t  nstr, cstr;
    /* temporary immutable docs created by `parse`, freed at teardown */
    yyjson_doc **tdoc;
    size_t       ntdoc, ctdoc;
    const char  *mod;
    const char  *name;
} vm;

static void vm_die(const char *m) {
    fprintf(stderr, "harness: vm: %s\n", m);
    exit(2);
}

/*------------------------------------------------------------ token decoding */

static int hexv(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

/** Decode %XX escapes in `tok` into a freshly interned buffer; returns the
    interned pointer and writes its length.  `~` is the empty string and `!`
    is a NULL pointer. */
static const char *vm_text(vm *v, const char *tok, size_t *out_len) {
    if (strcmp(tok, "!") == 0) { *out_len = 0; return NULL; }
    size_t n = strlen(tok);
    char *d = (char *)malloc(n + 1);
    if (!d) vm_die("oom");
    size_t w = 0;
    if (strcmp(tok, "~") != 0) {
        for (size_t i = 0; i < n;) {
            if (tok[i] == '%' && i + 2 < n && hexv(tok[i + 1]) >= 0 &&
                hexv(tok[i + 2]) >= 0) {
                d[w++] = (char)((hexv(tok[i + 1]) << 4) | hexv(tok[i + 2]));
                i += 3;
            } else {
                d[w++] = tok[i++];
            }
        }
    }
    d[w] = '\0';
    if (v->nstr == v->cstr) {
        v->cstr = v->cstr ? v->cstr * 2 : 32;
        v->str = (char **)realloc(v->str, v->cstr * sizeof(char *));
        if (!v->str) vm_die("oom");
    }
    v->str[v->nstr++] = d;
    *out_len = w;
    return d;
}

static size_t vm_num(const char *tok) {
    if (strcmp(tok, "max") == 0) return (size_t)-1;
    return (size_t)strtoull(tok, NULL, 0);
}

static int vm_reg(const char *tok) {
    if (tok[0] == '-' && tok[1] == '\0') return -1;
    if (tok[0] != 'r' || tok[1] < '0' || tok[1] > '9' || tok[2] != '\0')
        vm_die("bad register token");
    return tok[1] - '0';
}

static yyjson_mut_val *vm_get(vm *v, const char *tok) {
    if (strcmp(tok, "!") == 0) return NULL;
    int i = vm_reg(tok);
    return (i < 0) ? NULL : v->r[i];
}

static void vm_put(vm *v, const char *tok, yyjson_mut_val *val) {
    int i = vm_reg(tok);
    if (i >= 0) v->r[i] = val;
}

/*----------------------------------------------------------- field shorthands */

static void f_ptr_err(buf *b, const yyjson_ptr_err *e) {
    f_u(b, "pcode", (unsigned long long)e->code);
    f_cstr(b, "pmsg", e->msg);
    f_u(b, "ppos", (unsigned long long)e->pos);
}

static void f_ctx(buf *b, const yyjson_ptr_ctx *c) {
    f_tree_mut(b, "ctn", c->ctn);
    f_tree_mut(b, "pre", c->pre);
    f_tree_mut(b, "old", c->old);
}

/*==============================================================================
 * MARK: - the interpreter
 *============================================================================*/

void vm_run(const char *mod, const char *name, char *src, size_t len) {
    vm v;
    memset(&v, 0, sizeof(v));
    v.mod = mod;
    v.name = name;

    buf b;
    buf_init(&b);

    size_t p = 0, idx = 0;
    char tokbuf[1024];
    char *tok[VM_MAXTOK];

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
        if (n >= sizeof(tokbuf)) vm_die("script line too long");
        memcpy(tokbuf, src + p, n);
        tokbuf[n] = '\0';

        /* split on single spaces; an empty field is impossible because the
           grammar uses `~` for the empty string */
        int nt = 0;
        char *s = tokbuf;
        while (*s && nt < VM_MAXTOK) {
            tok[nt++] = s;
            char *sp = strchr(s, ' ');
            if (!sp) break;
            *sp = '\0';
            s = sp + 1;
        }
        for (int i = nt; i < VM_MAXTOK; i++) tok[i] = (char *)"-";

        const char *op = tok[0];
        buf_adds(&b, mod);
        buf_adds(&b, " vm");
        f_cstr(&b, "case", name);
        f_u(&b, "i", (unsigned long long)idx);
        f_cstr(&b, "op", op);

        int handled = 1;
        yyjson_mut_doc *doc = v.doc;

        /* ---- document ---- */
        if (strcmp(op, "new") == 0) {
            yyjson_mut_doc_free(v.doc);
            v.doc = yyjson_mut_doc_new(NULL);
            memset(v.r, 0, sizeof(v.r));
            f_bool(&b, "ret", v.doc != NULL);
        } else if (strcmp(op, "freedoc") == 0) {
            yyjson_mut_doc_free(v.doc);
            v.doc = NULL;
            memset(v.r, 0, sizeof(v.r));
            buf_adds(&b, " ret=1");
        } else if (strcmp(op, "strpool") == 0) {
            size_t sz = vm_num(tok[1]);
            f_u(&b, "n", (unsigned long long)sz);
            f_bool(&b, "ret", yyjson_mut_doc_set_str_pool_size(doc, sz));
        } else if (strcmp(op, "valpool") == 0) {
            size_t sz = vm_num(tok[1]);
            f_u(&b, "n", (unsigned long long)sz);
            f_bool(&b, "ret", yyjson_mut_doc_set_val_pool_size(doc, sz));
        } else if (strcmp(op, "root") == 0) {
            yyjson_mut_doc_set_root(doc, vm_get(&v, tok[1]));
            buf_adds(&b, " ret=1");
        } else if (strcmp(op, "getroot") == 0) {
            yyjson_mut_val *x = yyjson_mut_doc_get_root(doc);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);

        /* ---- constructors ---- */
        } else if (strcmp(op, "null") == 0) {
            vm_put(&v, tok[1], yyjson_mut_null(doc));
        } else if (strcmp(op, "true") == 0) {
            vm_put(&v, tok[1], yyjson_mut_true(doc));
        } else if (strcmp(op, "false") == 0) {
            vm_put(&v, tok[1], yyjson_mut_false(doc));
        } else if (strcmp(op, "bool") == 0) {
            vm_put(&v, tok[1], yyjson_mut_bool(doc, vm_num(tok[2]) != 0));
        } else if (strcmp(op, "uint") == 0) {
            vm_put(&v, tok[1], yyjson_mut_uint(doc,
                   (uint64_t)strtoull(tok[2], NULL, 0)));
        } else if (strcmp(op, "sint") == 0) {
            vm_put(&v, tok[1], yyjson_mut_sint(doc,
                   (int64_t)strtoll(tok[2], NULL, 0)));
        } else if (strcmp(op, "int") == 0) {
            vm_put(&v, tok[1], yyjson_mut_int(doc,
                   (int64_t)strtoll(tok[2], NULL, 0)));
        } else if (strcmp(op, "realbits") == 0) {
            /* reals travel as their exact IEEE-754 bit pattern, never as a
               decimal literal, so the script can never depend on strtod */
            uint64_t bits = (uint64_t)strtoull(tok[2], NULL, 16);
            double d;
            memcpy(&d, &bits, 8);
            vm_put(&v, tok[1], yyjson_mut_real(doc, d));
        } else if (strcmp(op, "arr") == 0) {
            vm_put(&v, tok[1], yyjson_mut_arr(doc));
        } else if (strcmp(op, "obj") == 0) {
            vm_put(&v, tok[1], yyjson_mut_obj(doc));
        } else if (strcmp(op, "str") == 0 || strcmp(op, "strcpy") == 0 ||
                   strcmp(op, "strn") == 0 || strcmp(op, "strncpy") == 0 ||
                   strcmp(op, "raw") == 0 || strcmp(op, "rawcpy") == 0 ||
                   strcmp(op, "rawn") == 0 || strcmp(op, "rawncpy") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[2], &sl);
            size_t use = (tok[3][0] == '-') ? sl : vm_num(tok[3]);
            f_str(&b, "s", s2, sl);
            f_u(&b, "n", (unsigned long long)use);
            yyjson_mut_val *x = NULL;
            if      (!strcmp(op, "str"))     x = yyjson_mut_str(doc, s2);
            else if (!strcmp(op, "strcpy"))  x = yyjson_mut_strcpy(doc, s2);
            else if (!strcmp(op, "strn"))    x = yyjson_mut_strn(doc, s2, use);
            else if (!strcmp(op, "strncpy")) x = yyjson_mut_strncpy(doc, s2,
                                                                    use);
            else if (!strcmp(op, "raw"))     x = yyjson_mut_raw(doc, s2);
            else if (!strcmp(op, "rawcpy"))  x = yyjson_mut_rawcpy(doc, s2);
            else if (!strcmp(op, "rawn"))    x = yyjson_mut_rawn(doc, s2, use);
            else                             x = yyjson_mut_rawncpy(doc, s2,
                                                                    use);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "parse") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[2], &sl);
            f_str(&b, "s", s2, sl);
            char *in = pad_copy(s2 ? s2 : "", sl);
            yyjson_doc *td = yyjson_read_opts(in, sl,
                YYJSON_READ_ALLOW_INF_AND_NAN | YYJSON_READ_ALLOW_EXT_NUMBER,
                NULL, NULL);
            free(in);
            yyjson_mut_val *x = td
                ? yyjson_val_mut_copy(doc, yyjson_doc_get_root(td)) : NULL;
            if (td) {
                if (v.ntdoc == v.ctdoc) {
                    v.ctdoc = v.ctdoc ? v.ctdoc * 2 : 16;
                    v.tdoc = (yyjson_doc **)realloc(v.tdoc,
                             v.ctdoc * sizeof(yyjson_doc *));
                    if (!v.tdoc) vm_die("oom");
                }
                v.tdoc[v.ntdoc++] = td;
            }
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);

        /* ---- array mutation ---- */
        } else if (strcmp(op, "aapp") == 0) {
            f_bool(&b, "ret", yyjson_mut_arr_append(vm_get(&v, tok[1]),
                                                    vm_get(&v, tok[2])));
        } else if (strcmp(op, "apre") == 0) {
            f_bool(&b, "ret", yyjson_mut_arr_prepend(vm_get(&v, tok[1]),
                                                     vm_get(&v, tok[2])));
        } else if (strcmp(op, "ains") == 0) {
            size_t i = vm_num(tok[2]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_arr_insert(vm_get(&v, tok[1]),
                                                    vm_get(&v, tok[3]), i));
        } else if (strcmp(op, "arem") == 0) {
            size_t i = vm_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            yyjson_mut_val *x = yyjson_mut_arr_remove(vm_get(&v, tok[2]), i);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "aremf") == 0) {
            yyjson_mut_val *x = yyjson_mut_arr_remove_first(vm_get(&v, tok[2]));
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "areml") == 0) {
            yyjson_mut_val *x = yyjson_mut_arr_remove_last(vm_get(&v, tok[2]));
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "aremr") == 0) {
            size_t i = vm_num(tok[2]), c = vm_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            f_u(&b, "cnt", (unsigned long long)c);
            f_bool(&b, "ret",
                   yyjson_mut_arr_remove_range(vm_get(&v, tok[1]), i, c));
        } else if (strcmp(op, "aclr") == 0) {
            f_bool(&b, "ret", yyjson_mut_arr_clear(vm_get(&v, tok[1])));
        } else if (strcmp(op, "arot") == 0) {
            size_t i = vm_num(tok[2]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_arr_rotate(vm_get(&v, tok[1]), i));
        } else if (strcmp(op, "arep") == 0) {
            size_t i = vm_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            yyjson_mut_val *x = yyjson_mut_arr_replace(vm_get(&v, tok[2]), i,
                                                       vm_get(&v, tok[4]));
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "aget") == 0) {
            size_t i = vm_num(tok[3]);
            f_u(&b, "idx", (unsigned long long)i);
            yyjson_mut_val *x = yyjson_mut_arr_get(vm_get(&v, tok[2]), i);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "agetf") == 0) {
            yyjson_mut_val *x = yyjson_mut_arr_get_first(vm_get(&v, tok[2]));
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "agetl") == 0) {
            yyjson_mut_val *x = yyjson_mut_arr_get_last(vm_get(&v, tok[2]));
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "asize") == 0) {
            f_u(&b, "ret",
                (unsigned long long)yyjson_mut_arr_size(vm_get(&v, tok[1])));

        /* ---- object mutation ---- */
        } else if (strcmp(op, "oadd") == 0) {
            f_bool(&b, "ret", yyjson_mut_obj_add(vm_get(&v, tok[1]),
                                                 vm_get(&v, tok[2]),
                                                 vm_get(&v, tok[3])));
        } else if (strcmp(op, "oput") == 0) {
            f_bool(&b, "ret", yyjson_mut_obj_put(vm_get(&v, tok[1]),
                                                 vm_get(&v, tok[2]),
                                                 vm_get(&v, tok[3])));
        } else if (strcmp(op, "oins") == 0) {
            size_t i = vm_num(tok[4]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_obj_insert(vm_get(&v, tok[1]),
                                                    vm_get(&v, tok[2]),
                                                    vm_get(&v, tok[3]), i));
        } else if (strcmp(op, "orep") == 0) {
            f_bool(&b, "ret", yyjson_mut_obj_replace(vm_get(&v, tok[1]),
                                                     vm_get(&v, tok[2]),
                                                     vm_get(&v, tok[3])));
        } else if (strcmp(op, "orem") == 0) {
            yyjson_mut_val *x = yyjson_mut_obj_remove(vm_get(&v, tok[2]),
                                                      vm_get(&v, tok[3]));
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "oremk") == 0 || strcmp(op, "oremkn") == 0 ||
                   strcmp(op, "orems") == 0 || strcmp(op, "oremsn") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[3], &sl);
            size_t use = (tok[4][0] == '-') ? sl : vm_num(tok[4]);
            f_str(&b, "k", s2, sl);
            f_u(&b, "n", (unsigned long long)use);
            yyjson_mut_val *o = vm_get(&v, tok[2]);
            yyjson_mut_val *x = NULL;
            if      (!strcmp(op, "oremk"))  x = yyjson_mut_obj_remove_key(o,
                                                                          s2);
            else if (!strcmp(op, "oremkn")) x = yyjson_mut_obj_remove_keyn(o,
                                                                    s2, use);
            else if (!strcmp(op, "orems"))  x = yyjson_mut_obj_remove_str(o,
                                                                          s2);
            else                            x = yyjson_mut_obj_remove_strn(o,
                                                                    s2, use);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "oclr") == 0) {
            f_bool(&b, "ret", yyjson_mut_obj_clear(vm_get(&v, tok[1])));
        } else if (strcmp(op, "orot") == 0) {
            size_t i = vm_num(tok[2]);
            f_u(&b, "idx", (unsigned long long)i);
            f_bool(&b, "ret", yyjson_mut_obj_rotate(vm_get(&v, tok[1]), i));
        } else if (strcmp(op, "oget") == 0 || strcmp(op, "ogetn") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[3], &sl);
            size_t use = (tok[4][0] == '-') ? sl : vm_num(tok[4]);
            f_str(&b, "k", s2, sl);
            f_u(&b, "n", (unsigned long long)use);
            yyjson_mut_val *x = (!strcmp(op, "oget"))
                ? yyjson_mut_obj_get(vm_get(&v, tok[2]), s2)
                : yyjson_mut_obj_getn(vm_get(&v, tok[2]), s2, use);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
        } else if (strcmp(op, "oren") == 0 || strcmp(op, "orenn") == 0) {
            size_t al = 0, bl = 0;
            const char *ka = vm_text(&v, tok[2], &al);
            const char *kb = vm_text(&v, tok[3], &bl);
            f_str(&b, "k", ka, al);
            f_str(&b, "k2", kb, bl);
            yyjson_mut_val *o = yyjson_mut_doc_get_root(doc);
            if (tok[4][0] == 'r') o = vm_get(&v, tok[4]);
            f_bool(&b, "ret", (!strcmp(op, "oren"))
                   ? yyjson_mut_obj_rename_key(doc, o, ka, kb)
                   : yyjson_mut_obj_rename_keyn(doc, o, ka, al, kb, bl));
        } else if (strcmp(op, "osize") == 0) {
            f_u(&b, "ret",
                (unsigned long long)yyjson_mut_obj_size(vm_get(&v, tok[1])));

        /* ---- iteration ---- */
        } else if (strcmp(op, "aiter") == 0) {
            /* full iterator contract: init, has_next, next, remove-at-k */
            size_t rm = vm_num(tok[2]);
            f_u(&b, "rm", (unsigned long long)rm);
            yyjson_mut_arr_iter it;
            f_bool(&b, "init",
                   yyjson_mut_arr_iter_init(vm_get(&v, tok[1]), &it));
            size_t k = 0;
            yyjson_mut_val *e;
            buf_adds(&b, " seq=");
            while ((e = yyjson_mut_arr_iter_next(&it)) != NULL) {
                if (k) buf_addc(&b, '+');
                buf_addf(&b, "%zu.%d.", k,
                         yyjson_mut_arr_iter_has_next(&it) ? 1 : 0);
                tree_mut(&b, e);
                if (k == rm) {
                    yyjson_mut_val *d2 = yyjson_mut_arr_iter_remove(&it);
                    buf_adds(&b, ".RM.");
                    tree_mut(&b, d2);
                }
                k++;
                if (k > 4096) break;
            }
            if (!k) buf_adds(&b, "!empty");
        } else if (strcmp(op, "oiter") == 0) {
            size_t rm = vm_num(tok[2]);
            f_u(&b, "rm", (unsigned long long)rm);
            yyjson_mut_obj_iter it;
            f_bool(&b, "init",
                   yyjson_mut_obj_iter_init(vm_get(&v, tok[1]), &it));
            size_t k = 0;
            yyjson_mut_val *kv;
            buf_adds(&b, " seq=");
            while ((kv = yyjson_mut_obj_iter_next(&it)) != NULL) {
                if (k) buf_addc(&b, '+');
                buf_addf(&b, "%zu.%d.", k,
                         yyjson_mut_obj_iter_has_next(&it) ? 1 : 0);
                tree_mut(&b, kv);
                buf_addc(&b, '^');
                tree_mut(&b, yyjson_mut_obj_iter_get_val(kv));
                if (k == rm) {
                    yyjson_mut_val *d2 = yyjson_mut_obj_iter_remove(&it);
                    buf_adds(&b, ".RM.");
                    tree_mut(&b, d2);
                }
                k++;
                if (k > 4096) break;
            }
            if (!k) buf_adds(&b, "!empty");

        /* ---- comparison / inspection ---- */
        } else if (strcmp(op, "eq") == 0) {
            f_bool(&b, "ret", yyjson_mut_equals(vm_get(&v, tok[1]),
                                                vm_get(&v, tok[2])));
        } else if (strcmp(op, "eqstr") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[2], &sl);
            f_str(&b, "s", s2, sl);
            f_bool(&b, "ret", yyjson_mut_equals_str(vm_get(&v, tok[1]), s2));
            f_bool(&b, "retn",
                   yyjson_mut_equals_strn(vm_get(&v, tok[1]), s2, sl));
        } else if (strcmp(op, "dump") == 0) {
            f_tree_mut(&b, "ret", vm_get(&v, tok[1]));
        } else if (strcmp(op, "write") == 0) {
            size_t ol = 0;
            yyjson_write_err we;
            memset(&we, 0, sizeof(we));
            char *o = yyjson_mut_write_opts(doc, YYJSON_WRITE_ALLOW_INF_AND_NAN,
                                            NULL, &ol, &we);
            f_u(&b, "code", (unsigned long long)we.code);
            f_cstr(&b, "msg", we.msg);
            f_bytes(&b, "ret", o, o ? ol : 0);
            free(o);

        /* ---- JSON Pointer ---- */
        } else if (strcmp(op, "pget") == 0 || strcmp(op, "pgetv") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[2], &sl);
            f_str(&b, "ptr", s2, sl);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            yyjson_mut_val *x;
            if (!strcmp(op, "pget")) {
                x = yyjson_mut_doc_ptr_getx(doc, s2, sl, &v.ctx, &v.perr);
            } else {
                x = yyjson_mut_ptr_getx(vm_get(&v, tok[3]), s2, sl, &v.ctx,
                                        &v.perr);
            }
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
            f_ptr_err(&b, &v.perr);
            f_ctx(&b, &v.ctx);
        } else if (strcmp(op, "padd") == 0 || strcmp(op, "pset") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[1], &sl);
            int cp = (int)vm_num(tok[3]);
            f_str(&b, "ptr", s2, sl);
            f_bool(&b, "cp", cp);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            bool ok = (!strcmp(op, "padd"))
                ? yyjson_mut_doc_ptr_addx(doc, s2, sl, vm_get(&v, tok[2]),
                                          cp != 0, &v.ctx, &v.perr)
                : yyjson_mut_doc_ptr_setx(doc, s2, sl, vm_get(&v, tok[2]),
                                          cp != 0, &v.ctx, &v.perr);
            f_bool(&b, "ret", ok);
            f_ptr_err(&b, &v.perr);
            f_ctx(&b, &v.ctx);
        } else if (strcmp(op, "prep") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[2], &sl);
            f_str(&b, "ptr", s2, sl);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            yyjson_mut_val *x = yyjson_mut_doc_ptr_replacex(
                doc, s2, sl, vm_get(&v, tok[3]), &v.ctx, &v.perr);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
            f_ptr_err(&b, &v.perr);
            f_ctx(&b, &v.ctx);
        } else if (strcmp(op, "pdel") == 0) {
            size_t sl = 0;
            const char *s2 = vm_text(&v, tok[2], &sl);
            f_str(&b, "ptr", s2, sl);
            memset(&v.ctx, 0, sizeof(v.ctx));
            memset(&v.perr, 0, sizeof(v.perr));
            yyjson_mut_val *x = yyjson_mut_doc_ptr_removex(doc, s2, sl, &v.ctx,
                                                           &v.perr);
            vm_put(&v, tok[1], x);
            f_tree_mut(&b, "ret", x);
            f_ptr_err(&b, &v.perr);
            f_ctx(&b, &v.ctx);
        } else if (strcmp(op, "cappend") == 0) {
            f_bool(&b, "ret", yyjson_ptr_ctx_append(&v.ctx,
                                                    vm_get(&v, tok[1]),
                                                    vm_get(&v, tok[2])));
            f_ctx(&b, &v.ctx);
        } else if (strcmp(op, "creplace") == 0) {
            f_bool(&b, "ret", yyjson_ptr_ctx_replace(&v.ctx,
                                                     vm_get(&v, tok[1])));
            f_ctx(&b, &v.ctx);
        } else if (strcmp(op, "cremove") == 0) {
            f_bool(&b, "ret", yyjson_ptr_ctx_remove(&v.ctx));
            f_ctx(&b, &v.ctx);
        } else if (strcmp(op, "pctx") == 0) {
            f_ctx(&b, &v.ctx);
        } else {
            handled = 0;
        }

        if (!handled) {
            fprintf(stderr, "harness: vm: unknown op '%s' in case %s\n", op,
                    name);
            exit(2);
        }

        f_tree_mut(&b, "root",
                   v.doc ? yyjson_mut_doc_get_root(v.doc) : NULL);
        rec_line(&b);
        rec_cases++;
        idx++;

        if (q >= len) break;
        p = q + 1;
    }

    /* final state, so a case that ends mid-sequence still pins its result */
    buf_adds(&b, mod);
    buf_adds(&b, " vmend");
    f_cstr(&b, "case", name);
    f_u(&b, "ops", (unsigned long long)idx);
    f_tree_mut(&b, "root", v.doc ? yyjson_mut_doc_get_root(v.doc) : NULL);
    {
        size_t ol = 0;
        char *o = v.doc ? yyjson_mut_write(v.doc, YYJSON_WRITE_ALLOW_INF_AND_NAN,
                                           &ol) : NULL;
        f_bytes(&b, "json", o, o ? ol : 0);
        free(o);
    }
    rec_line(&b);
    rec_cases++;

    /* teardown: the document first, then the arena its non-copying string
       values point into, then the scratch immutable documents */
    yyjson_mut_doc_free(v.doc);
    for (size_t i = 0; i < v.ntdoc; i++) yyjson_doc_free(v.tdoc[i]);
    free(v.tdoc);
    for (size_t i = 0; i < v.nstr; i++) free(v.str[i]);
    free(v.str);
    buf_free(&b);
}
