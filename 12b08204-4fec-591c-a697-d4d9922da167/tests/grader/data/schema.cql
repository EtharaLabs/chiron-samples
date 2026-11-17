CREATE KEYSPACE argus WITH replication = {'class': 'org.apache.cassandra.locator.NetworkTopologyStrategy', 'datacenter1': '1'} AND durable_writes = true AND tablets = {'enabled': true};
CREATE TYPE argus.packageversion_v2 (
    name text,
    version text,
    date text,
    revision_id text,
    build_id text
);
CREATE TYPE argus.cloudnodesinfo (
    image_id text,
    instance_type text,
    node_amount int,
    post_behaviour text
);
CREATE TYPE argus.column_metadata (
    name ascii,
    unit text,
    type ascii,
    higher_is_better boolean,
    visible boolean
);
CREATE TYPE argus.validation_rules (
    valid_from timestamp,
    best_pct double,
    best_abs double,
    fixed_limit double
);
CREATE TYPE argus.issue_assignee (
    login text,
    html_url text
);
CREATE TYPE argus.issue_label (
    id bigint,
    name text,
    color text,
    description text
);
CREATE TYPE argus.test_case (
    name text,
    status text,
    time float,
    classname text,
    message text
);
CREATE TYPE argus.sirenada_test (
    test_name text,
    class_name text,
    file_name text,
    browser_type text,
    cluster_type text,
    status text,
    duration float,
    message text,
    start_time timestamp,
    stack_trace text,
    screenshot_file text,
    s3_folder_id text,
    requests_file text,
    sirenada_test_id text,
    sirenada_user text,
    sirenada_password text
);
CREATE TYPE argus.cloudinstancedetails_v3 (
    provider text,
    instance_type text,
    region text,
    public_ip text,
    private_ip text,
    dc_name text,
    rack_name text,
    creation_time int,
    termination_time int,
    termination_reason text,
    shards_amount int
);
CREATE TYPE argus.performance_hdrhistogram (
    start_time int,
    percentile_90 float,
    percentile_50 float,
    percentile_99_999 float,
    percentile_95 float,
    end_time float,
    percentile_99_99 float,
    percentile_99 float,
    stddev float,
    percentile_99_9 float
);
CREATE TYPE argus.environment_info (
    key text,
    value text
);
CREATE TYPE argus.nodedescription (
    name text,
    ip text,
    shards int
);
CREATE TYPE argus.eventsbyseverity (
    severity text,
    event_amount int,
    last_events list<text>
);
CREATE TYPE argus.cloudsetupdetails (
    db_node frozen<cloudnodesinfo>,
    loader_node frozen<cloudnodesinfo>,
    monitor_node frozen<cloudnodesinfo>,
    backend text
);
CREATE TYPE argus.test_suite (
    name text,
    tests_total int,
    failures int,
    disabled int,
    skipped int,
    passed int,
    errors int,
    time float,
    cases list<frozen<test_case>>
);
CREATE TYPE argus.cloudresource_v3 (
    name text,
    state text,
    resource_type text,
    instance_info frozen<cloudinstancedetails_v3>
);
CREATE TYPE argus.nemesisruninfo (
    class_name text,
    name text,
    duration int,
    target_node frozen<nodedescription>,
    status text,
    start_time int,
    end_time int,
    stack_trace text
);
CREATE TYPE argus.test_collection (
    name text,
    driver text,
    tests_total int,
    failure_message text,
    failures int,
    disabled int,
    skipped int,
    passed int,
    errors int,
    timestamp timestamp,
    time float,
    suites list<frozen<test_suite>>
);
CREATE TABLE argus.argus_event (
    id uuid,
    body text,
    created_at timestamp,
    group_id uuid,
    kind text,
    release_id uuid,
    run_id uuid,
    test_id uuid,
    user_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX argus_event_group_id_idx ON argus.argus_event(group_id);

CREATE INDEX argus_event_kind_idx ON argus.argus_event(kind);

CREATE INDEX argus_event_release_id_idx ON argus.argus_event(release_id);

CREATE INDEX argus_event_run_id_idx ON argus.argus_event(run_id);

CREATE INDEX argus_event_test_id_idx ON argus.argus_event(test_id);

CREATE INDEX argus_event_user_id_idx ON argus.argus_event(user_id);

CREATE TABLE argus.argus_group_v2 (
    id uuid,
    assignee list<uuid>,
    build_system_id text,
    description text,
    enabled boolean,
    name text,
    pretty_name text,
    release_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX argus_group_v2_name_idx ON argus.argus_group_v2(name);

CREATE INDEX argus_group_v2_release_id_idx ON argus.argus_group_v2(release_id);

CREATE TABLE argus.argus_notification (
    receiver uuid,
    id timeuuid,
    content text,
    sender uuid,
    source_id uuid,
    source_type text,
    state smallint,
    title text,
    type text,
    PRIMARY KEY (receiver, id)
) WITH CLUSTERING ORDER BY (id DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.argus_planner_comment_v2 (
    release uuid,
    group uuid,
    test uuid,
    comment text,
    PRIMARY KEY (release, group, test)
) WITH CLUSTERING ORDER BY (group ASC, test ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.argus_release_plan (
    id timeuuid,
    assignee_mapping map<uuid, uuid>,
    completed boolean,
    created_from uuid,
    creation_time timestamp,
    description text,
    ends_at timestamp,
    groups list<uuid>,
    key text,
    last_updated timestamp,
    name text,
    options text,
    owner uuid,
    participants list<uuid>,
    release_id uuid,
    target_version ascii,
    tests list<uuid>,
    view_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX argus_release_plan_created_from_idx ON argus.argus_release_plan(created_from);

CREATE INDEX argus_release_plan_release_id_idx ON argus.argus_release_plan(release_id);

CREATE INDEX argus_release_plan_target_version_idx ON argus.argus_release_plan(target_version);

CREATE INDEX argus_release_plan_view_id_idx ON argus.argus_release_plan(view_id);

CREATE TABLE argus.argus_release_stats_snapshot (
    release_id uuid,
    filter_key text,
    generated_at timestamp,
    payload text,
    PRIMARY KEY (release_id, filter_key)
) WITH CLUSTERING ORDER BY (filter_key ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.argus_release_v2 (
    id uuid,
    assignee list<uuid>,
    description text,
    dormant boolean,
    enabled boolean,
    github_repo_url text,
    name text,
    perpetual boolean,
    picture_id uuid,
    pretty_name text,
    valid_version_regex text,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX argus_release_v2_name_idx ON argus.argus_release_v2(name);

CREATE TABLE argus.argus_test_run_comment (
    id uuid,
    posted_at int,
    mentions list<uuid>,
    message text,
    reactions map<text, int>,
    release_id uuid,
    test_id uuid,
    test_run_id uuid,
    user_id uuid,
    PRIMARY KEY (id, posted_at)
) WITH CLUSTERING ORDER BY (posted_at DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX argus_test_run_comment_release_id_idx ON argus.argus_test_run_comment(release_id);

CREATE INDEX argus_test_run_comment_test_id_idx ON argus.argus_test_run_comment(test_id);

CREATE INDEX argus_test_run_comment_test_run_id_idx ON argus.argus_test_run_comment(test_run_id);

CREATE INDEX argus_test_run_comment_user_id_idx ON argus.argus_test_run_comment(user_id);

CREATE TABLE argus.argus_test_v2 (
    id uuid,
    assignee list<uuid>,
    build_system_id text,
    build_system_url text,
    description text,
    enabled boolean,
    group_id uuid,
    name text,
    plugin_name text,
    plugin_subtype text,
    pretty_name text,
    release_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX argus_test_v2_build_system_id_idx ON argus.argus_test_v2(build_system_id);

CREATE INDEX argus_test_v2_group_id_idx ON argus.argus_test_v2(group_id);

CREATE INDEX argus_test_v2_name_idx ON argus.argus_test_v2(name);

CREATE INDEX argus_test_v2_release_id_idx ON argus.argus_test_v2(release_id);

CREATE TABLE argus.argus_user_view (
    id uuid,
    created timestamp,
    description text,
    display_name text,
    group_ids list<uuid>,
    last_updated timestamp,
    name text,
    plan_id uuid,
    release_ids list<uuid>,
    tests list<uuid>,
    user_id uuid,
    widget_settings text,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX argus_user_view_name_idx ON argus.argus_user_view(name);

CREATE INDEX argus_user_view_plan_id_idx ON argus.argus_user_view(plan_id);

CREATE INDEX argus_user_view_user_id_idx ON argus.argus_user_view(user_id);

CREATE TABLE argus.critical_event_embeddings (
    run_id uuid,
    event_index int,
    start_time timestamp STATIC,
    duplicates_list frozen<list<int>>,
    embedding frozen<list<float>>,
    similars_map frozen<map<uuid, int>>,
    PRIMARY KEY (run_id, event_index)
) WITH CLUSTERING ORDER BY (event_index ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.driver_test_run (
    build_id text,
    start_time timestamp,
    assignee uuid,
    build_job_url text,
    build_number int,
    end_time timestamp,
    environment_info list<frozen<environment_info>>,
    group_id uuid,
    heartbeat int,
    id uuid,
    investigation_status text,
    logs list<frozen<tuple<text, text>>>,
    product_version text,
    release_id uuid,
    scylla_version text,
    status text,
    test_collection list<frozen<test_collection>>,
    test_id uuid,
    PRIMARY KEY (build_id, start_time)
) WITH CLUSTERING ORDER BY (start_time DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX driver_test_run_assignee_idx ON argus.driver_test_run(assignee);

CREATE INDEX driver_test_run_group_id_idx ON argus.driver_test_run(group_id);

CREATE INDEX driver_test_run_id_idx ON argus.driver_test_run(id);

CREATE INDEX driver_test_run_product_version_idx ON argus.driver_test_run(product_version);

CREATE INDEX driver_test_run_release_id_idx ON argus.driver_test_run(release_id);

CREATE INDEX driver_test_run_test_id_idx ON argus.driver_test_run(test_id);

CREATE TABLE argus.error_event_embeddings (
    run_id uuid,
    event_index int,
    start_time timestamp STATIC,
    duplicates_list frozen<list<int>>,
    embedding frozen<list<float>>,
    similars_map frozen<map<uuid, int>>,
    PRIMARY KEY (run_id, event_index)
) WITH CLUSTERING ORDER BY (event_index ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.generic_result_best_v2 (
    test_id uuid,
    name text,
    result_date timestamp,
    key ascii,
    run_id uuid,
    value double,
    PRIMARY KEY ((test_id, name), result_date, key)
) WITH CLUSTERING ORDER BY (result_date DESC, key ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.generic_result_data_v1 (
    test_id uuid,
    name text,
    run_id uuid,
    column ascii,
    row ascii,
    status ascii,
    sut_timestamp timestamp,
    value double,
    value_text text,
    PRIMARY KEY ((test_id, name), run_id, column, row)
) WITH CLUSTERING ORDER BY (run_id ASC, column ASC, row ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX generic_result_data_v1_column_idx ON argus.generic_result_data_v1(column);

CREATE INDEX generic_result_data_v1_row_idx ON argus.generic_result_data_v1(row);

CREATE TABLE argus.generic_result_metadata_v1 (
    test_id uuid,
    name text,
    columns_meta list<frozen<column_metadata>>,
    description text,
    rows_meta list<ascii>,
    sut_package_name ascii,
    validation_rules map<ascii, frozen<list<frozen<validation_rules>>>>,
    PRIMARY KEY (test_id, name)
) WITH CLUSTERING ORDER BY (name ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.generic_run (
    build_id text,
    start_time timestamp,
    assignee uuid,
    build_job_url text,
    build_number int,
    end_time timestamp,
    group_id uuid,
    heartbeat int,
    id uuid,
    investigation_status text,
    logs map<text, text>,
    product_version text,
    release_id uuid,
    scylla_version text,
    started_by text,
    status text,
    sub_type text,
    test_id uuid,
    PRIMARY KEY (build_id, start_time)
) WITH CLUSTERING ORDER BY (start_time DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX generic_run_assignee_idx ON argus.generic_run(assignee);

CREATE INDEX generic_run_group_id_idx ON argus.generic_run(group_id);

CREATE INDEX generic_run_id_idx ON argus.generic_run(id);

CREATE INDEX generic_run_product_version_idx ON argus.generic_run(product_version);

CREATE INDEX generic_run_release_id_idx ON argus.generic_run(release_id);

CREATE INDEX generic_run_test_id_idx ON argus.generic_run(test_id);

CREATE TABLE argus.github_issue (
    id uuid,
    added_on timestamp,
    assignees list<frozen<issue_assignee>>,
    labels list<frozen<issue_label>>,
    number int,
    owner text,
    repo text,
    state text,
    title text,
    type text,
    url text,
    user_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX github_issue_url_idx ON argus.github_issue(url);

CREATE INDEX github_issue_user_id_idx ON argus.github_issue(user_id);

CREATE TABLE argus.graph_view_v1 (
    test_id uuid,
    id uuid,
    description text,
    graphs map<text, ascii>,
    name text,
    PRIMARY KEY (test_id, id)
) WITH CLUSTERING ORDER BY (id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.issue_link (
    run_id uuid,
    issue_id uuid,
    added_on timestamp,
    event_id uuid,
    group_id uuid,
    release_id uuid,
    test_id uuid,
    type text,
    user_id uuid,
    PRIMARY KEY (run_id, issue_id)
) WITH CLUSTERING ORDER BY (issue_id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX issue_link_event_id_idx ON argus.issue_link(event_id);

CREATE INDEX issue_link_group_id_idx ON argus.issue_link(group_id);

CREATE INDEX issue_link_release_id_idx ON argus.issue_link(release_id);

CREATE INDEX issue_link_test_id_idx ON argus.issue_link(test_id);

CREATE INDEX issue_link_user_id_idx ON argus.issue_link(user_id);

CREATE TABLE argus.jira_issue (
    id uuid,
    added_on timestamp,
    assignees list<text>,
    key text,
    labels list<frozen<issue_label>>,
    permalink text,
    project text,
    state text,
    summary text,
    user_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX jira_issue_permalink_idx ON argus.jira_issue(permalink);

CREATE INDEX jira_issue_user_id_idx ON argus.jira_issue(user_id);

CREATE TABLE argus.proxy_tunnel_config (
    id uuid,
    host text,
    host_key_fingerprint text,
    is_active boolean,
    port int,
    proxy_user text,
    service_user_id uuid,
    target_host text,
    target_port int,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.pytest_result_table (
    name text,
    id timeuuid,
    duration double,
    markers list<text>,
    message text,
    release_id uuid,
    run_id uuid,
    session_timestamp timestamp,
    status text,
    test_id uuid,
    test_timestamp timestamp,
    test_type text,
    user_fields map<text, text>,
    PRIMARY KEY (name, id)
) WITH CLUSTERING ORDER BY (id DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX pytest_result_table_release_id_idx ON argus.pytest_result_table(release_id);

CREATE INDEX pytest_result_table_run_id_idx ON argus.pytest_result_table(run_id);

CREATE INDEX pytest_result_table_test_id_idx ON argus.pytest_result_table(test_id);

CREATE INDEX pytest_result_table_user_entries_idx ON argus.pytest_result_table(entries(user_fields));

CREATE INDEX pytest_result_table_user_key_idx ON argus.pytest_result_table(keys(user_fields));

CREATE INDEX pytest_result_table_user_value_idx ON argus.pytest_result_table(values(user_fields));

CREATE TABLE argus.pytest_user_field (
    name text,
    id timestamp,
    field_name text,
    field_value text,
    PRIMARY KEY (name, id, field_name)
) WITH CLUSTERING ORDER BY (id ASC, field_name ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.pytest_v2 (
    name text,
    status text,
    id timestamp,
    duration double,
    markers list<text>,
    message text,
    release_id uuid,
    run_id uuid,
    session_timestamp timestamp,
    test_id uuid,
    test_timestamp timestamp,
    test_type text,
    PRIMARY KEY (name, status, id)
) WITH CLUSTERING ORDER BY (status ASC, id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX pytest_v2_release_id_idx ON argus.pytest_v2(release_id);

CREATE INDEX pytest_v2_run_id_idx ON argus.pytest_v2(run_id);

CREATE INDEX pytest_v2_test_id_idx ON argus.pytest_v2(test_id);

CREATE TABLE argus.release_distinct_images (
    release_id uuid,
    image_id text,
    PRIMARY KEY (release_id, image_id)
) WITH CLUSTERING ORDER BY (image_id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.release_distinct_versions (
    release_id uuid,
    version text,
    PRIMARY KEY (release_id, version)
) WITH CLUSTERING ORDER BY (version ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.run_config_param (
    name text,
    value text,
    run_id text,
    PRIMARY KEY ((name, value), run_id)
) WITH CLUSTERING ORDER BY (run_id ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.run_configuration (
    run_id uuid,
    name text,
    content text,
    PRIMARY KEY (run_id, name)
) WITH CLUSTERING ORDER BY (name ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.runtime_store (
    key text,
    value_boolean boolean,
    value_date timestamp,
    value_float double,
    value_int int,
    value_text text,
    value_ts time,
    value_type ascii,
    PRIMARY KEY (key)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.sct_event (
    run_id uuid,
    severity text,
    ts timestamp,
    duplicate_id uuid,
    duration float,
    event_id uuid,
    event_type text,
    known_issue text,
    message text,
    nemesis_name text,
    nemesis_status text,
    node text,
    received_timestamp timestamp,
    summary text,
    target_node text,
    PRIMARY KEY ((run_id, severity), ts)
) WITH CLUSTERING ORDER BY (ts ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX sct_event_event_id_idx ON argus.sct_event(event_id);

CREATE TABLE argus.sct_nemesis (
    run_id uuid,
    start_time int,
    class_name text,
    duration int,
    end_time int,
    name text,
    stack_trace text,
    status text,
    target_node frozen<nodedescription>,
    PRIMARY KEY (run_id, start_time)
) WITH CLUSTERING ORDER BY (start_time ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.sct_resource (
    run_id uuid,
    name text,
    instance_info frozen<cloudinstancedetails_v3>,
    resource_type text,
    state text,
    PRIMARY KEY (run_id, name)
) WITH CLUSTERING ORDER BY (name ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.sct_test_run (
    build_id text,
    start_time timestamp,
    allocated_resources list<frozen<cloudresource_v3>>,
    assignee uuid,
    branch_name text,
    build_job_url text,
    build_number int,
    cloud_setup frozen<cloudsetupdetails>,
    config_files list<text>,
    end_time timestamp,
    events list<frozen<eventsbyseverity>>,
    gemini_command text,
    gemini_read_errors int,
    gemini_read_ops int,
    gemini_seed text,
    gemini_status text,
    gemini_version text,
    gemini_write_errors int,
    gemini_write_ops int,
    group_id uuid,
    heartbeat int,
    histograms list<frozen<map<text, frozen<performance_hdrhistogram>>>>,
    id uuid,
    investigation_status text,
    logs list<frozen<tuple<text, text>>>,
    nemesis_data list<frozen<nemesisruninfo>>,
    nemesis_stats map<text, int>,
    oracle_node_ami_id text,
    oracle_node_instance_type text,
    oracle_node_scylla_version text,
    oracle_nodes_count int,
    origin_url text,
    packages list<frozen<packageversion_v2>>,
    perf_avg_latency_99th double,
    perf_avg_latency_mean double,
    perf_op_rate_average double,
    perf_op_rate_total double,
    perf_total_errors double,
    product_version text,
    region_name list<text>,
    release_id uuid,
    scm_revision_id text,
    screenshots list<text>,
    sct_runner_host frozen<cloudinstancedetails_v3>,
    scylla_version text,
    started_by text,
    status text,
    stress_cmd text,
    stress_duration float,
    subtest_name text,
    test_id uuid,
    test_method ascii,
    test_name text,
    version_source text,
    yaml_test_duration int,
    PRIMARY KEY (build_id, start_time)
) WITH CLUSTERING ORDER BY (start_time DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX sct_test_run_assignee_idx ON argus.sct_test_run(assignee);

CREATE INDEX sct_test_run_group_id_idx ON argus.sct_test_run(group_id);

CREATE INDEX sct_test_run_id_idx ON argus.sct_test_run(id);

CREATE INDEX sct_test_run_product_version_idx ON argus.sct_test_run(product_version);

CREATE INDEX sct_test_run_release_id_idx ON argus.sct_test_run(release_id);

CREATE INDEX sct_test_run_test_id_idx ON argus.sct_test_run(test_id);

CREATE TABLE argus.sct_unprocessed_events (
    run_id uuid,
    severity text,
    ts timestamp,
    PRIMARY KEY (run_id, severity, ts)
) WITH CLUSTERING ORDER BY (severity ASC, ts DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.sctjunit_reports (
    test_id uuid,
    file_name text,
    report text,
    PRIMARY KEY (test_id, file_name)
) WITH CLUSTERING ORDER BY (file_name ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.sirenada_run (
    build_id text,
    start_time timestamp,
    assignee uuid,
    browsers list<text>,
    build_job_url text,
    build_number int,
    clusters list<text>,
    end_time timestamp,
    group_id uuid,
    heartbeat int,
    id uuid,
    investigation_status text,
    logs map<text, text>,
    product_version text,
    region text,
    release_id uuid,
    results list<frozen<sirenada_test>>,
    s3_folder_ids list<frozen<tuple<text, text>>>,
    sct_test_id uuid,
    scylla_version text,
    sirenada_test_ids list<text>,
    status text,
    test_id uuid,
    PRIMARY KEY (build_id, start_time)
) WITH CLUSTERING ORDER BY (start_time DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX sirenada_run_assignee_idx ON argus.sirenada_run(assignee);

CREATE INDEX sirenada_run_group_id_idx ON argus.sirenada_run(group_id);

CREATE INDEX sirenada_run_id_idx ON argus.sirenada_run(id);

CREATE INDEX sirenada_run_product_version_idx ON argus.sirenada_run(product_version);

CREATE INDEX sirenada_run_release_id_idx ON argus.sirenada_run(release_id);

CREATE INDEX sirenada_run_test_id_idx ON argus.sirenada_run(test_id);

CREATE TABLE argus.sshtunnel_key (
    id uuid,
    created_at timestamp,
    expires_at timestamp,
    fingerprint text,
    public_key text,
    tunnel_id uuid,
    user_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX sshtunnel_key_fingerprint_idx ON argus.sshtunnel_key(fingerprint);

CREATE INDEX sshtunnel_key_tunnel_id_idx ON argus.sshtunnel_key(tunnel_id);

CREATE INDEX sshtunnel_key_user_id_idx ON argus.sshtunnel_key(user_id);

CREATE TABLE argus.stress_command (
    run_id uuid,
    ts timestamp,
    cmd text,
    log_name text,
    node_name text,
    PRIMARY KEY (run_id, ts)
) WITH CLUSTERING ORDER BY (ts DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.team (
    id uuid,
    leader uuid,
    members list<uuid>,
    motd text,
    name text,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX team_leader_idx ON argus.team(leader);

CREATE TABLE argus.user (
    id uuid,
    api_token text,
    email text,
    full_name text,
    password text,
    picture_id uuid,
    registration_date timestamp,
    roles list<text>,
    service_user boolean,
    username text,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX user_api_token_idx ON argus.user(api_token);

CREATE INDEX user_email_idx ON argus.user(email);

CREATE INDEX user_username_idx ON argus.user(username);

CREATE TABLE argus.user_oauth_token (
    id uuid,
    kind text,
    "token" text,
    user_id uuid,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE INDEX user_oauth_token_kind_idx ON argus.user_oauth_token(kind);

CREATE INDEX user_oauth_token_user_id_idx ON argus.user_oauth_token(user_id);

CREATE TABLE argus.web_file_storage (
    id uuid,
    filename text,
    filepath text,
    PRIMARY KEY (id)
) WITH bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.widget_comment (
    view_id uuid,
    "index" int,
    highlight_at timestamp,
    created_at timestamp,
    content text,
    creator_id uuid,
    PRIMARY KEY ((view_id, "index", highlight_at), created_at)
) WITH CLUSTERING ORDER BY (created_at ASC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus.widget_highlights (
    view_id uuid,
    "index" int,
    created_at timestamp,
    archived_at timestamp,
    assignee_id uuid,
    comments_count tinyint,
    completed boolean,
    content text,
    creator_id uuid,
    group text,
    PRIMARY KEY ((view_id, "index"), created_at)
) WITH CLUSTERING ORDER BY (created_at DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE KEYSPACE argus_tablets WITH replication = {'class': 'org.apache.cassandra.locator.NetworkTopologyStrategy', 'datacenter1': '1'} AND durable_writes = true AND tablets = {'enabled': true};
CREATE TABLE argus_tablets.sct_critical_event_embedding (
    run_id uuid,
    ts timestamp,
    embedding vector<float, 384>,
    PRIMARY KEY (run_id, ts)
) WITH CLUSTERING ORDER BY (ts DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE CUSTOM INDEX sct_critical_event_embedding_embedding_idx ON argus_tablets.sct_critical_event_embedding(embedding) USING 'vector_index'
ALTER TABLE argus_tablets.sct_critical_event_embedding_scylla_cdc_log WITH bloom_filter_fp_chance = 0.01
    AND caching = {'enabled': 'false', 'keys': 'NONE', 'rows_per_partition': 'NONE'}
    AND comment = 'CDC log for argus_tablets.sct_critical_event_embedding'
    AND compaction = {'class': 'TimeWindowCompactionStrategy', 'compaction_window_size': '60', 'compaction_window_unit': 'MINUTES', 'expired_sstable_check_frequency_seconds': '1800'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 0
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE TABLE argus_tablets.sct_error_event_embedding (
    run_id uuid,
    ts timestamp,
    embedding vector<float, 384>,
    PRIMARY KEY (run_id, ts)
) WITH CLUSTERING ORDER BY (ts DESC)
    AND bloom_filter_fp_chance = 0.01
    AND caching = {'keys': 'ALL', 'rows_per_partition': 'ALL'}
    AND comment = ''
    AND compaction = {'class': 'IncrementalCompactionStrategy'}
    AND compression = {'sstable_compression': 'LZ4WithDictsCompressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 864000
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

CREATE CUSTOM INDEX sct_error_event_embedding_embedding_idx ON argus_tablets.sct_error_event_embedding(embedding) USING 'vector_index'
ALTER TABLE argus_tablets.sct_error_event_embedding_scylla_cdc_log WITH bloom_filter_fp_chance = 0.01
    AND caching = {'enabled': 'false', 'keys': 'NONE', 'rows_per_partition': 'NONE'}
    AND comment = 'CDC log for argus_tablets.sct_error_event_embedding'
    AND compaction = {'class': 'TimeWindowCompactionStrategy', 'compaction_window_size': '60', 'compaction_window_unit': 'MINUTES', 'expired_sstable_check_frequency_seconds': '1800'}
    AND compression = {'sstable_compression': 'org.apache.cassandra.io.compress.LZ4Compressor'}
    AND crc_check_chance = 1
    AND default_time_to_live = 0
    AND gc_grace_seconds = 0
    AND max_index_interval = 2048
    AND memtable_flush_period_in_ms = 0
    AND min_index_interval = 128
    AND speculative_retry = '99.0PERCENTILE'
    AND tombstone_gc = {'mode': 'timeout', 'propagation_delay_in_seconds': '3600'};

