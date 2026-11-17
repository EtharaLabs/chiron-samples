# syntax=docker/dockerfile:1.28@sha256:bb22d9815c728170f72750f4e5b0d672e06176142e1d602c7e66c050100b7e5b

# fetch: runs on the build platform and downloads, checksums and stages everything the target
# architecture needs: CPython, uv, the target wheel set and the original wheel set.
FROM --platform=$BUILDPLATFORM scylladb/scylla:2025.4.0@sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07 AS fetch
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
USER 0
ARG BUILDARCH
ARG TARGETARCH
ARG SOURCE_DATE_EPOCH=1787310739
# UBI packages come from the RPM files in rpms/ (microdnf's resolution against this base, checked
# by sha256), so the build never reaches the live UBI repositories, which drop older builds.
COPY --chown=0:0 rpms/install.sh rpms/SHA256SUMS /tmp/rpms/
COPY --chown=0:0 rpms/aarch64/fetch/ /tmp/rpms/aarch64/fetch/
COPY --chown=0:0 rpms/x86_64/fetch/ /tmp/rpms/x86_64/fetch/
RUN bash /tmp/rpms/install.sh fetch && rm -rf /tmp/rpms
WORKDIR /fetch
COPY --chown=0:0 locks/ /fetch/locks/
RUN case "$(uname -m)" in x86_64) host=amd64 ;; aarch64) host=arm64 ;; *) host= ;; esac \
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
 && mkdir -p /opt/python /build/python /opt/uv \
 && tar -xzf python-target.tar.gz -C /opt/python --strip-components=1 \
 && tar -xzf python-build.tar.gz -C /build/python --strip-components=1 \
 && tar -xzf uv.tar.gz -C /opt/uv --strip-components=1 \
 && dl() { /build/python/bin/python3 -m pip download --no-cache-dir --quiet --retries 10 --timeout 60 --no-deps --require-hashes "$@"; } \
 && for set in target original; do \
      dl --only-binary=:all: --platform "manylinux_2_28_$arch" --platform "manylinux2014_$arch" --platform "manylinux_2_17_$arch" --platform "linux_$arch" --python-version 3.12 --implementation cp --abi cp312 --abi abi3 --abi none -d "/opt/wheels/$set" -r "locks/$set.txt" \
      && dl --no-binary=:all: -d "/opt/wheels/$set" -r "locks/$set-sdist.txt" \
      || exit 1; \
    done \
 && cp locks/*.txt /opt/wheels/ \
 && sha256sum /opt/wheels/target/* /opt/wheels/original/* | sed 's|  /opt/wheels/|  |' > /opt/wheels/SHA256SUMS

# The original application's tree, copied without version-control history.
COPY --chown=0:0 app/ /fetch/app/
# The bundle ships standard file modes only (*.sh 0755, every other file 0644); restore the two
# modes the upstream tree records, so /app and /opt/argus-original match upstream.
RUN chmod 0755 /fetch/app/run_with_profiler.py && chmod 0644 /fetch/app/start_argus.sh

# compile: runs on the target platform, because uWSGI is C. Builds the original application's
# virtual environment from the original wheel set, offline and hash-checked.
FROM scylladb/scylla:2025.4.0@sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07 AS compile
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
USER 0
COPY --chown=0:0 rpms/install.sh rpms/SHA256SUMS /tmp/rpms/
COPY --chown=0:0 rpms/aarch64/compile/ /tmp/rpms/aarch64/compile/
COPY --chown=0:0 rpms/x86_64/compile/ /tmp/rpms/x86_64/compile/
RUN bash /tmp/rpms/install.sh compile && rm -rf /tmp/rpms
COPY --from=fetch /opt/python /opt/python
COPY --from=fetch /opt/wheels /opt/wheels
RUN export CC=gcc LDFLAGS=-Wl,-rpath,/opt/python/lib \
 && ln -s /opt/python /install \
 && /opt/python/bin/python3 -m venv /opt/argus-original/.venv \
 && /opt/argus-original/.venv/bin/pip install --no-cache-dir --quiet --no-index --find-links /opt/wheels/original \
      --no-deps --require-hashes \
      -r /opt/wheels/original.txt -r /opt/wheels/original-sdist.txt \
 && rm /install \
 && /opt/argus-original/.venv/bin/uwsgi --version \
 && find /opt/argus-original/.venv -name '__pycache__' -prune -exec rm -rf {} +

# runtime: the agent sandbox. ScyllaDB from the base image, CPython and uv, both wheel sets, the
# original application runnable as the behavioural oracle, and /app to migrate.
FROM scylladb/scylla:2025.4.0@sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07 AS runtime
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
USER 0
COPY --chown=0:0 rpms/install.sh rpms/SHA256SUMS /tmp/rpms/
COPY --chown=0:0 rpms/aarch64/runtime/ /tmp/rpms/aarch64/runtime/
COPY --chown=0:0 rpms/x86_64/runtime/ /tmp/rpms/x86_64/runtime/
RUN bash /tmp/rpms/install.sh runtime && rm -rf /tmp/rpms \
 && useradd --uid 1000 --gid 1000 --no-user-group --create-home --home-dir /home/agent --shell /bin/bash agent \
 && mkdir -p /var/log/argus /var/lib/argus /tmp/promdb-argus-metrics \
 && chown 1000:1000 /var/log/argus /var/lib/argus /tmp/promdb-argus-metrics
COPY --from=fetch /opt/python /opt/python
COPY --from=fetch /opt/uv/ /usr/local/bin/
COPY --from=fetch /opt/wheels /opt/wheels
COPY --from=compile /opt/argus-original/.venv /opt/argus-original/.venv
COPY --from=fetch /fetch/app/ /opt/argus-original/
COPY --from=fetch --chown=1000:1000 /fetch/app/ /app/
COPY --chown=0:0 bin/argus-db bin/argus-original /usr/local/bin/
RUN chmod 0755 /usr/local/bin/argus-db /usr/local/bin/argus-original
COPY --chown=0:0 oracle/argus_web.yaml /opt/argus-original/argus_web.yaml
ENV PATH=/opt/python/bin:/usr/local/bin:/usr/bin:/usr/sbin:/bin \
    UV_OFFLINE=0 \
    UV_NO_INDEX=0 \
    UV_FIND_LINKS=/opt/wheels/target \
    UV_PYTHON_DOWNLOADS=manual \
    PIP_NO_INDEX=1 \
    PIP_FIND_LINKS=/opt/wheels/target \
    CQLENG_ALLOW_SCHEMA_MANAGEMENT=1 \
    PROMETHEUS_MULTIPROC_DIR=/tmp/promdb-argus-metrics \
    SETUPTOOLS_SCM_PRETEND_VERSION=0.0.0
LABEL org.opencontainers.image.title="argus-flask-to-fastapi agent environment" \
      org.opencontainers.image.source="https://github.com/scylladb/argus" \
      org.opencontainers.image.revision="8a0367430a024b78caff70cfb7059807b3e127da" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.base.name="docker.io/scylladb/scylla:2025.4.0" \
      org.opencontainers.image.base.digest="sha256:e754396ff9d66fb9a97e8df8f4b7595f239612469328c0bde9c2b99c8c6f9a07"
WORKDIR /app
USER 0
ENTRYPOINT []
CMD ["/bin/bash"]
