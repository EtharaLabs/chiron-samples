# Migrate folding-stats from Java EE 8 on WildFly to Spring Boot

`/app` is the folding-stats repository: a multi-module Maven application (Java 11) that runs as a
web archive on WildFly 24.0.0.Final using the Java EE 8 web profile. It exposes a REST API under
`/folding` backed by PostgreSQL, and it reads its settings from environment variables. Rewrite it
so it runs as a Spring Boot executable jar on JDK 17, with the HTTP behaviour of the API unchanged.
Work in place in `/app`; the repository as you leave it is the migrated service.

## Retired stack

Remove every part of the Java EE / WildFly stack from the repository: the Java EE and Jakarta EE
APIs, JAX-RS, EJB, CDI, the JSR-330 `javax.inject` / `jakarta.inject` API, RESTEasy, Jersey, Weld,
WildFly and JBoss modules. No artifact of them may appear in any `pom.xml`, lockfile, build
script, container file or source file, nor anywhere in the resolved module graph of the build, and
no WildFly deployment descriptor (`jboss-web.xml`, `jboss-deployment-structure.xml`, `beans.xml`,
`ejb-jar.xml`, `standalone.xml`, `standalone.conf`) may remain. Modules that never used the retired
stack may stay as they are.

The rewrite must be a real migration onto Spring's own container, dependency injection and
configuration model:

- Do not copy, vendor or re-create the retired frameworks' own source in the tree under any name,
  and do not keep the original application's compiled classes or archives.
- Do not forward to the original implementation: no bridge, adapter, compatibility layer,
  embedded container, child process or proxy that runs the old code or imitates the retired
  programming model (for example home-grown `@Path`/`@GET`/`@Stateless`-style annotations with a
  reflection layer that dispatches them). Requests must be served by Spring MVC: every request
  is dispatched through `org.springframework.web.servlet.DispatcherServlet`, and the jar's
  `Start-Class` is loaded.
- At run time the application listens only on its HTTP port, connects only to its PostgreSQL
  database and the two external services named below, starts no other process, and loads classes
  only from its own executable jar.
- Nothing in the repository may read, probe or depend on anything outside `/app` other than the
  inputs named in this statement. No file in the repository may name `/tests` or a path under it,
  and the running application may open no path under `/tests` or `/logs`.

The quality of the migrated code matters too. Collaborators that the Java EE container used
to inject should be supplied by the Spring container; container-managed singletons should stay
container-managed rather than become hand-written static singletons; request authentication, the
system-state gating and the cross-origin headers should each be applied in one place through a
Spring web extension point; the timer-driven work should use Spring's scheduling support and the
asynchronous statistics parsing Spring's asynchronous execution; start-up work should run through
Spring's application lifecycle; a shared controller must not keep per-request data in instance
fields; and URL matching should be left to Spring MVC rather than a second hand-maintained route
table. These apply to the files you add or change against the original, except files under a
`/src/test/` directory.

## Dependency sets (no network)

The environment has no network access. Two local Maven repositories are provided:

- **Target set, `/opt/m2/target`**: Spring Boot 2.6.3 and the application's other
  dependencies, three of them at newer patch versions than State A pins (jOOQ 3.16.3,
  PostgreSQL JDBC 42.3.2, Flyway 8.4.4), plus the Maven plugins the build needs. Apart from the
  build-tooling artifacts Maven's own plugin machinery loads (`javax.enterprise:cdi-api:1.0`,
  `javax.inject:javax.inject:1` and the `org.jboss.weld` parent poms they inherit), it holds no
  artifact of the retired stack; those files are build tooling, and the application's own
  resolved dependency graph must not name them.
  It is the only repository the build uses.
  Browse it to see what is available; anything it does not hold cannot be used.
- **Original set, `/opt/m2/original`**: everything the original build resolves on JDK 11. Use it
  only to build and run the original application as a reference.

Maven 3.8.6 is at `/opt/maven`, with offline settings at `/opt/m2/settings-offline.xml` (pass it
with `-s`, and point `-Dmaven.repo.local` at a writable copy of a set). Temurin JDK 11 is at
`/opt/jdk-11` and JDK 17 at `/opt/jdk-17`. PostgreSQL 16 is installed.

The original application stays runnable as a reference: `/opt/original/run-state-a` starts WildFly
24 (`/opt/wildfly-24`, JDK 11) with the original web archive and its stubbed external services,
and `/opt/original/README.md` explains how to point it at a database and the environment it reads.
The original repository's own `testsuite` module exercises much of the API against a running server.

## How the application is built and run

The build runs from a fresh copy of `/app` with every `.git/`, `target/`, `.m2/`,
`node_modules/`, `logs/` and IDE directory and every log, archive (`*.jar`, `*.war`, `*.ear`) and `*.class` file removed. Nothing you built or
installed elsewhere is carried over. It then runs, offline, as an unprivileged user:

```
JAVA_HOME=/opt/jdk-17 mvn -s /opt/m2/settings-offline.xml -gs /opt/m2/settings-offline.xml \
  -Dmaven.repo.local=<fresh copy of /opt/m2/target> -o -B -ntp -DskipTests \
  package org.apache.maven.plugins:maven-dependency-plugin:3.8.1:list
```

with the fresh copy of `/opt/m2/target` as the only local repository; the `list` goal records the
resolved module graph, in which no artifact of the retired stack may appear. The build must finish within 20
minutes and must produce exactly
one Spring Boot executable jar (a `Main-Class` from `org.springframework.boot.loader` and a
`Start-Class`). It is started with `/opt/jdk-17/bin/java -jar <jar>` from an empty working
directory, with only these environment variables set (plus `PATH`, `HOME`, `LANG` and `TZ=UTC`):

| Variable | Meaning |
| --- | --- |
| `JDBC_CONNECTION_URL`, `JDBC_CONNECTION_USER`, `JDBC_CONNECTION_PASSWORD`, `JDBC_CONNECTION_DRIVER`, `DEPLOYED_DATABASE` | the PostgreSQL database, as the original reads them |
| `LARS_URL_ROOT`, `STATS_URL_ROOT` | roots of the LARS GPU API and the Folding@Home stats API (stand-ins are served that behave like the original repository's `stubbed-folding-endpoints-war`: `/user/<name>/stats`, `/bonus` and `/gpu_ppd/overall_ranks`, cleared with `DELETE`; some sequences make the stats stand-in answer 500 or a `garbage` body that is not JSON) |
| `REDIRECT_URL` | where unknown URLs redirect |
| `ENABLE_LARS_HARDWARE_UPDATE`, `ENABLE_MONTHLY_RESULT_STORAGE`, `ENABLE_STATS_MONTHLY_RESET`, `ENABLE_STATS_SCHEDULED_PARSING` | feature flags |
| `STATS_PARSING_SCHEDULE_HOUR`, `STATS_PARSING_SCHEDULE_MINUTE`, `STATS_PARSING_SCHEDULE_SECOND`, `STATS_PARSING_SCHEDULE_FIRST_DAY_OF_MONTH`, `STATS_PARSING_SCHEDULE_LAST_DAY_OF_MONTH` | the schedule of the statistics parsing |
| `USERS_IN_AMD_GPU`, `USERS_IN_NVIDIA_GPU`, `USERS_IN_WILDCARD` | category capacity |

The application must answer `GET http://127.0.0.1:8080/folding/hardware` with 200 within 180
seconds of starting: port 8080, context path `/folding`, as the original serves it on WildFly. Before
each start the database is created from the original's
`docker/postgres/scripts/init/*.sql`, with admin user `root` / `shroot` and read-only user `root2` /
`shroot2`. The application runs on a fixed clock: each start begins at 2026-03-15T11:00:00Z (UTC)
and the clock advances in real time from there (`/opt/faketime/libfaketime-nopthread.so.1` is
preloaded, and this environment also carries it; the original's
`README` under `/opt/original` shows how to run it the same way). Start-up work must be done within
three seconds of the first 200 of every start, and the statistics parse that a request creating
users starts must finish within two seconds of that request's 2xx answer. A start reads its schedules and flags from
the environment of that start alone; nothing scheduled by an earlier start carries over.

## Recorded behaviour

Fixed sequences of HTTP requests, recorded against the original application
under the same database, environment, external stand-ins and clock, are replayed across the whole API:
hardware, teams and users (including batch creation and passkeys), authentication and roles,
system-state behaviour, caching headers and conditional requests, cross-origin headers, unknown-URL
redirects, team-competition statistics, leaderboards, historic statistics, monthly results, the LARS
hardware update, statistics parsing against the external services (including failing services),
resets, configuration from the environment, error responses, restarts against the same database,
and scheduled parsing. For each request these must match the recording: the status code; the `Content-Type` media type,
`Location`, `Cache-Control`, `X-Total-Count`, `WWW-Authenticate` and `Access-Control-*` headers;
the presence of an `ETag` (header names match case-insensitively: `content-type`, `location`,
`cache-control`, `x-total-count`, `www-authenticate`, `access-control-*`, `etag`); and the body, as JSON when the original answered JSON (key order
ignored) and as exact text otherwise. `Cache-Control` must match as a set of directives; where the
original derives `max-age` from the clock, the value must follow the same rule. Where the original's
body was produced by the application server rather than the application (container error pages),
only the status and headers must match. The original's server-produced headers must match too: an `OPTIONS` request
answers 200 with `Content-Type: text/plain;charset=UTF-8`; a request body that cannot be read as the
expected JSON (malformed, or an array where an object belongs, or the reverse) answers 400 and an
unhandled server error answers 500, each with a `text/html` page (the 500 page carries no
`Access-Control-*` headers); and the 303, 304, 401, 403, 404, 405, 406, 412, 415 and 503 answers carry
no `Content-Type`. Every request in a sequence must agree, and the service must behave like the
original for other inputs to the same API as well.
