use sprawl_core::Result;

pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

#[cfg(feature = "real-archivist")]
pub mod candle_embedder {
    use super::Embedder;
    use sprawl_core::Result;
    use std::path::Path;
    use candle_core::{Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::bert::{BertModel, Config, DTYPE};
    use tokenizers::Tokenizer;
    use std::sync::Arc;
    use tokio::io::AsyncWriteExt;
    
    pub struct CandleEmbedder {
        tokenizer: Arc<std::sync::Mutex<Tokenizer>>,
        model: Arc<std::sync::Mutex<BertModel>>,
    }

    impl CandleEmbedder {
        pub async fn load(model_dir: &Path) -> Result<Self> {
            let model_path = model_dir.join("model.safetensors");
            let tokenizer_path = model_dir.join("tokenizer.json");
            let config_path = model_dir.join("config.json");
            
            if !model_path.exists() || !tokenizer_path.exists() || !config_path.exists() {
                tracing::warn!("Models not found at {:?}. Downloading...", model_dir);
                Self::download_file("https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/model.safetensors", &model_path).await?;
                Self::download_file("https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json", &tokenizer_path).await?;
                Self::download_file("https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json", &config_path).await?;
            }

            let device = Device::Cpu; // Fallback to CPU for simplicity
            let config_content = std::fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&config_content)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            
            if let Some(pp) = tokenizer.get_padding_mut() {
                pp.pad_id = 0;
            } else {
                let pad_params = tokenizers::PaddingParams {
                    strategy: tokenizers::PaddingStrategy::BatchLongest,
                    direction: tokenizers::PaddingDirection::Right,
                    pad_to_multiple_of: None,
                    pad_id: 0,
                    pad_type_id: 0,
                    pad_token: String::from("[PAD]"),
                };
                tokenizer.with_padding(Some(pad_params));
            }

            let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model_path], DTYPE, &device) }
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            let model = BertModel::load(vb, &config)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            Ok(Self {
                tokenizer: Arc::new(std::sync::Mutex::new(tokenizer)),
                model: Arc::new(std::sync::Mutex::new(model)),
            })
        }

        async fn download_file(url: &str, dest: &Path) -> Result<()> {
            let mut res = reqwest::get(url).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            let mut file = tokio::fs::File::create(dest).await?;
            while let Some(chunk) = res.chunk().await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))? {
                file.write_all(&chunk).await?;
            }
            Ok(())
        }
    }

    impl Embedder for CandleEmbedder {
        fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
            if texts.is_empty() {
                return Ok(vec![]);
            }

            let tokenizer = self.tokenizer.lock().unwrap();
            let model = self.model.lock().unwrap();

            let tokens = tokenizer.encode_batch(texts.to_vec(), true)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let masks: Vec<Vec<u32>> = tokens.iter().map(|t| t.get_attention_mask().to_vec()).collect();

            let token_ids: Vec<Vec<u32>> = tokens.iter().map(|t| t.get_ids().to_vec()).collect();
            let device = &model.device;
            let n_sentences = token_ids.len();
            let n_tokens = token_ids[0].len();
            
            let mut flat_ids: Vec<u32> = Vec::with_capacity(n_sentences * n_tokens);
            for ids in &token_ids {
                flat_ids.extend(ids);
            }
            let mut flat_masks: Vec<u32> = Vec::with_capacity(n_sentences * n_tokens);
            for m in &masks {
                flat_masks.extend(m);
            }

            let token_ids = Tensor::from_vec(flat_ids, (n_sentences, n_tokens), device)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            let token_type_ids = token_ids.zeros_like()
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            let mask_tensor = Tensor::from_vec(flat_masks, (n_sentences, n_tokens), device)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .to_dtype(candle_core::DType::F32)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            
            let embeddings = model.forward(&token_ids, &token_type_ids, None)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let mask = mask_tensor.unsqueeze(2).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            let masked_embeddings = embeddings.broadcast_mul(&mask)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            let sum_embeddings = masked_embeddings.sum(1)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
            
            // clamp mask sum to avoid division by zero
            let mask_sum = mask.sum(1)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .clamp(1e-9f32, f32::MAX)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                
            let mean_pooled = sum_embeddings.broadcast_div(&mask_sum)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let norm = mean_pooled.sqr()
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .sum_keepdim(1)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .sqrt()
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
                .clamp(1e-9f32, f32::MAX)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                
            let normalized = mean_pooled.broadcast_div(&norm)
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                
            let embeddings_out = normalized.to_vec2::<f32>()
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            Ok(embeddings_out)
        }
    }
}
