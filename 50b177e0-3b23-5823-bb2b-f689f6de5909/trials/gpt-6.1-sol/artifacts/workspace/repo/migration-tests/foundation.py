#!/usr/bin/env python3
"""Differential checks for the ported foundation, using a separately built original.

Usage: python3 migration-tests/foundation.py ORIGINAL_SO RUST_SO
The original reader and writer are used only as test oracles.
"""
import ctypes as C
import random
import sys

P = C.c_void_p
Z = C.c_size_t
Malloc = C.CFUNCTYPE(P, P, Z)
Realloc = C.CFUNCTYPE(P, P, P, Z, Z)
Free = C.CFUNCTYPE(None, P, P)


class Alc(C.Structure):
    _fields_ = [("malloc", Malloc), ("realloc", Realloc), ("free", Free), ("ctx", P)]


class Pool(C.Structure):
    _fields_ = [("cur", P), ("end", P), ("size", Z), ("max", Z), ("chunks", P)]


class MutDoc(C.Structure):
    _fields_ = [("root", P), ("alc", Alc), ("strings", Pool), ("values", Pool)]


class Doc(C.Structure):
    _fields_ = [("root", P), ("alc", Alc), ("dat_read", Z), ("val_read", Z), ("strings", P)]


class MutVal(C.Structure):
    _fields_ = [("tag", C.c_uint64), ("payload", C.c_uint64), ("next", P)]


class Chunk(C.Structure):
    _fields_ = [("size", Z), ("next", P)]


libc = C.CDLL(None)
libc.malloc.argtypes, libc.malloc.restype = [Z], P
libc.realloc.argtypes, libc.realloc.restype = [P, Z], P
libc.free.argtypes, libc.free.restype = [P], None


def setup(path):
    lib = C.CDLL(path)
    signatures = {
        "yyjson_version": ([], C.c_uint32),
        "yyjson_locate_pos": ([P, Z, Z, P, P, P], C.c_bool),
        "yyjson_alc_pool_init": ([P, P, Z], C.c_bool),
        "yyjson_alc_dyn_new": ([], P),
        "yyjson_alc_dyn_free": ([P], None),
        "unsafe_yyjson_str_pool_grow": ([P, P, Z], C.c_bool),
        "unsafe_yyjson_val_pool_grow": ([P, P, Z], C.c_bool),
        "yyjson_mut_doc_new": ([P], P),
        "yyjson_mut_doc_free": ([P], None),
        "yyjson_mut_doc_set_str_pool_size": ([P, Z], C.c_bool),
        "yyjson_mut_doc_set_val_pool_size": ([P, Z], C.c_bool),
        "yyjson_doc_mut_copy": ([P, P], P),
        "yyjson_mut_doc_mut_copy": ([P, P], P),
        "yyjson_mut_doc_imut_copy": ([P, P], P),
        "yyjson_val_mut_copy": ([P, P], P),
        "yyjson_mut_val_mut_copy": ([P, P], P),
        "yyjson_mut_val_imut_copy": ([P, P], P),
        "unsafe_yyjson_equals": ([P, P], C.c_bool),
        "unsafe_yyjson_mut_equals": ([P, P], C.c_bool),
    }
    for name, (args, result) in signatures.items():
        fn = getattr(lib, name)
        fn.argtypes, fn.restype = args, result
    return lib


class Tracer:
    def __init__(self, fail=0):
        self.events = []
        self.blocks = {}
        self.attempts = 0
        self.fail = fail
        self.next_id = 0
        self.cb_malloc = Malloc(self.malloc)
        self.cb_realloc = Realloc(self.realloc)
        self.cb_free = Free(self.free)
        self.alc = Alc(self.cb_malloc, self.cb_realloc, self.cb_free, None)

    def malloc(self, ctx, size):
        self.attempts += 1
        self.events.append(("malloc", size))
        if self.attempts == self.fail:
            return None
        ptr = libc.malloc(size)
        assert ptr
        C.memset(ptr, 0xa5, size)
        self.blocks[ptr] = self.next_id
        self.next_id += 1
        return ptr

    def realloc(self, ctx, ptr, old, size):
        self.attempts += 1
        self.events.append(("realloc", self.blocks[ptr], old, size))
        if self.attempts == self.fail:
            return None
        new = libc.realloc(ptr, size)
        assert new
        self.blocks[new] = self.blocks.pop(ptr)
        return new

    def free(self, ctx, ptr):
        self.events.append(("free", self.blocks.pop(ptr)))
        libc.free(ptr)


def free_doc(ptr):
    if ptr:
        doc = Doc.from_address(ptr)
        alc = doc.alc
        if doc.strings:
            alc.free(alc.ctx, doc.strings)
        alc.free(alc.ctx, ptr)


def locate(lib, data, pos):
    line, col, char = Z(999), Z(999), Z(999)
    ok = lib.yyjson_locate_pos(data, len(data) if data else 0, pos,
                              C.byref(line), C.byref(col), C.byref(char))
    return ok, line.value, col.value, char.value


def check_positions(original, port):
    rng = random.Random(451)
    samples = [None, b"", b"\xef\xbb\xbfabc\n\xff", bytes(range(256))]
    samples += [rng.randbytes(rng.randrange(128)) for _ in range(300)]
    for data in samples:
        for pos in range((len(data) if data else 0) + 3):
            assert locate(original, data, pos) == locate(port, data, pos), (data, pos)


def check_fixed(original, port):
    for seed in range(32):
        rng = random.Random(seed)
        length = rng.randrange(64, 32768)
        offset = rng.randrange(16)
        bufs = [C.create_string_buffer(length + 32) for _ in range(2)]
        bases = [C.addressof(b) + offset for b in bufs]
        alcs = [Alc(), Alc()]
        for lib, alc, base in zip((original, port), alcs, bases):
            assert lib.yyjson_alc_pool_init(C.byref(alc), base, length)
        live = []
        for _ in range(1500):
            action = rng.randrange(3) if live else 0
            if action == 0:
                size = rng.randrange(1, 2048)
                ptrs = [a.malloc(a.ctx, size) for a in alcs]
                assert [p - b if p else None for p, b in zip(ptrs, bases)][0] == \
                       [p - b if p else None for p, b in zip(ptrs, bases)][1]
                if ptrs[0]:
                    pattern = rng.randrange(256)
                    for p in ptrs:
                        C.memset(p, pattern, size)
                    live.append((ptrs, size, pattern))
            elif action == 1:
                ptrs, _, _ = live.pop(rng.randrange(len(live)))
                for a, p in zip(alcs, ptrs):
                    a.free(a.ctx, p)
            else:
                i = rng.randrange(len(live))
                ptrs, old, pattern = live[i]
                size = old + rng.randrange(1, 2048)
                new = [a.realloc(a.ctx, p, old, size) for a, p in zip(alcs, ptrs)]
                offsets = [p - b if p else None for p, b in zip(new, bases)]
                assert offsets[0] == offsets[1], (seed, old, size, offsets)
                if new[0]:
                    for p in new:
                        assert C.string_at(p, old) == bytes([pattern]) * old
                        C.memset(p, pattern, size)
                    live[i] = (new, size, pattern)
        for ptrs, _, _ in live:
            for a, p in zip(alcs, ptrs):
                a.free(a.ctx, p)
        for lib in (original, port):
            invalid = Alc()
            assert not lib.yyjson_alc_pool_init(None, None, 0)
            assert not lib.yyjson_alc_pool_init(C.byref(invalid), None, 0)
            assert not invalid.malloc(invalid.ctx, 10)


def dyn_lists(alc):
    result = []
    for head in (alc.ctx, alc.ctx + C.sizeof(Chunk)):
        cur = Chunk.from_address(head).next
        sizes = []
        while cur:
            chunk = Chunk.from_address(cur)
            sizes.append(chunk.size)
            cur = chunk.next
        result.append(sizes)
    return result


def check_dynamic(original, port):
    for seed in range(8):
        rng = random.Random(seed)
        handles = [lib.yyjson_alc_dyn_new() for lib in (original, port)]
        assert all(handles)
        alcs = [Alc.from_address(p) for p in handles]
        live = []
        for _ in range(1000):
            action = rng.randrange(3) if live else 0
            if action == 0:
                size = rng.randrange(1, 10000)
                ptrs = [a.malloc(a.ctx, size) for a in alcs]
                assert all(ptrs)
                pattern = rng.randrange(256)
                for p in ptrs:
                    C.memset(p, pattern, size)
                live.append((ptrs, size, pattern))
            elif action == 1:
                ptrs, _, _ = live.pop(rng.randrange(len(live)))
                for a, p in zip(alcs, ptrs):
                    a.free(a.ctx, p)
            else:
                i = rng.randrange(len(live))
                ptrs, old, pattern = live[i]
                size = old + rng.randrange(1, 10000)
                new = [a.realloc(a.ctx, p, old, size) for a, p in zip(alcs, ptrs)]
                assert all(new)
                for p in new:
                    assert C.string_at(p, old) == bytes([pattern]) * old
                    C.memset(p, pattern, size)
                live[i] = (new, size, pattern)
            assert dyn_lists(alcs[0]) == dyn_lists(alcs[1])
        for a in alcs:
            assert not a.malloc(a.ctx, Z(-1).value)
        for lib, handle in zip((original, port), handles):
            lib.yyjson_alc_dyn_free(handle)
            lib.yyjson_alc_dyn_free(None)


def pool_state(pool, stride):
    remaining = ((pool.end or 0) - (pool.cur or 0)) // stride
    return pool.size, pool.max, remaining


def check_documents(original, port):
    for fail in range(12):
        results = []
        for lib in (original, port):
            tracer = Tracer(fail)
            doc = lib.yyjson_mut_doc_new(C.byref(tracer.alc))
            states = []
            if doc:
                d = MutDoc.from_address(doc)
                states.append((pool_state(d.strings, 1), pool_state(d.values, 24)))
                for count in (0, 1, 16, 100, 1024, Z(-1).value):
                    states.append(lib.yyjson_mut_doc_set_str_pool_size(doc, count))
                    states.append(lib.yyjson_mut_doc_set_val_pool_size(doc, count))
                for count in (1, 1000, 16, 2000, Z(-1).value):
                    states.append(lib.unsafe_yyjson_str_pool_grow(C.byref(d.strings), C.byref(d.alc), count))
                    states.append(lib.unsafe_yyjson_val_pool_grow(C.byref(d.values), C.byref(d.alc), count))
                    states.append((pool_state(d.strings, 1), pool_state(d.values, 24)))
                lib.yyjson_mut_doc_free(doc)
            assert not tracer.blocks
            results.append((states, tracer.events))
        assert results[0] == results[1], (fail, results)


def check_copying(original, port):
    original.yyjson_read_opts.argtypes, original.yyjson_read_opts.restype = [P, Z, C.c_uint32, P, P], P
    original.yyjson_mut_write_opts.argtypes, original.yyjson_mut_write_opts.restype = [P, C.c_uint32, P, P, P], P
    original.yyjson_write_opts.argtypes, original.yyjson_write_opts.restype = [P, C.c_uint32, P, P, P], P

    def render(doc, mutable):
        size = Z()
        fn = original.yyjson_mut_write_opts if mutable else original.yyjson_write_opts
        result = fn(doc, 0, None, C.byref(size), None)
        assert result
        data = C.string_at(result, size.value)
        libc.free(result)
        return data

    samples = [b'null', b'""', b'[]', b'{}', b'{"a":[],"b":{},"a":[1,2.0,-3,true,"\\u0000"]}',
               b'["abc",18446744073709551615,-9223372036854775808,3.141592653589793]',
               b'[' * 100 + b'"nested"' + b']' * 100,
               b'[' + b','.join([b'{"k":"string","v":[1,2,3]}'] * 100) + b']']
    for data in samples:
        src = original.yyjson_read_opts(data, len(data), 0, None, None)
        assert src
        expected = render(src, False)
        for fail in range(12):
            results = []
            for lib in (original, port):
                tracer = Tracer(fail)
                dst = lib.yyjson_doc_mut_copy(src, C.byref(tracer.alc))
                states = [bool(dst)]
                if dst:
                    states.append(render(dst, True))
                    imut = lib.yyjson_mut_doc_imut_copy(dst, C.byref(tracer.alc))
                    states.append(bool(imut))
                    if imut:
                        d = Doc.from_address(imut)
                        states.append((render(imut, False), d.dat_read, d.val_read))
                        free_doc(imut)
                    other = lib.yyjson_mut_doc_mut_copy(dst, C.byref(tracer.alc))
                    states.append(bool(other))
                    if other:
                        states.append(render(other, True))
                        lib.yyjson_mut_doc_free(other)
                    lib.yyjson_mut_doc_free(dst)
                assert not tracer.blocks
                results.append((states, tracer.events))
            assert results[0] == results[1], (data, fail, results)
            if results[1][0][0]:
                assert results[1][0][1] == expected
        free_doc(src)


def check_equality(original, port):
    # Include duplicate keys, order changes and signed zero. yyjson compares
    # doubles by their bit patterns, and object matching follows its iterator.
    samples = [b'null', b'true', b'false', b'0', b'-0', b'0.0', b'-0.0', b'1', b'1.0',
               b'"abc"', b'"ab"', b'[]', b'{}', b'[1,2]', b'[2,1]',
               b'{"a":1,"b":2}', b'{"b":2,"a":1}',
               b'{"a":1,"a":2}', b'{"a":2,"a":1}',
               b'{"a":1,"a":1}', b'{"a":1,"b":1}',
               b'{"a":[1,2],"b":{}}', b'{"b":{},"a":[1,2]}']
    rng = random.Random(99)
    for _ in range(50):
        pairs = [b'"' + rng.choice((b'a', b'b', b'c')) + b'":' + rng.choice((b'1', b'2', b'[]', b'{}', b'null'))
                 for _ in range(rng.randrange(1, 8))]
        samples.append(b'{' + b','.join(pairs) + b'}')
    imut, mut = [], []
    for data in samples:
        doc = original.yyjson_read_opts(data, len(data), 0, None, None)
        assert doc
        imut.append(doc)
        mut.append(original.yyjson_doc_mut_copy(doc, None))
    for lhs in range(len(samples)):
        for rhs in range(len(samples)):
            a, b = Doc.from_address(imut[lhs]).root, Doc.from_address(imut[rhs]).root
            assert original.unsafe_yyjson_equals(a, b) == port.unsafe_yyjson_equals(a, b), (samples[lhs], samples[rhs])
            a, b = MutDoc.from_address(mut[lhs]).root, MutDoc.from_address(mut[rhs]).root
            assert original.unsafe_yyjson_mut_equals(a, b) == port.unsafe_yyjson_mut_equals(a, b), (samples[lhs], samples[rhs])
    for tag_a in (4, 12, 20):
        for tag_b in (4, 12, 20):
            for bits_a in (0, 1, 1 << 63, (1 << 64) - 1, 0x7ff8000000000000):
                for bits_b in (0, 1, 1 << 63, (1 << 64) - 1, 0x7ff8000000000000):
                    a, b = MutVal(tag_a, bits_a, None), MutVal(tag_b, bits_b, None)
                    assert original.unsafe_yyjson_equals(C.byref(a), C.byref(b)) == port.unsafe_yyjson_equals(C.byref(a), C.byref(b))
    for doc in mut:
        original.yyjson_mut_doc_free(doc)
    for doc in imut:
        free_doc(doc)


if __name__ == "__main__":
    original, port = map(setup, sys.argv[1:])
    assert original.yyjson_version() == port.yyjson_version()
    for check in (check_positions, check_fixed, check_dynamic, check_documents, check_copying, check_equality):
        check(original, port)
        print(check.__name__ + ": passed", flush=True)
