<p align="center"><img src="images/hero.svg" alt="Chiron" width="720"></p>

# Chiron SWE Refactor Bench: samples

<p align="center"><b>Five sample tasks from Chiron SWE Refactor Bench, a Harbor benchmark of whole-repository migrations where an unmigrated repository scores zero.</b></p>

<p align="center"><a href="#summary"><img src="https://img.shields.io/badge/tasks-5-845EF7" alt="tasks: 5"></a> <a href="#summary"><img src="https://img.shields.io/badge/harbor_schema-1.4-845EF7" alt="harbor schema: 1.4"></a> <a href="#trials"><img src="https://img.shields.io/badge/gpt--6.1--sol_mean_reward-0.0000-845EF7" alt="gpt-6.1-sol mean reward: 0.0000"></a></p>

<p align="center"><sub><a href="#abstract">Abstract</a> | <a href="#summary">Summary</a> | <a href="#tasks">Tasks</a> | <a href="#trials">Trials</a> | <a href="#coverage">Coverage</a> | <a href="#dataset-structure">Dataset structure</a> | <a href="#reproduction">Reproduction</a> | <a href="#cite">Cite</a> | <a href="#contact">Contact</a></sub></p>

## Abstract

Chiron SWE Refactor Bench is a benchmark of whole-repository migration tasks in [Harbor](https://github.com/harbor-framework/harbor) format, built to measure whether an autonomous coding agent can move a real repository onto a new technology stack without changing what it does. It targets [SWE Refactor Bench](https://arxiv.org/abs/2608.23564) (arXiv:2608.23564), which calls the weakness of behaviour-only benchmarks Blindness: an agent can copy the original implementation and still pass the tests. Only 28 of its 520 frontier-model runs passed all three of its evaluation stages. Each Chiron task hands the agent an open source repository and asks it to replace a language, framework, major dependency, or build toolchain across the whole repository while every observable behaviour of the original stays the same.

Every task runs its agent in an offline container and carries a reference solution, a verifier in its own container, and a continuous reward from 0 to 1. As in SWE Refactor Bench, the verifier checks that the migration happened, not only that behaviour was kept: veto checks confirm that the old stack is gone, replay behaviours recorded from the original, and ask an LLM judge to reject a new stack that only imitates the old interface, hands off to the old build, or carries the old implementation over in another form. One failed veto sets the reward to 0, so an unmigrated repository scores zero; otherwise the reward weights the grader's checks at 0.8 and a second LLM-judged rubric, for what tests cannot see, at 0.2. Chiron keeps SWE Refactor Bench's four kinds of migration (language, framework, platform, and build toolchain) but searches for hidden behavioural differences while a task is authored rather than at scoring time, so each verifier ships the tests that search produced.

This repository holds the five-task sample, each a different migration: a web framework (Flask and uWSGI to FastAPI), a build system (GNU Make to CMake), an implementation language (C to Rust), a major dependency (Pydantic 1 to Pydantic 2), and an enterprise application framework (Java EE on WildFly to Spring Boot). The sample covers Python, C++, C, Rust, and Java, and every task includes its reference solution and one recorded trial of `gpt-6.1-sol`. Task images hold each repository at its starting state, without its upstream history or any copy of the migrated code, and the agent's container has no network access; with web search also disabled for the trials, a later upstream version of the code is out of reach.

## Summary

| Property | Value |
| --- | --- |
| Tasks | 5 |
| Domain | Software engineering (whole-repository migration) |
| Languages | Python, C++, C, Rust, Java |
| Format | Harbor task, schema 1.4 |
| Platforms | linux/amd64, linux/arm64 |
| Average handling time (AHT) | 10 hours per task |
| Agent | Codex CLI 0.166.0 with `gpt-6.1-sol`, reasoning effort high |
| Judge | Claude Opus 5.5, reasoning effort high |
| Runs per task | 1 |
| Mean reward | 0.0000 |

## Tasks

| Task | Project | Language | Objective | Reference migration |
| --- | --- | --- | --- | --- |
| [`12b08204`](12b08204-4fec-591c-a697-d4d9922da167/) | Argus (`scylladb/argus`) | Python | Move the Argus web backend from Flask and uWSGI to FastAPI on Starlette, served by Gunicorn with Uvicorn workers, keeping every observable behaviour of the original application. | 122 files (+6,195 -5,447 lines) |
| [`235c695a`](235c695a-89b4-50e4-96bd-ee0d2ae80c7a/) | Yosys 0.66 | C++ | Move the Yosys build, its CI workflows, and its Python wheel packaging from GNU Make to CMake, keeping every observable behaviour of the original build. | 223 files (+5,837 -2,625 lines) |
| [`50b177e0`](50b177e0-3b23-5823-bb2b-f689f6de5909/) | yyjson 0.13.0 | C to Rust | Move the library's implementation from C to Rust (std, core, and alloc only, no crates) while keeping the public header, the C ABI, the CMake build, the installed tree, and every observable behaviour unchanged. | 11,480 lines of C removed, 52 exported entry points kept |
| [`78f33146`](78f33146-cdab-57ce-83fd-5f2e030dd52c/) | datalab (`datalab-org/datalab`) | Python | Upgrade the datalab research data management server, built on Flask and MongoDB, from Pydantic 1 to Pydantic 2 with every served behaviour unchanged, offline against a fixed wheel set. | 63 files (+7,232 -4,032 lines) |
| [`cc04c7df`](cc04c7df-14cf-528f-bea7-d08b0d90e2ac/) | folding-stats (`zodac-personal/folding-stats`) | Java | Move the multi-module Maven service from Java EE 8 inside WildFly 24 (JAX-RS, EJB singletons and timers, CDI) onto Spring Boot, so that it builds from clean as one executable jar on JDK 17 and keeps the recorded HTTP behaviour against the same PostgreSQL schema. | 258 files (+4,330 -6,171 lines) |

Reference migration sizes are the `git apply --numstat` totals of each task's `solution/reference.patch`, blank lines included. yyjson's reference is a Rust port in `solution/port/` that replaces the 11,480-line `src/yyjson.c`.

## Trials

Each task was run once, using Codex CLI 0.166.0 with `gpt-6.1-sol` at reasoning effort high and web search disabled, and scored by the task's own verifier. Trials are stored in each task's `trials/gpt-6.1-sol/`.

| Task | Reward | Cost (USD) | Agent time |
| --- | ---: | ---: | ---: |
| [`12b08204-4fec-591c-a697-d4d9922da167`](12b08204-4fec-591c-a697-d4d9922da167/trials/gpt-6.1-sol/) | 0.0000 | 4.50 | 1h 10m 27s |
| [`235c695a-89b4-50e4-96bd-ee0d2ae80c7a`](235c695a-89b4-50e4-96bd-ee0d2ae80c7a/trials/gpt-6.1-sol/) | 0.0000 | 2.85 | 47m 30s |
| [`50b177e0-3b23-5823-bb2b-f689f6de5909`](50b177e0-3b23-5823-bb2b-f689f6de5909/trials/gpt-6.1-sol/) | 0.0000 | 0.47 | 10m 12s |
| [`78f33146-cdab-57ce-83fd-5f2e030dd52c`](78f33146-cdab-57ce-83fd-5f2e030dd52c/trials/gpt-6.1-sol/) | 0.0000 | 1.26 | 16m 22s |
| [`cc04c7df-14cf-528f-bea7-d08b0d90e2ac`](cc04c7df-14cf-528f-bea7-d08b0d90e2ac/trials/gpt-6.1-sol/) | 0.0000 | 1.34 | 19m 3s |
| **mean** | **0.0000** | **2.08** | **32m 43s** |

Every trial failed at least one veto check, so none reached the judged rubric:

| Task | Vetoed on | What differed |
| --- | --- | --- |
| [`12b08204`](12b08204-4fec-591c-a697-d4d9922da167/trials/gpt-6.1-sol/verifier/) | G03, G14, G15 | Stage I passed. An anonymous `GET /admin/api/v1/users` answers "Authorization required" where the original answers "Forbidden"; `/alert_debug` sets no session cookie; `/release/by-id/{id}/planner` no longer redirects. |
| [`235c695a`](235c695a-89b4-50e4-96bd-ee0d2ae80c7a/trials/gpt-6.1-sol/verifier/) | S2-ci-test-build, S2-ci-test-compile, S2-ci-test-sanitizers | Stage I passed. The CI build and sanitizer jobs fail the `various` and `xprop` tests and the vanilla-test exit check, and the gcc-11 compile job fails its `banner` case. |
| [`50b177e0`](50b177e0-3b23-5823-bb2b-f689f6de5909/trials/gpt-6.1-sol/verifier/) | 24 checks (T-02 to T-17, T-23, T-24, P-01 to P-06) | The agent stopped after 10 minutes with 19 of the 52 exported symbols in Rust and the C implementation still building the library. |
| [`78f33146`](78f33146-cdab-57ce-83fd-5f2e030dd52c/trials/gpt-6.1-sol/verifier/) | S2-info, S2-errors | Stage I passed. 6 of the 295 cases in these two groups differ, among them `/info/blocks`, which gives `accepted_file_extensions: null` where the original gives `[]`. |
| [`cc04c7df`](cc04c7df-14cf-528f-bea7-d08b0d90e2ac/trials/gpt-6.1-sol/verifier/) | g06, g10, g15, g18 | Stage I passed. Responses to an invalid, empty, or non-boolean `async` value on the manual update, and to requests without a body, differ from the original (6 cases). |

Each trial folder is Harbor's trial output:

| Path | Contents |
| --- | --- |
| `config.json`, `lock.json` | Trial configuration and resolved run settings |
| `result.json` | Agent, model, token usage, cost, timings, and verifier reward |
| `agent/` | Agent trajectory (`trajectory.json`), Codex session transcript (`sessions/`), and agent output (`codex.txt`) |
| `verifier/` | `reward.txt`, `reward-keys.json`, `reward-details.json`, `ctrf.json`, test output, and the grader's own results |
| `artifacts/` | The migrated repository, collected from the paths declared in `task.toml`, with `manifest.json` |

Log files and caches are left out. datalab's collected `.git` is stored as `.git.tar` (extracting it in place restores the repository), and its virtualenv is not included.

## Coverage

| Keyword | Tasks |
| --- | --- |
| abi | 1 |
| behaviour-preservation | 1 |
| build-toolchain | 1 |
| c | 1 |
| cdi | 1 |
| ci-workflow-repair | 1 |
| cmake | 2 |
| dependency-upgrade | 1 |
| ejb | 1 |
| enterprise-framework-application-migration | 1 |
| fastapi | 1 |
| flask | 2 |
| framework-migration | 1 |
| framework-rewrite | 2 |
| gnu-make | 1 |
| gunicorn | 1 |
| java | 1 |
| java-ee | 1 |
| jax-rs | 1 |
| json | 1 |
| language-rewrite | 1 |
| maven | 1 |
| migration | 3 |
| mongodb | 1 |
| offline | 1 |
| postgresql | 1 |
| pydantic | 1 |
| python | 1 |
| rust | 1 |
| scylladb | 1 |
| spring-boot | 1 |
| starlette | 1 |
| uwsgi | 1 |
| wildfly | 1 |
| yosys | 1 |

## Dataset structure

Each task is one Harbor task in a directory named by its uuid.

| Task | Keywords |
| --- | --- |
| [`12b08204-4fec-591c-a697-d4d9922da167`](12b08204-4fec-591c-a697-d4d9922da167/) | behaviour-preservation, fastapi, flask, framework-migration, gunicorn, python, scylladb, starlette, uwsgi |
| [`235c695a-89b4-50e4-96bd-ee0d2ae80c7a`](235c695a-89b4-50e4-96bd-ee0d2ae80c7a/) | build-toolchain, ci-workflow-repair, cmake, gnu-make, migration, yosys |
| [`50b177e0-3b23-5823-bb2b-f689f6de5909`](50b177e0-3b23-5823-bb2b-f689f6de5909/) | abi, c, cmake, json, language-rewrite, migration, rust |
| [`78f33146-cdab-57ce-83fd-5f2e030dd52c`](78f33146-cdab-57ce-83fd-5f2e030dd52c/) | dependency-upgrade, flask, framework-rewrite, migration, mongodb, pydantic |
| [`cc04c7df-14cf-528f-bea7-d08b0d90e2ac`](cc04c7df-14cf-528f-bea7-d08b0d90e2ac/) | cdi, ejb, enterprise-framework-application-migration, framework-rewrite, java, java-ee, jax-rs, maven, offline, postgresql, spring-boot, wildfly |

```
<uuid>/
├── task.toml         runner manifest, with the prebuilt images pinned by digest
├── instruction.md    objective the agent reads
├── environment/      agent image: the repository at its starting state
├── tests/            verifier image, hidden from the agent
├── solution/         reference solution, with TRUTH.md (how the task is graded)
└── trials/
    └── gpt-6.1-sol/  the agent model's trial (Harbor trial output)
```

## Reproduction

Each task directory is a complete Harbor task: `task.toml` is the runner manifest, `instruction.md` the objective, `environment/` the agent's image, `tests/` the verifier, and `solution/` the reference solution. `task.toml` pins both images on public ECR by digest, so Harbor pulls them instead of building; the Dockerfiles in `environment/` and `tests/` build them. The trials above were produced with Harbor 0.24.0 on linux/arm64:

```bash
harbor run -p <uuid> -a codex -m openai/gpt-6.1-sol \
  --ak version=0.166.0 --ak reasoning_effort=high --ak web_search=disabled
```

The verifier's LLM-judged rubric uses `ANTHROPIC_API_KEY` (Claude Opus 5.5, `anthropic/claude-opus-5-5`).

## Cite

```bibtex
@misc{chiron2026,
  title        = {Chiron: Whole-repository migration tasks where an unmigrated repository scores zero},
  author       = {{Ethara.AI}},
  year         = {2026}
}
```

## Contact

Chiron SWE Refactor Bench is built by [Ethara.AI](https://github.com/Ethara-Ai).

Copyright (c) 2026 Green Rider Technology LLP (Ethara AI). All rights reserved.
