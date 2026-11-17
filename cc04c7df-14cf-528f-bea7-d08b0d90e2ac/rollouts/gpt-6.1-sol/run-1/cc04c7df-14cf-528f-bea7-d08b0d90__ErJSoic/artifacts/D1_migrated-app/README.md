# folding-stats

A Java 17 Spring Boot service with a PostgreSQL database. Spring MVC serves the API on port 8080 under `/folding`.

Build the reactor with `mvn -DskipTests package`. The executable is
`folding-stats-service/target/folding-stats-service-1.0.0-SNAPSHOT.jar`:

```sh
java -jar folding-stats-service/target/folding-stats-service-1.0.0-SNAPSHOT.jar
```

For an offline build, supply Maven's offline settings and a writable copy of the supplied target repository:

```sh
JAVA_HOME=/opt/jdk-17 /opt/maven/bin/mvn -s /opt/m2/settings-offline.xml -gs /opt/m2/settings-offline.xml -Dmaven.repo.local=.m2 -o -B -ntp -DskipTests package org.apache.maven.plugins:maven-dependency-plugin:3.8.1:list
```

Configure the database through `JDBC_CONNECTION_URL`, `JDBC_CONNECTION_USER`,
`JDBC_CONNECTION_PASSWORD`, `JDBC_CONNECTION_DRIVER` and `DEPLOYED_DATABASE=postgresql`.
Initialize it with the SQL in `docker/postgres/scripts/init` (substitute the system-user placeholders).
The service uses the existing schema and preserves stored data across restarts.

`LARS_URL_ROOT` and `STATS_URL_ROOT` configure the external hardware and statistics services.
`REDIRECT_URL` is the destination for unknown URLs.

Feature flags (default `false`): `ENABLE_LARS_HARDWARE_UPDATE`,
`ENABLE_MONTHLY_RESULT_STORAGE`, `ENABLE_STATS_MONTHLY_RESET`, `ENABLE_STATS_SCHEDULED_PARSING`.

The parsing schedule uses UTC and `STATS_PARSING_SCHEDULE_HOUR` (default `*`),
`STATS_PARSING_SCHEDULE_MINUTE` (`55`), `STATS_PARSING_SCHEDULE_SECOND` (`0`),
`STATS_PARSING_SCHEDULE_FIRST_DAY_OF_MONTH` (`3`) and
`STATS_PARSING_SCHEDULE_LAST_DAY_OF_MONTH` (`31`). End-of-month work runs at 23:57
on the last day; the start-of-month reset runs at 00:15 on the configured first day.
Category capacities use `USERS_IN_AMD_GPU`, `USERS_IN_NVIDIA_GPU` and `USERS_IN_WILDCARD`.

The library modules contain the domain, database implementation and Java HTTP client.
The service module owns Spring lifecycle initialization, injected application services,
asynchronous parsing, schedules, MVC controllers, authorization and system-state interceptors.

The existing integration suite can target a running service using `testPort` (default 8080),
`stubUrl` (default http://127.0.0.1:8081), and `databasePort` (default 5432).
Set `skip-integration-tests=false` to enable it; its external stand-ins must already be running.
