# syntax=docker.io/docker/dockerfile:1.19@sha256:b6afd42430b15f2d2a4c5a02b919e98a525b785b1aaff16747d2f623364e39b6
# Agent sandbox: State A of datalab (pydantic 1) to migrate offline to the pydantic 2 target set.
ARG APT_SNAPSHOT=20261101T000000Z

FROM --platform=$BUILDPLATFORM docker.io/library/ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS fetch
ARG APT_SNAPSHOT
ARG TARGETOS
ARG TARGETARCH
ARG BUILDARCH
SHELL ["/bin/bash", "-euo", "pipefail", "-c"]
ENV DEBIAN_FRONTEND=noninteractive
RUN rm -f /etc/apt/sources.list.d/ubuntu.sources \
 && printf 'Types: deb\nURIs: http://snapshot.ubuntu.com/ubuntu/%s\nSuites: noble noble-updates noble-security\nComponents: main universe\nSigned-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n' "$APT_SNAPSHOT" > /etc/apt/sources.list.d/snapshot.sources \
 && for attempt in 1 2 3 4 5; do \
      apt-get -o Acquire::Retries=5 -o Acquire::https::Verify-Peer=false update -q \
      && apt-get install -o Acquire::Retries=5 -o Acquire::https::Verify-Peer=false -y -q --no-install-recommends ca-certificates=20260816ubuntu0.24.04.1 \
      && apt-get -o Acquire::Retries=5 update -q \
      && apt-get install -o Acquire::Retries=5 -y -q --no-install-recommends curl=8.5.0-2ubuntu10.16 git=1:2.43.0-1ubuntu7.4 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/*
COPY --chown=0:0 manifests/ /build/manifests/
COPY --chown=0:0 scripts/ /build/scripts/

# Toolchains: one sha256 per architecture for the build host's tools and the target's runtime.
RUN arch() { case $1 in arm64) echo aarch64;; amd64) echo x86_64;; *) echo "unsupported $1" >&2; return 1;; esac; } \
 && py_sha() { case $1 in aarch64) echo ecf28575cd67fa7a53c7bf2355bae5cff2c496efeb51780d50a8b54441cdce74;; x86_64) echo 17c4978a36c496dc53c08899ef8dad1ad2734f6325f08d6033a1e66419e0af26;; esac; } \
 && uv_sha() { case $1 in aarch64) echo 6aadf3c71600d594e16dabf382cc15282ead4c5ca768599b6bcb43c5004d9aa8;; x86_64) echo 8b3762374972daa7a74bbc6896cc73229ca69a07403dd9f9ea3805a51ffd7582;; esac; } \
 && mongo_sha() { case $1 in aarch64) echo 52655dd90adf0dbcbb87011075233285f44e262048ea30a9a48ad2a834c67736;; x86_64) echo 3e55cdcf7013fbc805d29d8195230df4b23c7688733ee15ad58ac6bc2a416611;; esac; } \
 && get() { curl -fsSL --retry 5 --retry-all-errors -o "$1" "$2" && printf '%s  %s\n' "$3" "$1" | sha256sum -c --quiet -; } \
 && py() { mkdir -p "$2" && get /tmp/py.tgz "https://github.com/astral-sh/python-build-standalone/releases/download/20261026/cpython-3.11.18%2B20261026-$1-unknown-linux-gnu-install_only_stripped.tar.gz" "$(py_sha "$1")" && tar -xzf /tmp/py.tgz -C "$2" --strip-components=1 && rm /tmp/py.tgz; } \
 && uvx() { mkdir -p "$2" && get /tmp/uv.tgz "https://github.com/astral-sh/uv/releases/download/0.9.30/uv-$1-unknown-linux-gnu.tar.gz" "$(uv_sha "$1")" && tar -xzf /tmp/uv.tgz -C "$2" --strip-components=1 && rm /tmp/uv.tgz; } \
 && host=$(dpkg --print-architecture) \
 && build=$(arch "${BUILDARCH:-$host}") && target=$(arch "${TARGETARCH:-$host}") \
 && test "${TARGETOS:-linux}" = linux \
 && py "$build" /build/host/python && uvx "$build" /build/host/bin \
 && py "$target" /out/opt/python && uvx "$target" /out/usr/local/bin \
 && get /tmp/mongodb.tgz "https://fastdl.mongodb.org/linux/mongodb-linux-$target-ubuntu2404-8.0.34.tgz" "$(mongo_sha "$target")" \
 && mkdir -p /out/opt/mongodb && tar -xzf /tmp/mongodb.tgz -C /out/opt/mongodb --strip-components=1 && rm /tmp/mongodb.tgz \
 && echo "$target" > /build/target-arch

# Target set: the only packages installable offline, with a file:// simple index and a listing.
RUN TARGETARCH="${TARGETARCH:-$(dpkg --print-architecture)}" \
 && /build/scripts/fetch-files.sh "/build/manifests/target-$TARGETARCH.txt" /out/opt/wheelhouse \
 && /build/host/python/bin/python3 /build/scripts/mkindex.py "/build/manifests/target-$TARGETARCH.txt" /out/opt/wheelhouse/simple ../.. \
 && /build/host/python/bin/python3 /build/scripts/listing.py "/build/manifests/target-$TARGETARCH.txt" /out/opt/wheelhouse/LISTING.txt \
 && /build/host/python/bin/python3 /build/scripts/mksrc.py /out/opt/wheelhouse /out/opt/pydantic-src \
      pydantic-2.13.4-py3-none-any.whl pydantic_settings-2.15.0-py3-none-any.whl \
      "$(basename /out/opt/wheelhouse/pydantic_core-2.46.4-*.whl)"

# State A, fetched by commit and checked by its pydatalab tree id, as a git repository with no later history.
# Its five links to files outside pydatalab/ are dropped: they dangle here, and some Docker stacks refuse to copy them out.
RUN retry() { for i in 1 2 3 4 5; do "$@" && return 0; if [ "$i" -lt 5 ]; then sleep $((i * 10)); fi; done; return 1; } \
 && git init -q /build/datalab \
 && retry git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=60 -C /build/datalab fetch -q --depth 1 https://github.com/datalab-org/datalab.git d153f80a71b411c074896f6559c775a91c531e4a \
 && test "$(git -C /build/datalab rev-parse FETCH_HEAD:pydatalab)" = 1e48065d59fc9a62706cc46c1aa2de80e7e8bab1 \
 && mkdir -p /build/src /out/app /out/opt/oracle /out/usr/share/doc/datalab \
 && git -C /build/datalab archive FETCH_HEAD pydatalab LICENSE | tar -x -C /build/src \
 && cp /build/src/LICENSE /out/usr/share/doc/datalab/LICENSE \
 && cp -a /build/src/pydatalab /out/app/pydatalab \
 && cp -a /build/src/pydatalab/example_data /out/opt/oracle/example_data \
 && git -C /out/app/pydatalab init -q -b main \
 && git -C /out/app/pydatalab add -A -f \
 && test "$(git -C /out/app/pydatalab write-tree)" = 1e48065d59fc9a62706cc46c1aa2de80e7e8bab1 \
 && git -C /out/app/pydatalab rm -q -f README.md docs/CHANGELOG.md docs/CODE_OF_CONDUCT.md docs/CONTRIBUTING.md docs/LICENSE \
 && GIT_AUTHOR_NAME=datalab GIT_AUTHOR_EMAIL=state-a@datalab.invalid GIT_COMMITTER_NAME=datalab GIT_COMMITTER_EMAIL=state-a@datalab.invalid \
    GIT_AUTHOR_DATE=@1787835116 GIT_COMMITTER_DATE=@1787835116 \
    git -C /out/app/pydatalab commit -q -m "State A: datalab-org/datalab d153f80a71b411c074896f6559c775a91c531e4a, pydatalab/" \
 && git -C /out/app/pydatalab gc -q --prune=now

# The two git-sourced plugins of the app-plugins-git extra, mirrored at their locked revisions so uv lock runs offline.
RUN retry() { for i in 1 2 3 4 5; do "$@" && return 0; if [ "$i" -lt 5 ]; then sleep $((i * 10)); fi; done; return 1; } \
 && mirror() { git init -q --bare "/out/opt/git-mirrors/$1.git" && retry git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=60 -C "/out/opt/git-mirrors/$1.git" fetch -q "$2" "refs/tags/$3:refs/tags/$3" && test "$(git -C "/out/opt/git-mirrors/$1.git" rev-parse "$3^{commit}")" = "$4"; } \
 && mirror datalab-app-plugin-insitu https://github.com/datalab-org/datalab-app-plugin-insitu.git v0.4.1 853840a211a3891cf900dd40d2ce0283366adad1 \
 && mirror datalab-app-plugin-xps https://github.com/datalab-industries/datalab-app-plugin-xps.git v0.2.0 57a719f6da2eba406cd5ddb2acb1b50c46bf77f0

# The oracle: State A installed on its own retired set (pydantic 1.10.26), cross-installed for the target.
RUN target=$(cat /build/target-arch) && TARGETARCH="${TARGETARCH:-$(dpkg --print-architecture)}" \
 && /build/scripts/fetch-files.sh "/build/manifests/oracle-$TARGETARCH.txt" /build/wh-oracle \
 && site=/out/opt/oracle/lib/python3.11/site-packages && mkdir -p "$site" /out/opt/oracle/bin \
 && printf 'home = /opt/python/bin\ninclude-system-site-packages = false\nversion = 3.11.18\n' > /out/opt/oracle/pyvenv.cfg \
 && ln -s /opt/python/bin/python3.11 /out/opt/oracle/bin/python \
 && /build/host/bin/uv pip install -q --python=/build/host/python/bin/python3.11 --python-platform="$target-manylinux_2_39" --python-version=3.11 --target="$site" --no-index --find-links=/build/wh-oracle --no-cache \
      --require-hashes -r /build/manifests/oracle-requirements.txt \
 && cp -a /build/src/pydatalab /build/oracle-src \
 && /build/host/bin/uv pip install -q --python=/build/host/python/bin/python3.11 --python-platform="$target-manylinux_2_39" --python-version=3.11 --target="$site" --no-index --find-links=/build/wh-oracle --no-cache \
      --no-deps /build/oracle-src \
 && rm -rf "${site:?}/bin"

FROM docker.io/library/ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS runtime
ARG APT_SNAPSHOT
SHELL ["/bin/bash", "-euo", "pipefail", "-c"]
ENV DEBIAN_FRONTEND=noninteractive
RUN rm -f /etc/apt/sources.list.d/ubuntu.sources \
 && printf 'Types: deb\nURIs: http://snapshot.ubuntu.com/ubuntu/%s\nSuites: noble noble-updates noble-security\nComponents: main universe\nSigned-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n' "$APT_SNAPSHOT" > /etc/apt/sources.list.d/snapshot.sources \
 && for attempt in 1 2 3 4 5; do \
      apt-get -o Acquire::Retries=5 -o Acquire::https::Verify-Peer=false update -q \
      && apt-get install -o Acquire::Retries=5 -o Acquire::https::Verify-Peer=false -y -q --no-install-recommends ca-certificates=20260816ubuntu0.24.04.1 \
      && apt-get -o Acquire::Retries=5 update -q \
      && apt-get install -o Acquire::Retries=5 -y -q --no-install-recommends git=1:2.43.0-1ubuntu7.4 libcurl4t64=8.5.0-2ubuntu10.16 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd -g 10001 agent && useradd -u 10001 -g 10001 -d /app -M -s /bin/bash agent \
 && find /usr -xdev -type f -perm /6000 -exec chmod a-s {} +
COPY --from=fetch /out/opt/ /opt/
COPY --from=fetch /out/usr/ /usr/
COPY --from=fetch --chown=10001:10001 /out/app/ /app/
COPY --chown=0:0 scripts/start-mongodb scripts/datalab-oracle /usr/local/bin/
COPY --chown=0:0 scripts/oracle_server.py /opt/oracle/oracle_server.py
RUN git config --system url."file:///opt/git-mirrors/datalab-app-plugin-insitu.git".insteadOf https://github.com/datalab-org/datalab-app-plugin-insitu.git \
 && git config --system url."file:///opt/git-mirrors/datalab-app-plugin-xps.git".insteadOf https://github.com/datalab-industries/datalab-app-plugin-xps.git \
 && git config --system --add safe.directory '*' \
 && chmod 0755 /usr/local/bin/start-mongodb /usr/local/bin/datalab-oracle \
 && chown 10001:10001 /app
ENV PATH=/opt/python/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    HOME=/app \
    UV_PYTHON=/opt/python/bin/python3.11 \
    UV_PYTHON_DOWNLOADS=never \
    UV_DEFAULT_INDEX=file:///opt/wheelhouse/simple
LABEL org.opencontainers.image.title="datalab pydantic 2 migration: agent sandbox" \
      org.opencontainers.image.source="https://github.com/datalab-org/datalab" \
      org.opencontainers.image.revision="d153f80a71b411c074896f6559c775a91c531e4a" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.base.name="docker.io/library/ubuntu:24.04" \
      org.opencontainers.image.base.digest="sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852"
WORKDIR /app
USER 0
CMD ["/bin/bash"]
