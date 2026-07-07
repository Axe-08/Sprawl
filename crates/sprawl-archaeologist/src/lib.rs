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
}

impl Archaeologist {
    pub fn new(host: PluginHost, plugin_registry: PluginRegistry) -> Self {
        Self {
            host,
            plugin_registry,
        }
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

        for root in roots {
            let (primary, _all_matches) = self.detect_stack(root).await?;

            match primary {
                Some(_info) => {
                    // For now we mock the DB interaction. In phase 2 we will upsert into ledger.
                    tracing::info!("Detected stack for project: {}", root.display());
                    report.detected += 1;
                }
                None => {
                    tracing::warn!("Unknown stack for project: {}", root.display());
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
