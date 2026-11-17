#!/bin/bash
# apt-snapshot.sh <snapshot-id>: point apt at the dated Ubuntu snapshot.
# The snapshot service answers only over https and the base image carries no CA bundle, so the
# bundle is first installed from the snapshot itself with peer verification off (the archive's
# signed Release files still authenticate every package); apt then checks https peers as usual.
set -euo pipefail
snapshot="$1"
cat > /etc/apt/sources.list.d/ubuntu.sources <<SRC
Types: deb
URIs: https://snapshot.ubuntu.com/ubuntu/${snapshot}/
Suites: noble noble-updates noble-security
Components: main universe
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
SRC
apt-get -o Acquire::Retries=5 -o Acquire::https::Verify-Peer=false update
apt-get -o Acquire::Retries=5 -o Acquire::https::Verify-Peer=false install -y --no-install-recommends \
  ca-certificates=20260816ubuntu0.24.04.1
apt-get -o Acquire::Retries=5 update
