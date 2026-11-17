#!/bin/bash
# fetch.sh <target-arch> <build-arch> <out> : download every pinned file named in downloads.txt
# for the target architecture (and the build architecture's JDK 11 for building State A), check
# each against its sha256, and name it by role.
set -euo pipefail
target="$1"; build="$2"; out="$3"; here="$(dirname "$0")"
mkdir -p "$out"
get() { # name arch dest
  local line
  line=$(awk -v n="$1" -v a="$2" '$1==n && ($2==a || $2=="all") {print $3, $4}' "$here/downloads.txt")
  [ -n "$line" ] || { echo "no pinned download for $1/$2" >&2; exit 1; }
  set -- $line "$3"
  curl -fsSL --retry 5 --retry-delay 3 --retry-all-errors -o "$3" "$2"
  echo "$1  $3" | sha256sum -c --quiet
}
get jdk11 "$target" "$out/jdk11-target.tar.gz"
get jdk17 "$target" "$out/jdk17-target.tar.gz"
get jdk11 "$build" "$out/jdk11-build.tar.gz"
get maven all "$out/maven.tar.gz"
get wildfly all "$out/wildfly.tar.gz"
get libfaketime all "$out/libfaketime.tar.gz"
