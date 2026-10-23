// Generated from datalib 0.44.0 (its `indexed_markdown::store_ddl`, `lookup_indexes`,
// `time_series_schema::store_ddl` and `indexes`). Byte for byte: the text is hashed.

pub const RENDER_DDL: &[&str] = &[
    r#"CREATE TABLE IF NOT EXISTS grid_rows (
    uuid VARCHAR(96) NOT NULL,
    provider VARCHAR(32) NOT NULL,
    kind VARCHAR(32) NOT NULL,
    source_label VARCHAR(32) NOT NULL,
    created_at VARCHAR(40),
    created_at_utc VARCHAR(40),
    created_offset VARCHAR(8),
    modified_at VARCHAR(40),
    modified_at_utc VARCHAR(40),
    modified_offset VARCHAR(8),
    touched_at VARCHAR(40),
    touched_at_utc VARCHAR(40),
    touched_offset VARCHAR(8),
    is_document INTEGER NOT NULL,
    author VARCHAR(255),
    author_handle VARCHAR(255),
    contact VARCHAR(255),
    email VARCHAR(255),
    phone VARCHAR(64),
    account VARCHAR(96),
    project VARCHAR(96),
    org_uuid VARCHAR(96),
    org_name VARCHAR(255),
    channel VARCHAR(255),
    conversation_name TEXT,
    conversation_uuid VARCHAR(96) NOT NULL,
    message_index INT,
    entire_chat VARCHAR(255) NOT NULL,
    preview TEXT NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    qmd_path VARCHAR(512),
    source_id VARCHAR(96),
    source_url VARCHAR(1024),
    git_sha VARCHAR(64),
    upstream_id VARCHAR(128),
    upstream_entity_kind VARCHAR(32),
    upstream_account VARCHAR(96),
    notion_page_uuid VARCHAR(96),
    notion_block_uuid VARCHAR(96),
    markdown_uuid VARCHAR(96),
    byte_size BIGINT,
    item_count BIGINT,
    diff_status VARCHAR(16),
    diff_changed_columns TEXT,
    supplied_terms TEXT,
    PRIMARY KEY (uuid)
)"#,
    r#"CREATE TABLE IF NOT EXISTS markdowns (
    markdown_uuid VARCHAR(96) NOT NULL,
    source_id VARCHAR(64) NOT NULL,
    provider VARCHAR(32) NOT NULL,
    kind VARCHAR(32) NOT NULL,
    title TEXT,
    created_at VARCHAR(40),
    modified_at VARCHAR(40),
    item_count BIGINT,
    md_path VARCHAR(1024),
    upstream_cursor VARCHAR(64),
    renderer_version VARCHAR(32),
    bucket_key VARCHAR(256),
    PRIMARY KEY (markdown_uuid)
)"#,
    r#"CREATE TABLE IF NOT EXISTS edges (
    edge_uuid VARCHAR(96) NOT NULL,
    src_markdown_uuid VARCHAR(96) NOT NULL,
    src_anchor_uuid VARCHAR(96),
    dst_markdown_uuid VARCHAR(96) NOT NULL,
    dst_anchor_uuid VARCHAR(96),
    label VARCHAR(64),
    PRIMARY KEY (edge_uuid)
)"#,
    r#"CREATE TABLE IF NOT EXISTS source_contacts (
    markdown_uuid VARCHAR(96) NOT NULL,
    contact_key VARCHAR(256) NOT NULL,
    source_id VARCHAR(128) NOT NULL,
    name TEXT,
    seen_items INTEGER NOT NULL,
    last_seen_at VARCHAR(64),
    contact_json TEXT NOT NULL,
    PRIMARY KEY (markdown_uuid, contact_key)
)"#,
    r#"CREATE TABLE IF NOT EXISTS source_contact_handles (
    markdown_uuid VARCHAR(96) NOT NULL,
    contact_key VARCHAR(256) NOT NULL,
    handle VARCHAR(256) NOT NULL,
    PRIMARY KEY (markdown_uuid, contact_key, handle)
)"#,
    r#"CREATE TABLE IF NOT EXISTS problems (
    problem_uuid VARCHAR(36) NOT NULL,
    source_id VARCHAR(64) NOT NULL,
    stage VARCHAR(16) NOT NULL,
    severity VARCHAR(16) NOT NULL,
    outcome VARCHAR(16) NOT NULL,
    reason VARCHAR(32) NOT NULL,
    scope_kind VARCHAR(16) NOT NULL,
    scope_key VARCHAR(96) NOT NULL,
    item_uuid VARCHAR(96),
    field VARCHAR(128),
    path VARCHAR(255),
    rule VARCHAR(128),
    sample TEXT NOT NULL,
    first_seen_at_utc VARCHAR(40) NOT NULL,
    changed_at_utc VARCHAR(40) NOT NULL,
    tz_offset VARCHAR(8),
    render_version INT,
    PRIMARY KEY (problem_uuid)
)"#,
    r#"CREATE TABLE IF NOT EXISTS source_measurements (
    subject VARCHAR(512) NOT NULL,
    kind VARCHAR(16) NOT NULL,
    measured_at_utc VARCHAR(40) NOT NULL,
    tz_offset VARCHAR(8),
    bytes BIGINT,
    items BIGINT,
    PRIMARY KEY (subject, measured_at_utc)
)"#,
    r#"CREATE TABLE IF NOT EXISTS render_cursor (
    source_id VARCHAR(64) NOT NULL,
    raw_commit VARCHAR(64) NOT NULL,
    params TEXT NOT NULL,
    rendered_at_utc VARCHAR(40) NOT NULL,
    tz_offset VARCHAR(8),
    PRIMARY KEY (source_id)
)"#,
    r#"CREATE TABLE IF NOT EXISTS render_inputs (
    bucket_key VARCHAR(256) NOT NULL,
    input_table VARCHAR(64) NOT NULL,
    input_id VARCHAR(256) NOT NULL,
    PRIMARY KEY (bucket_key, input_table, input_id)
)"#,
    r#"CREATE INDEX IF NOT EXISTS render_inputs_by_input ON render_inputs (input_table, input_id)"#,
];

pub const RENDER_INDEXES: &[&str] = &[
    r#"CREATE INDEX IF NOT EXISTS grid_rows_by_markdown ON grid_rows (markdown_uuid)"#,
    r#"CREATE INDEX IF NOT EXISTS edges_by_src_markdown ON edges (src_markdown_uuid)"#,
    r#"CREATE INDEX IF NOT EXISTS source_contact_handles_by_handle ON source_contact_handles (handle)"#,
    r#"CREATE INDEX IF NOT EXISTS problems_by_scope ON problems (scope_kind, scope_key)"#,
];

pub const RENDER_SCHEMA_HASH: &str =
    "c760920ad10f2e6e8d78b5bfa8d00e82cf1ee616cbaf23a050be7bbf732ba3b6";

pub const TIME_SERIES_DDL: &[&str] = &[
    r#"CREATE TABLE IF NOT EXISTS time_series (
    source_id VARCHAR(128) NOT NULL,
    ts_key INTEGER NOT NULL,
    time_series_id VARCHAR(96) NOT NULL,
    about_uuid VARCHAR(96),
    about_name TEXT,
    name VARCHAR(128) NOT NULL,
    human_name TEXT NOT NULL,
    unit VARCHAR(64) NOT NULL,
    value_type VARCHAR(16) NOT NULL,
    time_kind VARCHAR(16) NOT NULL,
    point_count INTEGER NOT NULL,
    first_t INTEGER,
    last_t INTEGER,
    PRIMARY KEY (source_id, ts_key)
)"#,
    r#"CREATE TABLE IF NOT EXISTS time_series_chunks (
    ts_key INTEGER NOT NULL,
    first_t INTEGER NOT NULL,
    last_t INTEGER NOT NULL,
    n INTEGER NOT NULL,
    min REAL,
    max REAL,
    sum REAL,
    encoding INTEGER NOT NULL,
    points BLOB NOT NULL,
    PRIMARY KEY (ts_key, first_t)
)"#,
];

pub const TIME_SERIES_INDEXES: &[&str] =
    &[r#"CREATE INDEX IF NOT EXISTS time_series_by_id ON time_series(time_series_id)"#];

pub const TIME_SERIES_SCHEMA_HASH: &str =
    "24862dbebaab365f61adc753d5fc9bd879b6fce12e9fbc4d6ae6fdb951449193";
