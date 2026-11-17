# Grading truth

Formula: R = 1[Stage I] x 1[Stage II] x (0.8 x (0.4 + 0.6 x f) + 0.2 x q)

Root weights: test_output 4, judge 1.
Clearing-route shares: Stage I and II 0.32, Stage III 0.48, q 0.2.

| id | stage | mode | weight |
| --- | --- | --- | --- |
| S1-build | I | compiled | 5 |
| S1-manifest | I | compiled | 5 |
| S1-lockfile | I | compiled | 5 |
| S1-installed | I | compiled | 5 |
| S1-v1-namespace | I | compiled | 5 |
| S1-default-path | I | compiled | 5 |
| S1-deprecated-aliases | I | compiled | 5 |
| S1-transplant | I | compiled | 5 |
| S1-shim | I | compiled | 5 |
| S1-verifier-isolation | I | compiled | 5 |
| S1-residue | I | judged gate | 5 |
| Q-dump-mode | q | judged | 3 |
| Q-settings-sources | q | judged | 3 |
| Q-discarded-error | q | judged | 3 |
| Q-loose-types | q | judged | 5 |
| Q-version-branch | q | judged | 1 |
| S2-settings | II | compiled | 5 |
| S2-info | II | compiled | 5 |
| S2-item-create | II | compiled | 5 |
| S2-item-read | II | compiled | 5 |
| S2-item-save | II | compiled | 5 |
| S2-versioning | II | compiled | 5 |
| S2-listing-search | II | compiled | 5 |
| S2-collections | II | compiled | 5 |
| S2-relationships | II | compiled | 5 |
| S2-blocks | II | compiled | 5 |
| S2-files | II | compiled | 5 |
| S2-users-roles | II | compiled | 5 |
| S2-auth-keys | II | compiled | 5 |
| S2-groups-admin | II | compiled | 5 |
| S2-export | II | compiled | 5 |
| S2-equipment-cells | II | compiled | 5 |
| S2-errors | II | compiled | 5 |
| S2-health-remotes | II | compiled | 5 |
| S3-a2-cfg-backup-assign-testing | III | compiled | 3 |
| S3-a2-cfg-backup-defaults-testing | III | compiled | 3 |
| S3-a2-cfg-cache-min-only-config-file | III | compiled | 3 |
| S3-a2-collection-synthesis-relationship-order | III | compiled | 3 |
| S3-a3-datetime-isoformat-offset-response | III | compiled | 3 |
| S3-a3-info-types-samples-schema-defaults | III | compiled | 3 |
| S3-a4-settings-cache-age-default-max | III | compiled | 3 |
| S3-a4-settings-cache-age-int-order | III | compiled | 3 |
| S3-a4-settings-list-json-from-env | III | compiled | 3 |
| S3-a5-cache-age-env-inverted | III | compiled | 3 |
| S3-a5-dotenv-syntax | III | compiled | 3 |
| S3-a5-testing-backups-from-config-file | III | compiled | 3 |
| S3-a5-update-testing-then-backups | III | compiled | 3 |
| S3-a6-compare-versions-objectid-validation | III | compiled | 3 |
| S3-a6-legacy-collection-without-collection-id | III | compiled | 3 |
| S3-b1-collection-last-modified-utc-offset | III | compiled | 3 |
| S3-b1-collection-null-collection-id-accepted | III | compiled | 3 |
| S3-b1-item-create-nested-group-display-name-dict | III | compiled | 3 |
| S3-b1-legacy-item-null-refcode-read | III | compiled | 3 |
| S3-b1-new-sample-underscore-id-null | III | compiled | 3 |
| S3-b1-settings-deployment-default-port | III | compiled | 3 |
| S3-b1-settings-deployment-scheme-case | III | compiled | 3 |
| S3-b2-collection-immutable-id-null-create | III | compiled | 3 |
| S3-b2-collection-relationship-objectid-json | III | compiled | 3 |
| S3-b2-constituent-objectid-json | III | compiled | 3 |
| S3-b2-group-immutable-id-null-patch | III | compiled | 3 |
| S3-b2-group-members-null-patch | III | compiled | 3 |
| S3-b2-relationship-objectid-json | III | compiled | 3 |
| S3-b2-stored-item-id-null-get-by-refcode | III | compiled | 3 |
| S3-b3-cache-age-config-min-env-max | III | compiled | 3 |
| S3-b3-deployment-homepage-trailing-slash | III | compiled | 3 |
| S3-b3-deployment-issue-tracker-host-case | III | compiled | 3 |
| S3-b3-deployment-source-repo-default-port | III | compiled | 3 |
| S3-b3-dotenv-null-clears-config-deployment-metadata | III | compiled | 3 |
| S3-b3-dotenv-null-email-domain-allow-list | III | compiled | 3 |
| S3-b3-dotenv-remote-filesystems-json | III | compiled | 3 |
| S3-b3-dotenv-secret-key-only | III | compiled | 3 |
| S3-b3-dotenv-smtp-json | III | compiled | 3 |
| S3-b3-dotenv-testing-backups-prefix | III | compiled | 3 |
| S3-b3-env-null-clears-config-email-domain-list | III | compiled | 3 |
| S3-b3-update-deployment-metadata-url | III | compiled | 3 |
| S3-b4-create-collection-null-collection-id-with-oid | III | compiled | 3 |
| S3-b4-new-sample-member-of-collection-without-collection-id | III | compiled | 3 |
| S3-b4-patch-collection-string-type-output | III | compiled | 3 |
| S3-b4-restore-version-missing-field-errors-body | III | compiled | 3 |
| S3-b4-restore-version-restored-data-invalid-output | III | compiled | 3 |
| S3-b4-save-item-constituent-none-not-allowed-message | III | compiled | 3 |
| S3-b4-save-item-int-parsing-wire-message | III | compiled | 3 |
| S3-b4-save-item-stored-collection-without-collection-id | III | compiled | 3 |
| S3-b4-stored-item-invalid-500-message | III | compiled | 3 |
| S3-b5-admin-groups-null-objectid-doc | III | compiled | 3 |
| S3-b5-admin-users-legacy-display-name-list | III | compiled | 3 |
| S3-b5-group-managers-nested-display-name-list | III | compiled | 3 |
| S3-b5-group-members-nested-null-objectid | III | compiled | 3 |
| S3-b5-group-patch-display-name-dict | III | compiled | 3 |
| S3-b5-nmr-block-metadata-dict-deprecation | III | compiled | 3 |
| S3-b5-remote-file-xrd-block-dict-deprecation | III | compiled | 3 |
| S3-b6-collection-create-string-type-error-text | III | compiled | 3 |
| S3-b6-collection-save-int-parsing-error-text | III | compiled | 3 |
| S3-b6-collection-save-string-type-error-text | III | compiled | 3 |
| S3-b6-restore-version-extra-key-wire | III | compiled | 3 |
| S3-b6-restore-version-missing-field-wire | III | compiled | 3 |
| S3-b6-restore-version-nonstring-field-wire | III | compiled | 3 |
| S3-b6-restore-version-null-field-wire | III | compiled | 3 |
| S3-b6-restore-version-value-error-wire | III | compiled | 3 |
| S3-b6-save-item-aliased-float-error-loc | III | compiled | 3 |
| S3-b6-save-item-int-parsing-error-text | III | compiled | 3 |
| S3-b6-settings-deployment-metadata-url-exact | III | compiled | 3 |
| S3-b6-settings-deployment-metadata-url-from-dotenv | III | compiled | 3 |
| S3-b6-settings-testing-false-string-keeps-backups | III | compiled | 3 |

Recordings:
- stage2: tests/grader/data/stage2.json sha256 77ac2828fcb1cda0eedf92bf68fcb3d713c900deb9c22aed840a4b47c74f474d
- stage3: tests/grader/data/stage3.json sha256 8d9b2a44fcf69e90483470487dde537acfecc8f829613d727d93b5f988551fb6

Reference repairs:
- rule: every Stage II step and Stage III family is graded against State A's answer; each departure of the upstream merged tree from it is repaired in the reference to State A's answer (listed here), or stated in instruction.md and compared only on what the statement fixes
- upstream: github.com/datalab-org/datalab 521e0fb8f81b98867074d0f4ea960cd06260e9b0 (pull request 1820 merged)
- R1: restores that relationships is absent or null where State A leaves it so (files, people, groups, collections, references, stored items), not an empty list (upstream commit 84e6a3ed)
- R2: restores that an entry's collections inline the whole collection as State A serves it (upstream commit 84e6a3ed)
- R3: restores that molar_mass is never derived from chemform when the client did not send it (upstream commit 6ec3c28a)
- R4: restores that schema property titles come from the field alias (upstream commit 84e6a3ed)
- R5: restores that cells carry no active_ion field (upstream commit 84e6a3ed)
- R6: restores that a group's members default to null (upstream commit 84e6a3ed)
- R7: restores that the block web update materialises every block field as State A does (upstream commit 6ec3c28a)
- R8: restores that the block response model has no processed field (upstream commit 7b258cea)
- R9: restores that collections is listed and served by the type-introspection endpoints (upstream commit 16baed1a)
- R10: restores that the item routes answer with State A's own JSON error bodies and status codes (upstream commit 20ccad49)
- R11: restores that stored item documents keep their null-valued keys, immutable_id and groups (upstream commit 6ec3c28a, 97061229)
- R12: restores that version request validation answers 400 with State A's errors list and messages (upstream commit 7b258cea)
- R13: restores that the user-update route validates each field and names the one that failed (upstream commit 6ec3c28a)
- R14: restores that an environment variable set to null means None for list settings (upstream commit bf410c92)
- R15: restores that configured URLs are served exactly as configured (upstream commit bf410c92)
- R16: restores that a relationship drops keys it does not declare (upstream commit 84e6a3ed)
- R17: restores that client-visible validation errors keep pydantic 1's text form (upstream commit none (rendering of pydantic 2 errors))

Stated departures (instruction.md fixes what is compared):
- D1-schema-shape: JSON schemas under /info/types compare only title, property names and required names; cause: 6e327b25 regenerated the shipped schemas with the new major's generator
- D2-unclosed-file-warnings: a block's unclosed-file warnings are dropped before comparison; an emptied list compares as an absent key; cause: the leaked object is the target set's file wrapper, so no port on the target set prints State A's text
- D3-model-check-after-field-error: a further error raised only because a model level check ran on the incomplete model is not reproduced; recorded from the reference and compared in full; cause: pydantic 1 ran post root validators after a field failure and turned the TypeError into a validation error; pydantic 2 does neither
- D4-export-progress: export status reads, stored task documents and downloads of an export the interaction started compare only what the worker's progress does not decide; downloads are not compared; cause: the replay never waits for the export worker

Excluded steps (State A has no stable answer, or the export progress rule leaves the read uncompared):
- harvest:test_auth#25 (S2-auth-keys): state-a-unstable
- harvest:test_blocks#139 (S2-blocks): state-a-unstable
- harvest:test_blocks#149 (S2-blocks): state-a-unstable
- harvest:test_blocks#150 (S2-item-read): state-a-unstable
- harvest:test_blocks#151 (S2-blocks): state-a-unstable
- harvest:test_blocks#152 (S2-blocks): state-a-unstable
- harvest:test_blocks#153 (S2-blocks): state-a-unstable
- harvest:test_blocks#155 (S2-blocks): state-a-unstable
- harvest:test_blocks#165 (S2-blocks): state-a-unstable
- harvest:test_blocks#166 (S2-blocks): state-a-unstable
- harvest:test_blocks#167 (S2-item-read): state-a-platform-dependent
- harvest:test_blocks#189 (S2-blocks): state-a-unstable
- harvest:test_blocks#199 (S2-blocks): state-a-unstable
- harvest:test_blocks#209 (S2-blocks): state-a-unstable
- harvest:test_blocks#219 (S2-blocks): state-a-unstable
- harvest:test_blocks#229 (S2-blocks): state-a-unstable
- harvest:test_blocks#239 (S2-blocks): state-a-unstable
- harvest:test_blocks#249 (S2-blocks): state-a-unstable
- harvest:test_blocks#270 (S2-blocks): state-a-unstable
- harvest:test_export#135 (S2-export): export-progress-download
- harvest:test_export#145 (S2-export): export-progress-download
- harvest:test_permissions#37 (S2-auth-keys): state-a-unstable
- harvest:test_samples#353 (S2-item-create): state-a-unstable

Steps recorded from the reference under a stated departure:
- harvest:test_samples#365: D3-model-check-after-field-error
