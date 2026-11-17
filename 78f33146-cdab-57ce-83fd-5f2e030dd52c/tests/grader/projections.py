"""The projections the statement fixes, applied alike to the recordings and to the graded run.

A recording keeps a step either as State A's normalized answer, or as a
projection record {"$projection": <name>, "value": <projected answer>} where the
statement compares only part of the answer. Every observation also passes
through the unclosed-file warning projection before it is compared.
"""
from __future__ import annotations

import re

SCHEMA_PATH = re.compile(r"^/info/types(/|$|\?)")
EXPORT_START = re.compile(r"^/(collections|items)/[^/]+/export$")
EXPORT_READ = re.compile(r"^/exports/\$\{([A-Za-z0-9_]+)\}/(status|download)$")
UNCLOSED = re.compile(r"^\S+ raised warning: unclosed file <.*>$", re.S)
PROGRESS_KEYS = ("status", "stages", "download_url", "completed_at", "error_message")
TASK_PROGRESS_KEYS = ("status", "completed_at", "error_message")
TASK_SPEC_PROGRESS_KEYS = ("stages", "file_path")


def schema_shape(obs):
    """The part of a generated JSON schema the statement fixes: title, property names, required names."""
    if isinstance(obs, dict):
        out = {}
        for key, value in obs.items():
            if key == "schema" and isinstance(value, dict) and "properties" in value:
                out[key] = {"title": value.get("title"), "properties": sorted(value["properties"]),
                            "required": sorted(value.get("required") or [])}
            else:
                out[key] = schema_shape(value)
        return out
    if isinstance(obs, list):
        return [schema_shape(v) for v in obs]
    return obs


def without_unclosed_warnings(obs):
    """Drop the interpreter's unclosed-file warnings from every block; an emptied list becomes an absent key."""
    if isinstance(obs, dict):
        out = {}
        for key, value in obs.items():
            if key == "warnings" and isinstance(obs.get("blocktype"), str) and isinstance(value, list):
                kept = [w for w in value if not (isinstance(w, str) and UNCLOSED.match(w))]
                if value and not kept:
                    continue
                out[key] = [without_unclosed_warnings(w) for w in kept]
            else:
                out[key] = without_unclosed_warnings(value)
        return out
    if isinstance(obs, list):
        return [without_unclosed_warnings(v) for v in obs]
    return obs


def export_status(obs):
    """An export status read without what the worker's progress decides."""
    if not isinstance(obs, dict) or not isinstance(obs.get("json"), dict):
        return obs
    body = {k: v for k, v in obs["json"].items()
            if k not in PROGRESS_KEYS and k.lstrip("/").split("/")[0] not in PROGRESS_KEYS}
    return {**obs, "json": body}


def export_tasks(obs):
    """Task documents without the fields the export worker writes as it progresses."""
    if not isinstance(obs, dict) or not isinstance(obs.get("db"), list):
        return obs
    docs = []
    for doc in obs["db"]:
        if isinstance(doc, dict):
            doc = {k: v for k, v in doc.items() if k not in TASK_PROGRESS_KEYS}
            if isinstance(doc.get("spec"), dict):
                doc["spec"] = {k: v for k, v in doc["spec"].items() if k not in TASK_SPEC_PROGRESS_KEYS}
        docs.append(doc)
    return {**obs, "db": docs}


PROJECTIONS = {"schema-shape": schema_shape, "export-status": export_status, "export-tasks": export_tasks}


def export_progress_reads(case: dict) -> dict:
    """Step index -> 'status' | 'download' | 'tasks' for the reads the worker's progress can change."""
    if case.get("kind") == "settings":
        return {}
    jobs: set = set()
    started = False
    reads = {}
    for idx, step in enumerate(case.get("steps", [])):
        path = str(step.get("path") or "")
        if step.get("method") == "POST" and EXPORT_START.match(path):
            started = True
            for name, pointer in (step.get("capture") or {}).items():
                if pointer == "/task_id":
                    jobs.add(name)
            continue
        read = EXPORT_READ.match(path)
        if read and read.group(1) in jobs:
            reads[str(idx)] = read.group(2)
        elif started and isinstance(step.get("db_find"), dict) and step["db_find"].get("collection") == "tasks":
            reads[str(idx)] = "tasks"
    return reads


def projection_for(case: dict, idx: str) -> str | None:
    """The projection the statement applies to one step, or None for a full comparison."""
    if case.get("kind") == "settings":
        return None
    kind = export_progress_reads(case).get(str(idx))
    if kind == "status":
        return "export-status"
    if kind == "tasks":
        return "export-tasks"
    path = str(case["steps"][int(idx)].get("path") or "")
    if SCHEMA_PATH.match(path):
        return "schema-shape"
    return None


def excluded_read(case: dict, idx: str) -> str | None:
    """The listed reason a step is not compared under the export progress rule, or None."""
    return "export-progress-download" if export_progress_reads(case).get(str(idx)) == "download" else None


def record(case: dict, idx: str, answer):
    """One answer in the form the recording keeps."""
    answer = without_unclosed_warnings(answer)
    name = projection_for(case, idx)
    return answer if name is None else {"$projection": name, "value": PROJECTIONS[name](answer)}


_TS_PLACEHOLDER = re.compile(r"<ts\|[^>]*>")
_TS_STAMP = re.compile(
    r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?"
)


def _ts_shape(raw: str) -> str:
    return "<ts|" + re.sub(r"\.d+", "", re.sub(r"\d", "d", raw)) + ">"


def _align_stamps(want, got):
    """Compare a recorded timestamp by its shape, without the recorder's freshness window.

    The recorder replaced a timestamp with its shape only while the value sat inside a
    freshness window of two days either side of the recording clock. The graded responses
    carry values stamped at that same moment, so after that window closed the live value is
    left raw and can never equal the frozen placeholder. Where the recording already decided
    a value is a timestamp, compare the observation by the same shape without the window.
    This loses nothing the recording still holds, because the recording keeps only the shape.
    """
    if isinstance(want, str) and isinstance(got, str) and _TS_PLACEHOLDER.search(want):
        return _TS_STAMP.sub(lambda m: _ts_shape(m.group(0)), got)
    if isinstance(want, dict) and isinstance(got, dict):
        return {k: _align_stamps(want.get(k), v) for k, v in got.items()}
    if isinstance(want, list) and isinstance(got, list):
        return [_align_stamps(want[i] if i < len(want) else None, v) for i, v in enumerate(got)]
    return got


def as_recorded(want, got):
    """A graded observation in the form its recording keeps."""
    got = without_unclosed_warnings(got)
    if isinstance(want, dict) and set(want) == {"$projection", "value"}:
        value = PROJECTIONS[want["$projection"]](got)
        return {"$projection": want["$projection"], "value": _align_stamps(want.get("value"), value)}
    return _align_stamps(want, got)


ROOT_AFTER_FIELD = re.compile(r"\d+ validation errors for \w+\n.*\n__root__\n", re.S)


def model_check_after_field_error(answer) -> bool:
    """Whether an answer carries a pydantic 1 message listing a model level error after a field error."""
    if isinstance(answer, str):
        return bool(ROOT_AFTER_FIELD.search(answer))
    if isinstance(answer, dict):
        return any(model_check_after_field_error(v) for v in answer.values())
    if isinstance(answer, list):
        return any(model_check_after_field_error(v) for v in answer)
    return False
