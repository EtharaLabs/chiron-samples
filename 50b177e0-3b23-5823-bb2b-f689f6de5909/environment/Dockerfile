# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
FROM --platform=$BUILDPLATFORM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS toolchain
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --chown=0:0 snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN for attempt in 1 2 3 4 5; do \
      timeout 600 apt-get -o Acquire::Retries=5 update \
      && timeout 1800 apt-get -o Acquire::Retries=5 install -y --no-install-recommends \
        ca-certificates=20250419+deb13u1 \
        curl=8.14.1-2+deb13u7 \
        xz-utils=5.8.1-1+deb13u2 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
    && rm -rf /var/lib/apt/lists/*
ARG TARGETARCH
WORKDIR /dist
# Rust 1.99.1 for the target: each archive is checked against the sha256 static.rust-lang.org publishes.
RUN TARGETARCH="${TARGETARCH:-$(dpkg --print-architecture)}" \
    && case "$TARGETARCH" in \
         amd64) triple=x86_64-unknown-linux-gnu ;; \
         arm64) triple=aarch64-unknown-linux-gnu ;; \
         *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
       esac \
    && printf '%s\n' \
         'acdcdfe09ae1f103aa9a2b7f88903a9588bc2e99288af10e6fbf5514e2e49f25  rustc-1.99.1-aarch64-unknown-linux-gnu.tar.xz' \
         '2ff05ff9c08a90861e7f6e4065b40a8b87463b4576186fd613871c3e37f2091f  rustc-1.99.1-x86_64-unknown-linux-gnu.tar.xz' \
         '5156832e0c1254e9b49a79edabdc6d4c35f32f8fc1cf3ca700f268ff96087ab0  cargo-1.99.1-aarch64-unknown-linux-gnu.tar.xz' \
         '53f3521949404d595efb06a55c12c5658d4ed1c75566f33d3d514eaebe70bd8a  cargo-1.99.1-x86_64-unknown-linux-gnu.tar.xz' \
         'e25668477083d6a9ddb87b9beb9c00742225f38233ccb4bfba3afa7a07427f52  rust-std-1.99.1-aarch64-unknown-linux-gnu.tar.xz' \
         '109b074225bf5b9ea3820890876bfdf4ed161adbfaf94b2b4b5984c85ee5d958  rust-std-1.99.1-x86_64-unknown-linux-gnu.tar.xz' \
         > SHA256SUMS \
    && for component in rustc cargo rust-std; do \
         name="$component-1.99.1-$triple.tar.xz" \
         && curl -fsSL --retry 5 --retry-all-errors -o "$name" "https://static.rust-lang.org/dist/$name" \
         && grep -F "  $name" SHA256SUMS | sha256sum -c --quiet - \
         && tar -xJf "$name" \
         && "$component-1.99.1-$triple/install.sh" --prefix=/opt/rust --disable-ldconfig > /dev/null \
         || exit 1; \
       done \
    && rm -rf /dist

# State A's tree, checked and unpacked outside the runtime image.
FROM --platform=$BUILDPLATFORM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS statea
COPY --chown=0:0 original.tar.gz original.sha256 /opt/state-a/
WORKDIR /opt/state-a
RUN sha256sum --check original.sha256 \
    && mkdir -p /out/repo \
    && tar -xzf original.tar.gz -C /out/repo

FROM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS runtime
LABEL org.opencontainers.image.title="yyjson C-to-Rust migration workspace" \
      org.opencontainers.image.source="https://github.com/ibireme/yyjson" \
      org.opencontainers.image.revision="6447536015f3d600f3d65323b10976103b337ca7" \
      org.opencontainers.image.version="0.13.0" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.base.name="docker.io/library/debian:trixie-slim" \
      org.opencontainers.image.base.digest="sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31"
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --chown=0:0 snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN for attempt in 1 2 3 4 5; do \
      timeout 600 apt-get -o Acquire::Retries=5 update \
      && timeout 1800 apt-get -o Acquire::Retries=5 install -y --no-install-recommends \
        binutils=2.44-3 \
        ca-certificates=20250419+deb13u1 \
        cmake=3.31.6-2 \
        file=1:5.46-5 \
        gcc=4:14.2.0-1 \
        git=1:2.47.3-0+deb13u1 \
        libc6-dev=2.41-12+deb13u4 \
        make=4.4.1-2 \
        pkgconf=1.8.1-4 \
        python3=3.13.5-1 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
    && rm -rf /var/lib/apt/lists/*
COPY --from=toolchain /opt/rust /opt/rust
COPY --chown=0:0 graded-cc.py /opt/graded/bin/graded-cc.py
RUN chmod 0755 /opt/graded/bin/graded-cc.py \
    && for tool in cc gcc c++ g++ cpp as; do \
         ln -s /opt/graded/bin/graded-cc.py "/opt/graded/bin/$tool"; \
       done
COPY --chown=0:0 original.tar.gz original.sha256 /opt/state-a/
COPY --from=statea /out/repo /workspace/repo
RUN groupadd --gid 1000 agent \
    && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash agent \
    && printf "PATH=/opt/rust/bin:\$PATH\nexport PATH\n" > /etc/profile.d/10-rust.sh \
    && git -C /workspace/repo -c init.defaultBranch=main init -q \
    && git -C /workspace/repo -c user.name=yyjson -c user.email=yyjson@localhost add -A \
    && GIT_AUTHOR_DATE="2026-09-08T18:17:12Z" GIT_COMMITTER_DATE="2026-09-08T18:17:12Z" \
       git -C /workspace/repo -c user.name=yyjson -c user.email=yyjson@localhost -c commit.gpgsign=false \
       commit -q -m "yyjson 0.13.0" \
    && chown -R 1000:1000 /workspace
ENV PATH="/opt/rust/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
    CARGO_HOME="/workspace/.cargo" \
    CARGO_NET_OFFLINE="true" \
    SOURCE_DATE_EPOCH="1788891432"
WORKDIR /workspace/repo
USER 0
CMD ["/bin/bash", "-l"]
