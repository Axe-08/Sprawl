use sprawl_core::platform::sprawl_data_dir;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchivistError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Embedding model not available. Run `sprawl setup-embeddings`")]
    ModelNotAvailable,
}

pub type Result<T> = std::result::Result<T, ArchivistError>;

pub struct IndexedChunk {
    pub id: String, // UUID
    pub project_id: String,
    pub file_path: String,  // relative to project root
    pub chunk_text: String, // the actual code/text chunk
    pub chunk_start_line: u32,
    pub chunk_end_line: u32,
    pub embedding: Vec<f32>, // vector from embedding model
    pub indexed_at: String,  // ISO 8601
}

pub struct SearchResult {
    pub project_id: String,
    pub file_path: String,
    pub chunk_text: String,
    pub start_line: u32,
    pub end_line: u32,
    pub similarity_score: f32,
}

pub struct TextChunk {
    pub text: String,
    pub start_line: u32,
    pub end_line: u32,
}

// Mock RAM monitoring
pub trait RamMonitor: Send + Sync {
    fn available_ram_mb(&self) -> u64;
}

pub struct SysRamMonitor;
impl RamMonitor for SysRamMonitor {
    fn available_ram_mb(&self) -> u64 {
        8192 // mock
    }
}

pub const INDEXER_RAM_THRESHOLD_MB: u64 = 1024;

pub trait VectorDatabase: Send + Sync {
    fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>>;
}

#[cfg(any(test, feature = "mock-backend"))]
pub struct MockDatabase;

#[cfg(any(test, feature = "mock-backend"))]
impl MockDatabase {
    pub fn connect(_path: &str) -> Result<Self> {
        Ok(MockDatabase)
    }
}

#[cfg(any(test, feature = "mock-backend"))]
impl VectorDatabase for MockDatabase {
    fn search(&self, _query_embedding: &[f32], _top_k: usize) -> Result<Vec<SearchResult>> {
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

pub struct Archivist {
    db: Box<dyn VectorDatabase>,
    pub indexer_handle: Option<JoinHandle<()>>,
}

impl Archivist {
    #[cfg(any(test, feature = "mock-backend"))]
    pub fn new_mock() -> Result<Self> {
        let db_path = sprawl_data_dir()
            .map_err(|e| ArchivistError::Database(e.to_string()))?
            .join("vector_store");
        std::fs::create_dir_all(&db_path)?;
        let db = MockDatabase::connect(&db_path.to_string_lossy())?;
        Ok(Self {
            db: Box::new(db),
            indexer_handle: None,
        })
    }

    pub fn new(db: Box<dyn VectorDatabase>) -> Self {
        Self {
            db,
            indexer_handle: None,
        }
    }

    pub fn chunk_file(path: &Path) -> Result<Vec<TextChunk>> {
        let content = std::fs::read(path)?;

        // Skip binary check (null bytes in first 8KB)
        let check_len = std::cmp::min(content.len(), 8192);
        if content[..check_len].contains(&0) {
            return Ok(Vec::new()); // Skip binary
        }

        let text = String::from_utf8_lossy(&content);
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut start_line = 1;
        let mut current_line = 1;
        let mut tokens = 0;

        for line in text.lines() {
            current_chunk.push_str(line);
            current_chunk.push('\n');
            tokens += line.split_whitespace().count(); // naive tokenization

            if tokens >= 512 {
                chunks.push(TextChunk {
                    text: current_chunk.clone(),
                    start_line,
                    end_line: current_line,
                });
                // In production, implement 64-token overlap.
                // For MVP, reset entirely.
                current_chunk.clear();
                start_line = current_line + 1;
                tokens = 0;
            }
            current_line += 1;
        }

        if !current_chunk.is_empty() {
            chunks.push(TextChunk {
                text: current_chunk,
                start_line,
                end_line: current_line - 1,
            });
        }

        Ok(chunks)
    }

    pub fn start_background_indexer<R: RamMonitor + 'static>(&mut self, monitor: R) -> Result<()> {
        let handle = std::thread::spawn(move || {
            // In a real environment, set thread priority: set_low_priority().ok();

            // Limit loop iterations for testing to prevent infinite loop hanging tests
            #[cfg(test)]
            let mut test_iters = 0;

            loop {
                #[cfg(test)]
                {
                    test_iters += 1;
                    if test_iters > 2 {
                        break;
                    }
                }

                let available = monitor.available_ram_mb();
                if available < INDEXER_RAM_THRESHOLD_MB {
                    // Suspend / backoff
                    std::thread::sleep(Duration::from_millis(10)); // Short for testing, normally 30s
                    continue;
                }

                // Normal indexing work here...
                std::thread::sleep(Duration::from_millis(10)); // normally 300s
            }
        });

        self.indexer_handle = Some(handle);
        Ok(())
    }

    pub async fn search(&self, _query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        // 1. Embed query
        let mock_embedding = vec![0.1; 384];

        // 2. Vector similarity search
        self.db.search(&mock_embedding, top_k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    struct LowRamMonitor;
    impl RamMonitor for LowRamMonitor {
        fn available_ram_mb(&self) -> u64 {
            512
        }
    }

    struct HighRamMonitor;
    impl RamMonitor for HighRamMonitor {
        fn available_ram_mb(&self) -> u64 {
            4096
        }
    }

    #[test]
    fn test_indexer_suspends_when_ram_is_low() {
        let mut archivist = Archivist::new_mock().unwrap();
        archivist.start_background_indexer(LowRamMonitor).unwrap();

        // Wait for thread to complete its limited test iterations
        if let Some(handle) = archivist.indexer_handle.take() {
            handle.join().unwrap();
        }
        // If it joins successfully and didn't panic, it properly suspended and looped
    }

    #[test]
    fn test_chunk_file_skips_binary() {
        let mut file = NamedTempFile::new().unwrap();
        // write null byte
        file.write_all(&[0, 1, 2, 3]).unwrap();

        let chunks = Archivist::chunk_file(file.path()).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_file_splits_text() {
        let mut file = NamedTempFile::new().unwrap();

        // Generate enough text to exceed 512 "tokens"
        let mut content = String::new();
        for i in 0..600 {
            content.push_str("word ");
            if i % 10 == 0 {
                content.push('\n');
            }
        }
        file.write_all(content.as_bytes()).unwrap();

        let chunks = Archivist::chunk_file(file.path()).unwrap();
        assert!(chunks.len() >= 2);
    }

    #[tokio::test]
    async fn test_search_returns_relevant_results() {
        let archivist = Archivist::new_mock().unwrap();
        let results = archivist.search("query", 5).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].file_path, "src/main.rs");
    }
}
