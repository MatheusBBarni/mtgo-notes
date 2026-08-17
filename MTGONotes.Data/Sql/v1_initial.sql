
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY CHECK (version > 0),
    checksum TEXT NOT NULL CHECK (length(checksum) = 64),
    applied_at INTEGER NOT NULL CHECK (applied_at >= 0)
);

CREATE TABLE runtime_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    clean_shutdown INTEGER NOT NULL CHECK (clean_shutdown IN (0, 1)),
    last_integrity_at INTEGER
);
INSERT INTO runtime_state(singleton, clean_shutdown) VALUES (1, 1);

CREATE TABLE provider_consents (
    provider_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL CHECK (version > 0),
    granted_at INTEGER,
    revoked_at INTEGER,
    disclosed_fields_json TEXT NOT NULL
);

CREATE TABLE opponent_profiles (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    primary_handle TEXT NOT NULL CHECK (length(trim(primary_handle)) > 0),
    normalized_handle TEXT NOT NULL CHECK (length(normalized_handle) > 0),
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    deleted_at INTEGER
);
CREATE UNIQUE INDEX active_profile_handle
    ON opponent_profiles(normalized_handle) WHERE deleted_at IS NULL;

CREATE TABLE opponent_aliases (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    profile_id TEXT NOT NULL REFERENCES opponent_profiles(id) ON DELETE CASCADE,
    display_handle TEXT NOT NULL CHECK (length(trim(display_handle)) > 0),
    normalized_handle TEXT NOT NULL CHECK (length(normalized_handle) > 0),
    provenance TEXT NOT NULL,
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    UNIQUE(profile_id, normalized_handle)
);
CREATE INDEX alias_lookup ON opponent_aliases(normalized_handle);

CREATE TABLE encounters (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    profile_id TEXT NOT NULL REFERENCES opponent_profiles(id) ON DELETE CASCADE,
    format TEXT NOT NULL,
    started_at INTEGER NOT NULL CHECK (started_at >= 0),
    ended_at INTEGER,
    status TEXT NOT NULL CHECK (status IN ('active','finished','incomplete','deleted')),
    phase TEXT NOT NULL CHECK (phase IN (
        'pre_match','in_game_restricted','between_games',
        'completion_pending','finished','incomplete'
    )),
    source TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    incomplete_reason TEXT,
    deleted_at INTEGER
);
CREATE UNIQUE INDEX one_active_encounter
    ON encounters(status) WHERE status = 'active' AND deleted_at IS NULL;
CREATE INDEX encounter_profile_time ON encounters(profile_id, started_at DESC, id DESC);

CREATE TABLE encounter_transitions (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    encounter_id TEXT NOT NULL REFERENCES encounters(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence >= 0),
    from_phase TEXT NOT NULL,
    to_phase TEXT NOT NULL,
    trigger TEXT NOT NULL,
    confidence_class TEXT NOT NULL,
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    undo_group_id TEXT,
    UNIQUE(encounter_id, sequence)
);

CREATE TABLE observations (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    encounter_id TEXT NOT NULL REFERENCES encounters(id) ON DELETE CASCADE,
    text TEXT NOT NULL CHECK (length(trim(text)) > 0),
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    edited_at INTEGER,
    revision INTEGER NOT NULL CHECK (revision > 0),
    searchable INTEGER NOT NULL DEFAULT 1 CHECK (searchable IN (0, 1)),
    deletion_deadline INTEGER,
    deleted_at INTEGER
);
CREATE INDEX observation_encounter_time
    ON observations(encounter_id, created_at DESC, id DESC);

CREATE TABLE card_observations (
    observation_id TEXT NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
    oracle_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    certainty TEXT NOT NULL CHECK (certainty IN ('observed','suspected')),
    context TEXT,
    PRIMARY KEY(observation_id, oracle_id, certainty)
);

CREATE TABLE tendency_tags (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    normalized_label TEXT NOT NULL UNIQUE,
    display_label TEXT NOT NULL
);

CREATE TABLE observation_tags (
    observation_id TEXT NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tendency_tags(id) ON DELETE RESTRICT,
    PRIMARY KEY(observation_id, tag_id)
);

CREATE TABLE deck_records (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    profile_id TEXT NOT NULL REFERENCES opponent_profiles(id) ON DELETE CASCADE,
    source_class TEXT NOT NULL CHECK (source_class IN ('public','user')),
    format TEXT NOT NULL,
    completeness TEXT NOT NULL CHECK (completeness IN ('complete','partial')),
    provider_label TEXT,
    user_label TEXT,
    current_revision INTEGER NOT NULL CHECK (current_revision > 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    deleted_at INTEGER
);

CREATE TABLE deck_revisions (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    deck_id TEXT NOT NULL REFERENCES deck_records(id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    canonical_digest TEXT NOT NULL,
    complete INTEGER NOT NULL CHECK (complete IN (0, 1)),
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    UNIQUE(deck_id, revision_number)
);

CREATE TABLE deck_cards (
    deck_revision_id TEXT NOT NULL REFERENCES deck_revisions(id) ON DELETE CASCADE,
    oracle_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    zone TEXT NOT NULL CHECK (zone IN ('main','sideboard')),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    basic_land INTEGER NOT NULL DEFAULT 0 CHECK (basic_land IN (0, 1)),
    PRIMARY KEY(deck_revision_id, oracle_id, zone)
);

CREATE TABLE public_snapshots (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    encounter_id TEXT NOT NULL REFERENCES encounters(id) ON DELETE CASCADE,
    deck_revision_id TEXT NOT NULL REFERENCES deck_revisions(id) ON DELETE RESTRICT,
    provider TEXT NOT NULL,
    event TEXT NOT NULL,
    format TEXT NOT NULL,
    publication_date INTEGER NOT NULL,
    source_url TEXT NOT NULL,
    confirmed INTEGER NOT NULL CHECK (confirmed IN (0, 1)),
    source_token TEXT NOT NULL,
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    UNIQUE(encounter_id, source_token)
);

CREATE TABLE classification_runs (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    deck_revision_id TEXT NOT NULL REFERENCES deck_revisions(id) ON DELETE CASCADE,
    classifier_version TEXT NOT NULL,
    classifier_digest TEXT NOT NULL,
    result_id TEXT NOT NULL,
    result_name TEXT NOT NULL,
    method TEXT NOT NULL CHECK (method IN ('signature','knn','unsupported')),
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    explanation_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('successful','failed','interrupted')),
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    UNIQUE(deck_revision_id, classifier_version)
);

CREATE TABLE profile_merges (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    primary_profile_id TEXT NOT NULL REFERENCES opponent_profiles(id),
    state TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    reversed_at INTEGER,
    reassignment_plan_json TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0)
);

CREATE TABLE deletion_tombstones (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    requested_at INTEGER NOT NULL,
    effective_at INTEGER NOT NULL,
    undo_token_digest TEXT NOT NULL,
    purge_state TEXT NOT NULL,
    PRIMARY KEY(entity_type, entity_id)
);

CREATE TABLE background_jobs (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    kind TEXT NOT NULL,
    payload_version INTEGER NOT NULL,
    cursor TEXT,
    state TEXT NOT NULL,
    priority INTEGER NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    total INTEGER NOT NULL DEFAULT 0,
    last_error_code TEXT,
    revision INTEGER NOT NULL CHECK (revision > 0)
);

CREATE TABLE operation_records (
    id TEXT PRIMARY KEY CHECK (
        length(id) = 36 AND substr(id, 15, 1) = '7'
        AND lower(substr(id, 20, 1)) IN ('8','9','a','b')
    ),
    kind TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE CHECK (
        length(idempotency_key) = 36 AND substr(idempotency_key, 15, 1) = '7'
        AND lower(substr(idempotency_key, 20, 1)) IN ('8','9','a','b')
    ),
    state TEXT NOT NULL,
    requested_at INTEGER NOT NULL,
    completed_at INTEGER,
    result_json TEXT,
    rollback_location TEXT,
    revision INTEGER NOT NULL CHECK (revision > 0)
);

CREATE TABLE restore_conflicts (
    id TEXT PRIMARY KEY CHECK (length(id) = 64),
    source_table TEXT NOT NULL,
    source_key TEXT NOT NULL,
    imported_record_json TEXT NOT NULL,
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    resolved_at INTEGER,
    UNIQUE(source_table, source_key, imported_record_json)
);

CREATE TABLE capture_drafts (
    encounter_id TEXT PRIMARY KEY REFERENCES encounters(id) ON DELETE CASCADE,
    encrypted_text BLOB NOT NULL,
    updated_at INTEGER NOT NULL,
    claimed_window_instance TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0)
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    CHECK (key NOT LIKE '%secret%' AND key NOT LIKE '%key%' AND key NOT LIKE '%passphrase%')
);

CREATE VIRTUAL TABLE history_fts USING fts5(
    entity_type UNINDEXED,
    entity_id UNINDEXED,
    sort_ms UNINDEXED,
    content,
    tokenize = 'unicode61'
);

CREATE TRIGGER profile_fts_insert AFTER INSERT ON opponent_profiles
WHEN new.deleted_at IS NULL
BEGIN
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
    VALUES ('profile', new.id, new.created_at, new.primary_handle);
END;
CREATE TRIGGER profile_fts_update AFTER UPDATE ON opponent_profiles
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'profile' AND entity_id = old.id;
    DELETE FROM history_fts
      WHERE entity_type = 'alias'
        AND entity_id IN (SELECT id FROM opponent_aliases WHERE profile_id = old.id);
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'profile', new.id, new.created_at, new.primary_handle
      WHERE new.deleted_at IS NULL;
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'alias', id, created_at, display_handle
      FROM opponent_aliases
      WHERE profile_id = new.id AND new.deleted_at IS NULL;
END;
CREATE TRIGGER profile_fts_delete AFTER DELETE ON opponent_profiles
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'profile' AND entity_id = old.id;
    DELETE FROM history_fts
      WHERE entity_type = 'alias'
        AND entity_id IN (SELECT id FROM opponent_aliases WHERE profile_id = old.id);
END;

CREATE TRIGGER alias_fts_insert AFTER INSERT ON opponent_aliases
WHEN EXISTS (
    SELECT 1 FROM opponent_profiles
    WHERE id = new.profile_id AND deleted_at IS NULL
)
BEGIN
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
    VALUES ('alias', new.id, new.created_at, new.display_handle);
END;
CREATE TRIGGER alias_fts_update AFTER UPDATE ON opponent_aliases
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'alias' AND entity_id = old.id;
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'alias', new.id, new.created_at, new.display_handle
      WHERE EXISTS (
        SELECT 1 FROM opponent_profiles
        WHERE id = new.profile_id AND deleted_at IS NULL
      );
END;
CREATE TRIGGER alias_fts_delete AFTER DELETE ON opponent_aliases
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'alias' AND entity_id = old.id;
END;

CREATE TRIGGER observation_fts_insert AFTER INSERT ON observations
WHEN new.deleted_at IS NULL AND new.searchable = 1
BEGIN
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
    VALUES ('observation', new.id, new.created_at, new.text);
END;
CREATE TRIGGER observation_fts_update AFTER UPDATE ON observations
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'observation' AND entity_id = old.id;
    DELETE FROM history_fts
      WHERE entity_type = 'card' AND entity_id GLOB old.id || ':*';
    DELETE FROM history_fts
      WHERE entity_type = 'tag' AND entity_id GLOB old.id || ':*';
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'observation', new.id, new.created_at, new.text
      WHERE new.deleted_at IS NULL AND new.searchable = 1;
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'card', card_observations.observation_id || ':' ||
        card_observations.oracle_id || ':' || card_observations.certainty,
        new.created_at,
        card_observations.display_name || ' ' || card_observations.certainty
      FROM card_observations
      WHERE card_observations.observation_id = new.id
        AND new.deleted_at IS NULL AND new.searchable = 1;
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'tag', observation_tags.observation_id || ':' || observation_tags.tag_id,
        new.created_at, tendency_tags.display_label
      FROM observation_tags
      JOIN tendency_tags ON tendency_tags.id = observation_tags.tag_id
      WHERE observation_tags.observation_id = new.id
        AND new.deleted_at IS NULL AND new.searchable = 1;
END;
CREATE TRIGGER observation_fts_delete AFTER DELETE ON observations
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'observation' AND entity_id = old.id;
    DELETE FROM history_fts
      WHERE entity_type = 'card' AND entity_id GLOB old.id || ':*';
    DELETE FROM history_fts
      WHERE entity_type = 'tag' AND entity_id GLOB old.id || ':*';
END;

CREATE TRIGGER deck_fts_insert AFTER INSERT ON deck_records
WHEN new.deleted_at IS NULL
BEGIN
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
    VALUES (
      'deck', new.id, new.created_at,
      trim(coalesce(new.provider_label, '') || ' ' || coalesce(new.user_label, ''))
    );
END;
CREATE TRIGGER deck_fts_update AFTER UPDATE ON deck_records
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'deck' AND entity_id = old.id;
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'deck', new.id, new.created_at,
        trim(coalesce(new.provider_label, '') || ' ' || coalesce(new.user_label, ''))
      WHERE new.deleted_at IS NULL;
END;
CREATE TRIGGER deck_fts_delete AFTER DELETE ON deck_records
BEGIN
    DELETE FROM history_fts WHERE entity_type = 'deck' AND entity_id = old.id;
END;

CREATE TRIGGER card_fts_insert AFTER INSERT ON card_observations
BEGIN
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'card', new.observation_id || ':' || new.oracle_id || ':' || new.certainty,
        observations.created_at, new.display_name || ' ' || new.certainty
      FROM observations
      WHERE observations.id = new.observation_id
        AND observations.deleted_at IS NULL
        AND observations.searchable = 1;
END;
CREATE TRIGGER card_fts_delete AFTER DELETE ON card_observations
BEGIN
    DELETE FROM history_fts
      WHERE entity_type = 'card'
        AND entity_id = old.observation_id || ':' || old.oracle_id || ':' || old.certainty;
END;

CREATE TRIGGER tag_link_fts_insert AFTER INSERT ON observation_tags
BEGIN
    INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
      SELECT 'tag', new.observation_id || ':' || new.tag_id,
        observations.created_at, tendency_tags.display_label
      FROM observations, tendency_tags
      WHERE observations.id = new.observation_id
        AND tendency_tags.id = new.tag_id
        AND observations.deleted_at IS NULL
        AND observations.searchable = 1;
END;
CREATE TRIGGER tag_link_fts_delete AFTER DELETE ON observation_tags
BEGIN
    DELETE FROM history_fts
      WHERE entity_type = 'tag'
        AND entity_id = old.observation_id || ':' || old.tag_id;
END;
