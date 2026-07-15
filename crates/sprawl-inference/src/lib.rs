//! ## Feature Flags
//!
//! - **`mock-backend`** (or `cfg(test)`): Replaces download, sha256, and LLM with fast in-memory
//!   mocks. Used in all CI runs and `cargo test`.
//!
//! - **`inference`**: Compiles in the real Candle GGUF model loader and Phi-3 generation loop.
//!   Requires a 2.4GB model file at `~/.sprawl/models/`. Enable with
//!   `cargo build --features real-inference`. Disabled by default for fast dev builds.
//!
//! These are independent: `mock-backend` has priority in tests; `real-inference` is for the
//! production binary only.

use sprawl_core::platform::sprawl_data_dir;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error(
        "INSUFFICIENT HEADROOM: {required_mb}MB required, {available_mb}MB available. Aborting."
    )]
    InsufficientRam { required_mb: u64, available_mb: u64 },
    #[error("Model checksum mismatch. File purged. Expected: {expected}, Actual: {actual}")]
    ModelChecksumMismatch { expected: String, actual: String },
    #[error("Model download failed. Check network and retry.")]
    DownloadFailed,
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, InferenceError>;

pub struct ModelConfig {
    pub name: &'static str,
    pub filename: &'static str,
    pub download_url: &'static str,
    pub fallback_url: &'static str,
    pub sha256: &'static str,
    pub size_bytes: u64,
    pub ram_requirement_mb: u64,
}

#[cfg(not(feature = "real-inference"))]
pub const DEFAULT_MODEL: ModelConfig = ModelConfig {
    name: "Phi-3 Mini 4K Instruct (Q4_K_M)",
    filename: "Phi-3-mini-4k-instruct-q4_k_m.gguf",
    download_url: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4_k_m.gguf",
    fallback_url: "https://mirror.example.com/microsoft/Phi-3-mini-4k-instruct-q4_k_m.gguf", 
    sha256: "mock_sha256_hash_for_testing_purposes",
    size_bytes: 2_400_000_000,
    ram_requirement_mb: 3072,
};

#[cfg(feature = "real-inference")]
pub const DEFAULT_MODEL: ModelConfig = ModelConfig {
    name: "Phi-3 Mini 4K Instruct (Q4)",
    filename: "Phi-3-mini-4k-instruct-q4.gguf",
    download_url: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf",
    fallback_url: "https://cdn-lfs.huggingface.co/repos/00/20/002016f4fc44cfb87b7aebdb9fcf63e9c402cd0811bda313d4bda12c3f1de9ea/8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef", 
    sha256: "8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef",
    size_bytes: 2_393_212_000,
    ram_requirement_mb: 3072,
};

pub const RAM_SAFETY_MARGIN_MB: u64 = 1024;

pub enum EngineProgress {
    Downloading {
        pct: u8,
        bytes_done: u64,
        bytes_total: u64,
    },
    Loading {
        pct: u8,
    },
    Running {
        tokens_generated: u32,
    },
    Complete,
    Failed(String),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum DeviceTarget {
    Cpu,
    Cuda,
    Metal,
}

pub enum InferenceStatus {
    Cold,
    Loading { progress_pct: u8 },
    Ready,
    Running,
}

// OS RAM mock trait for testing NFR-7
pub trait SysInfo {
    fn available_ram_mb(&self) -> u64;
}

pub struct RealSysInfo;
impl SysInfo for RealSysInfo {
    fn available_ram_mb(&self) -> u64 {
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_memory();
        sys.available_memory() / 1024 / 1024
    }
}

#[cfg(feature = "real-inference")]
pub struct LoadedModel {
    pub weights: candle_transformers::models::quantized_llama::ModelWeights,
    pub tokenizer: tokenizers::Tokenizer,
    pub device: candle_core::Device,
}

#[cfg(not(feature = "real-inference"))]
pub struct LoadedModel {}

pub struct InferenceEngine<S: SysInfo> {
    pub config: ModelConfig,
    pub device_target: DeviceTarget,
    pub state: InferenceStatus,
    sysinfo: S,

    pub loaded_model: Option<LoadedModel>,
}

impl<S: SysInfo> InferenceEngine<S> {
    pub fn new(config: ModelConfig, device_target: DeviceTarget, sysinfo: S) -> Self {
        Self {
            config,
            device_target,
            state: InferenceStatus::Cold,
            sysinfo,
            loaded_model: None,
        }
    }

    pub fn preflight_check(&self) -> Result<()> {
        let available = self.sysinfo.available_ram_mb();
        let required = self.config.ram_requirement_mb + RAM_SAFETY_MARGIN_MB;

        if available < required {
            return Err(InferenceError::InsufficientRam {
                required_mb: required,
                available_mb: available,
            });
        }
        Ok(())
    }

    // Mock download mechanism
    #[cfg(all(any(test, feature = "mock-backend"), not(feature = "real-inference")))]
    async fn download(
        &self,
        _url: &str,
        path: &Path,
        _tx: &Option<Sender<EngineProgress>>,
    ) -> Result<()> {
        std::fs::write(path, "mock_gguf_content").map_err(InferenceError::Io)
    }

    #[cfg(feature = "real-inference")]
    async fn download(
        &self,
        url: &str,
        path: &Path,
        tx: &Option<Sender<EngineProgress>>,
    ) -> Result<()> {
        use futures_util::StreamExt;

        let resp = reqwest::get(url)
            .await
            .map_err(|_| InferenceError::DownloadFailed)?;
        if !resp.status().is_success() {
            return Err(InferenceError::DownloadFailed);
        }

        let total = resp.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(path).await?;
        let mut downloaded = 0u64;
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| InferenceError::DownloadFailed)?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk.len() as u64;

            if let Some(ref progress_tx) = tx {
                let pct = if total > 0 {
                    ((downloaded as f64 / total as f64) * 100.0) as u8
                } else {
                    0
                };
                let _ = progress_tx.send(EngineProgress::Downloading {
                    pct,
                    bytes_done: downloaded,
                    bytes_total: total,
                });
            }
        }
        Ok(())
    }

    // Mock sha256
    #[cfg(all(any(test, feature = "mock-backend"), not(feature = "real-inference")))]
    fn sha256_file(&self, path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        if content == "mock_gguf_content" {
            Ok("mock_sha256_hash_for_testing_purposes".to_string())
        } else if content == "corrupted_content" {
            Ok("bad_hash".to_string())
        } else {
            Ok("unknown_hash".to_string())
        }
    }

    #[cfg(not(any(test, feature = "mock-backend", feature = "real-inference")))]
    async fn download(
        &self,
        _url: &str,
        _path: &Path,
        _tx: &Option<Sender<EngineProgress>>,
    ) -> Result<()> {
        Err(InferenceError::Other("Inference backend not compiled".into()))
    }

    #[cfg(not(any(test, feature = "mock-backend", feature = "real-inference")))]
    fn sha256_file(&self, _path: &Path) -> Result<String> {
        Err(InferenceError::Other("Inference backend not compiled".into()))
    }

    #[cfg(feature = "real-inference")]
    fn sha256_file(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        Ok(hex::encode(hasher.finalize()))
    }

    pub async fn ensure_model(
        &self,
        progress_tx: Option<Sender<EngineProgress>>,
    ) -> Result<PathBuf> {
        let model_dir = sprawl_data_dir()
            .map_err(|e| InferenceError::Other(e.to_string()))?
            .join("models");

        std::fs::create_dir_all(&model_dir)?;
        let model_path = model_dir.join(self.config.filename);

        if model_path.exists() {
            let hash = self.sha256_file(&model_path)?;
            if hash == self.config.sha256 {
                return Ok(model_path);
            }
            tracing::warn!("Model checksum mismatch, re-downloading");
            std::fs::remove_file(&model_path)?;
        }

        match self
            .download(self.config.download_url, &model_path, &progress_tx)
            .await
        {
            Ok(()) => {}
            Err(_) => {
                self.download(self.config.fallback_url, &model_path, &progress_tx)
                    .await?;
            }
        }

        let hash = self.sha256_file(&model_path)?;
        if hash != self.config.sha256 {
            std::fs::remove_file(&model_path)?;
            return Err(InferenceError::ModelChecksumMismatch {
                expected: self.config.sha256.to_string(),
                actual: hash,
            });
        }

        Ok(model_path)
    }

    pub fn load_model(
        &mut self,
        #[allow(unused_variables)] path: &Path,
        _progress_tx: Option<Sender<EngineProgress>>,
    ) -> Result<()> {
        self.state = InferenceStatus::Loading { progress_pct: 0 };

        match self.device_target {
            DeviceTarget::Cpu => { /* load to CPU */ }
            DeviceTarget::Cuda | DeviceTarget::Metal => {
                tracing::warn!(
                    "[!] GPU backend '{:?}' not yet supported in this build — falling back to CPU.",
                    self.device_target
                );
            }
        }

        #[cfg(feature = "real-inference")]
        {
            let mut file = std::fs::File::open(path)?;
            let model_content = candle_core::quantized::gguf_file::Content::read(&mut file)
                .map_err(|e| InferenceError::Other(e.to_string()))?;
            let device = candle_core::Device::Cpu;
            let weights = candle_transformers::models::quantized_llama::ModelWeights::from_gguf(model_content, &mut file, &device)
                .map_err(|e| InferenceError::Other(e.to_string()))?;
            let tokenizer_path = path.with_file_name("tokenizer.json");
            let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| InferenceError::Other(e.to_string()))?;
            self.loaded_model = Some(LoadedModel { weights, tokenizer, device });
        }
        
        #[cfg(not(feature = "real-inference"))]
        {
            self.loaded_model = Some(LoadedModel {});
        }

        self.state = InferenceStatus::Ready;
        Ok(())
    }

    pub async fn run_prompt(&mut self, prompt: &str) -> Result<String> {
        self.state = InferenceStatus::Running;

        #[cfg(any(test, feature = "mock-backend"))]
        let response = if prompt.contains("JSON") && prompt.contains("classification") {
            r#"{"classification": "likely_noise", "reason": "mock noise"}"#.to_string()
        } else if prompt.contains("JSON") {
            r#"{"name": "sprawl", "ecosystem": "rust", "frameworks": ["tokio", "clap"]}"#
                .to_string()
        } else if prompt.contains("sk_live") {
            // Refuse to process raw secrets as a safety check
            "ERROR: RAW SECRET DETECTED IN PROMPT".to_string()
        } else {
            "mock classification: likely_noise".to_string()
        };

        #[cfg(not(any(test, feature = "mock-backend", feature = "real-inference")))]
        let response = "ERROR: Inference backend not compiled. Build with --features real-inference.".to_string();

        #[cfg(feature = "real-inference")]
        let response = {
            use candle_transformers::generation::LogitsProcessor;
            use candle_core::Tensor;

            let loaded = self.loaded_model.as_mut()
                .ok_or_else(|| InferenceError::Other("Model not loaded".into()))?;
            
            // Encode prompt
            let tokens = loaded.tokenizer.encode(prompt, true)
                .map_err(|e| InferenceError::Other(e.to_string()))?;
            let token_ids = tokens.get_ids().to_vec();
            
            // Greedy decode up to 512 tokens
            let mut all_tokens = token_ids.clone();
            let mut logits_processor = LogitsProcessor::new(42, Some(0.7), None);
            let eos_token = loaded.tokenizer.token_to_id("</s>").unwrap_or(2);
            let mut output = String::new();
            
            for _ in 0..512 {
                let input = Tensor::new(all_tokens.as_slice(), &loaded.device)
                    .map_err(|e| InferenceError::Other(e.to_string()))?
                    .unsqueeze(0)
                    .map_err(|e| InferenceError::Other(e.to_string()))?;
                
                let logits = loaded.weights.forward(&input, all_tokens.len() - 1)
                    .map_err(|e| InferenceError::Other(e.to_string()))?;
                let next_token = logits_processor.sample(&logits)
                    .map_err(|e| InferenceError::Other(e.to_string()))?;
                
                if next_token == eos_token { break; }
                all_tokens.push(next_token);
                if let Ok(word) = loaded.tokenizer.decode(&[next_token], true) {
                    output.push_str(&word);
                }
            }
            output
        };

        self.state = InferenceStatus::Cold;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LowRamMock;
    impl SysInfo for LowRamMock {
        fn available_ram_mb(&self) -> u64 {
            2048
        }
    }

    struct HighRamMock;
    impl SysInfo for HighRamMock {
        fn available_ram_mb(&self) -> u64 {
            8192
        }
    }

    #[test]
    fn test_preflight_refuses_when_ram_too_low() {
        let engine = InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, LowRamMock);
        let result = engine.preflight_check();

        assert!(result.is_err());
        match result.unwrap_err() {
            InferenceError::InsufficientRam {
                required_mb,
                available_mb,
            } => {
                assert_eq!(required_mb, 4096);
                assert_eq!(available_mb, 2048);
            }
            _ => panic!("Expected InsufficientRam error"),
        }
    }

    #[test]
    fn test_preflight_passes_with_sufficient_ram() {
        let engine = InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, HighRamMock);
        assert!(engine.preflight_check().is_ok());
    }

    #[tokio::test]
    async fn test_corrupted_download_detected_by_checksum() {
        // Mock HOME directory to isolate test
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());

        let engine = InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, HighRamMock);

        // Manually place a corrupted file in the data dir
        let models_dir = temp_dir.path().join(".sprawl").join("models");
        std::fs::create_dir_all(&models_dir).unwrap();
        let corrupted_path = models_dir.join(DEFAULT_MODEL.filename);
        std::fs::write(&corrupted_path, "corrupted_content").unwrap();

        // ensure_model will see the file, check hash, fail, and re-download
        let path = engine
            .ensure_model(None)
            .await
            .expect("Should recover by re-downloading");

        // After re-download, it should be the valid mock content
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "mock_gguf_content");
    }
}
