#!/bin/sh
# fetch-m2.sh <manifest> <local-repository>: download every artifact a sha256 manifest names from
# Maven Central into a Maven local repository layout, and verify every digest.
set -eu
manifest="$1"; repo="$2"; base="${MAVEN_CENTRAL:-https://repo1.maven.org/maven2}"
mkdir -p "$repo"
awk '{print $2}' "$manifest" | xargs -P 8 -I{} sh -c 'mkdir -p "$(dirname "$1/$2")" && curl -fsSL --retry 5 --retry-delay 2 --retry-all-errors -o "$1/$2" "$0/$2"' "$base" "$repo" {}
cd "$repo" && sha256sum --quiet -c "$manifest"
echo "fetch-m2: $(wc -l < "$manifest") artifacts verified into $repo"
