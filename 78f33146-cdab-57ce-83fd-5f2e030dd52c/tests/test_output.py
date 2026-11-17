"""Compiled items: one criterion per item, read from the graded pipeline's results."""

import json
import os

import rewardkit as rk

ITEMS = [
    ("S1-build", "items", 5),
    ("S1-manifest", "items", 5),
    ("S1-lockfile", "items", 5),
    ("S1-installed", "items", 5),
    ("S1-v1-namespace", "items", 5),
    ("S1-default-path", "items", 5),
    ("S1-deprecated-aliases", "items", 5),
    ("S1-transplant", "items", 5),
    ("S1-shim", "items", 5),
    ("S1-verifier-isolation", "items", 5),
    ("S2-settings", "groups", 5),
    ("S2-info", "groups", 5),
    ("S2-item-create", "groups", 5),
    ("S2-item-read", "groups", 5),
    ("S2-item-save", "groups", 5),
    ("S2-versioning", "groups", 5),
    ("S2-listing-search", "groups", 5),
    ("S2-collections", "groups", 5),
    ("S2-relationships", "groups", 5),
    ("S2-blocks", "groups", 5),
    ("S2-files", "groups", 5),
    ("S2-users-roles", "groups", 5),
    ("S2-auth-keys", "groups", 5),
    ("S2-groups-admin", "groups", 5),
    ("S2-export", "groups", 5),
    ("S2-equipment-cells", "groups", 5),
    ("S2-errors", "groups", 5),
    ("S2-health-remotes", "groups", 5),
    ("S3-a2-cfg-backup-assign-testing", "families", 3),
    ("S3-a2-cfg-backup-defaults-testing", "families", 3),
    ("S3-a2-cfg-cache-min-only-config-file", "families", 3),
    ("S3-a2-collection-synthesis-relationship-order", "families", 3),
    ("S3-a3-datetime-isoformat-offset-response", "families", 3),
    ("S3-a3-info-types-samples-schema-defaults", "families", 3),
    ("S3-a4-settings-cache-age-default-max", "families", 3),
    ("S3-a4-settings-cache-age-int-order", "families", 3),
    ("S3-a4-settings-list-json-from-env", "families", 3),
    ("S3-a5-cache-age-env-inverted", "families", 3),
    ("S3-a5-dotenv-syntax", "families", 3),
    ("S3-a5-testing-backups-from-config-file", "families", 3),
    ("S3-a5-update-testing-then-backups", "families", 3),
    ("S3-a6-compare-versions-objectid-validation", "families", 3),
    ("S3-a6-legacy-collection-without-collection-id", "families", 3),
    ("S3-b1-collection-last-modified-utc-offset", "families", 3),
    ("S3-b1-collection-null-collection-id-accepted", "families", 3),
    ("S3-b1-item-create-nested-group-display-name-dict", "families", 3),
    ("S3-b1-legacy-item-null-refcode-read", "families", 3),
    ("S3-b1-new-sample-underscore-id-null", "families", 3),
    ("S3-b1-settings-deployment-default-port", "families", 3),
    ("S3-b1-settings-deployment-scheme-case", "families", 3),
    ("S3-b2-collection-immutable-id-null-create", "families", 3),
    ("S3-b2-collection-relationship-objectid-json", "families", 3),
    ("S3-b2-constituent-objectid-json", "families", 3),
    ("S3-b2-group-immutable-id-null-patch", "families", 3),
    ("S3-b2-group-members-null-patch", "families", 3),
    ("S3-b2-relationship-objectid-json", "families", 3),
    ("S3-b2-stored-item-id-null-get-by-refcode", "families", 3),
    ("S3-b3-cache-age-config-min-env-max", "families", 3),
    ("S3-b3-deployment-homepage-trailing-slash", "families", 3),
    ("S3-b3-deployment-issue-tracker-host-case", "families", 3),
    ("S3-b3-deployment-source-repo-default-port", "families", 3),
    ("S3-b3-dotenv-null-clears-config-deployment-metadata", "families", 3),
    ("S3-b3-dotenv-null-email-domain-allow-list", "families", 3),
    ("S3-b3-dotenv-remote-filesystems-json", "families", 3),
    ("S3-b3-dotenv-secret-key-only", "families", 3),
    ("S3-b3-dotenv-smtp-json", "families", 3),
    ("S3-b3-dotenv-testing-backups-prefix", "families", 3),
    ("S3-b3-env-null-clears-config-email-domain-list", "families", 3),
    ("S3-b3-update-deployment-metadata-url", "families", 3),
    ("S3-b4-create-collection-null-collection-id-with-oid", "families", 3),
    ("S3-b4-new-sample-member-of-collection-without-collection-id", "families", 3),
    ("S3-b4-patch-collection-string-type-output", "families", 3),
    ("S3-b4-restore-version-missing-field-errors-body", "families", 3),
    ("S3-b4-restore-version-restored-data-invalid-output", "families", 3),
    ("S3-b4-save-item-constituent-none-not-allowed-message", "families", 3),
    ("S3-b4-save-item-int-parsing-wire-message", "families", 3),
    ("S3-b4-save-item-stored-collection-without-collection-id", "families", 3),
    ("S3-b4-stored-item-invalid-500-message", "families", 3),
    ("S3-b5-admin-groups-null-objectid-doc", "families", 3),
    ("S3-b5-admin-users-legacy-display-name-list", "families", 3),
    ("S3-b5-group-managers-nested-display-name-list", "families", 3),
    ("S3-b5-group-members-nested-null-objectid", "families", 3),
    ("S3-b5-group-patch-display-name-dict", "families", 3),
    ("S3-b5-nmr-block-metadata-dict-deprecation", "families", 3),
    ("S3-b5-remote-file-xrd-block-dict-deprecation", "families", 3),
    ("S3-b6-collection-create-string-type-error-text", "families", 3),
    ("S3-b6-collection-save-int-parsing-error-text", "families", 3),
    ("S3-b6-collection-save-string-type-error-text", "families", 3),
    ("S3-b6-restore-version-extra-key-wire", "families", 3),
    ("S3-b6-restore-version-missing-field-wire", "families", 3),
    ("S3-b6-restore-version-nonstring-field-wire", "families", 3),
    ("S3-b6-restore-version-null-field-wire", "families", 3),
    ("S3-b6-restore-version-value-error-wire", "families", 3),
    ("S3-b6-save-item-aliased-float-error-loc", "families", 3),
    ("S3-b6-save-item-int-parsing-error-text", "families", 3),
    ("S3-b6-settings-deployment-metadata-url-exact", "families", 3),
    ("S3-b6-settings-deployment-metadata-url-from-dotenv", "families", 3),
    ("S3-b6-settings-testing-false-string-keeps-backups", "families", 3),
]

try:
    with open(os.environ.get("GRADE_RESULTS", "/tmp/grade/results.json"), encoding="utf-8") as handle:
        RESULTS = json.load(handle)
except (OSError, ValueError):
    RESULTS = {}


def passed(item: str, part: str) -> bool:
    return RESULTS.get(part, {}).get(item, {}).get("status") == "pass"


@rk.criterion
def compiled_item(workspace, item: str, part: str) -> float:
    return 1.0 if passed(item, part) else 0.0


for item, part, weight in ITEMS:
    rk.compiled_item(item, part, weight=weight, name=item)
