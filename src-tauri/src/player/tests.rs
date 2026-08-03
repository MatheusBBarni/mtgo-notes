use serde_json::json;

use super::*;
use crate::domain::{RepoError, Revision, UtcMillis};
use crate::notebook::NotebookBootstrap;
use crate::notebook::key::{DatabaseKey, KeyProtector};
use crate::notebook::migrations::{Migration, MigrationManager};
use crate::notebook::schema::{INITIAL_SCHEMA, RETIRED_TAGS_MIGRATION};

#[derive(Clone)]
struct TestProtector;

impl KeyProtector for TestProtector {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
        let mut result = vec![42];
        result.extend(plaintext.iter().map(|byte| byte ^ 42));
        Ok(result)
    }

    fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
        if ciphertext.first() != Some(&42) {
            return Err(RepoError::KeyUnavailable);
        }
        Ok(ciphertext[1..].iter().map(|byte| byte ^ 42).collect())
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    database: std::path::PathBuf,
    key: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        Self {
            database: directory.path().join("notebook.db"),
            key: directory.path().join("notebook.key"),
            _directory: directory,
        }
    }

    fn boot(&self) -> crate::notebook::NotebookRuntime {
        NotebookBootstrap::new(&self.database, &self.key, TestProtector)
            .initialize()
            .expect("boot")
    }

    fn db_key(&self) -> DatabaseKey {
        crate::notebook::key::KeyCustody::new(&self.key, &self.database, TestProtector)
            .load_or_create()
            .expect("key")
    }
}

fn evidence(identity: &PlayerIdentity, source_key: &str, digest: &str) -> PlayerEvidence {
    PlayerEvidence {
        id: PlayerEvidenceId::new(),
        player_identity_id: identity.id.clone(),
        evidence_schema_version: EVIDENCE_SCHEMA_VERSION,
        kind: EvidenceKind::MocsLeaderboardEntry,
        provenance_mode: EvidenceProvenance::ProviderObserved,
        provider_id: "census_mocs".into(),
        attribution_url: "https://census.daybreakgames.com/".into(),
        canonical_source_url: None,
        lookup_nickname: identity.display_nickname.clone(),
        source_nickname: identity.display_nickname.clone(),
        exact_match_rule: "case_insensitive_full_string".into(),
        scope: json!({"catalog": "mocs"}),
        observed_at: UtcMillis::new(100).expect("time"),
        imported_at: UtcMillis::new(101).expect("time"),
        source_key: source_key.into(),
        source_digest: digest.into(),
        preview_digest: "b".repeat(64),
        payload: json!({"points": 10}),
        selected_fields: json!({"points": true, "source_nickname": true, "attribution_url": true}),
        supersedes_evidence_id: None,
        cards: Vec::new(),
    }
}

fn batch(identity: &PlayerIdentity, source_key: &str, digest: &str) -> VerifiedImportBatch {
    VerifiedImportBatch {
        operation_key: PlayerOperationKey::new(),
        command_kind: "import_public_result".into(),
        request_digest: "c".repeat(64),
        evidence: evidence(identity, source_key, digest),
        selected_fields: json!({"points": true, "source_nickname": true, "attribution_url": true}),
        cards: Vec::new(),
        now: UtcMillis::new(102).expect("time"),
    }
}

#[test]
fn it_001_v2_to_v3_migration_adds_player_graph_without_opponent_changes() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    runtime
        .repository
        .with_connection(|connection| {
            for table in [
                "player_identities",
                "player_source_consents",
                "player_evidence",
                "player_evidence_cards",
                "player_selection_revisions",
                "player_empty_outcomes",
                "player_classification_runs",
                "player_tombstones",
                "player_operation_receipts",
            ] {
                let exists: i64 = connection
                    .connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                        [table],
                        |row| row.get(0),
                    )
                    .expect("table query");
                assert_eq!(exists, 1, "missing {table}");
            }
            assert_eq!(
                connection
                    .connection
                    .query_row::<i64, _, _>("SELECT count(*) FROM opponent_profiles", [], |row| row
                        .get(0))
                    .expect("opponents"),
                0
            );
            Ok(())
        })
        .expect("graph");
    assert_eq!(runtime.migration.current_version, 3);
}

#[test]
fn it_002_failed_v3_migration_restores_v2() {
    let fixture = Fixture::new();
    let key = fixture.db_key();
    MigrationManager::new(vec![
        Migration::new(1, INITIAL_SCHEMA),
        Migration::new(2, RETIRED_TAGS_MIGRATION),
    ])
    .migrate(&fixture.database, &key)
    .expect("v2");
    let failing = MigrationManager::new(vec![
        Migration::new(1, INITIAL_SCHEMA),
        Migration::new(2, RETIRED_TAGS_MIGRATION),
        Migration::new(
            3,
            "CREATE TABLE partial_player(id TEXT); SELECT no_such_function();",
        ),
    ]);
    assert!(matches!(
        failing.migrate(&fixture.database, &key),
        Err(RepoError::MigrationFailed)
    ));
    let restored = crate::notebook::connection::EncryptedConnection::open(&fixture.database, &key)
        .expect("restored");
    assert_eq!(
        crate::notebook::migrations::current_version(&restored).expect("version"),
        2
    );
    let partial: i64 = restored
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='partial_player')",
            [],
            |row| row.get(0),
        )
        .expect("partial");
    assert_eq!(partial, 0);
}

#[test]
fn it_003_identity_and_evidence_survive_encrypted_reopen() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    let store = PlayerStore::new(&runtime.repository);
    let identity = store
        .create_identity(
            PlayerId::new(),
            "Teichou_Aisu",
            UtcMillis::new(1).expect("time"),
        )
        .expect("identity");
    let outcome = store
        .import_batch(batch(&identity, "source-a", &"a".repeat(64)))
        .expect("import");
    assert!(outcome.inserted);
    drop(runtime);
    let key = fixture.db_key();
    let repository = crate::notebook::repository::NotebookRepository::open(&fixture.database, &key)
        .expect("reopen");
    let reopened = PlayerStore::new(&repository)
        .identity()
        .expect("identity")
        .expect("row");
    assert_eq!(reopened.id, identity.id);
    let page = PlayerStore::new(&repository)
        .evidence_page(&identity.id, None, 10)
        .expect("page");
    assert_eq!(page.items[0].id, outcome.evidence_id);
}

#[test]
fn it_004_singleton_and_revision_conflicts_are_fail_closed() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    let store = PlayerStore::new(&runtime.repository);
    let first = store
        .create_identity(PlayerId::new(), "Alpha", UtcMillis::new(1).expect("time"))
        .expect("first");
    assert!(matches!(
        store.create_identity(PlayerId::new(), "Beta", UtcMillis::new(2).expect("time")),
        Err(RepoError::IdentityConflict)
    ));
    assert!(
        store
            .update_identity(
                first.id.clone(),
                "Gamma",
                Revision::INITIAL,
                UtcMillis::new(3).expect("time")
            )
            .is_ok()
    );
    assert!(matches!(
        store.update_identity(
            first.id,
            "Delta",
            Revision::INITIAL,
            UtcMillis::new(4).expect("time")
        ),
        Err(RepoError::RevisionConflict)
    ));
}

#[test]
fn it_005_import_is_atomic_and_isolated() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    let store = PlayerStore::new(&runtime.repository);
    let identity = store
        .create_identity(PlayerId::new(), "Alpha", UtcMillis::new(1).expect("time"))
        .expect("identity");
    let mut import = batch(&identity, "source-a", &"a".repeat(64));
    let card = PlayerCard {
        oracle_id: "oracle-1".into(),
        display_name: "Card".into(),
        zone: "main".into(),
        quantity: 2,
        basic_land: false,
    };
    import.cards.push(card.clone());
    import.evidence.cards.push(card);
    import.evidence.kind = EvidenceKind::OfficialPublishedDecklist;
    import.evidence.payload = json!({"contents": "complete_deck", "points": 10});
    let outcome = store.import_batch(import).expect("batch");
    let page = store.evidence_page(&identity.id, None, 10).expect("page");
    assert_eq!(page.items[0].cards.len(), 1);
    runtime
        .repository
        .with_connection(|connection| {
            let foreign_key_violations: i64 = connection
                .connection
                .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get(0)
                })
                .expect("fk");
            assert_eq!(foreign_key_violations, 0);
            let opponent_count: i64 = connection
                .connection
                .query_row("SELECT count(*) FROM opponent_profiles", [], |row| {
                    row.get(0)
                })
                .expect("opponent");
            assert_eq!(opponent_count, 0);
            Ok(())
        })
        .expect("isolation");
    assert!(outcome.receipt.result_locator.is_some());
}

#[test]
fn it_006_and_it_007_receipts_replay_and_immutable_versions() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    let store = PlayerStore::new(&runtime.repository);
    let identity = store
        .create_identity(PlayerId::new(), "Alpha", UtcMillis::new(1).expect("time"))
        .expect("identity");
    let first = batch(&identity, "source-a", &"a".repeat(64));
    let operation_key = first.operation_key.clone();
    let first_outcome = store.import_batch(first.clone()).expect("first");
    let replay = store.import_batch(first).expect("replay");
    assert_eq!(replay.evidence_id, first_outcome.evidence_id);
    let mut mismatch = batch(&identity, "source-a", &"a".repeat(64));
    mismatch.operation_key = operation_key;
    mismatch.request_digest = "d".repeat(64);
    assert!(matches!(
        store.import_batch(mismatch),
        Err(RepoError::InvalidRequest)
    ));

    let mut changed = batch(&identity, "source-a", &"b".repeat(64));
    changed.evidence.supersedes_evidence_id = Some(first_outcome.evidence_id.clone());
    let changed_outcome = store.import_batch(changed).expect("changed");
    assert!(changed_outcome.inserted);
    assert_ne!(changed_outcome.evidence_id, first_outcome.evidence_id);

    let distinct = batch(&identity, "source-b", &"b".repeat(64));
    let distinct_outcome = store.import_batch(distinct).expect("distinct");
    assert_ne!(distinct_outcome.evidence_id, changed_outcome.evidence_id);
    assert_eq!(
        store
            .evidence_page(&identity.id, None, 10)
            .expect("page")
            .items
            .len(),
        3
    );
}

#[test]
fn ut_016_and_ut_053_to_055_source_versions_are_scoped_and_linked() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    let store = PlayerStore::new(&runtime.repository);
    let identity = store
        .create_identity(PlayerId::new(), "Alpha", UtcMillis::new(1).expect("time"))
        .expect("identity");
    let first = store
        .import_batch(batch(&identity, "source-a", &"a".repeat(64)))
        .expect("first");
    let mut changed = batch(&identity, "source-a", &"b".repeat(64));
    changed.evidence.supersedes_evidence_id = Some(first.evidence_id.clone());
    let second = store.import_batch(changed).expect("second");
    let page = store.evidence_page(&identity.id, None, 10).expect("page");
    let linked = page
        .items
        .iter()
        .find(|item| item.id == second.evidence_id)
        .expect("linked");
    assert_eq!(linked.supersedes_evidence_id, Some(first.evidence_id));
    let distinct = store
        .import_batch(batch(&identity, "source-b", &"b".repeat(64)))
        .expect("distinct");
    assert_ne!(distinct.evidence_id, second.evidence_id);
}

#[test]
fn ut_056_to_062_selection_revisions_are_append_only_and_paged() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    let store = PlayerStore::new(&runtime.repository);
    let identity = store
        .create_identity(PlayerId::new(), "Alpha", UtcMillis::new(1).expect("time"))
        .expect("identity");
    let imported = store
        .import_batch(batch(&identity, "source-a", &"a".repeat(64)))
        .expect("import");
    let revision = store
        .append_selection(AppendSelectionInput {
            operation_key: Some(PlayerOperationKey::new()),
            command_kind: "update_selection".into(),
            request_digest: Some("d".repeat(64)),
            evidence_id: imported.evidence_id.clone(),
            expected_revision: Revision::INITIAL,
            selected_fields: json!({
                "points": false,
                "source_nickname": true,
                "attribution_url": true
            }),
            now: UtcMillis::new(2).expect("time"),
        })
        .expect("selection");
    assert_eq!(revision.revision_number.get(), 2);
    let history = store
        .selection_history(&imported.evidence_id)
        .expect("history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].revision_number.get(), 1);
    assert_eq!(history[1].revision_number.get(), 2);
    let page = store
        .evidence_page(&identity.id, None, 1)
        .expect("first page");
    assert_eq!(page.items.len(), 1);
}
