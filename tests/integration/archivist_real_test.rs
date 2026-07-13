use sprawl_archivist::{Archivist, SysRamMonitor};
use sprawl_archivist::TextChunk;
use std::path::Path;
use std::io::Write;

#[test]
fn test_chunk_overlap_produces_correct_boundaries() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let mut content = String::new();
    for i in 1..=600 {
        content.push_str(&format!("word{} ", i));
        if i % 10 == 0 {
            content.push('\n');
        }
    }
    file.write_all(content.as_bytes()).unwrap();

    let chunks = Archivist::chunk_file(file.path()).unwrap();
    assert!(chunks.len() >= 2);
    // Ensure that the end line of first chunk is greater than the start line of second chunk (overlap)
    assert!(chunks[0].end_line >= chunks[1].start_line);
}

#[test]
fn test_sysram_monitor_returns_nonzero() {
    use sprawl_archivist::RamMonitor;
    let monitor = SysRamMonitor;
    let ram = monitor.available_ram_mb();
    assert!(ram > 0);
}

#[cfg(feature = "real-archivist")]
#[tokio::test]
async fn test_real_archivist_search() {
    // This test will only run when real-archivist feature is enabled.
    // It verifies that LanceDB and OnnxEmbedder instantiate successfully.
    let temp_dir = tempfile::tempdir().unwrap();
    let archivist = Archivist::new_real(temp_dir.path()).await.unwrap();
    
    // We shouldn't actually trigger download in automated tests if it's missing,
    // so this test assumes the model is present (or fails explicitly otherwise).
    let results = archivist.search("test query", 5).await.unwrap();
    // Since table is empty, results should be empty
    assert!(results.is_empty());
}
