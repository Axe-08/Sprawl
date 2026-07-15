use sprawl_core::ledger::schema::initialize_db;
use sprawl_sentinel::scanner::{LedgerBackend, SqliteLedgerStore};

#[test]
fn test_sqlite_ledger_store_inserts_secret() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_ledger.db");

    // Initialize DB with schema
    let conn = initialize_db(&db_path).unwrap();
    let ledger = SqliteLedgerStore::new(conn);

    let hash = "mock_hash_12345";
    let keyring_ref = "mock_ref_67890";

    // Save secret
    ledger.save_secret(hash, keyring_ref);

    // Verify it was saved
    let verify_conn = rusqlite::Connection::open(&db_path).unwrap();
    let mut stmt = verify_conn.prepare("SELECT key_hash, keyring_ref, classification FROM secrets WHERE key_hash = ?1").unwrap();
    let mut rows = stmt.query([hash]).unwrap();

    let row = rows.next().unwrap().expect("Expected at least one row");
    let saved_hash: String = row.get(0).unwrap();
    let saved_ref: String = row.get(1).unwrap();
    let saved_class: String = row.get(2).unwrap();

    assert_eq!(saved_hash, hash);
    assert_eq!(saved_ref, keyring_ref);
    assert_eq!(saved_class, "KnownProvider");
}
