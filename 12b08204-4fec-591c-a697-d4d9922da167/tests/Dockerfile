# syntax=docker/dockerfile:1.28@sha256:bb22d9815c728170f72750f4e5b0d672e06176142e1d602c7e66c050100b7e5b

# fetch: runs on the build platform and downloads and checksums every grading input for the
# target architecture: CPython, uv, the target wheel set, the grader's own wheel set, the retired
# stack's released source for the transplant check, and libfaketime's source.
FROM --platform=$BUILDPLATFORM scylladb/scylla:2025.4.0@sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07 AS fetch
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
USER 0
ARG BUILDARCH
ARG TARGETARCH
# UBI packages come from the RPM files in rpms/ (microdnf's resolution against this base, checked
# by sha256), so the build never reaches the live UBI repositories, which drop older builds.
COPY --chown=0:0 rpms/install.sh rpms/SHA256SUMS /tmp/rpms/
COPY --chown=0:0 rpms/aarch64/fetch/ /tmp/rpms/aarch64/fetch/
COPY --chown=0:0 rpms/x86_64/fetch/ /tmp/rpms/x86_64/fetch/
RUN bash /tmp/rpms/install.sh fetch && rm -rf /tmp/rpms
WORKDIR /fetch
COPY --chown=0:0 locks.json /fetch/locks.json
COPY --chown=0:0 retired-sources.py /fetch/retired-sources.py
RUN /opt/scylladb/python3/bin/python3 -c 'import json, pathlib; out = pathlib.Path("locks"); out.mkdir(); [(out / name).write_text(text, encoding="utf-8", newline="") for name, text in json.loads(pathlib.Path("locks.json").read_text(encoding="utf-8"))["files"].items()]' \
 && case "$(uname -m)" in x86_64) host=amd64 ;; aarch64) host=arm64 ;; *) host= ;; esac \
 && TARGETARCH="${TARGETARCH:-$host}" && BUILDARCH="${BUILDARCH:-$host}" \
 && case "$TARGETARCH" in \
      amd64) arch=x86_64; py_sha=178d3d83b6e8e99285a65d3fce9c16b87952cc5efdb4d1a0e3ff3be18ff0fc9c; uv_sha=2cf10babba653310606f8b49876cfb679928669e7ddaa1fb41fb00ce73e64f66 ;; \
      arm64) arch=aarch64; py_sha=9467a99bacc7d90b64b02de7fa9ecfc99b72905e64e74f7ff6b460ad95cfed5a; uv_sha=9db0c2f6683099f86bfeea47f4134e915f382512278de95b2a0e625957594ff3 ;; \
      *) echo "unsupported TARGETARCH $TARGETARCH" >&2; exit 1 ;; \
    esac \
 && case "$BUILDARCH" in \
      amd64) build=x86_64; build_sha=178d3d83b6e8e99285a65d3fce9c16b87952cc5efdb4d1a0e3ff3be18ff0fc9c ;; \
      arm64) build=aarch64; build_sha=9467a99bacc7d90b64b02de7fa9ecfc99b72905e64e74f7ff6b460ad95cfed5a ;; \
      *) echo "unsupported BUILDARCH $BUILDARCH" >&2; exit 1 ;; \
    esac \
 && pbs=https://github.com/astral-sh/python-build-standalone/releases/download/20261013 \
 && curl -fsSL --retry 5 --retry-all-errors -o python-target.tar.gz "$pbs/cpython-3.12.15%2B20261013-$arch-unknown-linux-gnu-install_only_stripped.tar.gz" \
 && echo "$py_sha  python-target.tar.gz" | sha256sum -c - \
 && curl -fsSL --retry 5 --retry-all-errors -o python-build.tar.gz "$pbs/cpython-3.12.15%2B20261013-$build-unknown-linux-gnu-install_only_stripped.tar.gz" \
 && echo "$build_sha  python-build.tar.gz" | sha256sum -c - \
 && curl -fsSL --retry 5 --retry-all-errors -o uv.tar.gz "https://github.com/astral-sh/uv/releases/download/0.9.5/uv-$arch-unknown-linux-gnu.tar.gz" \
 && echo "$uv_sha  uv.tar.gz" | sha256sum -c - \
 && curl -fsSL --retry 5 --retry-all-errors -o libfaketime.tar.gz "https://github.com/wolfcw/libfaketime/archive/refs/tags/v0.9.13.tar.gz" \
 && echo "8e56deeb805682b025107e095f1d94a6ea677472f05824cd475f5c5e6e1a5ddf  libfaketime.tar.gz" | sha256sum -c - \
 && mkdir -p /opt/python /build/python /opt/uv /src/libfaketime \
 && tar -xzf python-target.tar.gz -C /opt/python --strip-components=1 \
 && tar -xzf python-build.tar.gz -C /build/python --strip-components=1 \
 && tar -xzf uv.tar.gz -C /opt/uv --strip-components=1 \
 && tar -xzf libfaketime.tar.gz -C /src/libfaketime --strip-components=1 \
 && dl() { /build/python/bin/python3 -m pip download --no-cache-dir --quiet --retries 10 --timeout 60 --no-deps --require-hashes "$@"; } \
 && wheels() { dl --only-binary=:all: --platform "manylinux_2_28_$arch" --platform "manylinux2014_$arch" --platform "manylinux_2_17_$arch" --platform "linux_$arch" --python-version 3.12 --implementation cp --abi cp312 --abi abi3 --abi none "$@"; } \
 && wheels -d /opt/wheels/target -r locks/target.txt \
 && dl --no-binary=:all: -d /opt/wheels/target -r locks/target-sdist.txt \
 && wheels -d /wheels/grader -r locks/grader.txt \
 && wheels -d /wheels/retired -r locks/retired.txt \
 && dl --no-binary=:all: -d /wheels/retired -r locks/retired-sdist.txt \
 && cp locks/target.txt locks/target-sdist.txt /opt/wheels/ \
 && /build/python/bin/python3 /fetch/retired-sources.py

# compile: runs on the target platform. Builds libfaketime, which pins the served clock, and the
# grader's own virtual environment (Reward Kit and the database driver), offline and hash-checked.
FROM scylladb/scylla:2025.4.0@sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07 AS compile
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
USER 0
COPY --chown=0:0 rpms/install.sh rpms/SHA256SUMS /tmp/rpms/
COPY --chown=0:0 rpms/aarch64/compile/ /tmp/rpms/aarch64/compile/
COPY --chown=0:0 rpms/x86_64/compile/ /tmp/rpms/x86_64/compile/
RUN bash /tmp/rpms/install.sh compile && rm -rf /tmp/rpms
COPY --from=fetch /opt/python /opt/python
COPY --from=fetch /src/libfaketime /src/libfaketime
COPY --from=fetch /wheels/grader /wheels/grader
COPY --from=fetch /fetch/locks/grader.txt /locks/grader.txt
RUN make -C /src/libfaketime/src PREFIX=/opt/faketime LIBDIRNAME=/lib/faketime >/dev/null \
 && make -C /src/libfaketime/src PREFIX=/opt/faketime LIBDIRNAME=/lib/faketime install >/dev/null \
 && /opt/python/bin/python3 -m venv /opt/grader/venv \
 && /opt/grader/venv/bin/pip install --no-cache-dir --quiet --no-index --find-links /wheels/grader \
      --no-deps --require-hashes -r /locks/grader.txt \
 && /opt/grader/venv/bin/rewardkit --help >/dev/null \
 && find /opt/grader/venv -name '__pycache__' -prune -exec rm -rf {} +

# runtime: the separate verifier. Reward Kit reads /tests; the graded pipeline and its recorded
# State A answers live under /opt/grader, readable by the grading process alone.
FROM scylladb/scylla:2025.4.0@sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07 AS runtime
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
USER 0
# the same runtime tools the agent's image gives the application (it shells out to git)
COPY --chown=0:0 rpms/install.sh rpms/SHA256SUMS /tmp/rpms/
COPY --chown=0:0 rpms/aarch64/runtime/ /tmp/rpms/aarch64/runtime/
COPY --chown=0:0 rpms/x86_64/runtime/ /tmp/rpms/x86_64/runtime/
RUN bash /tmp/rpms/install.sh runtime && rm -rf /tmp/rpms
RUN groupadd --gid 1001 grader \
 && useradd --uid 1001 --gid 1001 --create-home --home-dir /home/grader --shell /bin/bash grader \
 && groupadd --gid 2000 submission \
 && useradd --uid 2000 --gid 2000 --create-home --home-dir /home/submission --shell /bin/bash submission
COPY --from=fetch /opt/python /opt/python
COPY --from=fetch /opt/uv/ /usr/local/bin/
COPY --from=fetch /opt/wheels /opt/wheels
COPY --from=fetch /opt/grader/retired /opt/grader/retired
COPY --from=compile /opt/faketime /opt/faketime
COPY --from=compile /opt/grader/venv /opt/grader/venv
COPY --chown=0:0 grader/ /opt/grader/
COPY --chown=0:0 test.sh test_output.py judge.toml prompt.md reward.toml /tests/
COPY --chown=0:0 .gate/judge.toml .gate/zero.toml .gate/prompt.md /tests/.gate/
RUN chmod 0755 /tests/test.sh \
 && chown -R root:root /opt/grader /tests \
 && chmod 0700 /opt/grader/data \
 && mkdir -p /logs/verifier
ENV PATH=/opt/grader/venv/bin:/opt/python/bin:/usr/local/bin:/usr/bin:/usr/sbin:/bin \
    PYTHONDONTWRITEBYTECODE=1
LABEL org.opencontainers.image.title="argus-flask-to-fastapi verifier" \
      org.opencontainers.image.source="https://github.com/scylladb/argus" \
      org.opencontainers.image.revision="8a0367430a024b78caff70cfb7059807b3e127da" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.base.name="docker.io/scylladb/scylla:2025.4.0" \
      org.opencontainers.image.base.digest="sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07"
WORKDIR /tests
USER 0
ENTRYPOINT []
CMD ["/bin/bash"]
