#!/bin/bash
# The oracle: apply the reference migration to the State A tree at /app.
# CANARY-BEGIN
# CANARY-SLOT-1: b4feaf386e9a333ecede5214ecae6932cb90beefaad959c16f1f021adf37e2ee
# CANARY-SLOT-2: 6c60abfcec2fb775da6dd799e22b496bfa4cbb3e32ae97d77d667ae2710d5e2b
# CANARY-SLOT-3: 8a478e777b2c9a466dc8145c95cb275027faf03e1a5cbe966e8fb637c1d81bae
# CANARY-END
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# The patch digest is bound under the canary normalization (slot values read as zeros).
actual=$(sed -E 's/^(# CANARY-SLOT-[0-9]+: )[0-9a-f]{64}$/\10000000000000000000000000000000000000000000000000000000000000000/' "$here/reference.patch" | sha256sum | cut -d' ' -f1)
test "$actual" = "b1ecea98a263e8b24e579631050df62968ebc797377365ef623f42df74072576"
cd /app
git apply --whitespace=nowarn "$here/reference.patch"
