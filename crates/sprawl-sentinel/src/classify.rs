use regex::Regex;
use std::sync::OnceLock;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum SecretClassification {
    KnownProvider(String), // Provider name e.g. "Stripe Live"
    FilteredNoise(String), // Reason e.g. "UUID", "Git SHA"
    Ambiguous,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Classification {
    pub raw: String,
    pub status: SecretClassification,
}

// Thread-safe compiled regexes for the built-in negative filters
fn negative_filters() -> &'static [(Regex, &'static str)] {
    static FILTERS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    FILTERS.get_or_init(|| {
        vec![
            (
                Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
                    .unwrap(),
                "UUID",
            ),
            (Regex::new(r"^[0-9a-f]{40}$").unwrap(), "Git SHA"),
            (Regex::new(r"^[0-9a-f]{64}$").unwrap(), "SHA-256 hash"),
            (
                Regex::new(r"^eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$").unwrap(),
                "JWT",
            ),
            (Regex::new(r"^\$2[aby]\$\d{2}\$").unwrap(), "bcrypt hash"),
        ]
    })
}

// Thread-safe compiled regexes for known provider prefixes
fn provider_prefixes() -> &'static [(Regex, &'static str)] {
    static PREFIXES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    PREFIXES.get_or_init(|| {
        vec![
            (
                Regex::new(r"^sk_live_[a-zA-Z0-9]{24,}").unwrap(),
                "Stripe Live",
            ),
            (
                Regex::new(r"^sk_test_[a-zA-Z0-9]{24,}").unwrap(),
                "Stripe Test",
            ),
            (Regex::new(r"^AKIA[0-9A-Z]{16}").unwrap(), "AWS Access Key"),
            (Regex::new(r"^ghp_[A-Za-z0-9]{36}").unwrap(), "GitHub PAT"),
            (Regex::new(r"^gho_[A-Za-z0-9]{36}").unwrap(), "GitHub OAuth"),
            (
                Regex::new(r"^glpat-[A-Za-z0-9\-]{20,}").unwrap(),
                "GitLab PAT",
            ),
            (
                Regex::new(r"^xox[bpors]-[0-9a-zA-Z\-]+").unwrap(),
                "Slack Token",
            ),
            (
                Regex::new(r"^AIza[0-9A-Za-z\-_]{35}").unwrap(),
                "Google API Key",
            ),
            (Regex::new(r"^npm_[A-Za-z0-9]{36}").unwrap(), "npm Token"),
            (
                Regex::new(r"^SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}").unwrap(),
                "SendGrid",
            ),
            (Regex::new(r"^SK[0-9a-fA-F]{32}").unwrap(), "Twilio"),
        ]
    })
}

pub fn classify_string(s: &str) -> Classification {
    // 1. Check Known Providers
    for (re, name) in provider_prefixes() {
        if re.is_match(s) {
            return Classification {
                raw: s.to_string(),
                status: SecretClassification::KnownProvider(name.to_string()),
            };
        }
    }

    // 2. Check Negative Filters
    for (re, reason) in negative_filters() {
        if re.is_match(s) {
            return Classification {
                raw: s.to_string(),
                status: SecretClassification::FilteredNoise(reason.to_string()),
            };
        }
    }

    // 3. Fallback
    Classification {
        raw: s.to_string(),
        status: SecretClassification::Ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_provider_classification() {
        let res = classify_string(&format!("sk_live_{}", "1234567890abcdefghijklmnopqr"));
        assert_eq!(
            res.status,
            SecretClassification::KnownProvider("Stripe Live".to_string())
        );

        let res2 = classify_string("AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            res2.status,
            SecretClassification::KnownProvider("AWS Access Key".to_string())
        );
    }

    #[test]
    fn test_negative_filter_classification() {
        let res = classify_string("123e4567-e89b-12d3-a456-426614174000");
        assert_eq!(
            res.status,
            SecretClassification::FilteredNoise("UUID".to_string())
        );

        let res2 = classify_string("f1d2d2f924e986ac86fdf7b36c94bcdf32beec15");
        assert_eq!(
            res2.status,
            SecretClassification::FilteredNoise("Git SHA".to_string())
        );
    }

    #[test]
    fn test_ambiguous_classification() {
        let res = classify_string("Vq1B9xLz4M6nPw3Xm0R8bQv7Kj2YcF5t");
        assert_eq!(res.status, SecretClassification::Ambiguous);
    }
}
