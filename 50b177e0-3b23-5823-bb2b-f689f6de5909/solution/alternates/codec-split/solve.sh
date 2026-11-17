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
mkdir -p -- "$repo/.cargo"
cp -- "$payload/.cargo/config.toml" "$repo/.cargo/config.toml"
cp -- "$payload/CMakeLists.txt" "$repo/CMakeLists.txt"
cp -- "$payload/Cargo.lock" "$repo/Cargo.lock"
cp -- "$payload/Cargo.toml" "$repo/Cargo.toml"
mkdir -p -- "$repo/cmake"
cp -- "$payload/cmake/yyjson-config-release.cmake.in" "$repo/cmake/yyjson-config-release.cmake.in"
cp -- "$payload/cmake/yyjson-config.cmake.in" "$repo/cmake/yyjson-config.cmake.in"
mkdir -p -- "$repo/src/abi"
cp -- "$payload/src/abi/libc.rs" "$repo/src/abi/libc.rs"
cp -- "$payload/src/abi/mod.rs" "$repo/src/abi/mod.rs"
mkdir -p -- "$repo/src/decode"
cp -- "$payload/src/decode/mod.rs" "$repo/src/decode/mod.rs"
cp -- "$payload/src/decode/numeric.rs" "$repo/src/decode/numeric.rs"
cp -- "$payload/src/decode/parser.rs" "$repo/src/decode/parser.rs"
cp -- "$payload/src/decode/streaming.rs" "$repo/src/decode/streaming.rs"
mkdir -p -- "$repo/src/encode"
cp -- "$payload/src/encode/mod.rs" "$repo/src/encode/mod.rs"
cp -- "$payload/src/encode/numeric.rs" "$repo/src/encode/numeric.rs"
cp -- "$payload/src/encode/printer.rs" "$repo/src/encode/printer.rs"
mkdir -p -- "$repo/src"
cp -- "$payload/src/lib.rs" "$repo/src/lib.rs"
mkdir -p -- "$repo/src/locate"
cp -- "$payload/src/locate/merge.rs" "$repo/src/locate/merge.rs"
cp -- "$payload/src/locate/mod.rs" "$repo/src/locate/mod.rs"
cp -- "$payload/src/locate/pointer.rs" "$repo/src/locate/pointer.rs"
mkdir -p -- "$repo/src/mem"
cp -- "$payload/src/mem/bytes.rs" "$repo/src/mem/bytes.rs"
cp -- "$payload/src/mem/mod.rs" "$repo/src/mem/mod.rs"
cp -- "$payload/src/mem/pools.rs" "$repo/src/mem/pools.rs"
cp -- "$payload/src/mem/tables.rs" "$repo/src/mem/tables.rs"
cp -- "$payload/src/mem/unicode.rs" "$repo/src/mem/unicode.rs"
mkdir -p -- "$repo/src/repr"
cp -- "$payload/src/repr/inline.rs" "$repo/src/repr/inline.rs"
cp -- "$payload/src/repr/mod.rs" "$repo/src/repr/mod.rs"
cp -- "$payload/src/repr/node.rs" "$repo/src/repr/node.rs"
cp -- "$payload/src/repr/shape.rs" "$repo/src/repr/shape.rs"

printf "solve: migrated %s with the codec-split alternate\n" "$repo"
