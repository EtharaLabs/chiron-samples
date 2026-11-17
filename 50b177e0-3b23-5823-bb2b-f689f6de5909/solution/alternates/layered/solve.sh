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
cp -- "$payload/build.rs" "$repo/build.rs"
mkdir -p -- "$repo/cmake"
cp -- "$payload/cmake/yyjson-config-release.cmake.in" "$repo/cmake/yyjson-config-release.cmake.in"
cp -- "$payload/cmake/yyjson-config.cmake.in" "$repo/cmake/yyjson-config.cmake.in"
mkdir -p -- "$repo/src/emit"
cp -- "$payload/src/emit/document.rs" "$repo/src/emit/document.rs"
cp -- "$payload/src/emit/mod.rs" "$repo/src/emit/mod.rs"
cp -- "$payload/src/emit/number.rs" "$repo/src/emit/number.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/lib.rs" "$repo/src/lib.rs"
mkdir -p -- "$repo/src/model"
cp -- "$payload/src/model/header.rs" "$repo/src/model/header.rs"
cp -- "$payload/src/model/layout.rs" "$repo/src/model/layout.rs"
cp -- "$payload/src/model/mod.rs" "$repo/src/model/mod.rs"
cp -- "$payload/src/model/value.rs" "$repo/src/model/value.rs"
mkdir -p -- "$repo/src/parse"
cp -- "$payload/src/parse/document.rs" "$repo/src/parse/document.rs"
cp -- "$payload/src/parse/incremental.rs" "$repo/src/parse/incremental.rs"
cp -- "$payload/src/parse/mod.rs" "$repo/src/parse/mod.rs"
cp -- "$payload/src/parse/number.rs" "$repo/src/parse/number.rs"
mkdir -p -- "$repo/src/query"
cp -- "$payload/src/query/mod.rs" "$repo/src/query/mod.rs"
cp -- "$payload/src/query/patch.rs" "$repo/src/query/patch.rs"
cp -- "$payload/src/query/pointer.rs" "$repo/src/query/pointer.rs"
mkdir -p -- "$repo/src/support"
cp -- "$payload/src/support/arena.rs" "$repo/src/support/arena.rs"
cp -- "$payload/src/support/mod.rs" "$repo/src/support/mod.rs"
cp -- "$payload/src/support/octets.rs" "$repo/src/support/octets.rs"
cp -- "$payload/src/support/tables.rs" "$repo/src/support/tables.rs"
cp -- "$payload/src/support/utf8_seq.rs" "$repo/src/support/utf8_seq.rs"
mkdir -p -- "$repo/src/sys"
cp -- "$payload/src/sys/abi.rs" "$repo/src/sys/abi.rs"
cp -- "$payload/src/sys/mod.rs" "$repo/src/sys/mod.rs"

printf "solve: migrated %s with the layered alternate\n" "$repo"
