use crate::classify::SecretClassification;
use sprawl_inference::{InferenceEngine, Result as InferenceResult};
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct DiscoveredSecret {
    pub id: Uuid,
    pub raw_value: String,
    pub filepath: String,
}

pub async fn batch_classify<S: sprawl_inference::SysInfo>(
    ambiguous: &[DiscoveredSecret],
    inference: &mut InferenceEngine<S>,
) -> InferenceResult<Vec<(Uuid, SecretClassification)>> {
    // 1. RAM pre-flight check
    inference.preflight_check()?;

    let mut results = Vec::new();

    // 2. Build a batch prompt and classify each
    for secret in ambiguous {
        // Redact the secret: only show first 4 and last 4 characters if long enough
        let redacted = if secret.raw_value.len() > 8 {
            let start = &secret.raw_value[0..4];
            let end = &secret.raw_value[secret.raw_value.len() - 4..];
            format!("{}...{}", start, end)
        } else {
            "***REDACTED***".to_string()
        };

        let prompt = format!(
            "Classify the string found in {}: {}\nRespond with JSON: {{\"classification\": \"likely_secret\" | \"likely_noise\" | \"ambiguous\", \"reason\": \"<short reason>\"}}",
            secret.filepath, redacted
        );

        let response = inference.run_prompt(&prompt).await?;

        #[derive(serde::Deserialize)]
        struct LlmClassification {
            classification: String,
            reason: String,
        }

        let parsed: std::result::Result<LlmClassification, _> = serde_json::from_str(&response);
        let classification = match parsed {
            Ok(c) if c.classification == "likely_noise" => {
                SecretClassification::FilteredNoise(c.reason)
            }
            Ok(c) if c.classification == "ambiguous" => SecretClassification::Ambiguous,
            Ok(c) => SecretClassification::KnownProvider(c.reason),
            Err(_) => SecretClassification::Ambiguous, // Malformed response -> conservative default
        };

        results.push((secret.id, classification));
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sprawl_inference::{DeviceTarget, SysInfo};

    struct HighRamMock;
    impl SysInfo for HighRamMock {
        fn available_ram_mb(&self) -> u64 {
            8192
        }
    }

    #[tokio::test]
    async fn test_batch_classify_redacts_secrets() {
        let mut engine = InferenceEngine::new(
            sprawl_inference::DEFAULT_MODEL,
            DeviceTarget::Cpu,
            HighRamMock,
        );

        let secrets = vec![DiscoveredSecret {
            id: Uuid::new_v4(),
            raw_value: format!("sk_live_{}", "1234567890abcdefghijklmnopqrstuv").to_string(),
            filepath: ".env".to_string(),
        }];

        let results = batch_classify(&secrets, &mut engine).await.unwrap();

        assert_eq!(results.len(), 1);
        // The mock engine returns likely_noise unless the exact string 'sk_live' is in the prompt.
        // Because we redacted it to 'sk_l...stuv', 'sk_live' won't be in the prompt.
        // Therefore, it will return likely_noise, proving redaction worked!
        assert!(matches!(
            results[0].1,
            SecretClassification::FilteredNoise(_)
        ));
    }
}
