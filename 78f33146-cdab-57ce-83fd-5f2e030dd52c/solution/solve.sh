#!/bin/bash
# Reference solution: upstream datalab PR #1820 (State A d153f80a to 521e0fb8) with the repairs listed in
# solution/TRUTH.md that restore State A's answers, applied to /app/pydatalab.
set -euo pipefail
cd /app/pydatalab
git apply --whitespace=nowarn "$(dirname "$0")/reference.patch"
