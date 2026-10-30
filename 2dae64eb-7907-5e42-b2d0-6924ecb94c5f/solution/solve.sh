#!/bin/sh
# GENERATED SECTION. DO NOT HAND-EDIT.
# Derived from solution/grounding.yaml by solution/recompute.py.
# FORGE-CANARY-BEGIN
# FORGE-CANARY-SLOT-1: 60ae0f0c079c366733615a2e6551dc4e64c530f1c56ffed9db295862b101ddc8
# FORGE-CANARY-SLOT-2: 87ab65e4ae3bd13b9cfb24e8b811c4ab101c8552a8b3d1f0d91c32bbd429958a
# FORGE-CANARY-SLOT-3: 76135f235d13f30c4446a51ada3f9a19b6fba2e250fa6b2ffc5a14d1b3f55ffb
# FORGE-CANARY-SLOT-4: e415e80ce167e9922758ef5d43162679b0fe51046883994e0fc30102c1e183f5
# FORGE-CANARY-END
#
# Turn State A into State B inside the agent's workspace: drop the C implementation,
# then lay the migration payload over the tree. The payload carries the Rust crate, the
# rewritten CMake build, the package-configuration templates and the design note.
set -eu

repo="${1:-/workspace/repo}"
payload="$(CDPATH= cd -- "$(dirname -- "$0")/../solution/port" && pwd)"

test -f "$repo/src/yyjson.h" || { echo "solve: $repo is not the task repository" >&2; exit 1; }

rm -f -- "$repo/src/yyjson.c"

# Every payload path, in the order the grounding record lists it.
mkdir -p -- "$repo/.cargo"
cp -- "$payload/.cargo/config.toml" "$repo/.cargo/config.toml"
cp -- "$payload/CMakeLists.txt" "$repo/CMakeLists.txt"
cp -- "$payload/Cargo.lock" "$repo/Cargo.lock"
cp -- "$payload/Cargo.toml" "$repo/Cargo.toml"
mkdir -p -- "$repo/cmake"
cp -- "$payload/cmake/yyjson-config-release.cmake.in" "$repo/cmake/yyjson-config-release.cmake.in"
mkdir -p -- "$repo/cmake"
cp -- "$payload/cmake/yyjson-config.cmake.in" "$repo/cmake/yyjson-config.cmake.in"
mkdir -p -- "$repo/doc"
cp -- "$payload/doc/rust-port.md" "$repo/doc/rust-port.md"
mkdir -p -- "$repo/src"
cp -- "$payload/src/alloc.rs" "$repo/src/alloc.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/ffi.rs" "$repo/src/ffi.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/hdr.rs" "$repo/src/hdr.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/incr.rs" "$repo/src/incr.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/lib.rs" "$repo/src/lib.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/patch.rs" "$repo/src/patch.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/pointer.rs" "$repo/src/pointer.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/read_num.rs" "$repo/src/read_num.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/reader.rs" "$repo/src/reader.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/tables.rs" "$repo/src/tables.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/types.rs" "$repo/src/types.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/utf8_seq.rs" "$repo/src/utf8_seq.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/util.rs" "$repo/src/util.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/value.rs" "$repo/src/value.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/write_num.rs" "$repo/src/write_num.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/writer.rs" "$repo/src/writer.rs"

printf "solve: migrated %s\n" "$repo"
