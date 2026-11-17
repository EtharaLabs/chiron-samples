#!/usr/bin/env bash
# CANARY-BEGIN
# CANARY-SLOT-1: 0c8c97365afb043f2d762b9d91f4bb18bcb2c5cc5004f12403b20a743b6ba66d
# CANARY-SLOT-2: 126abb96c874a445ebea61d6e95f91a187ca0a083395b6f67bb293dc0aef4971
# CANARY-SLOT-3: 754d95d162eac3d733a4ce142318e4aafe4b31572d8d9531ac47b6a381929a50
# CANARY-END
# Reference solution: the upstream migration (scylladb/argus pull request #1059, landed at
# 7e0b152a), with the repairs disclosed in TRUTH.md, applied to /app as one patch.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd /app
git apply --whitespace=nowarn --binary "$here/reference.patch"
