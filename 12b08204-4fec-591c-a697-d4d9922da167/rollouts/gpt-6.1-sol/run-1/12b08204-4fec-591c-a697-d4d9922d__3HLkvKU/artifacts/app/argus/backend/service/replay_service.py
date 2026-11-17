from argus.backend.util.config import Config
"Server-side replay-log ingest.\n\nReceives an archive of JSONL replay-log records (produced by the Argus\nPython client, or hand-packed by a user) and re-applies each recorded\nrequest against the running FastAPI app.\n\nSupported archive formats (detected from magic bytes, not the\n``Content-Type`` header):\n\n* ``tar.zst`` / ``tar.zstd`` -- canonical CLI output.\n* ``tar.gz`` / ``.tgz``.\n* plain ``tar``.\n* ``zip`` -- entries are walked in archive order.\n\nDesign: we deliberately do **not** reimplement the controller logic\nendpoint-by-endpoint. Each record carries enough information --\n``endpoint`` template, ``location_params`` for ``$placeholder`` substitution,\noptional ``params`` query string, ``method``, and JSON ``body`` -- to\nreconstruct the original HTTP call. We send that reconstructed call through\nStarlette's in-process :py:meth:`~starlette.testclient.TestClient`, so the real\ncontroller, decorators (auth, error handling) and idempotency logic kick in\nexactly as they would for a live client.\n\nTwo side concerns remain server-side:\n\n* **Ordering** -- ``submit_run`` must run before anything referencing the run;\n  terminal ``set_status`` and ``finalize`` run last; and heartbeats collapse to\n  the most recent record (per run). ``finalize`` is the only call that\n  back-fills ``end_time`` -- and, for the generic (pytest/dtest) and\n  driver-matrix plugins, the terminal ``status`` and ``scylla_version`` too --\n  so it must land after every middle record.\n* **Skip-list** -- only endpoints with side effects *outside* Argus\n  (``/testrun/report/email``, ``/planning/plan/trigger``) are skipped so\n  replay doesn't send emails or trigger CI jobs. ``finalize`` is deliberately\n  *not* skipped: it only writes to the run row and is the single source of\n  terminal status/version for non-SCT plugins, so skipping it silently left\n  replayed dtest/pytest runs stuck without a final status.\n\nWhen ``create_missing_tests`` is set, ``submit_run`` records get a pre-step\nthat ensures the ``ArgusRelease/Group/Test`` triple exists for the run's\nbuild_id, so that the controller's ``assign_categories`` can populate\n``test_id`` on the row. This is opt-in because curated instances already have\nthe hierarchy populated and don't want orphan auto-creates.\n\nWhen ``backfill_logs`` is set, a post-step lists each run's S3 log prefix\n(derived from the run-id in any recorded log link) and submits links for any\narchive that reached S3 but whose ``logs/submit`` was never recorded -- e.g.\nloader/monitor/sct-runner bundles collected at teardown. The ingest endpoint\nenables this by default (disable with ``?backfill_logs=false``); it re-uses\nthe app's existing read-only S3 credentials and the link-only log model\n(nothing is uploaded or hosted), and is idempotent, so the extra S3 list per\ningest is the only cost when there is nothing to back-fill.\n"
import gzip
import io
import json
import logging
import re
import tarfile
import zipfile
from dataclasses import dataclass, field
from typing import Iterable
import zstandard as zstd
from coodie.exceptions import DocumentNotFound
from argus.backend.error_handlers import APIException
LOGGER = logging.getLogger(__name__)
SKIP_ENDPOINTS: frozenset[str] = frozenset({'/testrun/report/email', '/planning/plan/trigger'})
TERMINAL_STATUSES = frozenset({'failed', 'aborted', 'passed', 'error', 'notrun', 'not_run'})
CLIENT_ROUTE_PREFIX = '/api/v1/client'
LOGS_SUBMIT_ENDPOINT = '/testrun/$type/$id/logs/submit'
FINALIZE_ENDPOINT = '/testrun/$type/$id/finalize'
_DEFAULT_LOG_BACKFILL_BUCKET = 'cloudius-jenkins-test'
_S3_LINK_RE = re.compile('(https:\\/\\/)?(?P<bucket>[\\w\\-]+)\\.s3(?P<region>\\.[\\w\\-\\d]*)?\\.amazonaws\\.com\\/(?P<key>.+)')
_LOG_NAME_SUFFIXES = ('.tar.zst', '.tar.gz', '.tar.zstd', '.tgz', '.tar', '.zip')

@dataclass
class ReplayOutcome:
    ts: int
    endpoint: str
    status: str
    error: str | None = None

@dataclass
class ReplaySummary:
    total: int = 0
    processed: int = 0
    succeeded: int = 0
    failed: int = 0
    skipped_no_replay: int = 0
    backfilled_logs: int = 0
    errors: list[dict] = field(default_factory=list)

    def as_dict(self) -> dict:
        return {'total': self.total, 'processed': self.processed, 'succeeded': self.succeeded, 'failed': self.failed, 'skipped_no_replay': self.skipped_no_replay, 'backfilled_logs': self.backfilled_logs, 'errors': self.errors}

class ReplayServiceError(APIException):
    """Raised when the replay archive itself cannot be processed.

    Inherits from :class:`APIException` so the registered error handler on
    the ingest blueprint converts it into the standard error envelope
    instead of letting the controller marshal it by hand.
    """

class ReplayService:
    """Replay one replay-log archive against the current FastAPI app.

    See the module docstring for the list of supported archive formats.
    """
    _ERROR_BODY_MAX = 400

    def __init__(self, *, app=None, auth_header: str | None=None, create_missing_tests: bool=False, backfill_logs: bool=False, s3_client=None, client=None) -> None:
        self._app = app
        self._client = client
        self._auth_header = auth_header
        self._create_missing_tests = create_missing_tests
        self._backfill_logs = backfill_logs
        self._s3 = s3_client

    def ingest(self, archive_bytes: bytes, *, dry_run: bool=False) -> ReplaySummary:
        records = list(self._extract_records(archive_bytes))
        records = self._apply_ordering(records)
        summary = ReplaySummary(total=len(records))
        if records:
            client = self._test_client()
            last_seen_ts = self._compute_last_seen_ts(records)
            for rec in records:
                self._process_one(client, rec, summary, dry_run=dry_run, last_seen_ts=last_seen_ts)
            if self._backfill_logs and (not dry_run):
                backfill_records = self._build_log_backfill_records(records)
                summary.total += len(backfill_records)
                for rec in backfill_records:
                    self._process_one(client, rec, summary, dry_run=dry_run, last_seen_ts=last_seen_ts)
                    summary.backfilled_logs += len((rec.get('body') or {}).get('logs') or [])
        LOGGER.info('Replay ingest complete: total=%d processed=%d ok=%d failed=%d skipped=%d backfilled_logs=%d (dry_run=%s, create_missing_tests=%s, backfill_logs=%s)', summary.total, summary.processed, summary.succeeded, summary.failed, summary.skipped_no_replay, summary.backfilled_logs, dry_run, self._create_missing_tests, self._backfill_logs)
        return summary

    def _test_client(self):
        """Resolve the FastAPI test client to dispatch through.

        Prefers an explicitly-injected app (used by unit tests); falls back
        to :py:obj:`the explicitly supplied application` so the controller doesn't need to
        thread the app through.
        """
        if self._client is not None:
            return self._client
        app = self._app
        from fastapi.testclient import TestClient
        return TestClient(app, raise_server_exceptions=False, follow_redirects=False)
    _ZSTD_MAGIC = b'(\xb5/\xfd'
    _GZIP_MAGIC = b'\x1f\x8b'
    _ZIP_MAGIC = (b'PK\x03\x04', b'PK\x05\x06', b'PK\x07\x08')
    _TAR_USTAR_OFFSET = 257
    _TAR_USTAR_MAGIC = b'ustar'

    @classmethod
    def _extract_records(cls, archive_bytes: bytes) -> Iterable[dict]:
        """Yield records from every JSONL file in the archive.

        Streams the decompressed bytes through the relevant archive reader
        so we never materialize the full uncompressed archive at once.
        Skips non-JSONL members and bad lines (with a warning) rather than
        aborting -- a single corrupt line should not invalidate the whole
        replay.
        """
        try:
            yield from cls._dispatch_archive(archive_bytes)
        except ReplayServiceError:
            raise
        except (zstd.ZstdError, tarfile.TarError, zipfile.BadZipFile, gzip.BadGzipFile, OSError, EOFError) as exc:
            raise ReplayServiceError(f'Failed to decode archive: {exc}') from exc

    @classmethod
    def _dispatch_archive(cls, archive_bytes: bytes) -> Iterable[dict]:
        if archive_bytes.startswith(cls._ZSTD_MAGIC):
            decompressor = zstd.ZstdDecompressor()
            with decompressor.stream_reader(io.BytesIO(archive_bytes)) as stream:
                yield from cls._iter_tar(stream, mode='r|')
            return
        if archive_bytes.startswith(cls._GZIP_MAGIC):
            yield from cls._iter_tar(io.BytesIO(archive_bytes), mode='r|gz')
            return
        if archive_bytes.startswith(cls._ZIP_MAGIC):
            yield from cls._iter_zip(archive_bytes)
            return
        if cls._looks_like_tar(archive_bytes):
            yield from cls._iter_tar(io.BytesIO(archive_bytes), mode='r:')
            return
        raise ReplayServiceError('Failed to decode archive: unrecognised format (expected tar.zst, tar.gz, tar, or zip)')

    @classmethod
    def _looks_like_tar(cls, head: bytes) -> bool:
        end = cls._TAR_USTAR_OFFSET + len(cls._TAR_USTAR_MAGIC)
        return len(head) >= end and head[cls._TAR_USTAR_OFFSET:end] == cls._TAR_USTAR_MAGIC

    @classmethod
    def _iter_tar(cls, fileobj, *, mode: str) -> Iterable[dict]:
        with tarfile.open(fileobj=fileobj, mode=mode) as tar:
            for member in tar:
                if not member.isfile() or not member.name.endswith('.jsonl'):
                    continue
                fobj = tar.extractfile(member)
                if fobj is None:
                    continue
                yield from cls._iter_jsonl_stream(fobj, member.name)

    @classmethod
    def _iter_zip(cls, archive_bytes: bytes) -> Iterable[dict]:
        with zipfile.ZipFile(io.BytesIO(archive_bytes)) as zf:
            for info in zf.infolist():
                if info.is_dir() or not info.filename.endswith('.jsonl'):
                    continue
                with zf.open(info, 'r') as fobj:
                    yield from cls._iter_jsonl_stream(fobj, info.filename)

    @staticmethod
    def _iter_jsonl_stream(fobj, source_name: str) -> Iterable[dict]:
        for line_no, raw in enumerate(fobj, 1):
            line = raw.strip()
            if not line:
                continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError as exc:
                LOGGER.warning('Skipping malformed line in %s:%d (%s)', source_name, line_no, exc)

    @staticmethod
    def _normalise_endpoint(endpoint: str) -> str:
        """Collapse repeated slashes in ``endpoint``.

        Earlier versions of the SCT client emitted
        ``/sct/$id//stress_cmd/submit`` (note the double slash) due to a
        route-constant typo. Logs recorded by those clients are still in the
        wild, so we normalise on read instead of letting them mismatch on
        dispatch.
        """
        if '//' not in endpoint:
            return endpoint
        if '://' in endpoint:
            scheme, _, rest = endpoint.partition('://')
            return f"{scheme}://{re.sub('/{2,}', '/', rest)}"
        return re.sub('/{2,}', '/', endpoint)

    @staticmethod
    def _apply_ordering(records: list[dict]) -> list[dict]:
        """Sort by ``ts``, then enforce the plan's invariants:

        * ``submit_run`` records run first (the run must exist before
          anything else touches it).
        * Terminal ``set_status`` and ``finalize`` records run last.
          ``finalize`` is the only call that back-fills ``end_time`` (and, for
          the generic/driver-matrix plugins, the terminal ``status`` and
          ``scylla_version``), so it must run after every middle record.
        * Heartbeats are collapsed to only the last one per ``(type, id)``.
        """
        records.sort(key=lambda r: r.get('ts', 0))
        submit_runs: list[dict] = []
        terminal_statuses: list[dict] = []
        finalizes: list[dict] = []
        middle: list[dict] = []
        last_heartbeat: dict[tuple, dict] = {}
        for r in records:
            endpoint = ReplayService._normalise_endpoint(r.get('endpoint', ''))
            if endpoint == '/testrun/$type/submit':
                submit_runs.append(r)
            elif endpoint == '/testrun/$type/$id/set_status' and ReplayService._is_terminal_status(r):
                terminal_statuses.append(r)
            elif endpoint == FINALIZE_ENDPOINT:
                finalizes.append(r)
            elif endpoint == '/testrun/$type/$id/heartbeat':
                loc = r.get('location_params') or {}
                key = (loc.get('type'), loc.get('id'))
                last_heartbeat[key] = r
            else:
                middle.append(r)
        return submit_runs + middle + list(last_heartbeat.values()) + terminal_statuses + finalizes

    @staticmethod
    def _is_terminal_status(record: dict) -> bool:
        body = record.get('body') or {}
        new_status = (body.get('new_status') or '').lower()
        return new_status in TERMINAL_STATUSES

    @staticmethod
    def _compute_last_seen_ts(records: list[dict]) -> dict[tuple, int]:
        """Map each run touched by ``records`` to the highest ``ts`` seen for it.

        Used as the ``finalize`` ``end_time`` fallback when a record's own
        ``ts`` is missing: recovering from an Argus outage can take days, and
        the run itself may be days-long, so defaulting to the replay moment
        would badly misrepresent when the run actually finished. The last
        timestamp Argus recorded for that run is a far closer approximation.
        """
        last_seen: dict[tuple, int] = {}
        for r in records:
            loc = r.get('location_params') or {}
            key = (loc.get('type'), loc.get('id'))
            ts = r.get('ts', 0)
            if ts and ts > last_seen.get(key, 0):
                last_seen[key] = ts
        return last_seen

    @staticmethod
    def _resolve_url(endpoint_template: str, location_params: dict) -> str:
        """Substitute ``$name`` placeholders in ``endpoint_template``.

        Returns the full FastAPI URL (with the ``/api/v1/client`` prefix the
        recorded templates are relative to).
        """
        url = endpoint_template
        for key, value in (location_params or {}).items():
            url = url.replace(f'${key}', str(value))
        return CLIENT_ROUTE_PREFIX + url

    @staticmethod
    def _extract_build_id(record: dict) -> str | None:
        body = record.get('body') or {}
        return body.get('job_name') or body.get('build_id') or body.get('buildId')

    @staticmethod
    def _diagnose_missing_hierarchy(record: dict) -> str | None:
        """Pre-check for ``submit_run`` records on the default
        ``create_missing_tests=False`` path.

        Returns ``None`` when the ``ArgusTest`` row referenced by the record's
        ``build_id`` already exists -- the dispatch is safe to proceed and
        ``assign_categories`` will populate ``test_id``.

        Returns a human-readable diagnosis string when the test entity is
        missing. The caller surfaces this as a per-record failure so the user
        sees exactly which release/group/test would be needed and which level
        is absent -- otherwise the dispatch would silently insert a run row
        with empty categorical fields, breaking every subsequent
        ``submit_results`` call (which partitions on ``test_id``).
        """
        build_id = ReplayService._extract_build_id(record)
        if not build_id:
            return None
        from argus.backend.models.web import ArgusRelease, ArgusGroup, ArgusTest
        from argus.backend.service.test_hierarchy import parse_build_id
        try:
            ArgusTest.get(build_system_id=build_id)
            return None
        except DocumentNotFound:
            pass
        release_name, group_name, test_name = parse_build_id(build_id)
        parts = [p for p in build_id.strip('/').split('/') if p]
        group_build_id = '/'.join(parts[:-1]) if len(parts) >= 2 else release_name
        missing: list[str] = []
        try:
            release = ArgusRelease.get(name=release_name)
        except DocumentNotFound:
            release = None
            missing.append('release')
        group_found = False
        if release is not None:
            for g in ArgusGroup.find(release_id=release.id).all():
                if g.build_system_id == group_build_id:
                    group_found = True
                    break
        if not group_found:
            missing.append('group')
        missing.append('test')
        return f"test entity missing for build_id={build_id!r}: would create release={release_name!r}, group={group_name!r}, test={test_name!r}; currently missing: {', '.join(missing)}. Re-run with --create-missing-tests to auto-create the hierarchy, or register the test in Argus first."

    @staticmethod
    def _ensure_hierarchy_for_submit_run(record: dict) -> None:
        """Auto-create the ``ArgusRelease/Group/Test`` triple referenced by a
        ``submit_run`` record's build identifier and, if a run row already
        exists for this ``run_id`` with empty categorical fields, back-fill
        them so subsequent ``submit_results`` calls (partitioned by
        ``test_id``) succeed.

        Only invoked when ``create_missing_tests`` is set. No-ops silently
        when the body lacks a recognisable build identifier; the controller
        will then fail with its own error.
        """
        body = record.get('body') or {}
        location = record.get('location_params') or {}
        run_type = location.get('type', '')
        build_id = ReplayService._extract_build_id(record)
        if not build_id:
            LOGGER.debug('No build_id in submit_run body for run_type=%s; skipping hierarchy auto-create', run_type)
            return
        build_url = body.get('job_url') or body.get('build_job_url')
        run_id = body.get('run_id')
        from argus.backend.service.test_hierarchy import ensure_test_hierarchy
        from argus.backend.service.client_service import ClientService
        test = ensure_test_hierarchy(build_id=build_id, build_url=build_url, plugin_name=run_type or None)
        if not run_id:
            return
        try:
            model = ClientService().get_model(run_type)
            from uuid import UUID
            run = model.get(id=UUID(run_id))
        except Exception:
            return
        changed = False
        if not getattr(run, 'test_id', None):
            run.test_id = test.id
            changed = True
        if not getattr(run, 'release_id', None):
            run.release_id = test.release_id
            changed = True
        if not getattr(run, 'group_id', None):
            run.group_id = test.group_id
            changed = True
        if changed:
            run.save()
            LOGGER.info('Back-filled categorical fields on existing run %s (test_id=%s)', run_id, test.id)

    def _s3_client(self):
        """Lazily build (or return the injected) boto3 S3 client.

        Uses the same credentials the app already reads log objects with, so
        listing a run's prefix works exactly where ``get_log`` presigning does.
        """
        if self._s3 is None:
            import boto3
            self._s3 = boto3.client(service_name='s3', aws_access_key_id=Config.load_yaml_config().get('AWS_CLIENT_ID'), aws_secret_access_key=Config.load_yaml_config().get('AWS_CLIENT_SECRET'))
        return self._s3

    @staticmethod
    def _s3_url(bucket: str, key: str) -> str:
        return f'https://{bucket}.s3.amazonaws.com/{key}'

    @staticmethod
    def _log_name_from_key(key: str) -> str:
        """Derive a display log name from an S3 key's basename.

        Strips the archive suffix so ``.../loader-set-15bb6cad.tar.zst`` becomes
        ``loader-set-15bb6cad``. Only used for links that were *not* recorded,
        so it never has to match a client-chosen name.
        """
        name = key.rsplit('/', 1)[-1]
        for suffix in _LOG_NAME_SUFFIXES:
            if name.endswith(suffix):
                return name[:-len(suffix)]
        return name

    @classmethod
    def _resolve_run_bucket(cls, recorded_links: set[str]) -> str | None:
        """Derive the S3 bucket from any recorded log link for the run.

        Falls back to ``REPLAY_LOG_BACKFILL_BUCKET`` from config, then to
        SCT's own default bucket, when the run recorded no S3 links to learn
        the bucket from.
        """
        for link in recorded_links:
            match = _S3_LINK_RE.match(link or '')
            if match:
                return match.group('bucket')
        try:
            configured = (Config.CONFIG or {}).get('REPLAY_LOG_BACKFILL_BUCKET')
        except RuntimeError:
            configured = None
        return configured or _DEFAULT_LOG_BACKFILL_BUCKET

    def _list_s3_run_objects(self, bucket: str, prefix: str) -> list[str]:
        """Return every (non-directory) object key under ``prefix``, paginated."""
        s3 = self._s3_client()
        keys: list[str] = []
        continuation: str | None = None
        while True:
            kwargs = {'Bucket': bucket, 'Prefix': prefix}
            if continuation:
                kwargs['ContinuationToken'] = continuation
            resp = s3.list_objects_v2(**kwargs)
            for obj in resp.get('Contents', []) or []:
                key = obj.get('Key', '')
                if key and (not key.endswith('/')):
                    keys.append(key)
            if not resp.get('IsTruncated'):
                break
            continuation = resp.get('NextContinuationToken')
            if not continuation:
                break
        return keys

    def _build_log_backfill_records(self, records: list[dict]) -> list[dict]:
        """Synthesise ``logs/submit`` records for a run's S3 log archives that
        were uploaded but never recorded.

        For each run touched by the archive we derive its S3 bucket + ``run_id``
        prefix from any recorded log link, list the prefix, and emit a record
        for every object whose URL is not already in the recorded set. The
        records are dispatched through the normal ``logs/submit`` path, whose
        ``submit_logs`` appends-and-dedups -- so this is idempotent across
        re-ingests (deterministic key-derived names) and never clobbers links
        the client already submitted.
        """
        runs: dict[tuple, set] = {}
        schema_version: str | None = None
        for rec in records:
            loc = rec.get('location_params') or {}
            run_type, run_id = (loc.get('type'), loc.get('id'))
            if not run_type or not run_id:
                continue
            links = runs.setdefault((run_type, run_id), set())
            if self._normalise_endpoint(rec.get('endpoint', '')) == LOGS_SUBMIT_ENDPOINT:
                body = rec.get('body') or {}
                schema_version = schema_version or body.get('schema_version')
                for log in body.get('logs') or []:
                    if log.get('log_link'):
                        links.add(log['log_link'])
        backfill: list[dict] = []
        for (run_type, run_id), recorded_links in runs.items():
            bucket = self._resolve_run_bucket(recorded_links)
            if not bucket:
                LOGGER.info('log backfill: no S3 bucket derivable for run %s; skipping', run_id)
                continue
            try:
                keys = self._list_s3_run_objects(bucket, f'{run_id}/')
            except Exception:
                LOGGER.exception('log backfill: listing s3://%s/%s/ failed', bucket, run_id)
                continue
            new_logs = [{'log_name': self._log_name_from_key(key), 'log_link': url} for key in keys if (url := self._s3_url(bucket, key)) not in recorded_links]
            if not new_logs:
                continue
            body: dict = {'logs': new_logs}
            if schema_version:
                body['schema_version'] = schema_version
            backfill.append({'ts': 0, 'method': 'POST', 'endpoint': LOGS_SUBMIT_ENDPOINT, 'location_params': {'type': run_type, 'id': run_id}, 'params': {}, 'body': body})
            LOGGER.info('log backfill: %d new log(s) for run %s from s3://%s/%s/', len(new_logs), run_id, bucket, run_id)
        return backfill

    def _process_one(self, client, record: dict, summary: ReplaySummary, *, dry_run: bool, last_seen_ts: dict[tuple, int] | None=None) -> None:
        endpoint = self._normalise_endpoint(record.get('endpoint', ''))
        ts = record.get('ts', 0)
        if endpoint in SKIP_ENDPOINTS:
            summary.skipped_no_replay += 1
            return
        if not self._create_missing_tests and endpoint == '/testrun/$type/submit':
            try:
                diagnosis = self._diagnose_missing_hierarchy(record)
            except Exception:
                LOGGER.exception('Hierarchy pre-check failed for ts=%s; allowing dispatch to proceed', ts)
                diagnosis = None
            if diagnosis is not None:
                summary.processed += 1
                summary.failed += 1
                summary.errors.append({'ts': ts, 'endpoint': endpoint, 'error': diagnosis})
                return
        if dry_run:
            summary.processed += 1
            summary.succeeded += 1
            return
        if self._create_missing_tests and endpoint == '/testrun/$type/submit':
            try:
                self._ensure_hierarchy_for_submit_run(record)
            except Exception:
                LOGGER.exception('Hierarchy auto-create failed for ts=%s; continuing with dispatch', ts)
        location_params = record.get('location_params') or {}
        params = record.get('params') or {}
        body = record.get('body')
        method = (record.get('method') or 'POST').upper()
        if endpoint == FINALIZE_ENDPOINT and isinstance(body, dict) and ('end_time' not in body):
            run_key = (location_params.get('type'), location_params.get('id'))
            effective_ts = ts or (last_seen_ts or {}).get(run_key, 0)
            if effective_ts:
                body = {**body, 'end_time': effective_ts / 1000}
        url = self._resolve_url(endpoint, location_params)
        headers = {}
        if self._auth_header:
            headers['Authorization'] = self._auth_header
        try:
            response = client.request(method, url, params=params or None, json=body if body is not None else None, headers=headers)
        except Exception as exc:
            summary.processed += 1
            summary.failed += 1
            summary.errors.append({'ts': ts, 'endpoint': endpoint, 'error': f'{type(exc).__name__}: {exc}'})
            LOGGER.exception('Replay raised dispatching ts=%s endpoint=%s url=%s', ts, endpoint, url)
            return
        summary.processed += 1
        status = response.status_code
        if 200 <= status < 300:
            envelope_error = self._envelope_error(response)
            if envelope_error is not None:
                summary.failed += 1
                summary.errors.append({'ts': ts, 'endpoint': endpoint, 'error': envelope_error[:self._ERROR_BODY_MAX]})
                return
            summary.succeeded += 1
            return
        body_text = response.text
        summary.failed += 1
        summary.errors.append({'ts': ts, 'endpoint': endpoint, 'error': f'HTTP {status}: {body_text[:self._ERROR_BODY_MAX]}'})

    @staticmethod
    def _envelope_error(response) -> str | None:
        """If a 2xx response carries the standard ``{"status": "error"}``
        envelope, return a one-line description; otherwise return None.
        """
        try:
            data = response.json()
        except Exception:
            return None
        if not isinstance(data, dict):
            return None
        if data.get('status') != 'error':
            return None
        resp = data.get('response') or {}
        exc = resp.get('exception') or ''
        args = resp.get('arguments') or []
        msg = data.get('message') or ''
        if exc and args:
            return f'{exc}: {args}'
        if exc:
            return exc
        if msg:
            return msg
        return json.dumps(data)
