# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# The verifier image: the graded pipeline (.verifier), its frozen recordings and families (.data),
# the target set the graded build uses, Temurin 17, Maven 3.8.6, PostgreSQL 16, strace, the JVM
# dispatch probe, the transplant corpus, and Reward Kit 0.2.1 from its published wheel. No WildFly,
# no JDK 11 and no original set: the retired stack cannot be reached from here.

ARG SNAPSHOT=20261101T000000Z

FROM --platform=$BUILDPLATFORM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS fetch
ARG SNAPSHOT
ARG TARGETOS
ARG TARGETARCH
ARG BUILDARCH
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --chown=0:0 .build/verifier-inputs.tar.gz.b64 /tmp/verifier-inputs.tar.gz.b64
RUN mkdir -p /fetch && base64 -d /tmp/verifier-inputs.tar.gz.b64 | tar -xz -C /fetch && rm /tmp/verifier-inputs.tar.gz.b64 \
 && for attempt in 1 2 3 4 5; do \
      /fetch/apt-snapshot.sh "${SNAPSHOT}" \
      && apt-get install -o Acquire::Retries=5 -y --no-install-recommends ca-certificates=20260816ubuntu0.24.04.1 curl=8.5.0-2ubuntu10.16 python3=3.12.3-0ubuntu2.1 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/*
RUN host=$(dpkg --print-architecture) && test "${TARGETOS:-linux}" = linux && /fetch/fetch.sh "${TARGETARCH:-$host}" "${BUILDARCH:-$host}" /dl
RUN /fetch/fetch-m2.sh /fetch/target.sha256 /out/m2/target && python3 /fetch/gen-metadata.py /out/m2/target \
 && /fetch/fetch-m2.sh /fetch/corpus.sha256 /out/corpus-m2 \
 && mkdir -p /out/corpus && find /out/corpus-m2 -name '*-sources.jar' -exec cp {} /out/corpus/ \;
RUN mkdir -p /out/opt/jdk-17 /out/opt/maven /out/build/jdk-17 \
 && tar -xzf /dl/jdk17-target.tar.gz -C /out/opt/jdk-17 --strip-components=1 \
 && tar -xzf /dl/jdk17-build.tar.gz -C /out/build/jdk-17 --strip-components=1 \
 && tar -xzf /dl/maven.tar.gz -C /out/opt/maven --strip-components=1

FROM --platform=$BUILDPLATFORM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS probe
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --from=fetch /out/build/jdk-17 /opt/jdk-17
COPY --chown=0:0 .verifier/probe/ /src/
RUN mkdir -p /classes /out && find /src -name '*.java' > /tmp/probe-sources.txt \
 && /opt/jdk-17/bin/javac -source 17 -target 17 \
      --add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED -d /classes @/tmp/probe-sources.txt \
 && /opt/jdk-17/bin/jar --create --file /out/probe.jar --manifest /src/MANIFEST.MF --date 2026-11-01T00:00:00Z -C /classes .

FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS faketime
ARG SNAPSHOT
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --from=fetch /fetch/apt-snapshot.sh /usr/local/sbin/apt-snapshot.sh
RUN for attempt in 1 2 3 4 5; do \
      /usr/local/sbin/apt-snapshot.sh "${SNAPSHOT}" \
      && apt-get install -o Acquire::Retries=5 -y --no-install-recommends gcc=4:13.2.0-7ubuntu1 make=4.3-4.1build2 libc6-dev=2.39-0ubuntu8.9 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/*
COPY --from=fetch /dl/libfaketime.tar.gz /src/libfaketime.tar.gz
RUN tar -xzf /src/libfaketime.tar.gz -C /src
WORKDIR /src/libfaketime-0.9.10/src
RUN sed -i 's/ -DFAKE_PTHREAD//' Makefile && ! grep -q -- '-DFAKE_PTHREAD' Makefile \
 && make FAKETIME_COMPILE_CFLAGS=-Wno-error libfaketime.so.1 \
 && install -D -m 0644 libfaketime.so.1 /out/libfaketime-nopthread.so.1

FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS runtime
ARG SNAPSHOT
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
LABEL org.opencontainers.image.title="folding-stats migration: verifier" \
      org.opencontainers.image.source="https://github.com/zodac-personal/folding-stats" \
      org.opencontainers.image.revision="03b2650225ee8926b62ecffc4b01bed305c0bf5f" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.base.name="docker.io/library/ubuntu:24.04" \
      org.opencontainers.image.base.digest="sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852"
ENV DEBIAN_FRONTEND=noninteractive LANG=C.UTF-8 TZ=UTC PYTHONDONTWRITEBYTECODE=1
COPY --from=fetch /fetch/apt-snapshot.sh /usr/local/sbin/apt-snapshot.sh
RUN for attempt in 1 2 3 4 5; do \
      /usr/local/sbin/apt-snapshot.sh "${SNAPSHOT}" \
      && apt-get install -o Acquire::Retries=5 -y --no-install-recommends \
        ca-certificates=20260816ubuntu0.24.04.1 postgresql-16=16.15-0ubuntu0.24.04.1 \
        postgresql-client-16=16.15-0ubuntu0.24.04.1 python3=3.12.3-0ubuntu2.1 python3-venv=3.12.3-0ubuntu2.1 \
        strace=6.8-0ubuntu2 procps=2:4.0.4-4ubuntu3.3 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/* \
 && if id ubuntu > /dev/null 2>&1; then userdel -r ubuntu; fi && groupadd -g 1000 app && useradd -m -u 1000 -g 1000 -s /bin/bash app
COPY --from=fetch /fetch/rk.lock /opt/rewardkit.lock
RUN python3 -m venv /opt/rewardkit \
 && /opt/rewardkit/bin/pip install --no-cache-dir --retries 10 --timeout 60 --require-hashes --only-binary=:all: --no-deps -r /opt/rewardkit.lock \
 && /opt/rewardkit/bin/rewardkit --help > /dev/null
COPY --from=fetch /out/opt/ /opt/
COPY --from=fetch /out/m2/target /opt/m2/target
COPY --from=fetch /out/corpus /opt/grader-corpus
COPY --from=probe /out/probe.jar /opt/grader-probe/probe.jar
COPY --from=faketime /out/libfaketime-nopthread.so.1 /opt/faketime/libfaketime-nopthread.so.1
COPY --from=fetch /fetch/settings-offline.xml /opt/m2/settings-offline.xml
COPY --chown=0:0 . /tests
RUN rm -rf /tests/.build && chmod -R go-w /tests /opt/m2 /opt/grader-corpus /opt/grader-probe \
 && chmod 0755 /tests/test.sh && mkdir -p /logs/verifier
WORKDIR /tests
# The graded pipeline runs as root (task.toml [verifier] user = "root" says so too): it starts the
# submission as uid 1000 (app) under strace and keeps its own state out of that uid's reach.
USER 0
CMD ["sleep", "infinity"]
