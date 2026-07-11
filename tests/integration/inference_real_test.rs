use sprawl_inference::{InferenceEngine, DeviceTarget, DEFAULT_MODEL, SysInfo};
use sprawl_core::platform::sprawl_data_dir;

struct RealRam;
impl SysInfo for RealRam {
    fn available_ram_mb(&self) -> u64 { 16384 }
}

/// This test only runs when:
///   1. `--features inference` is enabled
///   2. The model file is present at ~/.sprawl/models/
/// Run via `./scripts/run_inference_tests.sh`
#[tokio::test]
#[cfg_attr(not(feature = "inference"), ignore)]
async fn test_real_phi3_produces_output() {
    let model_path = sprawl_data_dir()
        .unwrap()
        .join("models")
        .join(DEFAULT_MODEL.filename);
    
    if !model_path.exists() {
        eprintln!("[SKIP] Model not found at {:?}. Run ./scripts/run_inference_tests.sh", model_path);
        return;
    }
    
    let mut engine = InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, RealRam);
    engine.load_model(&model_path, None).unwrap();
    
    let response = engine
        .run_prompt(r#"Classify: sk_live_abc123\nRespond with JSON: {"classification": "likely_secret" | "likely_noise"}"#)
        .await
        .unwrap();
    
    assert!(!response.is_empty(), "Model should produce non-empty output");
    println!("[Real inference output]: {}", response);
}
