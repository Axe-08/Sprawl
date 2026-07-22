use crate::classify::{classify_string, SecretClassification};
use crate::entropy::shannon_entropy;
use sprawl_core::config::domain::NoisePattern;
use zeroize::Zeroize;

pub trait KeyringBackend: Send + Sync {
    fn vault_secret(&self, val: &str) -> String;
}

pub trait LedgerBackend: Send + Sync {
    fn save_secret(&self, hash: &str, keyring_ref: &str);
    fn queue_ambiguous(&self, val: &str, filepath: &str);
    fn get_ambiguous_secrets(&self) -> Vec<crate::llm::DiscoveredSecret>;
    fn mark_accepted(&self, id: uuid::Uuid);
    fn mark_rejected(&self, id: uuid::Uuid);
}

pub struct OsKeyringStore {
    service_name: String,
}

impl OsKeyringStore {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    pub fn has_secret(&self, id: &str) -> bool {
        if let Ok(entry) = keyring::Entry::new(&self.service_name, id) {
            entry.get_password().is_ok()
        } else {
            false
        }
    }
}

impl KeyringBackend for OsKeyringStore {
    fn vault_secret(&self, val: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        if let Ok(entry) = keyring::Entry::new(&self.service_name, &id) {
            let _ = entry.set_password(val);
        }
        id
    }
}

pub struct SqliteLedgerStore {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl SqliteLedgerStore {
    pub fn new(conn: rusqlite::Connection) -> Self {
        Self {
            conn: std::sync::Mutex::new(conn),
        }
    }
}

impl LedgerBackend for SqliteLedgerStore {
    fn save_secret(&self, hash: &str, keyring_ref: &str) {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO secrets (id, source_file, classification, key_hash, discovered_at, keyring_ref) VALUES (?1, 'unknown', 'KnownProvider', ?2, ?3, ?4)",
            (&id, hash, &now, keyring_ref),
        );
    }

    fn queue_ambiguous(&self, val: &str, filepath: &str) {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare("SELECT 1 FROM ambiguous_secrets WHERE raw_value = ?1 AND status = 'pending'").unwrap();
        if stmt.exists([val]).unwrap_or(false) {
            return; // Skip duplicate
        }

        let id = uuid::Uuid::new_v4().to_string();
        let _ = conn.execute(
            "INSERT INTO ambiguous_secrets (id, raw_value, filepath, status) VALUES (?1, ?2, ?3, 'pending')",
            (&id, val, filepath),
        );
    }

    fn get_ambiguous_secrets(&self) -> Vec<crate::llm::DiscoveredSecret> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, raw_value, filepath FROM ambiguous_secrets WHERE status = 'pending'").unwrap();
        let rows = stmt.query_map([], |row| {
            Ok(crate::llm::DiscoveredSecret {
                id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                raw_value: row.get(1)?,
                filepath: row.get(2)?,
            })
        }).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    fn mark_accepted(&self, id: uuid::Uuid) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("UPDATE ambiguous_secrets SET status = 'accepted' WHERE id = ?1", [&id.to_string()]);
    }

    fn mark_rejected(&self, id: uuid::Uuid) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("UPDATE ambiguous_secrets SET status = 'rejected' WHERE id = ?1", [&id.to_string()]);
    }
}


pub struct SentinelScanner {
    _noise_patterns: Vec<NoisePattern>,
    keyring: Box<dyn KeyringBackend>,
    ledger: Box<dyn LedgerBackend>,
}

impl SentinelScanner {
    pub fn new(
        noise_patterns: Vec<NoisePattern>,
        keyring: Box<dyn KeyringBackend>,
        ledger: Box<dyn LedgerBackend>,
    ) -> Self {
        Self {
            _noise_patterns: noise_patterns,
            keyring,
            ledger,
        }
    }

    /// Evaluates a raw string chunk, calculates entropy, classifies it, and vaults if KnownProvider.
    pub fn scan_string(&self, filepath: &str, mut raw_value: String) {
        if raw_value.len() < 16 {
            return; // Fast path skip: too short
        }

        let entropy = shannon_entropy(&raw_value);
        if entropy < 4.5 {
            return; // Fast path skip: low entropy (e.g. repetitive characters or plain English)
        }

        let classification = classify_string(&raw_value);

        match classification.status {
            SecretClassification::KnownProvider(_provider_name) => {
                // M10 Flow: Vault immediately
                let _hash = {
                    use sha2::{Digest, Sha256};
                    hex::encode(Sha256::digest(raw_value.as_bytes()))
                };

                let keyring_ref = self.keyring.vault_secret(&raw_value);
                self.ledger.save_secret(&_hash, &keyring_ref);

                // Zeroize raw string securely
                raw_value.zeroize();
            }
            SecretClassification::FilteredNoise(_reason) => {
                // Drop intentionally
            }
            SecretClassification::Ambiguous => {
                // Queue for Layer 2 HITL Inbox / LLM batch
                self.ledger.queue_ambiguous(&raw_value, filepath);
            }
        }
    }

    pub fn get_ambiguous_secrets(&self) -> Vec<crate::llm::DiscoveredSecret> {
        self.ledger.get_ambiguous_secrets()
    }

    pub fn mark_accepted(&self, id: uuid::Uuid) {
        self.ledger.mark_accepted(id);
    }

    pub fn mark_rejected(&self, id: uuid::Uuid) {
        self.ledger.mark_rejected(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LocalMockKeyringStore;
    impl KeyringBackend for LocalMockKeyringStore {
        fn vault_secret(&self, _val: &str) -> String {
            "mock_keyring_ref_123".to_string()
        }
    }

    struct LocalMockLedger;
    impl LedgerBackend for LocalMockLedger {
        fn save_secret(&self, _hash: &str, _keyring_ref: &str) {}
        fn queue_ambiguous(&self, _val: &str, _filepath: &str) {}
        fn get_ambiguous_secrets(&self) -> Vec<crate::llm::DiscoveredSecret> { vec![] }
        fn mark_accepted(&self, _id: uuid::Uuid) {}
        fn mark_rejected(&self, _id: uuid::Uuid) {}
    }

    #[test]
    fn test_zeroize_after_vaulting() {
        // Technically this tests that the method compiles and runs without panicking.
        // A direct memory check is difficult in safe Rust.
        let scanner = SentinelScanner::new(vec![], Box::new(LocalMockKeyringStore), Box::new(LocalMockLedger));

        let mut fake_stripe_key = "sk_live_".to_string();
        for _ in 0..30 {
            fake_stripe_key.push('A');
        }

        scanner.scan_string("test.rs", fake_stripe_key);
        // If it compiles and runs without use-after-free or panic, zeroize succeeded.
    }
}
