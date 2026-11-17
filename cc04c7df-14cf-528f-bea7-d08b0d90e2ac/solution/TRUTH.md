# TRUTH

## Reward

R = 1[Stage I] x 1[Stage II] x (0.8 x (0.4 + 0.6 x f) + 0.2 x q). tests/test.sh decides the compiled
Stage I and Stage II vetoes first; a veto routes to tests/.gate/zero.toml with every item at zero and no
judge call; a clearing run reaches the gate route, which runs tests/.gate/judge.toml (all-pass) and then
scores tests/ over every compiled item and the judged quality items.

- Stage I compiled items: 6; Stage II groups: 20; Stage III families: 65
- test_output weights: Stage I and II 130, Stage III 195; judged q weight 32
- clearing route shares: Stage I and II 0.32, Stage III 0.48, q 0.2
- judge: anthropic/claude-opus-5-5, effort high, mode individual, timeout 600 s, one sample per call

Replay clock: every application start begins at 2026-03-15T11:00:00Z under libfaketime
(tests/.verifier/appctl.py), a restart on the same database (g15, g17, g19, g20) included, as State A was
recorded; g19 depends on it (after its second restart the offset row from the first start is the latest by
timestamp). A row written after such a restart can land on the very millisecond of an earlier start's row for
the same user, and the (user_id, utc_timestamp) key of user_initial_stats, user_offset_tc_stats,
user_total_stats or user_tc_stats_hourly refuses it. A group or family that disagrees while the grader's own
PostgreSQL log (pg.log) shows that refusal is replayed once more from a fresh database, its evidence and
verdict record the retry, and a second disagreement stands.

## Items

| id | stage | mode | weight | negate |
|---|---|---|---|---|
| s1_graded_build | I | compiled | 5 | false |
| s1_retired_stack_absent | I | compiled | 5 | true |
| s1_new_stack_serves | I | compiled | 5 | false |
| s1_no_transplant | I | compiled | 5 | true |
| s1_no_forwarding | I | compiled | 5 | true |
| s1_no_verifier_detection | I | compiled | 5 | true |
| g01_hardware_lifecycle | II | compiled | 5 | false |
| g02_team_lifecycle | II | compiled | 5 | false |
| g03_user_lifecycle | II | compiled | 5 | false |
| g04_batch_create | II | compiled | 5 | false |
| g05_authn_authz | II | compiled | 5 | false |
| g06_system_state_gating | II | compiled | 5 | false |
| g07_cache_validation_headers | II | compiled | 5 | false |
| g08_cors | II | compiled | 5 | false |
| g09_invalid_url_redirect | II | compiled | 5 | false |
| g10_competition_user_stats | II | compiled | 5 | false |
| g11_leaderboards | II | compiled | 5 | false |
| g12_historic_stats | II | compiled | 5 | false |
| g13_monthly_results | II | compiled | 5 | false |
| g14_lars_hardware_sync | II | compiled | 5 | false |
| g15_external_stats_parsing | II | compiled | 5 | false |
| g16_reset_and_rollover | II | compiled | 5 | false |
| g17_configuration_binding | II | compiled | 5 | false |
| g18_request_error_representation | II | compiled | 5 | false |
| g19_persistence_across_restart | II | compiled | 5 | false |
| g20_scheduled_parsing | II | compiled | 5 | false |
| s3_family_01 | III | compiled | 3 | false |
| s3_family_02 | III | compiled | 3 | false |
| s3_family_03 | III | compiled | 3 | false |
| s3_family_04 | III | compiled | 3 | false |
| s3_family_05 | III | compiled | 3 | false |
| s3_family_06 | III | compiled | 3 | false |
| s3_family_07 | III | compiled | 3 | false |
| s3_family_08 | III | compiled | 3 | false |
| s3_family_09 | III | compiled | 3 | false |
| s3_family_10 | III | compiled | 3 | false |
| s3_family_11 | III | compiled | 3 | false |
| s3_family_12 | III | compiled | 3 | false |
| s3_family_13 | III | compiled | 3 | false |
| s3_family_14 | III | compiled | 3 | false |
| s3_family_15 | III | compiled | 3 | false |
| s3_family_16 | III | compiled | 3 | false |
| s3_family_17 | III | compiled | 3 | false |
| s3_family_18 | III | compiled | 3 | false |
| s3_family_19 | III | compiled | 3 | false |
| s3_family_20 | III | compiled | 3 | false |
| s3_family_21 | III | compiled | 3 | false |
| s3_family_22 | III | compiled | 3 | false |
| s3_family_23 | III | compiled | 3 | false |
| s3_family_24 | III | compiled | 3 | false |
| s3_family_25 | III | compiled | 3 | false |
| s3_family_26 | III | compiled | 3 | false |
| s3_family_27 | III | compiled | 3 | false |
| s3_family_28 | III | compiled | 3 | false |
| s3_family_29 | III | compiled | 3 | false |
| s3_family_30 | III | compiled | 3 | false |
| s3_family_31 | III | compiled | 3 | false |
| s3_family_32 | III | compiled | 3 | false |
| s3_family_33 | III | compiled | 3 | false |
| s3_family_34 | III | compiled | 3 | false |
| s3_family_35 | III | compiled | 3 | false |
| s3_family_36 | III | compiled | 3 | false |
| s3_family_37 | III | compiled | 3 | false |
| s3_family_38 | III | compiled | 3 | false |
| s3_family_39 | III | compiled | 3 | false |
| s3_family_40 | III | compiled | 3 | false |
| s3_family_41 | III | compiled | 3 | false |
| s3_family_42 | III | compiled | 3 | false |
| s3_family_43 | III | compiled | 3 | false |
| s3_family_44 | III | compiled | 3 | false |
| s3_family_45 | III | compiled | 3 | false |
| s3_family_46 | III | compiled | 3 | false |
| s3_family_47 | III | compiled | 3 | false |
| s3_family_48 | III | compiled | 3 | false |
| s3_family_49 | III | compiled | 3 | false |
| s3_family_50 | III | compiled | 3 | false |
| s3_family_51 | III | compiled | 3 | false |
| s3_family_52 | III | compiled | 3 | false |
| s3_family_53 | III | compiled | 3 | false |
| s3_family_54 | III | compiled | 3 | false |
| s3_family_55 | III | compiled | 3 | false |
| s3_family_56 | III | compiled | 3 | false |
| s3_family_57 | III | compiled | 3 | false |
| s3_family_58 | III | compiled | 3 | false |
| s3_family_59 | III | compiled | 3 | false |
| s3_family_60 | III | compiled | 3 | false |
| s3_family_61 | III | compiled | 3 | false |
| s3_family_62 | III | compiled | 3 | false |
| s3_family_63 | III | compiled | 3 | false |
| s3_family_64 | III | compiled | 3 | false |
| s3_family_65 | III | compiled | 3 | false |
| q_collaborators_injected | q | judged | 5 | false | D-MIGRATED-APP | craft |
| q_hand_written_singleton | q | judged | 3 | true | D-MIGRATED-APP | craft |
| q_authentication_single_point | q | judged | 5 | false | D-MIGRATED-APP | craft |
| q_state_gating_single_point | q | judged | 3 | false | D-MIGRATED-APP | craft |
| q_cors_single_point | q | judged | 1 | false | D-MIGRATED-APP | craft |
| q_scheduling_support | q | judged | 3 | false | D-MIGRATED-APP | craft |
| q_async_execution | q | judged | 3 | false | D-MIGRATED-APP | craft |
| q_startup_lifecycle | q | judged | 3 | false | D-MIGRATED-APP | craft |
| q_hand_maintained_route_table | q | judged | 1 | true | D-MIGRATED-APP | craft |
| q_request_state_in_shared_field | q | judged | 5 | true | D-MIGRATED-APP | craft |
| gate_native_container | I-gate | judged gate | 5 | false | RL-SHIM | constraints |

## Criteria

### s1_graded_build

Clean offline graded build: the submission tree is copied with every target/, .m2/, build output, archive (*.jar, *.war, *.ear, *.class) and IDE state removed; mvn -o -B -DskipTests package runs on JDK 17 with a fresh copy of the target set as the only repository; exactly one Spring Boot executable jar (manifest Main-Class org.springframework.boot.loader.* and a Start-Class) results; it starts with the bound environment and answers GET /folding/hardware within 180 s. Any resolve, compile, package or start failure vetoes, with the failing step and its first error line named in reward-details.json and the CTRF report.

### s1_retired_stack_absent

The resolved module graph (mvn -o dependency:list in the graded build), every pom.xml, every lockfile, every Dockerfile or compose file, and every source file name a retired coordinate or import a retired package, or the tree keeps a retired deployment descriptor (jboss-web.xml, jboss-deployment-structure.xml, beans.xml, ejb-jar.xml, standalone.xml, standalone.conf).

### s1_new_stack_serves

The listening socket on the bound port belongs to the started jar's JVM, that JVM has no child process, its loaded classes include org.springframework.web.servlet.DispatcherServlet and the Start-Class, every replayed request is dispatched through DispatcherServlet (handler-mapping probe), and no class from a retired package is loaded.

### s1_no_transplant

The share of the retired release's own source (the transplant corpus kept in the verifier image) that reappears in the tree as normalised verbatim token spans reaches the bound transplant_check.threshold (largest per-file containment, transplant_check below), or any compiled class of the State A war modules is present byte-identical in the tree or the jar.

### s1_no_forwarding

During replay the application's process tree starts a process whose executable or arguments name the retired stack (WildFly, JBoss, RESTEasy, Weld, GlassFish, Payara, TomEE, Java EE, standalone.sh, standalone.conf or standalone.xml, /opt/jdk-11 or /opt/m2/original), connects to a port that such a process listens on, or the JVM loads a class from an archive outside BOOT-INF/classes and BOOT-INF/lib of the built jar whose location names the retired stack or one of its packages. Other processes and connections are not checked at run time.

### s1_no_verifier_detection

A text file of the tree names a grader path (/tests, /logs/verifier, /var/lib/grader, /var/lib/grader-probe, /opt/grader-probe, /opt/grader-corpus or /grade) or a grader name (rewardkit, reward-details.json, ctrf.json, GRADER_ROUTE, GATE_VERDICT or grader-probe), or the running JVM opens any file under /tests or /logs.

### g01_hardware_lifecycle

Every recorded case of this capability agrees with State A: Hardware create, read, list, update, delete, including validation failures, duplicate conflict, unknown id and malformed id.

### g02_team_lifecycle

Every recorded case of this capability agrees with State A: Team create, read, list, update, delete, including validation failures, duplicate conflict, unknown id and malformed id.

### g03_user_lifecycle

Every recorded case of this capability agrees with State A: User create, read, list, update, delete, passkey masking in reads and the passkey endpoint, captain and category rules.

### g04_batch_create

Every recorded case of this capability agrees with State A: Batch creation of hardware, teams and users: all valid, partially valid and all invalid payloads, and the batch response shape.

### g05_authn_authz

Every recorded case of this capability agrees with State A: HTTP Basic authentication and roles: missing, malformed and wrong credentials, read-only versus admin, and the login endpoints.

### g06_system_state_gating

Every recorded case of this capability agrees with State A: Requests refused or allowed according to system state (startup, updating, resetting) with the recorded status and body.

### g07_cache_validation_headers

Every recorded case of this capability agrees with State A: Entity tag and cache-control headers on reads and conditional requests with If-None-Match.

### g08_cors

Every recorded case of this capability agrees with State A: Cross-origin headers on simple and preflight requests.

### g09_invalid_url_redirect

Every recorded case of this capability agrees with State A: Unknown paths under /folding redirect to the configured REDIRECT_URL with the recorded status and Location.

### g10_competition_user_stats

Every recorded case of this capability agrees with State A: Team-competition statistics per user, including manual offsets and hardware multipliers.

### g11_leaderboards

Every recorded case of this capability agrees with State A: Team and category leaderboards, including an empty system and ties.

### g12_historic_stats

Every recorded case of this capability agrees with State A: Historic user and team statistics by day, month and year, including invalid and out-of-range dates.

### g13_monthly_results

Every recorded case of this capability agrees with State A: Monthly result reads and manual save.

### g14_lars_hardware_sync

Every recorded case of this capability agrees with State A: LARS hardware synchronisation against the grader's stub: multipliers, new and retired hardware.

### g15_external_stats_parsing

Every recorded case of this capability agrees with State A: Manual stats update against the grader's Folding@Home stub: points, units, multiplied points, and an unreachable or failing stub.

### g16_reset_and_rollover

Every recorded case of this capability agrees with State A: Manual reset and month rollover of team-competition statistics.

### g17_configuration_binding

Every recorded case of this capability agrees with State A: Environment-bound configuration: category capacity limits (USERS_IN_*), stub roots, redirect URL and feature flags.

### g18_request_error_representation

Every recorded case of this capability agrees with State A: Malformed JSON, missing body, unsupported media type and unsupported method: the recorded status and error representation.

### g19_persistence_across_restart

Every recorded case of this capability agrees with State A: State written through the API survives an application restart against the same database, and cached reads agree with uncached reads.

### g20_scheduled_parsing

Every recorded case of this capability agrees with State A: Scheduled stats parsing fires according to the STATS_PARSING_SCHEDULE_* values and is disabled by ENABLE_STATS_SCHEDULED_PARSING=false.

### s3_family_01

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_02

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_03

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_04

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_05

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_06

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_07

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_08

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_09

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_10

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_11

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_12

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_13

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_14

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_15

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_16

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_17

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_18

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_19

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_20

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_21

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_22

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_23

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_24

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_25

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_26

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_27

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_28

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_29

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_30

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_31

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_32

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_33

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_34

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_35

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_36

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_37

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_38

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_39

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_40

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_41

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_42

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_43

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_44

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_45

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_46

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_47

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_48

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_49

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_50

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_51

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_52

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_53

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_54

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_55

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_56

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_57

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_58

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_59

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_60

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_61

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_62

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_63

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_64

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### s3_family_65

Template, one item per frozen Stage III counterexample family: the family's executable input replayed deterministically produces the State A result recorded for it.

### q_collaborators_injected

Every collaborator that State A's container supplied to a class by @EJB or @Inject injection reaches the migrated class from the Spring container, through constructor, setter or field injection or through construction inside Spring configuration from injected beans.

Residue: how a class acquires its collaborators spans many files and many equivalent Spring forms; whether each acquisition is container-supplied rather than constructed or looked up is a reading of the code, not a pattern over named evidence

### q_hand_written_singleton

A component that State A ran as a singleton or stateless session bean is held in the migrated code as a hand-written singleton: a static field or static accessor in the migrated code constructs and holds its one instance.

Residue: whether a static holder is a hand-written replacement for a container singleton, rather than framework-neutral code State A already had, needs the component's provenance in State A

### q_authentication_single_point

The authentication and role check that State A applied to requests in its SecurityInterceptor request filter is applied in one place for every request, through a Spring web extension point: a servlet filter, a handler interceptor or Spring Security configuration.

Residue: behaviour shows the check happens, not where; placement in one extension point versus repetition across endpoint methods is a reading of the code

### q_state_gating_single_point

The system-state gating that State A applied to requests in its StateInterceptor request filter is applied in one place for every request, through a Spring web extension point: a servlet filter or a handler interceptor.

Residue: as for authentication: behaviour shows the gating, not its placement

### q_cors_single_point

The cross-origin response headers that State A's CorsFilter response filter added to every response are added in one place, through a Spring web extension point or Spring's own cross-origin configuration.

Residue: behaviour shows the headers, not whether one mechanism or many produce them

### q_scheduling_support

The recurring and calendar work that State A ran on EJB timers (statistics parsing, start-of-month and end-of-month processing, and the LARS hardware update) is scheduled through Spring's scheduling support: @Scheduled, a TaskScheduler or a SchedulingConfigurer.

Residue: the start-of-month, end-of-month and LARS schedules cannot fire inside a graded run on the application clock, so only the code shows what drives them

### q_async_execution

The statistics parsing that State A ran asynchronously through an EJB @Asynchronous method runs asynchronously under Spring's asynchronous execution support: an @Async method with asynchronous execution enabled, or a Spring-managed task executor.

Residue: a silently synchronous parse returns the same bodies; only the code shows whether asynchronous execution is enabled and Spring-managed

### q_startup_lifecycle

The cache warm-up and system-state initialisation that State A ran in its @Startup singleton runs once at application start through Spring's application lifecycle: an application runner, an application event listener or a bean initialisation callback.

Residue: a lazy first-request initialisation answers every recorded request identically; only the code shows when the work runs

### q_hand_maintained_route_table

A filter, interceptor or controller in the migrated code decides whether a request matches an endpoint by comparing the request path against a hand-maintained list of paths or patterns.

Residue: the redirect for unmatched paths behaves identically either way; a second route table beside Spring MVC's handler mapping shows only in the code

### q_request_state_in_shared_field

A migrated endpoint class that the Spring container holds as one shared instance stores request-specific data (the request URI, headers, parameters or credentials) in an instance field that a later request overwrites.

Residue: State A's request-scoped endpoints held per-request context in fields; in a shared controller that becomes a race no sequential recorded case or family replay can show

### gate_native_container

One standard, about provenance and shape, not behaviour. Does this application genuinely run on Spring's own container, dependency injection and configuration model? A finding is: the retired framework's programming model reproduced in shape rather than retired (home-grown @Path/@GET/@Stateless/@Schedule-style annotations with a processor or reflection layer that re-creates JAX-RS resource dispatch, EJB lifecycle or CDI resolution); an adapter, bridge or compatibility layer over the State A application; or a request path whose default route is served by something other than the Spring stack. Not a finding: hand-written code that reproduces what the old service did in the submission's own Spring shape; that is the assignment. Framework-neutral code State A already had, carried over unchanged (such as its own environment-variable helper), is not a finding. A mention in prose, comments or a changelog is not a finding. Every fail must cite a path, a line number and a verbatim quote from the submission; a fail whose citation is missing, does not exist, or does not show the claimed shape counts as a pass.

Residue: whether code reproduces the retired framework's programming model in shape, wraps the State A application, or routes the default path around the Spring stack is a provenance and shape reading across the tree; equal behaviour and the compiled transplant and forwarding checks cannot decide it

## Fixtures

| fixture | sha256 |
|---|---|
| tests/.data/retired.json | d924fb094a3927823267b8fcc897a6180cfa4c2b1c4f6a83044b1251568ad491 |
| tests/.data/stage2/g01_hardware_lifecycle.json | 44a4d4ce2a8fc4d45b614fe474c97dd9fca4e57e57b08acca1cb4fd42d3dabbb |
| tests/.data/stage2/g02_team_lifecycle.json | dbe5d9d39f6c59a5c64d590cb753e271797cf55b5abf31fd8aafd68a4b631606 |
| tests/.data/stage2/g03_user_lifecycle.json | 70d747bd9317a33855c89915138120c2a4bfbf18cf560833d994c55e10d69fa6 |
| tests/.data/stage2/g04_batch_create.json | 0e7227f28b2adda6165867d2ebaa402b2118fd8dc00afbbe639921847aaac624 |
| tests/.data/stage2/g05_authn_authz.json | 5d7cc5d94b2ebf05527ccfb83a6e1bc536318a3cfcddf79954927683569c8ba2 |
| tests/.data/stage2/g06_system_state_gating.json | 52cbf7f203023a1f97388817a227e58c343df2d0da2d7656accdd2f9f05c9cf7 |
| tests/.data/stage2/g07_cache_validation_headers.json | 25616b4de6a312706d93a1a3960320118c2014bca2935d5e5e89401b84e9f6bf |
| tests/.data/stage2/g08_cors.json | 4f4956ed38e66ec910828162faccc426a791c00a12b7d72c0842f55573b5ce93 |
| tests/.data/stage2/g09_invalid_url_redirect.json | e1ae9ea7d04e53395e7ab3f244b535fb0c354e133fd7d549d7e006c00b7a2fdb |
| tests/.data/stage2/g10_competition_user_stats.json | 8be4b9bdac6c2bc76d6e7136363b07c04e481dc7643d393c0e7ce6bba7b989aa |
| tests/.data/stage2/g11_leaderboards.json | 31dcd9186228135316fba9da90ea882e32b9b3cbd093bb7db49d4857769dcf25 |
| tests/.data/stage2/g12_historic_stats.json | 566d9960e78050c80d3731cf50fb474fd2bc70dc37a1e2c69538fa8d0eee0d76 |
| tests/.data/stage2/g13_monthly_results.json | 041a1665274549a63b4896f55c0dd0dadc2ca2c66dde4a2594909836b49e0f0f |
| tests/.data/stage2/g14_lars_hardware_sync.json | 4a52608d0532628f2b3c0e2c49ca2057333b273a5514a68b2b319a8bb5337e15 |
| tests/.data/stage2/g15_external_stats_parsing.json | 726ec8502ed3bf808e90c915d017c401725e9e11bc9336c99e4455a8efdaab15 |
| tests/.data/stage2/g16_reset_and_rollover.json | e196afcd96b26ff8b8e7956d2138bc4771e4b760763530b06ea4acbd9a2a76ce |
| tests/.data/stage2/g17_configuration_binding.json | 34be86ef403b2b6a8cf0ec0c35b9766f817aeaf75202879f161382ee0cce9ae8 |
| tests/.data/stage2/g18_request_error_representation.json | 36f9f0e2d6cb1442472a976890d753af56ecdcb769bbf1b194922bc33017b1bc |
| tests/.data/stage2/g19_persistence_across_restart.json | b69fbab7797164aaf49a8a07fad351e7bedb1cfd4fea4e2250264fe19a3b5d98 |
| tests/.data/stage2/g20_scheduled_parsing.json | d7280132c9fb8a213c80e190b656dad865e1ec0fe34c4768084be84f2b7165a4 |
| tests/.data/stage3/s3_family_01.json | 1255bd958c1a41d02fe0e661332b05c4e93780f0a9c239e9f1a418e8ba7702bb |
| tests/.data/stage3/s3_family_02.json | e737e5ae73ed57590088555a6fcb4d619956ac9454bf88b966725c35009ae872 |
| tests/.data/stage3/s3_family_03.json | 9309750b1a36e742229cb921a971a24981588078ef9a2acedd8088174c28b999 |
| tests/.data/stage3/s3_family_04.json | 59d9adab164a6b0476d7a1ec6ec447916f170dc449c3a28e86042f6f78b45089 |
| tests/.data/stage3/s3_family_05.json | 403881181fb6b48c307dafb9be7076a8ca475bcf66e8574d63b3dec0bd05025e |
| tests/.data/stage3/s3_family_06.json | cd7b244f9a92b55251229732a37091c532e1a3bf2b886d059e8c69a65640e1a3 |
| tests/.data/stage3/s3_family_07.json | bb1d2d9f9f77de9c026c1aef339a8848f950976a4c5d6868999a89bba02f63f7 |
| tests/.data/stage3/s3_family_08.json | 3f364de26b3c84f2dcf3368e059d354491c74e90ef80408b1d3df4bca525680f |
| tests/.data/stage3/s3_family_09.json | cb8644245e6416247f84114a6c4aef91115dd0a57fc60d556b7b15c6ec5024f2 |
| tests/.data/stage3/s3_family_10.json | cb89ef797f62bffac6243ce0f3b8add7ae4107a4d8e9de4bbef991403062ea24 |
| tests/.data/stage3/s3_family_11.json | 73352c7c51ed44aa738a764ceb0edd0e252fcfbaf24ab5fb1beb32d5b434fc9b |
| tests/.data/stage3/s3_family_12.json | 28eacb83bb15563ff4c04fa664b1e91c1a5d95d6126583274ff63a1f76d4a1f7 |
| tests/.data/stage3/s3_family_13.json | 8049e8b17c0cc9e3be013baf578034bc4316ac5da309721aea4c7fc05478082d |
| tests/.data/stage3/s3_family_14.json | d0cf9f27efb69734636e062dc63822d5c3276dcf4550b87c8224f69a5c00ada6 |
| tests/.data/stage3/s3_family_15.json | e5bffca4b92122b65f75af82caa07599f1e9a3784a41918988f5f455ab5f47b4 |
| tests/.data/stage3/s3_family_16.json | 8cd2f7b90bbe0ca40afe61f2304c5c482d43cc1885deac43739ec4db77f97e6b |
| tests/.data/stage3/s3_family_17.json | b1af77b0de4f736c29e99535cc11f627c30db79ce7775837548ef2326ff3536f |
| tests/.data/stage3/s3_family_18.json | 10890d4cece8167fabe3ee2a400ed002b091e277cbef97dcdab6b7d54aa549a4 |
| tests/.data/stage3/s3_family_19.json | 5b8d3a7f8064e4a6925b7c7b1fd484d067302ef02c2fdce86dd5a9ec99ae343a |
| tests/.data/stage3/s3_family_20.json | 533fb25e2473c97000671aca43565cb57322a3d7379c1d8534c030dadeb8e1bc |
| tests/.data/stage3/s3_family_21.json | 6e080b52865cf0eabacbdcc246e3909fb2d99ec477cf69eeb99a65a4e45dad78 |
| tests/.data/stage3/s3_family_22.json | b53bb14eed4200993fffae7cac9e7113aeaa250623b3813c8430c8bc949f7af7 |
| tests/.data/stage3/s3_family_23.json | de48b9d5a4dd61e1d8b6b010df72981c202f10232d4db97cec4d4c05b428c8cd |
| tests/.data/stage3/s3_family_24.json | 64452c5721c295455572c1da5e9ad686df776f3105a527f8a7b695471b89cca8 |
| tests/.data/stage3/s3_family_25.json | 325a8891c650cce3d39d4f8e2b8354af7646615e6cd0f32135973be086188464 |
| tests/.data/stage3/s3_family_26.json | 5727a11e4eaea86b9a935ab80938d5df9d07f1f77ba098d852df66693a55aec9 |
| tests/.data/stage3/s3_family_27.json | b3e872098ed53707226aa1f9193aabad3adf9d63bc4c685b1db631076d4dba6b |
| tests/.data/stage3/s3_family_28.json | 3363210b10056e6fc5c4c481c7dc5df8afefc91ad92064e600f8c70bca76bc70 |
| tests/.data/stage3/s3_family_29.json | e14ea0e5219dbcb36f7af0bd8c8782f7858a19e90906ffa0d47c0be59889c10b |
| tests/.data/stage3/s3_family_30.json | 68608351048d7d906166c87f934084a26e3b949c5b89a807c7f266abec07414b |
| tests/.data/stage3/s3_family_31.json | 32a00fd4c6354b9e8350ac2f10948ce60dca3afed30f5c07aba787fa07335bda |
| tests/.data/stage3/s3_family_32.json | 33343210da6d9244995f791da390f13cbffbe69534dd669ea31a86a762dfc243 |
| tests/.data/stage3/s3_family_33.json | 3762efe1fd787bed7767cad7def13f39e9ea4119046446bb8a8c09848d7f3ba0 |
| tests/.data/stage3/s3_family_34.json | fd943b3447081a771cad8ad24cbc5f3f974de6e078d32324ceda17ccc7de9638 |
| tests/.data/stage3/s3_family_35.json | 72d532608f95d59652d9062b5788ed13b9f42ce5e52cda049d763337e01884a3 |
| tests/.data/stage3/s3_family_36.json | e9f6c15630d8e3a2870957d4d3e9484978fb0ba5cc9c0f40be4459a7c4334071 |
| tests/.data/stage3/s3_family_37.json | ac64090949bcfd04a4526b6496a172c2390c506c5b3da58ffbecac7b7dd7ac14 |
| tests/.data/stage3/s3_family_38.json | 859ed86358985dbff190635ee1cf824049d36d61d6246bd89eec05bb6f209ea3 |
| tests/.data/stage3/s3_family_39.json | 9d035f26a45d1f35743b252414be83dea3c6cfd8a4907721bb6b3965bb6dbbab |
| tests/.data/stage3/s3_family_40.json | 32e1224e405a086270f9b76affd56d0753bf463131ef46ffe8f9a57b03adf041 |
| tests/.data/stage3/s3_family_41.json | 3b370babc49de90210a4cbf9429a55b7c0703bccc21443b50f8353658ec4bbee |
| tests/.data/stage3/s3_family_42.json | 9ab8a2b0a15d552d6984c5fc26a95d3523e480cf0e8fe28dc36b035b66f78e07 |
| tests/.data/stage3/s3_family_43.json | d5c066912342bc8735314bd45a21554d29c98f9647e34a337a4b776357dd745b |
| tests/.data/stage3/s3_family_44.json | d8c2084fc8728b82b65c0dcb496712369206c574ce5afa45ac029e021da20f7c |
| tests/.data/stage3/s3_family_45.json | bd57dc2f26d8af1e810f815408f35607e9e4d8d673593f2ca1d8dceeb21dbd46 |
| tests/.data/stage3/s3_family_46.json | dbca2f66719bcb2a8d99dc8ad574edcded33d50946bd53b183f42fc00659e3eb |
| tests/.data/stage3/s3_family_47.json | b056d3b9d35ab0f1657668d50dac3835494405cdd9e478a0575a4ea7426f3881 |
| tests/.data/stage3/s3_family_48.json | 2e6aa0072d61551e42acb56ee62c53721e0ec4d51b0e75f898cf414b8b25e456 |
| tests/.data/stage3/s3_family_49.json | aa09cb46f674ab88c8e8d56a954b89a6ec3d049236193e53a9ef8be9f8b716f8 |
| tests/.data/stage3/s3_family_50.json | fb2a49730f15f849c38dcc62d52e232b884e0f9be45dbe2cd08aa169228497a6 |
| tests/.data/stage3/s3_family_51.json | 3cc90c0a1e2aaee274de594a1408f6bf536fad6d6729c40a9a3097021781dfe0 |
| tests/.data/stage3/s3_family_52.json | f0c9aac10fa530c977d4aa0fb0cabe5b00ebc85907b5a6ce363c7bb5ffe2b22a |
| tests/.data/stage3/s3_family_53.json | d88b2e41670079aa79f0804715f6a09c243948d3b767a6d02b029b3141410293 |
| tests/.data/stage3/s3_family_54.json | 04b9df8e68e040d732aa79fa8dacf1168e7189f295919b8e85d4cb99a76fc439 |
| tests/.data/stage3/s3_family_55.json | 319af079953a9a009e52d3e58fc9d039f269aa744034d0e68ede46ba3d85c35a |
| tests/.data/stage3/s3_family_56.json | 3373d58df00c06627fa62213094d4ec833b0f5cd48e1d53c3dda4bbd4eb1a34d |
| tests/.data/stage3/s3_family_57.json | f5ac5c5d67a326213b2e1d95933d2b227b0039194ef4126f29ff0fae1cdd3e67 |
| tests/.data/stage3/s3_family_58.json | 608655482641ca26dced79a6da14239ac513cad9e1fcb7ba6a577aa00bee0e5c |
| tests/.data/stage3/s3_family_59.json | 4132ffac63ddf54d69e25fd3eee32508a6f77623924252efdf382a096db80fe3 |
| tests/.data/stage3/s3_family_60.json | 7134f69a381bbf2d9721c6fc43a4a45843ba87dbd815a77c72486ffe782962cd |
| tests/.data/stage3/s3_family_61.json | f37e7a4e23ca06b8c865daf215412a5648cd476f93d967f7a3603a6aff18b4f9 |
| tests/.data/stage3/s3_family_62.json | 1feb2b2a662f68274a797a17d9ff8c042f1e767cd10573e5c2c3b63f9db8b42e |
| tests/.data/stage3/s3_family_63.json | a62e91cb48ff27c045135ff4c6fffec5a9cb13ca5083ecb5b6724c5f3d2d79f0 |
| tests/.data/stage3/s3_family_64.json | 6d8f9ff37ed2fd270f9a0563a4189f67f23a581cfd30c04b71c612ab23bf7c86 |
| tests/.data/stage3/s3_family_65.json | 90eb5e9c5358df87047270cf65500512527eec724abd0714bd5887c2022458ee |
| tests/.data/statea-classes.sha256 | d76d1ff0b5a00f81ad238528eb72a36687432e007bdf627c1f074b4f079a8caa |
| tests/.data/statea-source.json | e22d8c4af22462e5cf64f6b5d089a38c6cfb147cef1aa8a902f4d7a1e54dd589 |
| tests/.data/statea-tree.sha256 | 21358824659429e0e946e30ee319c6a8b27c6549e366a61067ecf3002a51c4a6 |

## Oracle

solution/solve.sh applies reference.patch (sha256 b1ecea98a263e8b24e579631050df62968ebc797377365ef623f42df74072576 with its canary values set to zeros) to State A at 03b2650225ee8926b62ecffc4b01bed305c0bf5f; the patch is the upstream merge 9306831d8f13d068f54b32fc797474921ca1bc7b plus the 28 repairs below. Outside the paths they touch the reference takes the merge verbatim, and State A is the merge's first parent, the base commit.

Repairs over the upstream merge (the merge dropped server behaviour State A had, and the task is graded against State A's recorded behaviour):

1. Restores State A's empty-database startup bootstrap: getFirstHourlyTcStats() through DbManager, PostgresDbManager, Storage and StatsRepository, and an initTcStats() startup step that starts an asynchronous parse when no hourly stats exist (FoldingStatsApplication).
2. Adds @EnableAsync, without which the @Async stats parse runs on the caller's thread (FoldingStatsApplication).
3. Builds the parse schedule from the STATS_PARSING_SCHEDULE_* environment variables in UTC through SchedulingConfigurer, where the merge hard-coded 0 55 * 3-31 * * (StatsScheduler).
4. Restores manualTeamCompetitionStatsParsing(ProcessingType), its per-team iteration with the per-team no-users warning, and the serialising lock that stood in for the original singleton's container lock (StatsScheduler).
5. Moves the ParsingState and SystemState transitions from the parser into the scheduler, so parsing one user no longer flips the system into UPDATING_STATS; the parser keeps parseTcStatsForUser and parseTcStatsForUserAndWait (UserStatsParser, StatsScheduler).
6. Points every caller of the old collection parser at the scheduler's manual parse, as State A's call graph had it (FoldingRepository, UserStatsResetter, TeamCompetitionStatsEndpoint).
7. Injects the scheduler into the repository @Lazy, breaking the circular bean dependency repair 6 creates (FoldingRepository).
8. Drops the merge's isEmptyStats() guard, so a user who changes team always gets a retired stats row, as in State A (FoldingRepository).
9. Restores POST /hardware/batch, /teams/batch and /users/batch with the BatchCreateResponse payload and their partial-success semantics (HardwareEndpoint, TeamEndpoint, UserEndpoint, new BatchCreateResponse).
10. Serialises new ArrayList<>() instead of Collections.emptyList(), which Gson cannot reflect into on JDK 17 (MonthlyResult, LeaderboardStatsGenerator).
11. Makes the unmatched-URL handler a @RestControllerAdvice, so an unknown URL still answers 303 with the REDIRECT_URL Location (InvalidUrlRedirecter).
12. Adds the original server's answers that Spring Boot renders differently: 405 with Allow, 415, 406, 400 with a plain HTML page for an unreadable payload, and 500 with an HTML page after a response reset, so errors carry no cross-origin headers (InvalidUrlRedirecter).
13. Sets spring.mvc.throw-exception-if-no-handler-found and turns off spring.web.resources.add-mappings, without which repair 11 never fires (application.properties).
14. Disables the Actuator endpoints the merge exposed, as the original exposed none (application.properties).
15. Removes logging.config=file:/var/backend/logback.xml, a file the built image does not carry (application.properties).
16. Makes the security and state interceptors set the status and return false instead of throwing @ResponseStatus exceptions, so a refusal is a bare status (SecurityInterceptor, StateInterceptor).
17. Deletes the three marker exceptions repair 16 leaves unused (ForbiddenException, ServiceUnavailableException, UnauthorizedException).
18. Lets a request through when validation itself throws, as the original filters did, and drops the dead preflight detection (SecurityInterceptor, StateInterceptor).
19. Stops exempting /login/admin from the state check, as in State A (InterceptorRegister).
20. Adds MediaTypeInterceptor, registered first, which answers 415 to a JSON endpoint called without a Content-Type (new MediaTypeInterceptor, InterceptorRegister).
21. Marks every @RequestBody and the two name-lookup @RequestParams required = false, so the hand-written null and validation answers are reached (HardwareEndpoint, TeamEndpoint, UserEndpoint, LoginEndpoint, TeamCompetitionStatsEndpoint).
22. Takes the manual update's async parameter as a string parsed with parseBoolean, so a non-boolean value means false instead of a 400 (TeamCompetitionStatsEndpoint).
23. Sends Location only with 201 created, not with 200 (Responses).
24. Adds no-transform to Cache-Control, which JAX-RS emitted by default (Responses).
25. Re-implements the JAX-RS preconditions on cached responses: If-Match naming neither * nor the tag answers 412, and on GET and HEAD If-None-Match naming * or the tag answers 304 (Responses).
26. Restores State A's CORS header set and order and answers OPTIONS as text/plain (AccessControlFilter).
27. Makes the three stubbed Folding@Home endpoints thread-safe with ConcurrentHashMap and an atomic merge (StubbedLarsEndpoint, StubbedPointsEndpoint, StubbedUnitsEndpoint).
28. Removes the javaee-web-api property and dependency-management entry the merge left behind (pom.xml).

## Canary

<!-- CANARY-BEGIN -->
<!-- CANARY-SLOT-1: b4feaf386e9a333ecede5214ecae6932cb90beefaad959c16f1f021adf37e2ee -->
<!-- CANARY-SLOT-2: 6c60abfcec2fb775da6dd799e22b496bfa4cbb3e32ae97d77d667ae2710d5e2b -->
<!-- CANARY-SLOT-3: 8a478e777b2c9a466dc8145c95cb275027faf03e1a5cbe966e8fb637c1d81bae -->
<!-- CANARY-END -->
