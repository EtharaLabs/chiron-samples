from __future__ import annotations
from argus.backend.util.config import Config
'Unit tests for the build-id parser in\n:mod:`argus.backend.service.test_hierarchy`.\n\nOnly ``parse_build_id`` is exercised here -- the row-creation helpers require\na live ScyllaDB and are covered by integration tests.\n'
import pytest
from argus.backend.service.test_hierarchy import UNTRACKED, parse_build_id

@pytest.mark.parametrize('build_id, expected', [('scylla-staging/dusan/longevity-10gb-3h-gce-test', ('scylla-staging', 'dusan', 'longevity-10gb-3h-gce-test')), ('scylla-master/teamA/longevity/nightly', ('scylla-master', 'teamA-longevity', 'nightly')), ('scylla-master/perf-regression', ('scylla-master', 'scylla-master-root', 'perf-regression')), ('solo-test', (UNTRACKED, UNTRACKED, 'solo-test')), ('/scylla-staging/dusan/longevity/', ('scylla-staging', 'dusan', 'longevity')), ('', (UNTRACKED, UNTRACKED, UNTRACKED)), ('scylla-staging//longevity', ('scylla-staging', 'scylla-staging-root', 'longevity'))])
def test_parse_build_id(build_id: str, expected: tuple[str, str, str]) -> None:
    assert parse_build_id(build_id) == expected
