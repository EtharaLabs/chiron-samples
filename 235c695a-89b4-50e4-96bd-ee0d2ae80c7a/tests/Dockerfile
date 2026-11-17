# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e

# Verifier image. It carries the same pinned toolchain as the task environment, so a
# submission builds here exactly as it built for the agent, plus the grading engine, the
# expectations recorded from the original project and the pinned Reward Kit.

# Trust bundle used only to reach the dated package snapshot before its own
# ca-certificates package is installed; copied in just before the steps that need it and
# deleted when they finish.
FROM --platform=$BUILDPLATFORM alpine:3@sha256:be2656061b536c1883c4034790e21d0a09c5aaaf51968b89368a3c0371ea2c9a AS bootstrap-ca

# The original project, exported from its pinned commits without history. The grader
# runs the original test harness from this copy against every submission.
FROM --platform=$BUILDPLATFORM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS original
ARG SNAPSHOT=20261101T000000Z
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --from=bootstrap-ca /etc/ssl/certs/ca-certificates.crt /run/bootstrap-ca.crt
RUN rm -f /etc/apt/sources.list.d/ubuntu.sources \
 && printf 'Types: deb\nURIs: https://snapshot.ubuntu.com/ubuntu/%s\nSuites: noble noble-updates noble-security\nComponents: main universe\nSigned-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n' "$SNAPSHOT" \
      > /etc/apt/sources.list.d/snapshot.sources \
 && for attempt in 1 2 3 4 5; do \
      apt-get -o Acquire::https::CaInfo=/run/bootstrap-ca.crt -o Acquire::Retries=20 update \
      && DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::https::CaInfo=/run/bootstrap-ca.crt -o Acquire::Retries=20 install -y --no-install-recommends \
        ca-certificates=20260816ubuntu0.24.04.1 \
        git=1:2.43.0-1ubuntu7.4 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/* /run/bootstrap-ca.crt
RUN retry() { for i in 1 2 3 4 5; do "$@" && return 0; if [ "$i" -lt 5 ]; then sleep $((i * 10)); fi; done; return 1; } \
 && fetch_export() { \
      local url="$1" rev="$2" dest="$3" work; \
      work="$(mktemp -d)"; \
      git -C "$work" init -q \
      && retry git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=60 -C "$work" fetch -q --depth 1 "$url" "$rev" \
      && test "$(git -C "$work" rev-parse FETCH_HEAD)" = "$rev" \
      && mkdir -p "$dest" \
      && git -C "$work" archive --format=tar "$rev" | tar -x -C "$dest" \
      && rm -rf "$work"; \
    } \
 && fetch_export https://github.com/YosysHQ/yosys 8125af88d61c61dc47c23537abb32e87bd1206e8 /out/original \
 && fetch_export https://github.com/YosysHQ/abc 5d51a5e420f5de493d07bf61109a977248c86ffb /out/original/abc \
 && fetch_export https://github.com/jarro2783/cxxopts 4bf61f08697b110d9e3991864650a405b3dd515d /out/original/libs/cxxopts \
 && chmod -R u+rwX,go+rX,go-w /out/original

# Offline inputs the project's CI jobs download at run time, pinned by digest.
FROM --platform=$BUILDPLATFORM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS fetched
ARG SNAPSHOT=20261101T000000Z
ARG TARGETARCH
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --from=bootstrap-ca /etc/ssl/certs/ca-certificates.crt /run/bootstrap-ca.crt
RUN rm -f /etc/apt/sources.list.d/ubuntu.sources \
 && printf 'Types: deb\nURIs: https://snapshot.ubuntu.com/ubuntu/%s\nSuites: noble noble-updates noble-security\nComponents: main universe\nSigned-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n' "$SNAPSHOT" \
      > /etc/apt/sources.list.d/snapshot.sources \
 && for attempt in 1 2 3 4 5; do \
      apt-get -o Acquire::https::CaInfo=/run/bootstrap-ca.crt -o Acquire::Retries=20 update \
      && DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::https::CaInfo=/run/bootstrap-ca.crt -o Acquire::Retries=20 install -y --no-install-recommends \
        ca-certificates=20260816ubuntu0.24.04.1 \
        curl=8.5.0-2ubuntu10.16 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/* /run/bootstrap-ca.crt
RUN mkdir -p /out/wheelhouse \
 && printf '%s\n' \
      'pybind11-3.0.4-py3-none-any.whl 961720ee652da51d531b7b2451a6bd2bc042b0106e6d9baa48ecb7d58034ce63 https://files.pythonhosted.org/packages/b3/06/c3a23c9a0263b136c519f033a58d4641e73065fefc7754e9667ec206d992/pybind11-3.0.4-py3-none-any.whl' \
      'cxxheaderparser-1.9.2-py3-none-any.whl db2abd1467c43a03d4e1c3b357ba87095aee6d673b7284538d8e0d82adc83637 https://files.pythonhosted.org/packages/6a/5d/43e242d090569ac95edd0fa5ff67c9998a04333fd246bb4ff244a88b7843/cxxheaderparser-1.9.2-py3-none-any.whl' \
      'setuptools-82.0.1-py3-none-any.whl a59e362652f08dcd477c78bb6e7bd9d80a7995bc73ce773050228a348ce2e5bb https://files.pythonhosted.org/packages/9d/76/f789f7a86709c6b087c5a2f52f911838cad707cc613162401badc665acfe/setuptools-82.0.1-py3-none-any.whl' \
      'wheel-0.48.0-py3-none-any.whl 3217dcc807155e45db462d7ef2431f5ddda0d7273b700d05a67b271ceb1287ab https://files.pythonhosted.org/packages/2e/29/69cfbb602cd91690c55d38ba9fe53e6a7e76a6fa647bf38f19c138d25449/wheel-0.48.0-py3-none-any.whl' \
      'packaging-26.3-py3-none-any.whl d7193f7c8e4e93f444fde0262bf90af30e16fa0ad0ad44cb553c87339b23cd1c https://files.pythonhosted.org/packages/63/34/ba1c580383c9eada3711951fef0795c80b829a078d72188184bcab9dd527/packaging-26.3-py3-none-any.whl' \
    | while read -r name sha url; do \
        curl -fsSL --retry 5 --retry-all-errors -o "/out/wheelhouse/$name" "$url" \
        && echo "$sha  /out/wheelhouse/$name" | sha256sum -c - || exit 1; \
      done
RUN TARGETARCH="${TARGETARCH:-$(dpkg --print-architecture)}" \
 && case "$TARGETARCH" in \
      amd64) name=wasi-sdk-33.0-x86_64-linux; sha=0ba8b5bfaeb2adf3f29bab5841d76cf5318ab8e1642ea195f88baba1abd47bce ;; \
      arm64) name=wasi-sdk-33.0-arm64-linux; sha=4f98ee738c7abb45c81a94d1461fc53cc569d1cd01498951c8184d841a027844 ;; \
      *) echo "no WASI SDK for $TARGETARCH" >&2; exit 1 ;; \
    esac \
 && curl -fsSL --retry 5 --retry-all-errors -o /tmp/wasi-sdk.tar.gz "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-33/$name.tar.gz" \
 && echo "$sha  /tmp/wasi-sdk.tar.gz" | sha256sum -c - \
 && mkdir -p /out/wasi-sdk \
 && tar -xzf /tmp/wasi-sdk.tar.gz -C /out/wasi-sdk --strip-components=1 \
 && rm /tmp/wasi-sdk.tar.gz

FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS verifier
ARG SNAPSHOT=20261101T000000Z
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
LABEL org.opencontainers.image.source="https://github.com/YosysHQ/yosys" \
      org.opencontainers.image.revision="8125af88d61c61dc47c23537abb32e87bd1206e8" \
      org.opencontainers.image.licenses="ISC" \
      org.opencontainers.image.base.name="docker.io/library/ubuntu:24.04" \
      org.opencontainers.image.base.digest="sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852"
# Packages come from the dated snapshot. install-packages.sh fetches them in parallel into apt's
# archive directory, each checked against the hash the snapshot's signed index gives it, and apt
# installs them. A fetch that still fails after its retries, minutes of growing delays, fails the
# build. The script deletes the downloaded packages, so none stays in the image.
COPY --from=bootstrap-ca /etc/ssl/certs/ca-certificates.crt /run/bootstrap-ca.crt
COPY --chown=0:0 install-packages.sh /run/install-packages.sh
RUN bash -euo pipefail /run/install-packages.sh \
 && rm -f /run/install-packages.sh /run/bootstrap-ca.crt
COPY --chown=0:0 .grader/rewardkit.lock.txt /run/rewardkit.lock.txt
RUN python3 -m venv /opt/rewardkit \
 && /opt/rewardkit/bin/pip install --no-cache-dir --retries 10 --timeout 60 --no-deps --only-binary=:all: --require-hashes \
      -r /run/rewardkit.lock.txt \
 && rm /run/rewardkit.lock.txt \
 && /opt/rewardkit/bin/rewardkit --help > /dev/null
RUN groupadd --gid 10002 grader \
 && useradd --uid 10002 --gid 10002 --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin grader \
 && install -d -o 0 -g 0 -m 0755 /opt/grader /var/grader \
 && install -d -o 0 -g 0 -m 0700 /tests /tests/.gate
COPY --from=fetched /out/wheelhouse /opt/wheelhouse
COPY --from=fetched /out/wasi-sdk /opt/wasi-sdk
COPY --from=original /out/original /opt/grader/statea
COPY --chown=0:0 .grader/engine /opt/grader/engine
COPY --chown=0:0 .grader/expect /opt/grader/expect
COPY --chown=0:0 .grader/items.json /opt/grader/items.json
COPY --chown=0:0 test.sh /tests/test.sh
COPY --chown=0:0 test_output.py reward.toml judge.toml judge-prompt.md /tests/
COPY --chown=0:0 .gate/judge.toml .gate/zero.toml .gate/prompt.md /tests/.gate/
RUN chmod 0700 /opt/grader/engine /opt/grader/expect /tests /tests/.gate /tests/test.sh \
 && find /opt/grader/engine /opt/grader/expect -type d -exec chmod 0700 {} + \
 && find /opt/grader/engine /opt/grader/expect -type f -exec chmod 0600 {} + \
 && chmod 0600 /opt/grader/items.json /tests/test_output.py /tests/reward.toml /tests/judge.toml \
      /tests/judge-prompt.md /tests/.gate/judge.toml /tests/.gate/zero.toml /tests/.gate/prompt.md \
 && chown -R 0:0 /opt/grader /tests
WORKDIR /opt/grader
USER 0
CMD ["/bin/bash"]
