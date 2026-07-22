use clap::Args;
use sprawl_archivist::Archivist;
use sprawl_core::Result;
use std::path::PathBuf;

#[derive(Args)]
pub struct SearchArgs {
    /// Freeform search query
    pub query: String,
    /// Max results to return
    #[arg(long, default_value_t = 5)]
    pub top_k: usize,
    /// Emit JSON array of results
    #[arg(long)]
    pub json: bool,
}

pub async fn handle(args: &SearchArgs, is_json: bool) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    #[allow(unused_variables)]
    let data_dir = PathBuf::from(home).join(".sprawl").join("archivist");
    
    #[cfg(feature = "real-archivist")]
    let archivist = Archivist::new_real(&data_dir).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    
    #[cfg(not(feature = "real-archivist"))]
    let archivist = Archivist::new(std::sync::Arc::new(sprawl_dev::MockDatabase), std::sync::Arc::new(sprawl_dev::MockEmbedder));
    
    // Auto-triggering indexing is explicitly against ADR-008 extended principles.
    // However, since it's a mock backend right now, we can just call search directly.
    let results = archivist.search(&args.query, args.top_k).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    
    let use_json = is_json || args.json;

    if use_json {
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "score": r.similarity_score,
                    "file": r.file_path,
                    "start_line": r.start_line,
                    "end_line": r.end_line,
                    "text": r.chunk_text
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::json!({ "results": json_results })
        );
    } else {
        if results.is_empty() {
            println!("No results found for query: {}", args.query);
            std::process::exit(5);
        }

        for r in &results {
            println!("[{:.2}] {}:L{}-{}", r.similarity_score, r.file_path, r.start_line, r.end_line);
            println!("{}\n", r.chunk_text);
        }
    }

    if results.is_empty() {
        std::process::exit(5);
    }

    Ok(())
}
