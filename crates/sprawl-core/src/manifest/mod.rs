use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub project_path: String,
    pub action: String,
    pub target: String,
    pub original_size_bytes: u64,
    pub restored: bool,
}

pub struct Manifest {
    path: PathBuf,
}

impl Manifest {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Append a single entry to the manifest in NDJSON format.
    /// This implementation is simple and performs O(1) appends.
    /// In production, we should use a file lock (e.g. fs2) before writing.
    pub fn append(&self, entry: &ManifestEntry) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| crate::SprawlError::Other(format!("Manifest open error: {}", e)))?;

        let mut json = serde_json::to_string(entry)
            .map_err(|e| crate::SprawlError::Other(format!("JSON serialization error: {}", e)))?;
        json.push('\n');

        file.write_all(json.as_bytes())
            .map_err(|e| crate::SprawlError::Other(format!("Manifest write error: {}", e)))?;

        Ok(())
    }

    /// Retrieve all entries for a specific project.
    pub fn find_by_project(&self, project_path: &str) -> Result<Vec<ManifestEntry>> {
        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => {
                return Err(crate::SprawlError::Other(format!(
                    "Manifest read error: {}",
                    e
                )))
            }
        };

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line =
                line.map_err(|e| crate::SprawlError::Other(format!("Manifest line error: {}", e)))?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: ManifestEntry = serde_json::from_str(&line)
                .map_err(|_| crate::SprawlError::Other("Manifest corruption detected".into()))?;

            if entry.project_path == project_path {
                entries.push(entry);
            }
        }

        Ok(entries)
    }
}
