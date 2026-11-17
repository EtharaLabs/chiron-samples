#!/bin/sh
# CANARY-BEGIN
# CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
# CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
# CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
# CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
# CANARY-END
# Build the Rust library for the CMake rule that depends on its artifact.
#
# The rule delegates here rather than inlining `cmake -E env`, so the whole cargo
# environment lives in one file that can be read and run on its own. Every value the build
# depends on arrives as an argument; nothing is read from the ambient environment, so the
# build does not change behaviour with the caller's shell.
set -eu

cargo=${1:?cargo executable}
manifest=${2:?path to Cargo.toml}
target_dir=${3:?cargo target directory}
soname=${4:?soname for the shared library}

CARGO_TARGET_DIR="$target_dir"
CARGO_HOME="$target_dir/home"
CARGO_NET_OFFLINE=true
# rustc sets no SONAME for a cdylib, so the one link argument the C consumers
# depend on is supplied here.
RUSTFLAGS="-C link-arg=-Wl,-soname,$soname"
export CARGO_TARGET_DIR CARGO_HOME CARGO_NET_OFFLINE RUSTFLAGS

exec "$cargo" build --release --offline --locked --manifest-path "$manifest"
