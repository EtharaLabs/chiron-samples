#!/bin/sh
# CANARY-BEGIN
# CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
# CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
# CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
# CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
# CANARY-END
#
# Turn State A into State B inside the agent's workspace: drop the C implementation,
# then lay the migration payload over the tree. The payload carries the Rust crate, the
# rewritten CMake build and the package-configuration templates.
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
