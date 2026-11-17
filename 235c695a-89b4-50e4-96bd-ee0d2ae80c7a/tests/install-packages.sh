#!/bin/bash
# Run once by the Dockerfile next to it, with the bootstrap trust bundle at /run/bootstrap-ca.crt.
set -euo pipefail
packages=(
  ca-certificates=20260816ubuntu0.24.04.1
  gawk=1:5.2.1-2ubuntu0.1
  git=1:2.43.0-1ubuntu7.4
  make=4.3-4.1build2
  python3=3.12.3-0ubuntu2.1
  python3-dev=3.12.3-0ubuntu2.1
  python3-pip=24.0+dfsg-1ubuntu1.3
  python3-venv=3.12.3-0ubuntu2.1
  python3-click=8.1.6-2
  python3-yaml=6.0.1-2build2
  bison=2:3.8.2+dfsg-1ubuntu0.24.04.1
  flex=2.6.4-8.2build1
  clang=1:18.0-59~exp2
  gcc=4:13.2.0-7ubuntu1
  g++=4:13.2.0-7ubuntu1
  libffi-dev=3.4.6-1build1
  libfl-dev=2.6.4-8.2build1
  libreadline-dev=8.2-4build1
  pkg-config=1.8.1-2build1
  tcl-dev=8.6.14build1
  zlib1g-dev=1:1.3.dfsg-3.1ubuntu2.3
  libgtest-dev=1.14.0-1
  libgmock-dev=1.14.0-1
  cmake=3.28.3-1build7
  ninja-build=1.11.1-2
  clang-14=1:14.0.6-19build4
  gcc-11=11.5.0-1ubuntu1~24.04.1
  g++-11=11.5.0-1ubuntu1~24.04.1
  g++-12=12.4.0-2ubuntu1~24.04.1
  libclang-rt-18-dev=1:18.1.3-1ubuntu1
  llvm-18=1:18.1.3-1ubuntu1
  iverilog=12.0-2build2
  gtkwave=3.3.116-1build2
  graphviz=2.42.2-9ubuntu0.1
  binutils=2.42-4ubuntu2.10
  curl=8.5.0-2ubuntu10.16
  xz-utils=5.6.1+really5.4.5-1ubuntu0.4
  zip=3.0-13ubuntu0.2
  unzip=6.0-28ubuntu4.1
  file=1:5.45-3build1
  procps=2:4.0.4-4ubuntu3.3
  time=1.9-0.2build1
)
apt=(apt-get -o Acquire::https::CaInfo=/run/bootstrap-ca.crt -o Acquire::Retries=20 -o APT::Sandbox::User=root)
rm -f /etc/apt/sources.list.d/ubuntu.sources
printf 'Types: deb\nURIs: https://snapshot.ubuntu.com/ubuntu/%s\nSuites: noble noble-updates noble-security\nComponents: main universe\nSigned-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n' "$SNAPSHOT" \
  > /etc/apt/sources.list.d/snapshot.sources
mv /etc/apt/apt.conf.d/docker-clean /tmp/docker-clean.saved
install_packages() {
  "${apt[@]}" update \
    && "${apt[@]}" install -y --no-install-recommends --print-uris -qq "${packages[@]}" \
    | sed -n "s/^'\([^']*\)' \([^ ]*\) [0-9]* \([A-Za-z0-9]*:[0-9a-f]*\)\$/\1 \2 \3/p" \
    | xargs -r -n 3 -P 8 sh -c '/usr/lib/apt/apt-helper -o Acquire::https::CaInfo=/run/bootstrap-ca.crt -o APT::Sandbox::User=root -o Acquire::Retries=20 download-file "$0" "/var/cache/apt/archives/$1" "$2" > /dev/null' \
    && DEBIAN_FRONTEND=noninteractive "${apt[@]}" install -y --no-install-recommends "${packages[@]}"
}
for attempt in 1 2 3 4 5; do
  install_packages && break
  [ "$attempt" -lt 5 ] || exit 1
  sleep 60
done
mv /tmp/docker-clean.saved /etc/apt/apt.conf.d/docker-clean
rm -rf /var/lib/apt/lists/*
rm -f /var/cache/apt/archives/*.deb
rm -rf /var/cache/apt/archives/partial/*
