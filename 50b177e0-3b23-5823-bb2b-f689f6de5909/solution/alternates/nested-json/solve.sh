#!/bin/sh
# CANARY-BEGIN
# CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
# CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
# CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
# CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
# CANARY-END
# Turn State A into an alternate State B inside the agent's workspace: drop the C
# implementation, then lay this alternate's payload over the tree. The payload carries the
# Rust crate, the rewritten CMake build, the package-configuration templates and the design
# note, and differs from the reference solution in identifier names, file layout, build
# integration and every diagnostic the grader does not require.
set -eu

repo="${1:-/workspace/repo}"
payload="$(CDPATH= cd -- "$(dirname -- "$0")/port" && pwd)"

test -f "$repo/src/yyjson.h" || { echo "solve: $repo is not the task repository" >&2; exit 1; }

rm -f -- "$repo/src/yyjson.c"
# Every payload path, generated from the payload itself.
cp -- "$payload/CMakeLists.txt" "$repo/CMakeLists.txt"
cp -- "$payload/Cargo.lock" "$repo/Cargo.lock"
cp -- "$payload/Cargo.toml" "$repo/Cargo.toml"
mkdir -p -- "$repo/cmake"
cp -- "$payload/cmake/run-cargo.sh" "$repo/cmake/run-cargo.sh"
cp -- "$payload/cmake/yyjson-config-release.cmake.in" "$repo/cmake/yyjson-config-release.cmake.in"
cp -- "$payload/cmake/yyjson-config.cmake.in" "$repo/cmake/yyjson-config.cmake.in"
mkdir -p -- "$repo/src/base"
cp -- "$payload/src/base/abi.rs" "$repo/src/base/abi.rs"
cp -- "$payload/src/base/heap.rs" "$repo/src/base/heap.rs"
cp -- "$payload/src/base/inline.rs" "$repo/src/base/inline.rs"
cp -- "$payload/src/base/mod.rs" "$repo/src/base/mod.rs"
cp -- "$payload/src/base/node.rs" "$repo/src/base/node.rs"
cp -- "$payload/src/base/octets.rs" "$repo/src/base/octets.rs"
cp -- "$payload/src/base/repr.rs" "$repo/src/base/repr.rs"
cp -- "$payload/src/base/tables.rs" "$repo/src/base/tables.rs"
cp -- "$payload/src/base/utf8.rs" "$repo/src/base/utf8.rs"
mkdir -p -- "$repo/src/json"
cp -- "$payload/src/json/mod.rs" "$repo/src/json/mod.rs"
cp -- "$payload/src/json/patch.rs" "$repo/src/json/patch.rs"
cp -- "$payload/src/json/ptr.rs" "$repo/src/json/ptr.rs"
mkdir -p -- "$repo/src/json/read"
cp -- "$payload/src/json/read/chunk.rs" "$repo/src/json/read/chunk.rs"
cp -- "$payload/src/json/read/doc.rs" "$repo/src/json/read/doc.rs"
cp -- "$payload/src/json/read/mod.rs" "$repo/src/json/read/mod.rs"
cp -- "$payload/src/json/read/num.rs" "$repo/src/json/read/num.rs"
mkdir -p -- "$repo/src/json/write"
cp -- "$payload/src/json/write/doc.rs" "$repo/src/json/write/doc.rs"
cp -- "$payload/src/json/write/mod.rs" "$repo/src/json/write/mod.rs"
cp -- "$payload/src/json/write/num.rs" "$repo/src/json/write/num.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/lib.rs" "$repo/src/lib.rs"
chmod 0755 -- "$repo/cmake/run-cargo.sh"

printf "solve: migrated %s with the nested-json alternate\n" "$repo"
