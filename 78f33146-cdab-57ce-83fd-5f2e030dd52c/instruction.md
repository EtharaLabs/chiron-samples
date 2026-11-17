# Migrate datalab from Pydantic 1 to Pydantic 2

`/app/pydatalab` is the Python server of [datalab](https://github.com/datalab-org/datalab), a research data management platform built on Flask and MongoDB. It is a git repository at the state you start from. The application is written against Pydantic 1. Migrate it to Pydantic 2 so that it runs on the target set below, while its observable behaviour stays exactly the same.

## The stack you retire and the stack you move to

- Retired: `pydantic >=1.0,<2.0` (the tree locks 1.10.26), with the `email` and `dotenv` extras it declares.
- Target: `pydantic==2.13.4`, `pydantic-core==2.46.4` and `pydantic-settings==2.15.0`, and every other distribution at exactly the version in `/opt/wheelhouse/LISTING.txt`.

`/opt/wheelhouse` holds the target set: these are the only packages installable in this environment, there is no network, and it is exactly the set the migrated tree must build and run against. It carries no Pydantic 1 release and no package that restores Pydantic 1 interfaces. uv is configured to use it (`UV_DEFAULT_INDEX=file:///opt/wheelhouse/simple`), so `uv lock` and `uv sync` work offline. The two git-sourced plugins of the `app-plugins-git` extra are mirrored locally so that `uv lock` can resolve them; they are not part of the install below. The sources of the target `pydantic`, `pydantic-core` and `pydantic-settings` are readable under `/opt/pydantic-src`.

## Required rebuild and behaviour

`/app/pydatalab` as you leave it must rebuild from clean, offline, against the target set alone. Nothing you built or installed carries over: virtualenvs, `__pycache__`, `*.egg-info`, `build/` and `dist/` are not part of the rebuild. Each of the two installs below must finish within 30 minutes, the replay within 30 minutes, and each settings case within 5 minutes. The rebuild then:

1. exports the pins of your `uv.lock` for the extras `server` and `apps` with their hashes (`uv export --frozen --no-emit-project --extra server --extra apps`), installs them with hashes required and no index from `/opt/wheelhouse`, then installs the project itself without dependencies. The `chat`, `deploy` and `app-plugins-git` extras and the `dev` dependency group are not installed;
2. starts the server (`pydatalab.main.create_app`) against a fresh MongoDB and replays recorded interactions through its HTTP API, through the settings object `pydatalab.config.CONFIG` loaded from environment variables, a `.env` file and the JSON config file, and through the documents the server stores in MongoDB. Each result must equal what the original application returns on Pydantic 1.
   - Identifiers the server generates (ObjectIds, refcode suffixes, task ids, Bokeh model ids) need not have the original's values, but the same identifier must recur wherever the original repeats it; a wall-clock timestamp must match only in its format (digit layout and timezone notation), not in its value or in whether it equals another timestamp; everything else, including the format of serialized dates, `null` fields, key sets, status codes and error bodies, must match.
   - One change is expected: the JSON schemas served under `/info/types` come from Pydantic 2's schema generator, so for each served schema only its `title`, the names of its `properties` and the names in `required` must equal the original's; the rest of the response must match in full.
   - Three further rules say what must match.
     - In a block's `warnings` list, entries of the form `<block name> raised warning: unclosed file <...>` need not match, since they name the interpreter's file object rather than the application's behaviour; a list left empty without them counts as the `warnings` key being absent, which is how the original represents a block that raised no warning, and every other warning entry must match exactly.
     - Where the original reported, after a field's own validation error, a further error raised only because a model level check ran on the incomplete model, that further error is not reproduced: the message lists the remaining errors as the original renders them, and its header counts only those.
     - An export job runs in a background worker, so in a read of the status of an export the interaction started (`GET /exports/<task_id>/status`) the `status`, `stages`, `download_url`, `completed_at` and `error_message` fields need not match, in a stored task document of such an export `status`, `completed_at`, `error_message`, `spec.stages` and `spec.file_path` need not match, and a download of such an export need not match.

The rebuild must succeed, the dependency manifest and `uv.lock` must admit no Pydantic 1 release and pin no package that is not in the wheelhouse, and no recorded behaviour may differ. These also do not count as a migration and must not appear:

- importing `pydantic.v1` anywhere in the tree;
- application models that are not Pydantic 2 models (for example models taken from a re-export of `pydantic.v1`);
- keeping the code on Pydantic 2's deprecated Pydantic 1 aliases (`.dict()`, `.json()`, `.copy()`, `parse_obj`, `@validator`, `@root_validator`, inner `class Config` and any other use that Pydantic 2 reports with a deprecation warning): at most 5 such sites may remain in `src/` and `plugins/`, against 117 in the original;
- vendoring or copying Pydantic 1 source into the tree under any name, or adding archives, wheels or compiled modules;
- a compatibility layer that keeps Pydantic 1 interfaces alive for the application to call, an adapter around the original application, `sitecustomize`, `usercustomize` or `.pth` files, import-machinery redirects, or modules named `pydantic`, `pydantic_core`, `pydantic_settings` or `pydantic_v1`;
- code that looks for, reads or probes anything outside the application beyond what this statement describes, or a file you add or change that names a path under `/tests`.

Restoring Pydantic 1 behaviour through Pydantic 2's own mechanisms (`model_config`, validators with explicit modes, serializers, custom core schemas) is a migration and is expected wherever the behaviour would otherwise change. Validators, serialization, settings loading and error handling must be written in Pydantic 2's own terms, with no code paths kept for Pydantic 1. No field the original typed precisely may be retyped as `Any`, a bare `dict` or an arbitrary type, and no model's configuration may allow arbitrary types, only so that Pydantic 2 does not validate it.

No literal or identifier is required beyond those in this statement and in the original tree.

## The original application on Pydantic 1

The original application stays runnable on its own Pydantic 1 set so you can compare behaviour:

- `/opt/oracle/bin/python` imports the original `pydatalab` on Pydantic 1.10.26 (for example `/opt/oracle/bin/python -c "from pydatalab.models import Sample; ..."`). Running the original test suite against it works too: `cd /app/pydatalab && /opt/oracle/bin/python -m pytest tests` (start MongoDB first).
- `datalab-oracle` serves the original application at `http://127.0.0.1:5001` against its own MongoDB on port 27018. It seeds an admin user with API key `oracle-admin-key` and a normal user with API key `oracle-user-key`; send the key in the `DATALAB_API_KEY` header.

`start-mongodb [port] [dbpath]` starts MongoDB 8.0 on loopback (port 27017 by default), which the project's own tests expect.
