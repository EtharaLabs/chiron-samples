#include "common.h"

/*==============================================================================
 * Read-flag sweeps.
 *
 * The 14 read flags give 2^14 combinations; the harness uses a curated set
 * instead.  It contains every individual flag, the documented JSON5 bundle,
 * the pairs that interact in the reader (INSITU x STOP_WHEN_DONE,
 * NUMBER_AS_RAW x BIGNUM_AS_RAW, EXT_NUMBER x INF_AND_NAN,
 * ALLOW_INVALID_UNICODE x ESCAPE paths, unquoted key x single-quoted string)
 * and the all-flags-on combination.  Names are stable record keys.
 *============================================================================*/

#define R(n) YYJSON_READ_##n

const rflag_case RFLAGS[] = {
    { "none",            R(NOFLAG) },
    { "insitu",          R(INSITU) },
    { "stop",            R(STOP_WHEN_DONE) },
    { "insitu_stop",     R(INSITU) | R(STOP_WHEN_DONE) },
    { "trailcomma",      R(ALLOW_TRAILING_COMMAS) },
    { "comments",        R(ALLOW_COMMENTS) },
    { "infnan",          R(ALLOW_INF_AND_NAN) },
    { "numraw",          R(NUMBER_AS_RAW) },
    { "invunicode",      R(ALLOW_INVALID_UNICODE) },
    { "bignumraw",       R(BIGNUM_AS_RAW) },
    { "bom",             R(ALLOW_BOM) },
    { "extnum",          R(ALLOW_EXT_NUMBER) },
    { "extesc",          R(ALLOW_EXT_ESCAPE) },
    { "extws",           R(ALLOW_EXT_WHITESPACE) },
    { "sqstr",           R(ALLOW_SINGLE_QUOTED_STR) },
    { "unqkey",          R(ALLOW_UNQUOTED_KEY) },
    { "json5",           R(JSON5) },
    { "numraw_bignum",   R(NUMBER_AS_RAW) | R(BIGNUM_AS_RAW) },
    { "extnum_infnan",   R(ALLOW_EXT_NUMBER) | R(ALLOW_INF_AND_NAN) },
    { "invuni_extesc",   R(ALLOW_INVALID_UNICODE) | R(ALLOW_EXT_ESCAPE) },
    { "sqstr_unqkey",    R(ALLOW_SINGLE_QUOTED_STR) | R(ALLOW_UNQUOTED_KEY) },
    { "json5_insitu",    R(JSON5) | R(INSITU) },
    { "json5_numraw",    R(JSON5) | R(NUMBER_AS_RAW) },
    { "all",             R(INSITU) | R(STOP_WHEN_DONE) |
                         R(ALLOW_TRAILING_COMMAS) | R(ALLOW_COMMENTS) |
                         R(ALLOW_INF_AND_NAN) | R(NUMBER_AS_RAW) |
                         R(ALLOW_INVALID_UNICODE) | R(BIGNUM_AS_RAW) |
                         R(ALLOW_BOM) | R(ALLOW_EXT_NUMBER) |
                         R(ALLOW_EXT_ESCAPE) | R(ALLOW_EXT_WHITESPACE) |
                         R(ALLOW_SINGLE_QUOTED_STR) | R(ALLOW_UNQUOTED_KEY) },
};
const size_t RFLAGS_N = sizeof(RFLAGS) / sizeof(RFLAGS[0]);

/* Every individual read flag, used where a full cross product is affordable. */
const rflag_case RFLAGS_ALL[] = {
    { "none",       R(NOFLAG) },
    { "insitu",     R(INSITU) },
    { "stop",       R(STOP_WHEN_DONE) },
    { "trailcomma", R(ALLOW_TRAILING_COMMAS) },
    { "comments",   R(ALLOW_COMMENTS) },
    { "infnan",     R(ALLOW_INF_AND_NAN) },
    { "numraw",     R(NUMBER_AS_RAW) },
    { "invunicode", R(ALLOW_INVALID_UNICODE) },
    { "bignumraw",  R(BIGNUM_AS_RAW) },
    { "bom",        R(ALLOW_BOM) },
    { "extnum",     R(ALLOW_EXT_NUMBER) },
    { "extesc",     R(ALLOW_EXT_ESCAPE) },
    { "extws",      R(ALLOW_EXT_WHITESPACE) },
    { "sqstr",      R(ALLOW_SINGLE_QUOTED_STR) },
    { "unqkey",     R(ALLOW_UNQUOTED_KEY) },
};
const size_t RFLAGS_ALL_N = sizeof(RFLAGS_ALL) / sizeof(RFLAGS_ALL[0]);

/* The 12 read-flag combinations upstream's test_number.c uses, which are the
   ones that change how a numeric literal is tagged. */
const rflag_case RFLAGS_NUM[] = {
    { "n",      R(NOFLAG) },
    { "b",      R(BIGNUM_AS_RAW) },
    { "r",      R(NUMBER_AS_RAW) },
    { "i",      R(ALLOW_INF_AND_NAN) },
    { "e",      R(ALLOW_EXT_NUMBER) },
    { "ie",     R(ALLOW_INF_AND_NAN) | R(ALLOW_EXT_NUMBER) },
    { "bi",     R(BIGNUM_AS_RAW) | R(ALLOW_INF_AND_NAN) },
    { "ri",     R(NUMBER_AS_RAW) | R(ALLOW_INF_AND_NAN) },
    { "be",     R(BIGNUM_AS_RAW) | R(ALLOW_EXT_NUMBER) },
    { "re",     R(NUMBER_AS_RAW) | R(ALLOW_EXT_NUMBER) },
    { "bie",    R(BIGNUM_AS_RAW) | R(ALLOW_INF_AND_NAN) | R(ALLOW_EXT_NUMBER) },
    { "rie",    R(NUMBER_AS_RAW) | R(ALLOW_INF_AND_NAN) | R(ALLOW_EXT_NUMBER) },
};
const size_t RFLAGS_NUM_N = sizeof(RFLAGS_NUM) / sizeof(RFLAGS_NUM[0]);

#undef R

/*==============================================================================
 * Write-flag sweeps.
 *============================================================================*/

#define W(n) YYJSON_WRITE_##n

const wflag_case WFLAGS[] = {
    { "none",        W(NOFLAG) },
    { "pretty",      W(PRETTY) },
    { "pretty2",     W(PRETTY_TWO_SPACES) },
    { "escuni",      W(ESCAPE_UNICODE) },
    { "escslash",    W(ESCAPE_SLASHES) },
    { "infnan",      W(ALLOW_INF_AND_NAN) },
    { "infnannull",  W(INF_AND_NAN_AS_NULL) },
    { "invuni",      W(ALLOW_INVALID_UNICODE) },
    { "newline",     W(NEWLINE_AT_END) },
    { "lowerhex",    W(LOWERCASE_HEX) },
    { "fpflt",       W(FP_TO_FLOAT) },
    { "pretty_nl",   W(PRETTY) | W(NEWLINE_AT_END) },
    { "pretty2_nl",  W(PRETTY_TWO_SPACES) | W(NEWLINE_AT_END) },
    { "escuni_lhex", W(ESCAPE_UNICODE) | W(LOWERCASE_HEX) },
    { "escuni_invu", W(ESCAPE_UNICODE) | W(ALLOW_INVALID_UNICODE) },
    { "escuni_slsh", W(ESCAPE_UNICODE) | W(ESCAPE_SLASHES) },
    { "pretty_escu", W(PRETTY) | W(ESCAPE_UNICODE) | W(ESCAPE_SLASHES) },
    { "infnan_flt",  W(ALLOW_INF_AND_NAN) | W(FP_TO_FLOAT) },
    { "nannull_flt", W(INF_AND_NAN_AS_NULL) | W(FP_TO_FLOAT) },
    { "all",         W(PRETTY) | W(ESCAPE_UNICODE) | W(ESCAPE_SLASHES) |
                     W(ALLOW_INF_AND_NAN) | W(ALLOW_INVALID_UNICODE) |
                     W(NEWLINE_AT_END) | W(LOWERCASE_HEX) },
    { "all2",        W(PRETTY_TWO_SPACES) | W(ESCAPE_UNICODE) |
                     W(ESCAPE_SLASHES) | W(INF_AND_NAN_AS_NULL) |
                     W(ALLOW_INVALID_UNICODE) | W(NEWLINE_AT_END) },
    /* YYJSON_WRITE_FP_TO_FIXED(prec) for every documented precision 1..15,
       plus the out-of-range 0 and 16 which the header masks to 4 bits. */
    { "fix0",  W(FP_TO_FIXED(0)) },
    { "fix1",  W(FP_TO_FIXED(1)) },
    { "fix2",  W(FP_TO_FIXED(2)) },
    { "fix3",  W(FP_TO_FIXED(3)) },
    { "fix4",  W(FP_TO_FIXED(4)) },
    { "fix5",  W(FP_TO_FIXED(5)) },
    { "fix6",  W(FP_TO_FIXED(6)) },
    { "fix7",  W(FP_TO_FIXED(7)) },
    { "fix8",  W(FP_TO_FIXED(8)) },
    { "fix9",  W(FP_TO_FIXED(9)) },
    { "fix10", W(FP_TO_FIXED(10)) },
    { "fix11", W(FP_TO_FIXED(11)) },
    { "fix12", W(FP_TO_FIXED(12)) },
    { "fix13", W(FP_TO_FIXED(13)) },
    { "fix14", W(FP_TO_FIXED(14)) },
    { "fix15", W(FP_TO_FIXED(15)) },
    { "fix16", W(FP_TO_FIXED(16)) },
    { "fix3_pretty", W(FP_TO_FIXED(3)) | W(PRETTY) },
    { "fix15_infnan", W(FP_TO_FIXED(15)) | W(ALLOW_INF_AND_NAN) },
};
const size_t WFLAGS_N = sizeof(WFLAGS) / sizeof(WFLAGS[0]);

const wflag_case WFLAGS_CORE[] = {
    { "none",       W(NOFLAG) },
    { "pretty",     W(PRETTY) },
    { "escuni",     W(ESCAPE_UNICODE) },
    { "allowall",   W(ALLOW_INF_AND_NAN) | W(ALLOW_INVALID_UNICODE) },
    { "all",        W(PRETTY_TWO_SPACES) | W(ESCAPE_UNICODE) |
                    W(ESCAPE_SLASHES) | W(ALLOW_INF_AND_NAN) |
                    W(ALLOW_INVALID_UNICODE) | W(NEWLINE_AT_END) |
                    W(LOWERCASE_HEX) },
};
const size_t WFLAGS_CORE_N = sizeof(WFLAGS_CORE) / sizeof(WFLAGS_CORE[0]);

const wflag_case WFLAGS_NUM[] = {
    { "n",     W(NOFLAG) },
    { "i",     W(ALLOW_INF_AND_NAN) },
    { "z",     W(INF_AND_NAN_AS_NULL) },
    { "f",     W(FP_TO_FLOAT) },
    { "fi",    W(FP_TO_FLOAT) | W(ALLOW_INF_AND_NAN) },
    { "fz",    W(FP_TO_FLOAT) | W(INF_AND_NAN_AS_NULL) },
    { "x1",    W(FP_TO_FIXED(1)) },
    { "x2",    W(FP_TO_FIXED(2)) },
    { "x3",    W(FP_TO_FIXED(3)) },
    { "x4",    W(FP_TO_FIXED(4)) },
    { "x5",    W(FP_TO_FIXED(5)) },
    { "x6",    W(FP_TO_FIXED(6)) },
    { "x7",    W(FP_TO_FIXED(7)) },
    { "x8",    W(FP_TO_FIXED(8)) },
    { "x9",    W(FP_TO_FIXED(9)) },
    { "x10",   W(FP_TO_FIXED(10)) },
    { "x11",   W(FP_TO_FIXED(11)) },
    { "x12",   W(FP_TO_FIXED(12)) },
    { "x13",   W(FP_TO_FIXED(13)) },
    { "x14",   W(FP_TO_FIXED(14)) },
    { "x15",   W(FP_TO_FIXED(15)) },
};
const size_t WFLAGS_NUM_N = sizeof(WFLAGS_NUM) / sizeof(WFLAGS_NUM[0]);

#undef W
