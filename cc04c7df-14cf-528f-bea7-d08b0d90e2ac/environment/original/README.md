# The original application

The original folding-stats web archive, built from `/app` as you received it, is deployed in
`/opt/wildfly-24/standalone/deployments` together with the original repository's
`stubbed-folding-endpoints-war`, which stands in for the LARS GPU API and the Folding@Home stats API.
Use it to observe the behaviour your migration must keep.

1. `/opt/original/start-postgres [port]` creates a private PostgreSQL 16 cluster under
   `~/.folding-pg` (default port 5432) and recreates `folding_db` from
   `/app/docker/postgres/scripts/init/*.sql` with admin `root` / `shroot`, read-only `root2` /
   `shroot2` and owner `folding_user` / `shroot`. Run it again to start from an empty database.
2. `/opt/original/run-state-a [http-port]` starts WildFly 24 on JDK 11 (default port 8081). The
   application answers under `http://127.0.0.1:<port>/folding`, the stubs under
   `http://127.0.0.1:<port>/`. Any of the application's environment variables you export are
   passed through; the rest default to the values the script prints in its header.
3. The original `testsuite` module (`/app/testsuite`) runs its integration tests against a running
   server; its own build resolves from the original set
   (`-Dmaven.repo.local=<copy of /opt/m2/original>`).

To reproduce the fixed clock for either application, start the JVM with
`LD_PRELOAD=/opt/faketime/libfaketime-nopthread.so.1`, `FAKETIME="@2026-03-15 11:00:00"`,
`FAKETIME_DONT_FAKE_MONOTONIC=1` and `NO_FAKE_STAT=1` in its environment. This build of libfaketime
leaves the JVM's thread timing alone; the distribution build makes the JVM spin.
