#!/bin/bash
# install.sh <set>: install the UBI packages one build stage needs (set: fetch, compile or runtime)
# from the RPM files beside this script. microdnf resolved each set against the pinned base image on
# 2026-11-13; shipping the files keeps the build off the live UBI repositories, which drop older
# builds. Every file is checked against SHA256SUMS before rpm sees it.
set -euo pipefail
cd "$(dirname "$0")"
set_dir="$(uname -m)/$1"
grep -F "  $set_dir/" SHA256SUMS > "/tmp/rpms-$1.sha256"
sha256sum --quiet -c "/tmp/rpms-$1.sha256"
test "$(find "$set_dir" -name '*.rpm' | wc -l)" -eq "$(wc -l < "/tmp/rpms-$1.sha256")"
rpm -U --excludedocs "$set_dir"/*.rpm
rm -f "/tmp/rpms-$1.sha256"
