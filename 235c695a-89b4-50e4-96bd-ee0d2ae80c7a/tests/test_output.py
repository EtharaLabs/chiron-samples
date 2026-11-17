"""One criterion per compiled item, reading the grader verdict for that item."""

import json
from pathlib import Path

import rewardkit as rk

RESULTS = Path('/logs/verifier/grader/results.json')


def _results():
    try:
        data = json.loads(RESULTS.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return data if isinstance(data, dict) else None


@rk.criterion(description="{item}")
def compiled(workspace, stage: str, item: str) -> float:
    data = _results()
    if data is None or data.get("error"):
        return 0.0
    veto = data.get("veto")
    if not isinstance(veto, dict) or veto.get("fired") is not False:
        return 0.0
    verdicts = data.get(stage)
    row = verdicts.get(item) if isinstance(verdicts, dict) else None
    return 1.0 if isinstance(row, dict) and row.get("pass") is True else 0.0


rk.compiled('stage1', 'S1-make-build-absent', weight=5, name='S1-make-build-absent')
rk.compiled('stage1', 'S1-cmake-default-installs', weight=5, name='S1-cmake-default-installs')
rk.compiled('stage1', 'S1-no-embedded-old-build', weight=5, name='S1-no-embedded-old-build')
rk.compiled('stage1', 'S1-no-build-forwarding', weight=5, name='S1-no-build-forwarding')
rk.compiled('stage1', 'S1-no-verifier-awareness', weight=5, name='S1-no-verifier-awareness')
rk.compiled('stage2', 'S2-ci-test-build', weight=5, name='S2-ci-test-build')
rk.compiled('stage2', 'S2-ci-test-compile', weight=5, name='S2-ci-test-compile')
rk.compiled('stage2', 'S2-ci-test-sanitizers', weight=5, name='S2-ci-test-sanitizers')
rk.compiled('stage2', 'S2-ci-extra-builds', weight=5, name='S2-ci-extra-builds')
rk.compiled('stage2', 'S2-ci-wheels', weight=5, name='S2-ci-wheels')
rk.compiled('stage2', 'S2-ci-source-vendor', weight=5, name='S2-ci-source-vendor')
rk.compiled('stage2', 'S2-ci-docs', weight=5, name='S2-ci-docs')
rk.compiled('stage2', 'S2-suite-frontends', weight=5, name='S2-suite-frontends')
rk.compiled('stage2', 'S2-suite-passes', weight=5, name='S2-suite-passes')
rk.compiled('stage2', 'S2-suite-techmap-arch', weight=5, name='S2-suite-techmap-arch')
rk.compiled('stage2', 'S2-suite-sim-formal', weight=5, name='S2-suite-sim-formal')
rk.compiled('stage2', 'S2-command-reference', weight=5, name='S2-command-reference')
rk.compiled('stage2', 'S2-compiled-features', weight=5, name='S2-compiled-features')
rk.compiled('stage2', 'S2-install-layout', weight=5, name='S2-install-layout')
rk.compiled('stage2', 'S2-binary-abi', weight=5, name='S2-binary-abi')
rk.compiled('stage2', 'S2-consumer-routes', weight=5, name='S2-consumer-routes')
rk.compiled('stage2', 'S2-option-matrix', weight=5, name='S2-option-matrix')
rk.compiled('stage3', 'F-alt-installed-consumer', weight=3, name='F-alt-installed-consumer')
rk.compiled('stage3', 'F-readonly-outside-build', weight=3, name='F-readonly-outside-build')
rk.compiled('stage3', 'F-edit-share-file', weight=3, name='F-edit-share-file')
rk.compiled('stage3', 'F-off-readline', weight=3, name='F-off-readline')
rk.compiled('stage3', 'F-alt-relocated-share', weight=3, name='F-alt-relocated-share')
rk.compiled('stage3', 'F-static-consumer', weight=3, name='F-static-consumer')
rk.compiled('stage3', 'F-off-tcl', weight=3, name='F-off-tcl')
rk.compiled('stage3', 'F-edit-python-tool', weight=3, name='F-edit-python-tool')
rk.compiled('stage3', 'F-edit-cellhelp', weight=3, name='F-edit-cellhelp')
rk.compiled('stage3', 'F-alt-python-tools', weight=3, name='F-alt-python-tools')
rk.compiled('stage3', 'F-datdir-copied-binary', weight=3, name='F-datdir-copied-binary')
rk.compiled('stage3', 'F-install-without-build', weight=3, name='F-install-without-build')
rk.compiled('stage3', 'F-delete-share-product', weight=3, name='F-delete-share-product')
rk.compiled('stage3', 'F-wasi-default', weight=3, name='F-wasi-default')
rk.compiled('stage3', 'F-edit-config-template', weight=3, name='F-edit-config-template')
rk.compiled('stage3', 'F-usr-libdir', weight=3, name='F-usr-libdir')
rk.compiled('stage3', 'F-help-source', weight=3, name='F-help-source')
rk.compiled('stage3', 'F-off-plugins', weight=3, name='F-off-plugins')
rk.compiled('stage3', 'F-off-zlib', weight=3, name='F-off-zlib')
rk.compiled('stage3', 'F-debug-install', weight=3, name='F-debug-install')
rk.compiled('stage3', 'F-alt-noabc-synth', weight=3, name='F-alt-noabc-synth')
rk.compiled('stage3', 'F-noop-rebuild', weight=3, name='F-noop-rebuild')
rk.compiled('stage3', 'F-version-checkout', weight=3, name='F-version-checkout')
rk.compiled('stage3', 'F-edit-generator', weight=3, name='F-edit-generator')
rk.compiled('stage3', 'F-tools-relocated', weight=3, name='F-tools-relocated')
rk.compiled('stage3', 'F-debug-backtrace', weight=3, name='F-debug-backtrace')
rk.compiled('stage3', 'F-debug-banner', weight=3, name='F-debug-banner')
