#!/bin/bash
# CANARY-BEGIN
# CANARY-SLOT-1: 34f0b97a4489d4563e45a8fa359f8255e984bbb35d3f62fc57865d6a1254ec74
# CANARY-SLOT-2: e58cc12e370dd698e69912df2f06f1d37184f6b5b1f00c775976cb520f7ef719
# CANARY-END
# Applies the reference migration to the repository the task works in.
set -euo pipefail
patch=/solution/reference.patch
expected=926f3ec14332c1f0916510636c03b9f7e2ef520aab992c725f7552e5b26c1400
actual="$(LC_ALL=C sed -n '/^diff --git /,$p' "$patch" | sha256sum | cut -d' ' -f1)"
if [ "$actual" != "$expected" ]; then
  echo "reference patch body does not match its recorded digest" >&2
  exit 1
fi
cd /app
git apply --binary --whitespace=nowarn "$patch"
