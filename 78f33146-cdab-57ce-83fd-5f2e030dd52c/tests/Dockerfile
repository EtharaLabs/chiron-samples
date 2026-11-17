# syntax=docker.io/docker/dockerfile:1.19@sha256:b6afd42430b15f2d2a4c5a02b919e98a525b785b1aaff16747d2f623364e39b6
# Separate verifier: rebuilds the submitted tree offline against the target set and grades it.
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
COPY --chown=0:0 manifests.tar /build/manifests.tar
COPY --chown=0:0 grader/locks/ /build/locks/

RUN arch() { case $1 in arm64) echo aarch64;; amd64) echo x86_64;; *) echo "unsupported $1" >&2; return 1;; esac; } \
 && py311_sha() { case $1 in aarch64) echo ecf28575cd67fa7a53c7bf2355bae5cff2c496efeb51780d50a8b54441cdce74;; x86_64) echo 17c4978a36c496dc53c08899ef8dad1ad2734f6325f08d6033a1e66419e0af26;; esac; } \
 && py312_sha() { case $1 in aarch64) echo bb4b6392670c5d3f93876996fcaa19858a4a413a0c7720144c18b0d6f1f0db2d;; x86_64) echo ce4f6d0a6cfdb3bed6e44c2f9affd85943c76a42e25ad3a259002f31d6fb8a20;; esac; } \
 && uv_sha() { case $1 in aarch64) echo 6aadf3c71600d594e16dabf382cc15282ead4c5ca768599b6bcb43c5004d9aa8;; x86_64) echo 8b3762374972daa7a74bbc6896cc73229ca69a07403dd9f9ea3805a51ffd7582;; esac; } \
 && mongo_sha() { case $1 in aarch64) echo 52655dd90adf0dbcbb87011075233285f44e262048ea30a9a48ad2a834c67736;; x86_64) echo 3e55cdcf7013fbc805d29d8195230df4b23c7688733ee15ad58ac6bc2a416611;; esac; } \
 && get() { curl -fsSL --retry 5 --retry-all-errors -o "$1" "$2" && printf '%s  %s\n' "$3" "$1" | sha256sum -c --quiet -; } \
 && py() { mkdir -p "$3" && get /tmp/py.tgz "https://github.com/astral-sh/python-build-standalone/releases/download/20261026/cpython-$1%2B20261026-$2-unknown-linux-gnu-install_only_stripped.tar.gz" "$4" && tar -xzf /tmp/py.tgz -C "$3" --strip-components=1 && rm /tmp/py.tgz; } \
 && uvx() { mkdir -p "$2" && get /tmp/uv.tgz "https://github.com/astral-sh/uv/releases/download/0.9.30/uv-$1-unknown-linux-gnu.tar.gz" "$(uv_sha "$1")" && tar -xzf /tmp/uv.tgz -C "$2" --strip-components=1 && rm /tmp/uv.tgz; } \
 && host=$(dpkg --print-architecture) \
 && build=$(arch "${BUILDARCH:-$host}") && target=$(arch "${TARGETARCH:-$host}") \
 && test "${TARGETOS:-linux}" = linux \
 && py 3.12.16 "$build" /build/host/python "$(py312_sha "$build")" && uvx "$build" /build/host/bin \
 && py 3.11.18 "$target" /out/opt/python "$(py311_sha "$target")" \
 && py 3.12.16 "$target" /out/opt/grader-python "$(py312_sha "$target")" \
 && uvx "$target" /out/usr/local/bin \
 && get /tmp/mongodb.tgz "https://fastdl.mongodb.org/linux/mongodb-linux-$target-ubuntu2404-8.0.34.tgz" "$(mongo_sha "$target")" \
 && mkdir -p /out/opt/mongodb && tar -xzf /tmp/mongodb.tgz -C /out/opt/mongodb --strip-components=1 && rm /tmp/mongodb.tgz \
 && echo "$target" > /build/target-arch

# Target set (the same files the agent sees) and the retired release's source, kept for the transplant check only.
# The target manifests travel as one archive holding the agent image's manifests byte for byte.
RUN TARGETARCH="${TARGETARCH:-$(dpkg --print-architecture)}" \
 && mkdir -p /build/manifests && tar -xf /build/manifests.tar -C /build/manifests \
 && while read -r name url sha _; do \
      curl -fsSL --retry 5 --retry-all-errors -o "/out/opt/wheelhouse/$name" --create-dirs "$url"; \
      printf '%s  %s\n' "$sha" "/out/opt/wheelhouse/$name" | sha256sum -c --quiet -; \
    done < "/build/manifests/target-$TARGETARCH.txt" \
 && curl -fsSL --retry 5 --retry-all-errors -o /out/opt/grader-data/corpus/pydantic-1.10.26.tar.gz --create-dirs \
      https://files.pythonhosted.org/packages/7b/da/fd89f987a376c807cd81ea0eff4589aade783bbb702637b4734ef2c743a2/pydantic-1.10.26.tar.gz \
 && echo "8c6aa39b494c5af092e690127c283d84f363ac36017106a9e66cb33a22ac412e  /out/opt/grader-data/corpus/pydantic-1.10.26.tar.gz" | sha256sum -c --quiet -

# State A, fetched by commit and checked by its pydatalab tree id.
RUN retry() { for i in 1 2 3 4 5; do "$@" && return 0; if [ "$i" -lt 5 ]; then sleep $((i * 10)); fi; done; return 1; } \
 && git init -q /build/datalab \
 && retry git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=60 -C /build/datalab fetch -q --depth 1 https://github.com/datalab-org/datalab.git d153f80a71b411c074896f6559c775a91c531e4a \
 && test "$(git -C /build/datalab rev-parse FETCH_HEAD:pydatalab)" = 1e48065d59fc9a62706cc46c1aa2de80e7e8bab1 \
 && mkdir -p /out/opt/state-a \
 && git -C /build/datalab archive FETCH_HEAD pydatalab | tar -x -C /out/opt/state-a

# Reward Kit: the harbor-rewardkit 0.2.1 release wheel and its closure, hash-locked, cross-installed for the target.
RUN target=$(cat /build/target-arch) \
 && site=/out/opt/rewardkit/lib/python3.12/site-packages && mkdir -p "$site" /out/opt/rewardkit/bin \
 && printf 'home = /opt/grader-python/bin\ninclude-system-site-packages = false\nversion = 3.12.16\n' > /out/opt/rewardkit/pyvenv.cfg \
 && ln -s /opt/grader-python/bin/python3.12 /out/opt/rewardkit/bin/python \
 && UV_HTTP_RETRIES=10 UV_HTTP_TIMEOUT=60 /build/host/bin/uv pip install -q --python=/build/host/python/bin/python3.12 --python-platform="$target-manylinux_2_39" \
      --python-version=3.12 --target="$site" --no-cache --no-build --require-hashes -r /build/locks/rewardkit.txt \
 && rm -rf "${site:?}/bin" \
 && chmod -R a+rX /out/opt

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
      && apt-get install -o Acquire::Retries=5 -y -q --no-install-recommends libcurl4t64=8.5.0-2ubuntu10.16 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd -g 10002 verifier && useradd -u 10002 -g 10002 -d /home/verifier -m -s /bin/bash verifier \
 && find /usr -xdev -type f -perm /6000 -exec chmod a-s {} +
COPY --from=fetch /out/opt/ /opt/
COPY --from=fetch /out/usr/ /usr/
COPY --chown=0:0 grader/ /opt/grader/
COPY --chown=0:0 test.sh test_output.py judge.toml judge-prompt.md reward.toml /tests/
COPY --chown=0:0 .gate/ /tests/.gate/
COPY --chown=0:0 .veto/ /tests/.veto/
RUN printf '#!/bin/sh\nexec /opt/rewardkit/bin/python -m rewardkit "$@"\n' > /usr/local/bin/rewardkit \
 && chmod 0755 /usr/local/bin/rewardkit /tests/test.sh \
 && rm -rf /opt/grader/locks
ENV PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    UV_PYTHON_DOWNLOADS=never
LABEL org.opencontainers.image.title="datalab pydantic 2 migration: verifier" \
      org.opencontainers.image.source="https://github.com/datalab-org/datalab" \
      org.opencontainers.image.revision="d153f80a71b411c074896f6559c775a91c531e4a" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.base.name="docker.io/library/ubuntu:24.04" \
      org.opencontainers.image.base.digest="sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852"
WORKDIR /home/verifier
USER 0
CMD ["/bin/bash"]
