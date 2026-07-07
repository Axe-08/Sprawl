pub mod data;

use sprawl_archivist::{SearchResult, VectorDatabase};
use sprawl_core::Result;
use sprawl_inference::SysInfo;
use sprawl_sentinel::scanner::{KeyringBackend, LedgerBackend};

pub struct MockDatabase;

impl MockDatabase {
    pub fn connect(_path: &str) -> Result<Self> {
        Ok(MockDatabase)
    }
}

impl VectorDatabase for MockDatabase {
    fn search(
        &self,
        _query_embedding: &[f32],
        _top_k: usize,
    ) -> sprawl_archivist::Result<Vec<SearchResult>> {
        Ok(vec![SearchResult {
            project_id: "test_proj".into(),
            file_path: "src/main.rs".into(),
            chunk_text: "fn main() { println!(\"Hello\"); }".into(),
            start_line: 1,
            end_line: 3,
            similarity_score: 0.95,
        }])
    }
}

pub struct MockKeyringStore;

impl KeyringBackend for MockKeyringStore {
    fn vault_secret(&self, _val: &str) -> String {
        "mock_keyring_ref_123".to_string()
    }
}

pub struct MockLedger;

impl LedgerBackend for MockLedger {
    fn save_secret(&self, _hash: &str, _keyring_ref: &str) {}
    fn queue_ambiguous(&self, _val: &str) {}
}

pub struct HighRamMock;
impl SysInfo for HighRamMock {
    fn available_ram_mb(&self) -> u64 {
        8192
    }
}
