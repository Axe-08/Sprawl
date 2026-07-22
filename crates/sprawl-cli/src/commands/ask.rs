use clap::Args;
use sprawl_archivist::Archivist;
use sprawl_core::Result;
use sprawl_inference::{DeviceTarget, InferenceEngine, RealSysInfo, DEFAULT_MODEL};
use std::path::PathBuf;
use std::io::{self, Write};

#[derive(Args)]
pub struct AskArgs {
    /// Freeform question about the codebase
    pub query: String,
}

pub async fn handle(args: &AskArgs, is_json: bool) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    #[allow(unused_variables)]
    let data_dir = PathBuf::from(home).join(".sprawl").join("archivist");
    
    if !is_json {
        println!("Retrieving codebase context...");
    }

    #[cfg(feature = "real-archivist")]
    let archivist = Archivist::new_real(&data_dir).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    
    #[cfg(not(feature = "real-archivist"))]
    let archivist = Archivist::new(std::sync::Arc::new(sprawl_dev::MockDatabase), std::sync::Arc::new(sprawl_dev::MockEmbedder));
    
    let results = archivist.search(&args.query, 8).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    
    if results.is_empty() {
        if is_json {
            println!("{}", serde_json::json!({"error": "No context found"}));
        } else {
            println!("No context found in the codebase to answer the query.");
        }
        return Ok(());
    }

    let mut context_block = String::new();
    for (i, r) in results.iter().enumerate() {
        context_block.push_str(&format!("--- FILE: {} (Match {}) ---\n{}\n\n", r.file_path, i+1, r.chunk_text));
    }

    let prompt = format!(
        "<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\n\
        You are a highly intelligent and helpful senior software engineer assisting a developer with their codebase. \
        Use ONLY the provided context blocks to answer their question. Formulate your answer as a clear, \
        coherent, natural language explanation. Do not output raw machine dumps. If the context does not \
        contain enough information to fully answer the question, state that clearly.<|eot_id|>\n\
        <|start_header_id|>user<|end_header_id|>\n\n\
        Context:\n{}\n\nQuestion: {}<|eot_id|>\n\
        <|start_header_id|>assistant<|end_header_id|>\n\n",
        context_block, args.query
    );

    if !is_json {
        println!("Initializing Inference Engine to generate answer...");
    }

    let mut engine = InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, RealSysInfo);
    
    // Check available RAM before loading model
    if let Err(e) = engine.preflight_check() {
        return Err(sprawl_core::SprawlError::Other(format!("Preflight check failed: {}", e)));
    }

    // Load the model
    #[cfg(feature = "real-inference")]
    {
        // First ensure it's downloaded
        let model_path = match engine.ensure_model(None).await {
            Ok(p) => p,
            Err(sprawl_inference::InferenceError::ModelNotInstalled) => {
                if !is_json {
                    println!("Model not installed. Downloading...");
                }
                engine.download_model(None).await.map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to download model: {}", e)))?
            }
            Err(e) => return Err(sprawl_core::SprawlError::Other(e.to_string())),
        };
        engine.load_model(&model_path, None).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    }
    #[cfg(not(feature = "real-inference"))]
    {
        engine.load_model(PathBuf::from("mock").as_path(), None).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    }

    if !is_json {
        print!("Answer:\n");
        io::stdout().flush().unwrap();
    }

    let response = engine.run_prompt(&prompt).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    if is_json {
        println!("{}", serde_json::json!({ "query": args.query, "answer": response.trim() }));
    } else {
        println!("{}", response.trim());
    }

    Ok(())
}
