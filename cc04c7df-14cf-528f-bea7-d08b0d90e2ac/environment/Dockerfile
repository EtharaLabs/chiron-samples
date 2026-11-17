# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# The agent's environment: State A at /app on its own, offline. Two Maven sets: the target set
# (/opt/m2/target, exactly what the graded build may use) and the original set (/opt/m2/original,
# State A's own closure), Temurin 11 and 17, Maven 3.8.6, PostgreSQL 16, and State A runnable on
# WildFly 24 as the oracle. Every download is pinned by digest; apt resolves from a dated snapshot.

ARG SNAPSHOT=20261101T000000Z

# ---- fetch: platform-independent and per-target-architecture downloads, on the build platform
FROM --platform=$BUILDPLATFORM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS fetch
ARG SNAPSHOT
ARG TARGETOS
ARG TARGETARCH
ARG BUILDARCH
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --chown=0:0 apt-snapshot.sh /usr/local/sbin/apt-snapshot.sh
RUN for attempt in 1 2 3 4 5; do \
      /usr/local/sbin/apt-snapshot.sh "${SNAPSHOT}" \
      && apt-get install -o Acquire::Retries=5 -y --no-install-recommends ca-certificates=20260816ubuntu0.24.04.1 curl=8.5.0-2ubuntu10.16 python3=3.12.3-0ubuntu2.1 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/*
COPY --chown=0:0 downloads.txt fetch.sh fetch-m2.sh gen-metadata.py target.sha256 original.sha256 /fetch/
RUN host=$(dpkg --print-architecture) && test "${TARGETOS:-linux}" = linux && /fetch/fetch.sh "${TARGETARCH:-$host}" "${BUILDARCH:-$host}" /dl
RUN /fetch/fetch-m2.sh /fetch/target.sha256 /out/m2/target && python3 /fetch/gen-metadata.py /out/m2/target \
 && /fetch/fetch-m2.sh /fetch/original.sha256 /out/m2/original && python3 /fetch/gen-metadata.py /out/m2/original
RUN mkdir -p /out/opt/jdk-11 /out/opt/jdk-17 /out/opt/maven /out/build/jdk-11 \
 && tar -xzf /dl/jdk11-target.tar.gz -C /out/opt/jdk-11 --strip-components=1 \
 && tar -xzf /dl/jdk17-target.tar.gz -C /out/opt/jdk-17 --strip-components=1 \
 && tar -xzf /dl/jdk11-build.tar.gz -C /out/build/jdk-11 --strip-components=1 \
 && tar -xzf /dl/maven.tar.gz -C /out/opt/maven --strip-components=1 \
 && tar -xzf /dl/wildfly.tar.gz -C /out/opt && mv /out/opt/wildfly-24.0.0.Final /out/opt/wildfly-24

# ---- statea: build State A's two wars offline from the original set, on the build platform
FROM --platform=$BUILDPLATFORM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS statea
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --from=fetch /out/build/jdk-11 /opt/jdk-11
COPY --from=fetch /out/opt/maven /opt/maven
COPY --from=fetch /out/m2/original /m2
COPY --chown=0:0 settings-offline.xml /opt/settings-offline.xml
COPY --chown=0:0 state-a.tar.gz /tmp/state-a.tar.gz
RUN mkdir -p /src && tar -xzf /tmp/state-a.tar.gz -C /src
WORKDIR /src/state-a
RUN JAVA_HOME=/opt/jdk-11 PATH=/opt/jdk-11/bin:$PATH /opt/maven/bin/mvn -s /opt/settings-offline.xml -gs /opt/settings-offline.xml \
    -Dmaven.repo.local=/m2 -o -B -ntp -DskipTests install -pl folding-stats-war,stubbed-folding-endpoints-war -am \
 && mkdir -p /out && cp folding-stats-war/target/folding-stats-war*.war /out/folding-stats-war.war \
 && cp stubbed-folding-endpoints-war/target/stubbed-folding-endpoints-war*.war /out/ROOT.war

# ---- faketime: libfaketime without FAKE_PTHREAD (the stock build spins a JVM), for the target
FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS faketime
ARG SNAPSHOT
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
COPY --chown=0:0 apt-snapshot.sh /usr/local/sbin/apt-snapshot.sh
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

# ---- appsrc: State A's sources for /app, unpacked here so the archive leaves no layer in the runtime image
FROM --platform=$BUILDPLATFORM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS appsrc
COPY --chown=0:0 state-a.tar.gz /tmp/state-a.tar.gz
RUN mkdir -p /out/app && tar -xzf /tmp/state-a.tar.gz -C /out/app --strip-components=1

# ---- runtime
FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 AS runtime
ARG SNAPSHOT
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
LABEL org.opencontainers.image.title="folding-stats migration: agent environment" \
      org.opencontainers.image.source="https://github.com/zodac-personal/folding-stats" \
      org.opencontainers.image.revision="03b2650225ee8926b62ecffc4b01bed305c0bf5f" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.base.name="docker.io/library/ubuntu:24.04" \
      org.opencontainers.image.base.digest="sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852"
ENV DEBIAN_FRONTEND=noninteractive LANG=C.UTF-8 TZ=UTC
COPY --chown=0:0 apt-snapshot.sh /usr/local/sbin/apt-snapshot.sh
RUN for attempt in 1 2 3 4 5; do \
      /usr/local/sbin/apt-snapshot.sh "${SNAPSHOT}" \
      && apt-get install -o Acquire::Retries=5 -y --no-install-recommends \
        ca-certificates=20260816ubuntu0.24.04.1 git=1:2.43.0-1ubuntu7.4 postgresql-16=16.15-0ubuntu0.24.04.1 \
        postgresql-client-16=16.15-0ubuntu0.24.04.1 python3=3.12.3-0ubuntu2.1 procps=2:4.0.4-4ubuntu3.3 \
        unzip=6.0-28ubuntu4.1 zip=3.0-13ubuntu0.2 less=590-2ubuntu2.1 curl=8.5.0-2ubuntu10.16 \
      && break; \
      [ "$attempt" -lt 5 ] || exit 1; sleep 60; \
    done \
 && rm -rf /var/lib/apt/lists/* \
 && if id ubuntu > /dev/null 2>&1; then userdel -r ubuntu; fi && groupadd -g 1000 agent && useradd -m -u 1000 -g 1000 -s /bin/bash agent
COPY --from=fetch --chown=1000:1000 /out/opt/ /opt/
COPY --from=fetch --chown=1000:1000 /out/m2/ /opt/m2/
COPY --from=statea --chown=1000:1000 /out/folding-stats-war.war /out/ROOT.war /opt/wildfly-24/standalone/deployments/
COPY --from=faketime /out/libfaketime-nopthread.so.1 /opt/faketime/libfaketime-nopthread.so.1
COPY --chown=1000:1000 settings-offline.xml /opt/m2/settings-offline.xml
COPY --chown=1000:1000 original/ /opt/original/
COPY --from=appsrc /out/app/ /app/
RUN cp /app/docker/wildfly/config/standalone.xml /opt/wildfly-24/standalone/configuration/standalone.xml \
 && cp /app/docker/wildfly/config/standalone.conf /opt/wildfly-24/bin/standalone.conf \
 && mkdir -p /opt/original/init && cp /app/docker/postgres/scripts/init/*.sql /opt/original/init/ \
 && cp /opt/m2/settings-offline.xml /opt/maven/conf/settings.xml \
 && mkdir -p /home/agent/.m2 \
 && sed 's|<interactiveMode>|<localRepository>/opt/m2/target</localRepository>\n  <interactiveMode>|' /opt/m2/settings-offline.xml > /home/agent/.m2/settings.xml \
 && chmod 0755 /opt/original/run-state-a /opt/original/start-postgres \
 && chown -R 1000:1000 /app /home/agent /opt/wildfly-24 \
 && chmod 0755 /opt/faketime && chmod 0644 /opt/faketime/libfaketime-nopthread.so.1
USER 0
ENV JAVA_HOME=/opt/jdk-17 PATH=/opt/jdk-17/bin:/opt/maven/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
WORKDIR /app
CMD ["sleep", "infinity"]
