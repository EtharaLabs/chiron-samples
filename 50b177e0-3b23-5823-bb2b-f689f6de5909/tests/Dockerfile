# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
#
# The verifier image for the yyjson C-to-Rust migration.
#
# Three stages. `toolchain` unpacks the pinned Rust release the graded build needs. `freeze`
# builds State A with a real C compiler and records everything a submission is later compared
# against: its two install trees, its symbol table, its header digest, its consumer routes
# and ~150 MB of behavioural and differential record text, which it leaves xz-compressed.
# `runtime` carries those frozen records, both corpora, the pre-compiled harness objects,
# the stage-3 corpus, State A's own tree, the recorder, the graded pipeline, the stage-3
# family table, the judge packet builder and the stager under /opt/verifier, and places only
# test.sh, reward.toml, test_output.py, judge.toml and judge-prompt.md at /tests. Reward Kit
# scores every .py file and judge TOML it finds at that root, so the stager stays out of
# /tests: it is run by test.sh, not by Reward Kit. Reward Kit's own child-directory scan skips
# the three dot directories; test.sh hands .gate and .zero to it and reads .veto itself.

FROM --platform=$BUILDPLATFORM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS toolchain
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN printf '%s\n' \
        'Types: deb' \
        'URIs: http://snapshot.debian.org/archive/debian/20261027T000000Z' \
        'Suites: trixie trixie-updates' \
        'Components: main' \
        'Check-Valid-Until: no' \
        'Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg' \
        '' \
        'Types: deb' \
        'URIs: http://snapshot.debian.org/archive/debian-security/20261027T000000Z' \
        'Suites: trixie-security' \
        'Components: main' \
        'Check-Valid-Until: no' \
        'Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg' \
        > /etc/apt/sources.list.d/debian.sources
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

FROM debian:trixie-slim@sha256:89c1ea91e897f0fe0a3731595330bc45b8bdef4047464a07116f891a94054b31 AS freeze
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN printf '%s\n' \
        'Types: deb' \
        'URIs: http://snapshot.debian.org/archive/debian/20261027T000000Z' \
        'Suites: trixie trixie-updates' \
        'Components: main' \
        'Check-Valid-Until: no' \
        'Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg' \
        '' \
        'Types: deb' \
        'URIs: http://snapshot.debian.org/archive/debian-security/20261027T000000Z' \
        'Suites: trixie-security' \
        'Components: main' \
        'Check-Valid-Until: no' \
        'Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg' \
        > /etc/apt/sources.list.d/debian.sources
RUN for attempt in 1 2 3 4 5; do \
      timeout 600 apt-get -o Acquire::Retries=5 update \
      && timeout 1800 apt-get -o Acquire::Retries=5 install -y --no-install-recommends \
        binutils=2.44-3 \
        cmake=3.31.6-2 \
        gcc=4:14.2.0-1 \
        libc6-dev=2.41-12+deb13u4 \
        make=4.4.1-2 \
        pkgconf=1.8.1-4 \
        python3=3.13.5-1 \
        xz-utils=5.8.1-1+deb13u2 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
    && rm -rf /var/lib/apt/lists/*
COPY --chown=0:0 verifier /ctx
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
RUN printf '%s\n' \
        'Types: deb' \
        'URIs: http://snapshot.debian.org/archive/debian/20261027T000000Z' \
        'Suites: trixie trixie-updates' \
        'Components: main' \
        'Check-Valid-Until: no' \
        'Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg' \
        '' \
        'Types: deb' \
        'URIs: http://snapshot.debian.org/archive/debian-security/20261027T000000Z' \
        'Suites: trixie-security' \
        'Components: main' \
        'Check-Valid-Until: no' \
        'Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg' \
        > /etc/apt/sources.list.d/debian.sources
RUN for attempt in 1 2 3 4 5; do \
      timeout 600 apt-get -o Acquire::Retries=5 update \
      && timeout 1800 apt-get -o Acquire::Retries=5 install -y --no-install-recommends \
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
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
    && rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /opt/rewardkit \
    && /opt/rewardkit/bin/pip install --no-cache-dir --disable-pip-version-check --retries 10 --timeout 60 \
        harbor-rewardkit==0.2.1 \
        litellm==1.106.5
# `rewardkit` on PATH runs a pass up to three times, so one refused judge call does not zero the run.
COPY --chown=0:0 rewardkit-retry.sh /usr/local/bin/rewardkit
RUN chmod 0755 /usr/local/bin/rewardkit
COPY --from=toolchain /opt/rust /opt/rust
COPY --from=freeze /opt/verifier /opt/verifier
COPY --chown=0:0 verifier/stage_note.py /opt/verifier/stage_note.py
COPY --chown=0:0 test.sh reward.toml test_output.py judge.toml judge-prompt.md /tests/
# The judged gate, scored on its own before anything is paid. Its directory name begins with a
# dot, so Reward Kit's child-directory scan skips it under `rewardkit /tests` and the gate is
# never aggregated into the reward it gates; test.sh hands it to Reward Kit directly. zero.toml
# beside it is the reward table the gate route swaps in for a gate that did not score 1, and
# Reward Kit ignores it here because it declares no judge and is not named reward.toml.
COPY --chown=0:0 .gate /tests/.gate
# The compiled veto test.sh's veto route reads: the stage I/II items, one group each, k = 1.
# Reward Kit never reads it as an input, because its directory name begins with a dot.
COPY --chown=0:0 .veto/veto.toml /tests/.veto/veto.toml
# The zero route, which answers a different question from the gate above: not whether the
# packet may be read, but whether anything may be credited at all. zeroed.py lands here AS
# test_output.py, and zeroed-judge.py AS judge.py, because Reward Kit resolves a weight to the
# input of that name: the zero route then carries the same two inputs at the same weights as
# the paying route, and differs only in what they return. Nothing here calls a judge.
COPY --chown=0:0 zeroed.py /tests/.zero/test_output.py
COPY --chown=0:0 zeroed-judge.py /tests/.zero/judge.py
COPY --chown=0:0 zero-reward.toml /tests/.zero/reward.toml
# The compiler proper leaves every hierarchy the graded build is allowed to execute from.
# A PATH shim can only mediate a name, and two routes go around a name: a driver spelling
# the shim did not enumerate, and cc1 called by its own absolute path. Moving cc1 and lto1
# into /opt/gcc-private, which sandbox.py never grants, closes both at once: a driver
# stripped of cc1 still links, which is all the recorder needs when it passes a link
# through, and it can no longer turn C into assembly by any spelling or any path. The
# directory stays world-readable on purpose. Landlock does the denying, per process, so
# the verifier's own later stages reach the same compiler through `-B` as uid 1000 while
# the graded build cannot reach it at all.
RUN libexec="$(dirname "$(gcc -print-prog-name=cc1)")" \
    && test -x "$libexec/cc1" \
    && mkdir -p /opt/gcc-private \
    && mv "$libexec/cc1" /opt/gcc-private/cc1 \
    && if [ -x "$libexec/lto1" ]; then mv "$libexec/lto1" /opt/gcc-private/lto1; fi \
    && test ! -e "$libexec/cc1" \
    && test -x /opt/gcc-private/cc1 \
    && test -x "$libexec/collect2" \
    && chmod 0755 /opt/gcc-private \
    && printf '%s\n' "$libexec" > /opt/gcc-private/origin.txt
# cargo and rustc sit on the canonical system PATH, so the pipeline finds the real tools in
# the same place whether or not the submission's build has the recorder in front of them.
RUN ln -s /opt/rust/bin/cargo /usr/local/bin/cargo \
    && ln -s /opt/rust/bin/rustc /usr/local/bin/rustc \
    && chmod 0755 /tests/test.sh \
    && groupadd --gid 1000 verifier \
    && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash verifier \
    && mkdir -p /logs/verifier /workspace \
    && chown -R 1000:1000 /logs /workspace
# HOME was supplied by `USER 1000:1000` before the entry became root. The entry is root now, so
# it has to be named here instead: setpriv hands uid 1000 the environment it was given, and a
# grader whose HOME it cannot write is not the environment the graded steps were built for.
# uid 1000 runs the graded build and owns that HOME, so Python's user site stays off for every
# interpreter the grader starts, root's included: a .pth file the build left under HOME would
# otherwise run before the reward is read.
ENV HOME="/home/verifier" \
    PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
    VERIFIER_ROOT="/opt/verifier" \
    VERIFIER_SCRATCH="/tmp/verifier-run" \
    VERIFIER_LOG_DIR="/logs/verifier" \
    CARGO_NET_OFFLINE="true" \
    LC_ALL="C" \
    LANG="C" \
    TZ="UTC" \
    SOURCE_DATE_EPOCH="1788891432" \
    LITELLM_LOCAL_MODEL_COST_MAP="True" \
    PYTHONNOUSERSITE="1"
WORKDIR /tests
# The entry is root so that test.sh can normalise the owner and mode of every path under
# /workspace/repo before it grades anything. It has to: a real agent's editor writes new files
# through NamedTemporaryFile at mode 0600, Harbor's separate verifier copies the tree in with
# `docker compose cp` preserving the numeric owner and mode, and uid 1000 then cannot read them,
# which failed every compiled item on a correct port. Nothing that runs or reads the submission
# grades as root: test.sh runs chown and chmod and then drops to uid 1000 with setpriv for the
# stager, the gate and Reward Kit, because the sweep and the staging design both assume the
# grader is not root. The compiled veto route alone stays at root; it runs no submitted code and
# scores only the verdicts the stager published into the environment.
USER 0:0
CMD ["/bin/bash", "/tests/test.sh"]
