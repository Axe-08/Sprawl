use crate::bundle::{BundleOptions, Bundler};
use serde::{Deserialize, Serialize};
use sprawl_core::Result;
use sprawl_inference::{InferenceEngine, SysInfo};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DerivedConfig {
    pub name: String,
    pub ecosystem: String,
    pub frameworks: Vec<String>,
}

pub async fn analyze_deep<S: SysInfo>(
    dir: &Path,
    inference: &mut InferenceEngine<S>,
) -> Result<DerivedConfig> {
    // 1. Preflight check
    inference
        .preflight_check()
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    // 2. Generate bundle
    let bundler = Bundler::new();
    let bundle_opts = BundleOptions::default();
    let bundle_content = bundler.bundle_directory(dir, &bundle_opts)?;

    // 3. Construct prompt
    let prompt = format!(
        "Analyze the following source code bundle and determine the project name, primary ecosystem, and frameworks used. Return the result strictly in JSON format as {{\"name\": \"...\", \"ecosystem\": \"...\", \"frameworks\": [\"...\"]}}.\n\n{}",
        bundle_content
    );

    // 4. Run inference
    tracing::info!("Running deep analysis using LLM...");
    let result_text = inference
        .run_prompt(&prompt)
        .await
        .map_err(|e| sprawl_core::SprawlError::Other(format!("Inference failed: {}", e)))?;

    // 5. Parse JSON (naive extraction assuming the LLM outputs valid JSON block)
    // Find the first { and last }
    let start = result_text.find('{').unwrap_or(0);
    let end = result_text
        .rfind('}')
        .unwrap_or(result_text.len().saturating_sub(1));

    let json_text = if start <= end && start < result_text.len() {
        &result_text[start..=end]
    } else {
        &result_text
    };

    let derived: DerivedConfig = serde_json::from_str(json_text).map_err(|e| {
        sprawl_core::SprawlError::Other(format!("Failed to parse LLM output: {}", e))
    })?;

    Ok(derived)
}
