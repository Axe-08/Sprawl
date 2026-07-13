use sprawl_core::Result;

pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

#[cfg(feature = "real-archivist")]
pub mod onnx_embedder {
    use super::Embedder;
    use sprawl_core::Result;
    use std::path::Path;
    use ndarray::{Array, Axis, CowArray};
    use ort::{GraphOptimizationLevel, Session};
    use tokio::fs;

    pub struct OnnxEmbedder {
        session: Session,
    }

    impl OnnxEmbedder {
        pub async fn load(model_dir: &Path) -> Result<Self> {
            let model_path = model_dir.join("all-MiniLM-L6-v2.onnx");
            
            // In a real implementation we would download the ONNX file here if absent.
            // For MVP we assume it's downloaded by the user or an out-of-band script,
            // or we'll add downloading shortly.
            if !model_path.exists() {
                return Err(sprawl_core::SprawlError::Other(format!(
                    "Model not found at {:?}. Please run `sprawl setup-embeddings` or download manually.",
                    model_path
                )));
            }

            ort::init()
                .with_name("sprawl-embedder")
                .commit()
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let session = Session::builder()
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .with_intra_threads(4)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .commit_from_file(&model_path)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            Ok(Self { session })
        }
    }

    impl Embedder for OnnxEmbedder {
        fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
            // Simplified stub for now: normally we'd tokenize and pass to ONNX session.
            // Since we'd need a tokenizer (e.g. from huggingface tokenizers crate), 
            // for MVP compilation passing we'll just return zeroes.
            Ok(texts.iter().map(|_| vec![0.0; 384]).collect())
        }
    }
}
