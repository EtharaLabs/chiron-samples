#include "common.h"

#include <dirent.h>
#include <sys/stat.h>

/*==============================================================================
 * MARK: - buffer
 *============================================================================*/

static void die(const char *m) {
    fprintf(stderr, "harness: fatal: %s\n", m);
    exit(2);
}

void buf_init(buf *b) { b->dat = NULL; b->len = 0; b->cap = 0; }
void buf_free(buf *b) { free(b->dat); buf_init(b); }
void buf_clear(buf *b) { b->len = 0; }

static void buf_need(buf *b, size_t n) {
    if (b->len + n + 1 <= b->cap) return;
    size_t cap = b->cap ? b->cap : 256;
    while (cap < b->len + n + 1) cap *= 2;
    char *p = (char *)realloc(b->dat, cap);
    if (!p) die("out of memory");
    b->dat = p;
    b->cap = cap;
}

void buf_add(buf *b, const void *p, size_t n) {
    if (!n) return;
    buf_need(b, n);
    memcpy(b->dat + b->len, p, n);
    b->len += n;
    b->dat[b->len] = '\0';
}

void buf_addc(buf *b, char c) { buf_add(b, &c, 1); }
void buf_adds(buf *b, const char *s) { buf_add(b, s, strlen(s)); }

void buf_addf(buf *b, const char *fmt, ...) {
    va_list ap;
    char tmp[512];
    va_start(ap, fmt);
    int n = vsnprintf(tmp, sizeof(tmp), fmt, ap);
    va_end(ap);
    if (n < 0) die("vsnprintf");
    if ((size_t)n < sizeof(tmp)) { buf_add(b, tmp, (size_t)n); return; }
    char *big = (char *)malloc((size_t)n + 1);
    if (!big) die("out of memory");
    va_start(ap, fmt);
    vsnprintf(big, (size_t)n + 1, fmt, ap);
    va_end(ap);
    buf_add(b, big, (size_t)n);
    free(big);
}

/* Safe bytes for percent-encoding: [A-Za-z0-9] and "_-./+()[]#~^".
   Deliberately excluded: space, '=', '%', '!', ',', ':', '{', '}', quotes,
   every control byte and every byte >= 0x80. */
static const unsigned char SAFE[256] = {
    /* 0x00-0x1F */
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    /* 0x20 ' '  !  "  #  $  %  &  '  (  )  *  +  ,  -  .  / */
    0,  0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 1,
    /* 0x30 0-9 then :  ;  <  =  >  ? */
    1,1,1,1,1,1,1,1,1,1, 0, 0, 0, 0, 0, 0,
    /* 0x40 @ A-O */
    0, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    /* 0x50 P-Z then [  \  ]  ^  _ */
    1,1,1,1,1,1,1,1,1,1,1, 1, 0, 1, 1, 1,
    /* 0x60 ` a-o */
    0, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    /* 0x70 p-z then {  |  }  ~  DEL */
    1,1,1,1,1,1,1,1,1,1,1, 0, 0, 0, 1, 0,
    /* 0x80-0xFF */
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
};
static const char HEX[] = "0123456789ABCDEF";

void buf_q(buf *b, const void *p, size_t n) {
    const unsigned char *s = (const unsigned char *)p;
    buf_need(b, n * 3);
    for (size_t i = 0; i < n; i++) {
        unsigned char c = s[i];
        if (SAFE[c]) {
            b->dat[b->len++] = (char)c;
        } else {
            b->dat[b->len++] = '%';
            b->dat[b->len++] = HEX[c >> 4];
            b->dat[b->len++] = HEX[c & 15];
        }
    }
    b->dat[b->len] = '\0';
}

/*==============================================================================
 * MARK: - records
 *============================================================================*/

static FILE *g_rec = NULL;
unsigned long long rec_lines = 0;
unsigned long long rec_cases = 0;

void rec_open(const char *path) {
    g_rec = fopen(path, "wb");
    if (!g_rec) die("cannot open output file");
}

void rec_close(void) {
    if (g_rec) {
        if (fflush(g_rec) != 0) die("record flush failed");
        fclose(g_rec);
        g_rec = NULL;
    }
}

void rec_line(buf *b) {
    if (!g_rec) die("record file not open");
    if (b->len) fwrite(b->dat, 1, b->len, g_rec);
    fputc('\n', g_rec);
    rec_lines++;
    buf_clear(b);
}

/*==============================================================================
 * MARK: - fields
 *============================================================================*/

uint64_t fnv1a(const void *p, size_t n) {
    const unsigned char *s = (const unsigned char *)p;
    uint64_t h = 0xCBF29CE484222325ULL;
    for (size_t i = 0; i < n; i++) {
        h ^= s[i];
        h *= 0x100000001B3ULL;
    }
    return h;
}

void f_str(buf *b, const char *name, const char *s, size_t n) {
    buf_addc(b, ' ');
    buf_adds(b, name);
    buf_addc(b, '=');
    if (!s)     { buf_adds(b, "!null");  return; }
    if (n == 0) { buf_adds(b, "!empty"); return; }
    buf_q(b, s, n);
}

void f_cstr(buf *b, const char *name, const char *s) {
    f_str(b, name, s, s ? strlen(s) : 0);
}

void f_bytes(buf *b, const char *name, const char *s, size_t n) {
    if (!s) { f_str(b, name, NULL, 0); return; }
    buf_addc(b, ' ');
    buf_adds(b, name);
    buf_addf(b, "_len=%zu", n);
    if (n <= F_CAP) {
        f_str(b, name, s, n);
    } else {
        buf_addc(b, ' ');
        buf_adds(b, name);
        buf_addf(b, "_fnv=0x%016llX", (unsigned long long)fnv1a(s, n));
        buf_addc(b, ' ');
        buf_adds(b, name);
        buf_adds(b, "_head=");
        buf_q(b, s, F_EDGE);
        buf_addc(b, ' ');
        buf_adds(b, name);
        buf_adds(b, "_tail=");
        buf_q(b, s + n - F_EDGE, F_EDGE);
    }
}

void f_u(buf *b, const char *name, unsigned long long v) {
    buf_addf(b, " %s=%llu", name, v);
}
void f_i(buf *b, const char *name, long long v) {
    buf_addf(b, " %s=%lld", name, v);
}
void f_x64(buf *b, const char *name, unsigned long long v) {
    buf_addf(b, " %s=0x%016llX", name, v);
}
void f_bool(buf *b, const char *name, int v) {
    buf_addf(b, " %s=%d", name, v ? 1 : 0);
}

/*==============================================================================
 * MARK: - corpus
 *============================================================================*/

static int cmp_str(const void *a, const void *b) {
    return strcmp(*(const char *const *)a, *(const char *const *)b);
}

typedef struct {
    char **v;
    size_t n, cap;
} svec;

static void svec_push(svec *s, const char *str) {
    if (s->n == s->cap) {
        s->cap = s->cap ? s->cap * 2 : 64;
        s->v = (char **)realloc(s->v, s->cap * sizeof(char *));
        if (!s->v) die("out of memory");
    }
    s->v[s->n] = strdup(str);
    if (!s->v[s->n]) die("out of memory");
    s->n++;
}

static void walk(svec *out, const char *root, const char *rel) {
    char full[4096];
    snprintf(full, sizeof(full), "%s/%s", root, rel);
    DIR *d = opendir(full);
    if (!d) return;
    /* Collect this level's entries first, then recurse in sorted order, so
       the result does not depend on readdir order. */
    svec here = {0};
    struct dirent *e;
    while ((e = readdir(d)) != NULL) {
        if (e->d_name[0] == '.') continue;
        svec_push(&here, e->d_name);
    }
    closedir(d);
    qsort(here.v, here.n, sizeof(char *), cmp_str);
    for (size_t i = 0; i < here.n; i++) {
        char sub[4096];
        snprintf(sub, sizeof(sub), "%s/%s", rel, here.v[i]);
        /* strictly larger than `root` + `sub`, so the concatenation is
           provably non-truncating and -Wformat-truncation stays quiet */
        char path[8320];
        snprintf(path, sizeof(path), "%s/%s", root, sub);
        struct stat st;
        if (stat(path, &st) == 0) {
            if (S_ISDIR(st.st_mode)) {
                walk(out, root, sub);
            } else if (S_ISREG(st.st_mode) &&
                       strcmp(here.v[i], "README.md") != 0) {
                svec_push(out, sub);
            }
        }
        free(here.v[i]);
    }
    free(here.v);
}

flist corpus_list(const char *root, const char *sub) {
    svec s = {0};
    walk(&s, root, sub);
    qsort(s.v, s.n, sizeof(char *), cmp_str);
    flist f;
    f.name = s.v;
    f.n = s.n;
    if (f.n == 0) {
        fprintf(stderr, "harness: corpus subdirectory '%s/%s' is empty\n",
                root, sub);
        exit(2);
    }
    return f;
}

void flist_free(flist *f) {
    for (size_t i = 0; i < f->n; i++) free(f->name[i]);
    free(f->name);
    f->name = NULL;
    f->n = 0;
}

int file_slurp(const char *path, char **out, size_t *outlen) {
    FILE *fp = fopen(path, "rb");
    if (!fp) return 0;
    buf b;
    buf_init(&b);
    char tmp[65536];
    size_t r;
    while ((r = fread(tmp, 1, sizeof(tmp), fp)) > 0) buf_add(&b, tmp, r);
    fclose(fp);
    if (!b.dat) {
        b.dat = (char *)calloc(1, 1);
        if (!b.dat) die("out of memory");
        b.len = 0;
    }
    *out = b.dat;
    *outlen = b.len;
    return 1;
}

/*==============================================================================
 * MARK: - tree serialization (iterative, no recursion depth limit)
 *============================================================================*/

static char type_ch(yyjson_type t) {
    switch (t) {
        case YYJSON_TYPE_NONE: return '?';
        case YYJSON_TYPE_RAW:  return 'R';
        case YYJSON_TYPE_NULL: return 'N';
        case YYJSON_TYPE_BOOL: return 'B';
        case YYJSON_TYPE_NUM:  return 'M';
        case YYJSON_TYPE_STR:  return 'S';
        case YYJSON_TYPE_ARR:  return 'A';
        case YYJSON_TYPE_OBJ:  return 'O';
        default:               return 'X';
    }
}

/* ---- immutable ---- */

typedef struct {
    int is_obj;
    union {
        yyjson_arr_iter a;
        yyjson_obj_iter o;
    } it;
} iframe;

static void imut_head(buf *b, yyjson_val *v) {
    yyjson_type t = yyjson_get_type(v);
    unsigned sub = (unsigned)yyjson_get_subtype(v) >> 3;
    buf_addf(b, "%c%u.%zu", type_ch(t), sub, yyjson_get_len(v));
    if (t == YYJSON_TYPE_NUM) {
        /* raw 64-bit payload pattern */
        uint64_t bits = yyjson_get_uint(v);
        if (yyjson_get_subtype(v) == YYJSON_SUBTYPE_REAL) {
            double d = yyjson_get_real(v);
            memcpy(&bits, &d, 8);
        }
        buf_addf(b, ".0x%016llX", (unsigned long long)bits);
    } else if (t == YYJSON_TYPE_STR || t == YYJSON_TYPE_RAW) {
        const char *s = (t == YYJSON_TYPE_STR) ? yyjson_get_str(v)
                                               : yyjson_get_raw(v);
        size_t n = yyjson_get_len(v);
        buf_addc(b, '.');
        if (!s)     buf_adds(b, "!null");
        else if (!n) buf_adds(b, "!empty");
        else         buf_q(b, s, n);
    }
}

void tree_imut(buf *b, yyjson_val *v) {
    if (!v) { buf_adds(b, "!null"); return; }
    iframe *st = NULL;
    size_t sn = 0, scap = 0;
    yyjson_val *cur = v;
    int fresh = 1;

    for (;;) {
        if (fresh) {
            imut_head(b, cur);
            yyjson_type t = yyjson_get_type(cur);
            if (t == YYJSON_TYPE_ARR || t == YYJSON_TYPE_OBJ) {
                if (sn == scap) {
                    scap = scap ? scap * 2 : 64;
                    st = (iframe *)realloc(st, scap * sizeof(iframe));
                    if (!st) die("out of memory");
                }
                iframe *f = &st[sn++];
                f->is_obj = (t == YYJSON_TYPE_OBJ);
                if (f->is_obj) yyjson_obj_iter_init(cur, &f->it.o);
                else           yyjson_arr_iter_init(cur, &f->it.a);
                buf_addc(b, '[');
                fresh = 0;
                continue;
            }
        }
        if (sn == 0) break;
        iframe *f = &st[sn - 1];
        int first = f->is_obj ? (f->it.o.idx == 0) : (f->it.a.idx == 0);
        if (f->is_obj) {
            yyjson_val *k = yyjson_obj_iter_next(&f->it.o);
            if (!k) { buf_addc(b, ']'); sn--; fresh = 0; continue; }
            if (!first) buf_addc(b, '+');
            imut_head(b, k);
            buf_addc(b, '^');
            cur = yyjson_obj_iter_get_val(k);
            fresh = 1;
        } else {
            yyjson_val *e = yyjson_arr_iter_next(&f->it.a);
            if (!e) { buf_addc(b, ']'); sn--; fresh = 0; continue; }
            if (!first) buf_addc(b, '+');
            cur = e;
            fresh = 1;
        }
    }
    free(st);
}

/* ---- mutable ---- */

typedef struct {
    int is_obj;
    union {
        yyjson_mut_arr_iter a;
        yyjson_mut_obj_iter o;
    } it;
} mframe;

static void mut_head(buf *b, yyjson_mut_val *v) {
    yyjson_type t = yyjson_mut_get_type(v);
    unsigned sub = (unsigned)yyjson_mut_get_subtype(v) >> 3;
    buf_addf(b, "%c%u.%zu", type_ch(t), sub, yyjson_mut_get_len(v));
    if (t == YYJSON_TYPE_NUM) {
        uint64_t bits = yyjson_mut_get_uint(v);
        if (yyjson_mut_get_subtype(v) == YYJSON_SUBTYPE_REAL) {
            double d = yyjson_mut_get_real(v);
            memcpy(&bits, &d, 8);
        }
        buf_addf(b, ".0x%016llX", (unsigned long long)bits);
    } else if (t == YYJSON_TYPE_STR || t == YYJSON_TYPE_RAW) {
        const char *s = (t == YYJSON_TYPE_STR) ? yyjson_mut_get_str(v)
                                               : yyjson_mut_get_raw(v);
        size_t n = yyjson_mut_get_len(v);
        buf_addc(b, '.');
        if (!s)      buf_adds(b, "!null");
        else if (!n) buf_adds(b, "!empty");
        else         buf_q(b, s, n);
    }
}

void tree_mut(buf *b, yyjson_mut_val *v) {
    if (!v) { buf_adds(b, "!null"); return; }
    mframe *st = NULL;
    size_t sn = 0, scap = 0;
    yyjson_mut_val *cur = v;
    int fresh = 1;

    for (;;) {
        if (fresh) {
            mut_head(b, cur);
            yyjson_type t = yyjson_mut_get_type(cur);
            if (t == YYJSON_TYPE_ARR || t == YYJSON_TYPE_OBJ) {
                if (sn == scap) {
                    scap = scap ? scap * 2 : 64;
                    st = (mframe *)realloc(st, scap * sizeof(mframe));
                    if (!st) die("out of memory");
                }
                mframe *f = &st[sn++];
                f->is_obj = (t == YYJSON_TYPE_OBJ);
                if (f->is_obj) yyjson_mut_obj_iter_init(cur, &f->it.o);
                else           yyjson_mut_arr_iter_init(cur, &f->it.a);
                buf_addc(b, '[');
                fresh = 0;
                continue;
            }
        }
        if (sn == 0) break;
        mframe *f = &st[sn - 1];
        int first = f->is_obj ? (f->it.o.idx == 0) : (f->it.a.idx == 0);
        if (f->is_obj) {
            yyjson_mut_val *k = yyjson_mut_obj_iter_next(&f->it.o);
            if (!k) { buf_addc(b, ']'); sn--; fresh = 0; continue; }
            if (!first) buf_addc(b, '+');
            mut_head(b, k);
            buf_addc(b, '^');
            cur = yyjson_mut_obj_iter_get_val(k);
            fresh = 1;
        } else {
            yyjson_mut_val *e = yyjson_mut_arr_iter_next(&f->it.a);
            if (!e) { buf_addc(b, ']'); sn--; fresh = 0; continue; }
            if (!first) buf_addc(b, '+');
            cur = e;
            fresh = 1;
        }
    }
    free(st);
}

static buf g_scratch;

void f_tree_imut(buf *b, const char *name, yyjson_val *v) {
    buf_clear(&g_scratch);
    tree_imut(&g_scratch, v);
    f_bytes(b, name, g_scratch.dat ? g_scratch.dat : "", g_scratch.len);
}

void f_tree_mut(buf *b, const char *name, yyjson_mut_val *v) {
    buf_clear(&g_scratch);
    tree_mut(&g_scratch, v);
    f_bytes(b, name, g_scratch.dat ? g_scratch.dat : "", g_scratch.len);
}

/*==============================================================================
 * MARK: - tracing allocator
 *============================================================================*/

static void ev_push(alc_trace *t, char kind, size_t size, size_t old_size) {
    if (!t->enabled) return;
    if (t->n == t->cap) {
        t->cap = t->cap ? t->cap * 2 : 256;
        t->ev = (alc_ev *)realloc(t->ev, t->cap * sizeof(alc_ev));
        if (!t->ev) die("out of memory");
    }
    t->ev[t->n].kind = kind;
    t->ev[t->n].size = size;
    t->ev[t->n].old_size = old_size;
    t->n++;
}

static void *tr_malloc(void *ctx, size_t size) {
    alc_trace *t = (alc_trace *)ctx;
    ev_push(t, 'M', size, 0);
    t->n_alloc++;
    if (t->fail_after && t->n_alloc > t->fail_after) return NULL;
    void *p = malloc(size);
    if (p) t->live++;
    return p;
}

static void *tr_realloc(void *ctx, void *ptr, size_t old_size, size_t size) {
    alc_trace *t = (alc_trace *)ctx;
    ev_push(t, 'R', size, old_size);
    t->n_alloc++;
    if (t->fail_after && t->n_alloc > t->fail_after) return NULL;
    return realloc(ptr, size);
}

static void tr_free(void *ctx, void *ptr) {
    alc_trace *t = (alc_trace *)ctx;
    ev_push(t, 'F', 0, 0);
    if (ptr) {
        if (t->live) t->live--;
        free(ptr);
    }
}

void alc_trace_init(alc_trace *t, yyjson_alc *alc) {
    memset(t, 0, sizeof(*t));
    t->enabled = 1;
    alc->malloc = tr_malloc;
    alc->realloc = tr_realloc;
    alc->free = tr_free;
    alc->ctx = t;
}

void alc_trace_reset(alc_trace *t) {
    t->n = 0;
    t->n_alloc = 0;
    t->fail_after = 0;
}

void alc_trace_free(alc_trace *t) { free(t->ev); t->ev = NULL; t->cap = 0; }

void f_alc_trace(buf *b, const char *name, alc_trace *t) {
    buf_clear(&g_scratch);
    for (size_t i = 0; i < t->n; i++) {
        if (i) buf_addc(&g_scratch, '+');
        if (t->ev[i].kind == 'R') {
            buf_addf(&g_scratch, "R.%zu.%zu", t->ev[i].old_size,
                     t->ev[i].size);
        } else if (t->ev[i].kind == 'M') {
            buf_addf(&g_scratch, "M.%zu", t->ev[i].size);
        } else {
            buf_adds(&g_scratch, "F");
        }
    }
    buf_addf(b, " %s_n=%zu", name, t->n);
    f_bytes(b, name, g_scratch.dat ? g_scratch.dat : "", g_scratch.len);
}
