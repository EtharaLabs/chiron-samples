/*==============================================================================
 * Frozen behavioural harness for the yyjson 0.13.0 C -> Rust migration task.
 *
 * This translation unit set includes ONLY <yyjson.h> and links against the
 * installed library under test.  It must never include yyjson.c.
 *============================================================================*/
#ifndef HARNESS_COMMON_H
#define HARNESS_COMMON_H

#include <yyjson.h>

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*==============================================================================
 * MARK: - growable byte buffer
 *============================================================================*/

typedef struct {
    char  *dat;
    size_t len;
    size_t cap;
} buf;

void buf_init(buf *b);
void buf_free(buf *b);
void buf_clear(buf *b);
void buf_add(buf *b, const void *p, size_t n);
void buf_addc(buf *b, char c);
void buf_adds(buf *b, const char *s);
void buf_addf(buf *b, const char *fmt, ...);

/** Percent-encode `n` bytes of `p` into `b`.
    Safe bytes are [A-Za-z0-9] and "_-./+()[]#~^"; everything else becomes
    %XX with upper-case hex.  The output therefore never contains a space,
    an '=', a '%' that is not an escape introducer, or a non-ASCII byte. */
void buf_q(buf *b, const void *p, size_t n);

/*==============================================================================
 * MARK: - record output
 *============================================================================*/

void rec_open(const char *path);
void rec_close(void);
/** Write `b` followed by '\n' to the record file, then clear `b`. */
void rec_line(buf *b);
extern unsigned long long rec_lines;
extern unsigned long long rec_cases;

/*==============================================================================
 * MARK: - field helpers
 *
 * Every field is `name=value`.  Values never contain a space or an '='.
 * Sentinels: `!null` for a NULL char*, `!empty` for a zero-length string.
 * `!` is not in the safe set, so a real payload can never produce them.
 *============================================================================*/

/** Exact bytes, no length cap.  Use for short strings (messages, keys). */
void f_str(buf *b, const char *name, const char *s, size_t n);
/** NUL-terminated string, NULL-safe. */
void f_cstr(buf *b, const char *name, const char *s);
/** Length-capped byte field: verbatim up to F_CAP, else len+fnv+head+tail. */
void f_bytes(buf *b, const char *name, const char *s, size_t n);
#define F_CAP 768u
#define F_EDGE 128u

void f_u(buf *b, const char *name, unsigned long long v);
void f_i(buf *b, const char *name, long long v);
void f_x64(buf *b, const char *name, unsigned long long v);
void f_bool(buf *b, const char *name, int v);

uint64_t fnv1a(const void *p, size_t n);

/*==============================================================================
 * MARK: - corpus
 *============================================================================*/

typedef struct {
    char **name; /* paths relative to the corpus root, byte-sorted */
    size_t n;
} flist;

/** Recursively list regular files under `<root>/<sub>`; names are returned
    relative to `root` and sorted with strcmp (byte order).  Files whose name
    starts with '.' and files named README.md are skipped. */
flist corpus_list(const char *root, const char *sub);
void  flist_free(flist *f);

/** Read a whole file.  `*out` gets a malloc'd buffer with one extra NUL
    byte past `*outlen` (not counted).  Returns 1 on success. */
int file_slurp(const char *path, char **out, size_t *outlen);

/*==============================================================================
 * MARK: - canonical tree serialization
 *
 * Grammar (all bytes are in the percent-encoding safe set):
 *
 *   value  := scalar | array | object
 *   scalar := TYPE SUB '.' LEN [ '.' PAYLOAD ]
 *   array  := 'A' SUB '.' LEN '[' value ( '+' value )* ']'
 *   object := 'O' SUB '.' LEN '[' key '^' value ( '+' key '^' value )* ']'
 *   TYPE   := '?' none | 'R' raw | 'N' null | 'B' bool | 'M' num
 *           | 'S' str | 'A' arr | 'O' obj
 *   SUB    := decimal subtype as returned by yyjson_get_subtype >> 3
 *
 * PAYLOAD is emitted only where every bit of it is defined by the library:
 *   - num: the raw 64-bit uni payload as 0x%016llX
 *   - str/raw: the exact bytes, percent-encoded (0 bytes -> !empty)
 * null and bool never write `uni`, and arr/obj `uni` is an address or an
 * offset, so no payload is emitted for them.
 *============================================================================*/

void tree_imut(buf *b, yyjson_val *v);
void tree_mut(buf *b, yyjson_mut_val *v);
/** Serialize into scratch and emit as a capped `name` field. */
void f_tree_imut(buf *b, const char *name, yyjson_val *v);
void f_tree_mut(buf *b, const char *name, yyjson_mut_val *v);

/*==============================================================================
 * MARK: - tracing allocator
 *============================================================================*/

typedef struct {
    char   kind;    /* 'M' malloc, 'R' realloc, 'F' free */
    size_t size;
    size_t old_size;
} alc_ev;

typedef struct {
    alc_ev *ev;
    size_t  n;
    size_t  cap;
    size_t  live;       /* outstanding allocations */
    size_t  fail_after; /* return NULL once n_malloc_calls > fail_after */
    size_t  n_alloc;    /* malloc+realloc call count */
    int     enabled;
} alc_trace;

/** Install a tracing allocator.  `t` must outlive `alc`. */
void alc_trace_init(alc_trace *t, yyjson_alc *alc);
void alc_trace_reset(alc_trace *t);
void alc_trace_free(alc_trace *t);
/** Emit the recorded event sequence as capped fields. */
void f_alc_trace(buf *b, const char *name, alc_trace *t);

/*==============================================================================
 * MARK: - flag tables
 *============================================================================*/

typedef struct {
    const char      *name;
    yyjson_read_flag flg;
} rflag_case;

typedef struct {
    const char       *name;
    yyjson_write_flag flg;
} wflag_case;

extern const rflag_case RFLAGS[];      /* B-01 sweep */
extern const size_t      RFLAGS_N;
extern const rflag_case RFLAGS_NUM[];  /* B-03 sweep */
extern const size_t      RFLAGS_NUM_N;
extern const rflag_case RFLAGS_ALL[];  /* every individual read flag */
extern const size_t      RFLAGS_ALL_N;
extern const wflag_case WFLAGS[];      /* B-04 full sweep */
extern const size_t      WFLAGS_N;
extern const wflag_case WFLAGS_CORE[]; /* B-04 pass over the whole corpus */
extern const size_t      WFLAGS_CORE_N;
extern const wflag_case WFLAGS_NUM[];  /* B-03 write sweep */
extern const size_t      WFLAGS_NUM_N;

/*==============================================================================
 * MARK: - module entry points
 *============================================================================*/

typedef void (*mod_fn)(const char *corpus);

void mod_reader(const char *corpus);   /* B-01 */
void mod_errors(const char *corpus);   /* B-02 */
void mod_numbers(const char *corpus);  /* B-03 */
void mod_writer(const char *corpus);   /* B-04 */
void mod_mutable(const char *corpus);  /* B-05 */
void mod_ptrpatch(const char *corpus); /* B-06 */
void mod_alloc(const char *corpus);    /* B-07 */
void mod_incr(const char *corpus);     /* B-08 */
void mod_hdr(const char *corpus);      /* B-09 */
void mod_files(const char *corpus);    /* B-10 */

/** Temporary directory for B-10, from argv[4] or $HARNESS_TMPDIR. */
extern const char *g_tmpdir;

/** Writable copy of `len` bytes padded with YYJSON_PADDING_SIZE zero bytes,
    as YYJSON_READ_INSITU requires.  Caller frees. */
char *pad_copy(const char *dat, size_t len);
/** The standard read-result field group used by B-01 and B-02. */
void  f_read_result(buf *b, yyjson_doc *doc, const yyjson_read_err *err);

/*==============================================================================
 * MARK: - script VM (shared by B-05 and B-06)
 *============================================================================*/

/** Run one `.vm` script file.  `mod` is the record prefix, `name` the case
    key.  Emits one record line per executed op plus a final state dump. */
void vm_run(const char *mod, const char *name, char *src, size_t len);

#endif /* HARNESS_COMMON_H */
