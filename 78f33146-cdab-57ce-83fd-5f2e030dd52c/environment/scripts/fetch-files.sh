#!/bin/bash
# Download every file a manifest names (filename url sha256 [upload-time]) and verify its sha256.
set -euo pipefail
manifest=$1 out=$2
mkdir -p "$out"
while read -r name url sha _; do
  [ -n "$name" ] || continue
  curl -fsSL --retry 5 --retry-all-errors -o "$out/$name" "$url"
  printf '%s  %s\n' "$sha" "$out/$name" | sha256sum -c --quiet -
done < "$manifest"
