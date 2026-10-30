# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
#
# The verifier image for the yyjson C-to-Rust migration.
#
# Three stages. `toolchain` unpacks the pinned Rust release the graded build needs. `freeze`
# builds State A with a real C compiler and records everything a submission is later compared
# against: its two install trees, its symbol table, its header digest, its consumer routes
# and ~150 MB of behavioural and differential record text, which it leaves xz-compressed.
# `runtime` carries those frozen records, both corpora, the pre-compiled harness objects,
# the stage-3 corpus, the recorder, the graded pipeline and the note stager under
# /opt/verifier, and places only test.sh, reward.toml, test_output.py, judge.toml and
# reference.md at /tests. Reward Kit scores every .py file and judge TOML it finds at that
# root, so the stager stays out of /tests: it is run by test.sh, not by Reward Kit.

FROM --platform=$BUILDPLATFORM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS toolchain
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get -o Acquire::Retries=3 update \
    && apt-get install -y --no-install-recommends \
        xz-utils=5.8.1-1+deb13u2 \
    && rm -rf /var/lib/apt/lists/*
ADD --checksum=sha256:acdcdfe09ae1f103aa9a2b7f88903a9588bc2e99288af10e6fbf5514e2e49f25 https://static.rust-lang.org/dist/rustc-1.99.1-aarch64-unknown-linux-gnu.tar.xz /dist/
ADD --checksum=sha256:2ff05ff9c08a90861e7f6e4065b40a8b87463b4576186fd613871c3e37f2091f https://static.rust-lang.org/dist/rustc-1.99.1-x86_64-unknown-linux-gnu.tar.xz /dist/
ADD --checksum=sha256:5156832e0c1254e9b49a79edabdc6d4c35f32f8fc1cf3ca700f268ff96087ab0 https://static.rust-lang.org/dist/cargo-1.99.1-aarch64-unknown-linux-gnu.tar.xz /dist/
ADD --checksum=sha256:53f3521949404d595efb06a55c12c5658d4ed1c75566f33d3d514eaebe70bd8a https://static.rust-lang.org/dist/cargo-1.99.1-x86_64-unknown-linux-gnu.tar.xz /dist/
ADD --checksum=sha256:e25668477083d6a9ddb87b9beb9c00742225f38233ccb4bfba3afa7a07427f52 https://static.rust-lang.org/dist/rust-std-1.99.1-aarch64-unknown-linux-gnu.tar.xz /dist/
ADD --checksum=sha256:109b074225bf5b9ea3820890876bfdf4ed161adbfaf94b2b4b5984c85ee5d958 https://static.rust-lang.org/dist/rust-std-1.99.1-x86_64-unknown-linux-gnu.tar.xz /dist/
ARG TARGETARCH
RUN case "$TARGETARCH" in \
         amd64) triple=x86_64-unknown-linux-gnu ;; \
         arm64) triple=aarch64-unknown-linux-gnu ;; \
         *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
       esac \
    && cd /dist \
    && for component in rustc cargo rust-std; do \
         tar -xJf "$component-1.99.1-$triple.tar.xz"; \
         "$component-1.99.1-$triple/install.sh" --prefix=/opt/rust --disable-ldconfig >/dev/null; \
       done \
    && rm -rf /dist

FROM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS freeze
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get -o Acquire::Retries=3 update \
    && apt-get install -y --no-install-recommends \
        binutils=2.44-3 \
        cmake=3.31.6-2 \
        gcc=4:14.2.0-1 \
        libc6-dev=2.41-12+deb13u4 \
        make=4.4.1-2 \
        pkgconf=1.8.1-4 \
        python3=3.13.5-1 \
        xz-utils=5.8.1-1+deb13u2 \
    && rm -rf /var/lib/apt/lists/*
COPY verifier /ctx
RUN bash /ctx/freeze.sh /ctx

FROM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS runtime
LABEL org.opencontainers.image.title="yyjson C-to-Rust migration verifier" \
      org.opencontainers.image.source="https://github.com/ibireme/yyjson" \
      org.opencontainers.image.revision="6447536015f3d600f3d65323b10976103b337ca7" \
      org.opencontainers.image.version="0.13.0" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.base.name="docker.io/library/debian:trixie-slim" \
      org.opencontainers.image.base.digest="sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31"
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get -o Acquire::Retries=3 update \
    && apt-get install -y --no-install-recommends \
        binutils=2.44-3 \
        ca-certificates=20250419+deb13u1 \
        cmake=3.31.6-2 \
        file=1:5.46-5 \
        gcc=4:14.2.0-1 \
        libc6-dev=2.41-12+deb13u4 \
        make=4.4.1-2 \
        pkgconf=1.8.1-4 \
        python3=3.13.5-1 \
        python3-venv=3.13.5-1 \
        xz-utils=5.8.1-1+deb13u2 \
    && rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /opt/rewardkit \
    && /opt/rewardkit/bin/pip install --no-cache-dir --disable-pip-version-check \
        harbor-rewardkit==0.2.1 \
        litellm==1.106.5 \
    && ln -s /opt/rewardkit/bin/rewardkit /usr/local/bin/rewardkit
COPY --from=toolchain /opt/rust /opt/rust
COPY --from=freeze /opt/verifier /opt/verifier
COPY verifier/stage_note.py /opt/verifier/stage_note.py
COPY test.sh reward.toml test_output.py judge.toml reference.md /tests/
# The fallback test.sh routes to when the judged gate stays shut: the compiled criteria, plus
# withheld.py, which carries the judged weight with every judged item scoring 0 so this route
# cannot pay more than the compiled share. Its name begins with a dot, so Reward Kit's
# child-directory scan skips it under `rewardkit /tests`, but test.sh can still hand it to
# Reward Kit directly. It is root-owned like the rest of /tests, so a submission running as
# uid 1000 cannot write a judge TOML into it.
COPY test_output.py /tests/.compiled/test_output.py
COPY withheld.py /tests/.compiled/withheld.py
COPY compiled-reward.toml /tests/.compiled/reward.toml
# cargo and rustc sit on the canonical system PATH, so the pipeline finds the real tools in
# the same place whether or not the submission's build has the recorder in front of them.
RUN ln -s /opt/rust/bin/cargo /usr/local/bin/cargo \
    && ln -s /opt/rust/bin/rustc /usr/local/bin/rustc \
    && chmod 0755 /tests/test.sh \
    && groupadd --gid 1000 verifier \
    && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash verifier \
    && mkdir -p /logs/verifier /workspace \
    && chown -R 1000:1000 /logs /workspace
ENV PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
    VERIFIER_ROOT="/opt/verifier" \
    VERIFIER_SCRATCH="/tmp/verifier-run" \
    VERIFIER_LOG_DIR="/logs/verifier" \
    CARGO_NET_OFFLINE="true" \
    LC_ALL="C" \
    LANG="C" \
    TZ="UTC" \
    SOURCE_DATE_EPOCH="1788891432" \
    LITELLM_LOCAL_MODEL_COST_MAP="True"
WORKDIR /tests
USER 1000:1000
CMD ["/bin/bash", "/tests/test.sh"]
