use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;

pub const CURRENT_SCHEMA_VERSION: i32 = 1;

pub fn initialize_db(db_path: &Path) -> crate::Result<Connection> {
    let conn = Connection::open(db_path)
        .map_err(|e| crate::SprawlError::Other(format!("Failed to open DB: {}", e)))?;
        
    // Enable WAL mode
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|e| crate::SprawlError::Other(format!("Failed to enable WAL: {}", e)))?;
        
    let user_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| crate::SprawlError::Other(format!("Failed to read user_version: {}", e)))?;

    if user_version == 0 {
        // Initial setup
        conn.execute_batch(
            "
            CREATE TABLE projects (
                id              TEXT PRIMARY KEY,
                root_path       TEXT NOT NULL UNIQUE,
                ecosystem       TEXT,
                last_seen       TEXT NOT NULL,
                idle_days       INTEGER DEFAULT 0,
                stack_source    TEXT,
                config_source   TEXT,
                created_at      TEXT NOT NULL
            );
            CREATE TABLE secrets (
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
            CREATE TABLE sweep_history (
                id              TEXT PRIMARY KEY,
                project_id      TEXT REFERENCES projects(id),
                target_path     TEXT NOT NULL,
                action          TEXT NOT NULL,
                original_size   INTEGER,
                destination     TEXT,
                executed_at     TEXT NOT NULL,
                restored_at     TEXT
            );
            CREATE TABLE schema_meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT INTO schema_meta VALUES ('schema_version', '1');
            "
        ).map_err(|e| crate::SprawlError::Other(format!("Failed to initialize schema: {}", e)))?;
        
        conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
            .map_err(|e| crate::SprawlError::Other(format!("Failed to update user_version: {}", e)))?;
    } else if user_version < CURRENT_SCHEMA_VERSION {
        // Here we would perform schema migration. 
        // As per Step 3.6: Always backup before migrate.
    }
    
    Ok(conn)
}
