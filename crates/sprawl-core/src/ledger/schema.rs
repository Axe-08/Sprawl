use rusqlite::Connection;
use std::path::Path;

pub const CURRENT_SCHEMA_VERSION: i32 = 1;

pub fn initialize_db(db_path: &Path) -> crate::Result<Connection> {
    let conn = Connection::open(db_path)
        .map_err(|e| crate::SprawlError::Other(format!("Failed to open DB: {}", e)))?;

    // Enable WAL mode
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|e| crate::SprawlError::Other(format!("Failed to enable WAL: {}", e)))?;

    let user_version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| crate::SprawlError::Other(format!("Failed to read user_version: {}", e)))?;

    // Check if schema needs to be created or migrated.
    // We always use IF NOT EXISTS to be safe against partially-initialized databases
    // (e.g. where other code paths created tables outside the versioned schema path).
    if user_version < CURRENT_SCHEMA_VERSION {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id              TEXT PRIMARY KEY,
                root_path       TEXT NOT NULL UNIQUE,
                ecosystem       TEXT,
                status          TEXT NOT NULL DEFAULT 'active',
                last_seen       TEXT NOT NULL,
                idle_days       INTEGER DEFAULT 0,
                stack_source    TEXT,
                config_source   TEXT,
                created_at      TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS secrets (
                id              TEXT PRIMARY KEY,
                project_id      TEXT REFERENCES projects(id),
                source_file     TEXT NOT NULL,
                line_number     INTEGER,
                classification  TEXT NOT NULL,
                provider_prefix TEXT,
                key_hash        TEXT NOT NULL,
                entropy         REAL,
                keyring_ref     TEXT,
                discovered_at   TEXT NOT NULL,
                verified_at     TEXT,
                verified_status TEXT
            );
            CREATE TABLE IF NOT EXISTS ambiguous_secrets (
                id TEXT PRIMARY KEY,
                raw_value TEXT NOT NULL,
                filepath TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                reviewed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS sweep_history (
                id              TEXT PRIMARY KEY,
                project_id      TEXT REFERENCES projects(id),
                target_path     TEXT NOT NULL,
                action          TEXT NOT NULL,
                original_size   INTEGER,
                destination     TEXT,
                executed_at     TEXT NOT NULL,
                restored_at     TEXT
            );
            CREATE TABLE IF NOT EXISTS schema_meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT OR IGNORE INTO schema_meta VALUES ('schema_version', '1');
            ",
        )
        .map_err(|e| crate::SprawlError::Other(format!("Failed to initialize schema: {}", e)))?;

        conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
            .map_err(|e| {
                crate::SprawlError::Other(format!("Failed to update user_version: {}", e))
            })?;
    }

    Ok(conn)
}
