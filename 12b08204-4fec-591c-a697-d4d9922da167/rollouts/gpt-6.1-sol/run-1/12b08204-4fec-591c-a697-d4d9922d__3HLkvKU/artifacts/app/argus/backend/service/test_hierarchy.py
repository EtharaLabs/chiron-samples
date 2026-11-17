from __future__ import annotations
'Resolve or auto-create the ArgusRelease / ArgusGroup / ArgusTest hierarchy\nneeded by ``PluginModelBase.assign_categories``.\n\nThe build_id stored on every run is the Jenkins job path (e.g.\n``scylla-staging/dusan/longevity-10gb-3h-gce-test``). When a run is submitted\nagainst an Argus instance that has not been pre-populated by the Jenkins\nmonitor (``argus/backend/service/build_system_monitor.py``), the matching\n``ArgusTest`` row is missing and ``test_id`` ends up empty on the run --\nwhich then breaks every subsequent ``submit_results`` (whose C* partition key\nis ``test_id``).\n\n``ensure_test_hierarchy`` parses the build_id and idempotently creates the\nrelease / group / test triple so the next ``assign_categories`` call resolves\n``test_id`` correctly. It is intended to be opted into by the replay-ingest\nendpoint (``?create_missing_tests=true``) and is safe to call on every\nsubmit_run record.\n'
import logging
from coodie.exceptions import DocumentNotFound
from argus.backend.models.web import ArgusRelease, ArgusGroup, ArgusTest, ArgusTestException
LOGGER = logging.getLogger(__name__)
UNTRACKED = 'untracked'

def parse_build_id(build_id: str) -> tuple[str, str, str]:
    """Split ``build_id`` into ``(release_name, group_name, test_name)``.

    ``build_id`` is the Jenkins job path. The mapping mirrors the
    folder layout the Jenkins monitor uses:

    * ``release/group/test``           -> as written
    * ``release/test``                 -> group becomes ``"<release>-root"``
                                          (matches the root-folder convention
                                          in :class:`JenkinsMonitor`)
    * ``test`` (no slash)              -> both release and group become
                                          ``UNTRACKED`` so the orphan run is
                                          still navigable.
    * ``release/folder/sub/.../test``  -> middle segments are joined with
                                          ``-`` to form the group name; the
                                          first segment is the release and the
                                          last is the test.

    Empty segments produced by leading/trailing slashes are stripped.
    """
    if not build_id:
        return (UNTRACKED, UNTRACKED, UNTRACKED)
    parts = [p for p in build_id.strip('/').split('/') if p]
    if not parts:
        return (UNTRACKED, UNTRACKED, UNTRACKED)
    if len(parts) == 1:
        return (UNTRACKED, UNTRACKED, parts[0])
    if len(parts) == 2:
        return (parts[0], f'{parts[0]}-root', parts[1])
    release = parts[0]
    test = parts[-1]
    group = '-'.join(parts[1:-1])
    return (release, group, test)

def _get_or_create_release(name: str) -> ArgusRelease:
    try:
        return ArgusRelease.get(name=name)
    except DocumentNotFound:
        release = ArgusRelease()
        release.name = name
        release.save()
        LOGGER.info('Auto-created ArgusRelease %s', name)
        return release

def _get_or_create_group(release: ArgusRelease, name: str, build_system_id: str) -> ArgusGroup:
    for g in ArgusGroup.find(release_id=release.id).all():
        if g.build_system_id == build_system_id:
            return g
    group = ArgusGroup()
    group.release_id = release.id
    group.name = name
    group.build_system_id = build_system_id
    group.save()
    LOGGER.info('Auto-created ArgusGroup %s (release=%s)', name, release.name)
    return group

def _get_or_create_test(release: ArgusRelease, group: ArgusGroup, name: str, build_system_id: str, build_system_url: str | None, plugin_name: str | None) -> ArgusTest:
    try:
        return ArgusTest.get(build_system_id=build_system_id)
    except DocumentNotFound:
        test = ArgusTest()
        test.name = name
        test.group_id = group.id
        test.release_id = release.id
        test.build_system_id = build_system_id
        if build_system_url:
            test.build_system_url = build_system_url
        if plugin_name:
            test.plugin_name = plugin_name
        try:
            test.validate_build_system_id()
        except ArgusTestException:
            return ArgusTest.get(build_system_id=build_system_id)
        test.save()
        LOGGER.info('Auto-created ArgusTest %s (build_id=%s)', name, build_system_id)
        return test

def ensure_test_hierarchy(build_id: str, build_url: str | None=None, plugin_name: str | None=None) -> ArgusTest:
    """Idempotently ensure the release/group/test row triple exists.

    Returns the resolved or freshly-created :class:`ArgusTest`. Safe to call
    repeatedly for the same ``build_id``.
    """
    release_name, group_name, test_name = parse_build_id(build_id)
    parts = [p for p in build_id.strip('/').split('/') if p]
    if len(parts) >= 2:
        group_build_id = '/'.join(parts[:-1])
    else:
        group_build_id = release_name
    release = _get_or_create_release(release_name)
    group = _get_or_create_group(release, group_name, group_build_id)
    return _get_or_create_test(release, group, test_name, build_id, build_url, plugin_name)
