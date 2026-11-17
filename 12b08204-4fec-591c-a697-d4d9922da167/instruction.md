# Migrate the Argus web backend from Flask and uWSGI to FastAPI and Gunicorn

`/app` holds Argus, ScyllaDB's test-run tracking service. Its web backend is a Flask application
served by uWSGI. Move the web backend to FastAPI (on Starlette), served by Gunicorn with Uvicorn
workers, so that every client of the service, its browser UI and its API clients alike, sees the
same application it sees today. The frontend under `frontend/` and the client library under
`argus/client/` are not part of this task.

## The retired stack

When you are done, nothing in the tree, its dependency declarations (`pyproject.toml` extras,
dependency groups and build requirements), its lockfile (`uv.lock`) or its launch and deployment
files depends on, imports (including through `importlib.import_module` or `__import__`) or
starts any of these:

- Flask, Flask-Login, Flask-WTF, WTForms, prometheus-flask-exporter, Blinker and uWSGI
  (including `uwsgi.ini` and every `uwsgi` or `flask` invocation in the launch and deployment
  files);
- any distribution or module that adapts a WSGI application to ASGI or serves WSGI: a2wsgi,
  asgiref, the WSGI middleware of Starlette, FastAPI and Uvicorn, `werkzeug.serving`,
  `werkzeug.wsgi` and `wsgiref`.

Wrapping the existing Flask application, or its handlers, in a bridge or adapter is not a
migration, and neither is re-creating Flask's programming surface (request, `g`, `session` or
`current_app` proxies, blueprints, context stacks) inside the new framework for the handlers to
keep using: write the handlers in FastAPI's own request, dependency, routing and response model.
Werkzeug may stay for one purpose only: `werkzeug.security` for password hashing. Copying the
retired stack's source into the tree, under any name, is not allowed, and nothing in the tree may
look for, read or adapt to anything outside `/app` beyond what this statement describes, or to the
identity of the tree itself.

A coroutine (`async def`) route handler, or a coroutine dependency a route depends on, must not
call the synchronous database driver, or a model, query or service method that queries the
database, directly on the event loop, and a coroutine route handler must not do other blocking I/O
there either (a request through a synchronous HTTP client, a blocking file read or write, a sleep):
hand such a call to a worker thread, or write the handler as a plain `def`. Per-request HTTP
metrics come from the one request instrumentation the application installs for every route; no
handler counts requests or observes latency or status codes in its own body.

## What is installed

The environment is offline.

- `/opt/wheels/target` is the only set of distributions you can install, and it is exactly the
  set the clean rebuild below installs from. `uv` and `pip` are configured to use it. It holds:
  annotated-doc 0.0.5, annotated-types 0.7.0, anyio 4.12.1, argcomplete 3.6.3, asttokens 3.0.1,
  attrs 25.4.0, autopep8 2.3.2, boto3 1.38.46, boto3-stubs 1.38.46, botocore 1.38.46,
  botocore-stubs 1.42.26, certifi 2026.1.4, cffi 2.0.0, cfgv 3.5.0, charset-normalizer 3.4.4,
  click 8.3.1, codecov 2.1.13, colorlog 6.10.1, coodie 1.7.3, coverage 7.8.2, cryptography
  46.0.3, decorator 5.2.1, defusedxml 0.7.1, dependency-groups 1.3.1, distlib 0.4.0, docker
  7.1.0, exceptiongroup 1.3.1, execnet 2.1.2, executing 2.2.1, fastapi 0.141.1, filelock 3.20.3,
  geomet 1.1.0, gunicorn 26.0.0, h11 0.16.0, httpcore2 2.12.0, httpx2 2.12.0, humanize 3.13.1,
  identify 2.6.16, idna 3.19, iniconfig 2.3.0, ipython 8.36.0, itsdangerous 2.2.0, jedi 0.19.2,
  jinja2 3.1.6, jira 3.10.5, jmespath 1.0.1, lz4 4.4.5, markupsafe 3.0.3, matplotlib-inline
  0.2.1, multi-key-dict 2.0.3, nodeenv 1.10.0, nox 2025.5.1, oauthlib 3.3.1, packaging 25.0,
  parso 0.8.5, pbr 7.0.3, pexpect 4.9.0, platformdirs 4.5.1, pluggy 1.6.0, pre-commit 4.5.1,
  prometheus-client 0.26.0, prometheus-fastapi-instrumentator 8.1.0, prompt-toolkit 3.0.52,
  ptyprocess 0.7.0, pure-eval 0.2.3, pycodestyle 2.14.0, pycparser 2.23, pydantic 2.12.5,
  pydantic-core 2.41.5, pygithub 2.8.1, pygments 2.19.2, pyjwt 2.10.1, pynacl 1.6.2, pytest
  8.3.5, pytest-cov 6.1.1, pytest-subtests 0.14.2, pytest-xdist 3.7.0, python-dateutil
  2.9.0.post0, python-jenkins 1.8.3, python-magic 0.4.27, python-multipart 0.0.32,
  python-slugify 6.1.2, pyyaml 6.0.3, requests 2.32.5, requests-mock 1.12.1, requests-oauthlib
  2.0.0, requests-toolbelt 1.0.0, ruff 0.11.13, s3transfer 0.13.1, scylla-driver 3.29.7,
  setuptools 80.9.0, setuptools-scm 9.2.2, six 1.17.0, stack-data 0.6.3, starlette 1.6.0,
  text-unidecode 1.3, tomli 2.4.1, traitlets 5.14.3, truststore 0.10.4, types-awscrt 0.31.0,
  types-s3transfer 0.16.0, typing-extensions 4.15.0, typing-inspection 0.4.2, urllib3 2.6.3,
  uvicorn 0.40.0, virtualenv 20.36.1, wcwidth 0.2.14, werkzeug 3.1.5, wheel 0.45.1, zstandard
  0.25.0.
- `/opt/argus-original` is the original Flask application on its own set (`/opt/wheels/original`),
  runnable as the reference for today's behaviour. `argus-db start` starts a local ScyllaDB on
  127.0.0.1:9042 (user and password `cassandra`); `argus-original sync`, `argus-original seed` and
  `argus-original start` then serve the original application on http://127.0.0.1:8010 with its
  own keyspace. `argus-original --help` and `argus-db --help` list every command.

## How the result is built and served

The build copies `/app` without any virtual environment, build output or cache you created,
and rebuilds it from clean and offline: it creates a new virtual environment with
`/opt/python/bin/python3` and runs
`uv pip install --offline --no-index --find-links /opt/wheels/target '.[web-backend,dev]'`
from the repository root, with `SETUPTOOLS_SCM_PRETEND_VERSION=0.0.0`. Nothing you built or
installed is carried over, so the project must resolve, build and install from the target set
alone.

It then writes its own `argus_web.yaml` in the repository root (the same keys as today's
configuration), sets `CQLENG_ALLOW_SCHEMA_MANAGEMENT=1` and an empty
`PROMETHEUS_MULTIPROC_DIR=/tmp/promdb-argus-metrics`, provides writable `/var/log/argus` and
`/var/lib/argus`, and runs from the repository root:

- the schema synchronisation command `python -m argus.backend.cli sync-models` on an empty
  keyspace, which must leave the same schema that today's
  `flask --app argus_backend:argus_app cli sync-models` leaves. Every command of today's `cli`
  group stays reachable as `python -m argus.backend.cli <command>`;
- the server, with
  `gunicorn --worker-class uvicorn.workers.UvicornWorker --workers 1 --bind 127.0.0.1:<port> 'argus_backend:create_app()'`,
  so `argus_backend.create_app()` must return the FastAPI application, and every route the
  original application serves must be served by FastAPI routes (or a static files mount for
  `/s/`).

Keep a `gunicorn.conf.py` in the repository root, which Gunicorn loads when started from there
with that command. It must keep the deployment behaviour `uwsgi.ini` gives today: the same worker
recycling (`max-requests`, Gunicorn's `max_requests`), the same graceful reload window
(`worker-reload-mercy`, Gunicorn's `graceful_timeout`), and, in
Prometheus multiprocess mode, removal of an exited worker's live metrics files from a Gunicorn
exit hook (`child_exit` or `worker_exit`). Started from
the repository root without those flags, it must run the application through Uvicorn's Gunicorn
worker.

## Behaviour

The migrated application answers every request the way the original does. Each HTTP exchange must
match the original's in its status code, its `Location` (path and query), its `Content-Type` media
type and charset, the names and attributes of the cookies it sets (`Path`, `HttpOnly`, `SameSite`,
`Secure`, and whether an `Expires` or `Max-Age` is present), and its body: JSON as data with key
order ignored and values exact, HTML and text with whitespace runs collapsed and CSRF tokens
masked, and Prometheus metrics by series names and label sets. The `Server`, `Date`,
`Content-Length`, `Connection`, `Transfer-Encoding` and `Cache-Control` headers, header order,
the session cookie's value, and identifiers and timestamps generated while serving need not
match.

Two changes from the original are intended, and only these two:

1. API responses no longer carry the `Cache-Control` headers some `api.py` endpoints set.
2. Two kinds of JSON API request are answered with HTTP 200 and the error envelope of FastAPI's
   request validation, with exception `RequestValidationError`, instead of the original's answer:
   a request with a path id that is not a UUID, which the original refuses with the `ValueError`
   it raises converting that id to a UUID (`badly formed hexadecimal UUID string`), and a request
   sent without a JSON `Content-Type` (`application/json` or an `application/*+json` type), which
   the original refuses with its `Content-Type mismatch` exception. For those requests only the
   HTTP status code, the envelope's `status` and its `response.exception` must match. Every
   other error keeps the original's answer exactly, including what the original answers
   when a parameter or body field is missing or has the wrong type (its own HTTP 400 answers, and
   the exceptions its handlers raise). A request the original application accepts is still
   accepted, whatever its `Content-Type`.

## Tests

Keep the repository's test suite and migrate it to the new framework: at least 95 percent of
the test ids that `python -m pytest --collect-only -q` collects in `/app` today (matched by node
id) must still be collected in the rebuilt tree, and no module-level skip may be added. A test that
sends an HTTP request to a route today must still send it as a request, through the new framework's
own test client and its response interface: not by calling the route's handler or the service code
beneath it directly, and not through a hand-made emulation of the old framework's test client (a
wrapper or adapter that reproduces its call signatures or response attributes).
