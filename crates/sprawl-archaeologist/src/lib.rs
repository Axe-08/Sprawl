pub mod bundle;

use sprawl_core::Result;
use sprawl_plugin_host::{PluginHost, PluginRegistry, StackInfo};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ScanReport {
    pub detected: usize,
    pub unknown: usize,
}

pub struct DriftAlert {
    pub unknown_count: usize,
    pub message: String,
}

pub struct Archaeologist {
    pub plugin_registry: PluginRegistry,
    pub host: PluginHost,
    pub db_conn: Option<std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>>,
}

impl Archaeologist {
    pub fn new(host: PluginHost, plugin_registry: PluginRegistry) -> Self {
        Self {
            host,
            plugin_registry,
            db_conn: None,
        }
    }

    pub fn with_db(mut self, db_conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
        self.db_conn = Some(db_conn);
        self
    }

    /// Detect the stack for a project using the WASM fast path.
    /// Returns (Primary StackInfo, All Matching StackInfo list).
    pub async fn detect_stack(
        &self,
        project_root: &Path,
    ) -> Result<(Option<StackInfo>, Vec<StackInfo>)> {
        let all_matches = tokio::task::block_in_place(|| {
            self.plugin_registry.run_discovery(&self.host, project_root)
        })
        .map_err(|e| sprawl_core::SprawlError::Other(format!("Discovery failed: {}", e)))?;

        let primary = all_matches.first().cloned();
        Ok((primary, all_matches))
    }

    /// Scan a list of project roots and update the ledger.
    pub async fn scan_projects(&self, roots: &[PathBuf]) -> Result<ScanReport> {
        let mut report = ScanReport::default();

        if let Some(conn_mu) = &self.db_conn {
            let conn = conn_mu.lock().unwrap();
            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS projects (
                    id TEXT PRIMARY KEY,
                    root_path TEXT UNIQUE NOT NULL,
                    ecosystem TEXT,
                    status TEXT NOT NULL DEFAULT 'active',
                    last_seen TEXT,
                    created_at TEXT
                )",
                []
            );
        }

        for root in roots {
            let (primary, _all_matches) = self.detect_stack(root).await?;

            match primary {
                Some(info) => {
                    tracing::info!("Detected stack for project: {}", root.display());
                    
                    if let Some(conn_mu) = &self.db_conn {
                        let conn = conn_mu.lock().unwrap();
                        let id = uuid::Uuid::new_v4().to_string();
                        let now = chrono::Utc::now().to_rfc3339();
                        let ecosystem = info.ecosystem.clone();
                        let _ = conn.execute(
                            "INSERT INTO projects (id, root_path, ecosystem, last_seen, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
                             ON CONFLICT(root_path) DO UPDATE SET last_seen=excluded.last_seen, ecosystem=excluded.ecosystem",
                            (&id, root.to_string_lossy().to_string(), &ecosystem, &now, &now),
                        );
                    }
                    
                    report.detected += 1;
                }
                None => {
                    tracing::warn!("Unknown stack for project: {}", root.display());
                    
                    if let Some(conn_mu) = &self.db_conn {
                        let conn = conn_mu.lock().unwrap();
                        let id = uuid::Uuid::new_v4().to_string();
                        let now = chrono::Utc::now().to_rfc3339();
                        let ecosystem = "Unknown".to_string();
                        let _ = conn.execute(
                            "INSERT INTO projects (id, root_path, ecosystem, last_seen, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
                             ON CONFLICT(root_path) DO UPDATE SET last_seen=excluded.last_seen, ecosystem=excluded.ecosystem",
                            (&id, root.to_string_lossy().to_string(), &ecosystem, &now, &now),
                        );
                    }
                    
                    report.unknown += 1;
                }
            }
        }

        Ok(report)
    }

    /// Track unique project roots with Unknown ecosystem
    pub fn check_drift_alert(&self, db_unknown_count: usize) -> Result<Option<DriftAlert>> {
        if db_unknown_count >= 5 {
            Ok(Some(DriftAlert {
                unknown_count: db_unknown_count,
                message: format!(
                    "{} projects have unknown stacks — consider running `sprawl profile-machine`",
                    db_unknown_count
                ),
            }))
        } else {
            Ok(None)
        }
    }
}
pub mod analyze;
