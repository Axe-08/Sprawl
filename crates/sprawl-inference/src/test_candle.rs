#[cfg(feature = "inference")]
fn test_compile() {
    let _ = candle_transformers::models::quantized_llama::ModelWeights::from_gguf;
}
