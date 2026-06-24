use crate::Result;
use keyring::Entry;
use aes_gcm::aead::{OsRng, rand_core::RngCore};
use base64::Engine;
use zeroize::Zeroize;

pub struct KeyringStore {
    service_name: String,
}

impl KeyringStore {
    pub fn new() -> Self {
        Self {
            service_name: "sprawl".to_string(),
        }
    }

    /// Get or generate the master encryption key for ledger field encryption.
    pub fn get_or_create_master_key(&self) -> Result<[u8; 32]> {
        let entry = Entry::new(&self.service_name, "master-key")
            .map_err(|e| crate::SprawlError::Other(format!("Cannot access OS credential store: {}. Sprawl requires a working keyring.", e)))?;
            
        match entry.get_password() {
            Ok(key_b64) => {
                let bytes = base64::engine::general_purpose::STANDARD.decode(key_b64)
                    .map_err(|_| crate::SprawlError::Other("Corrupted master key in keyring".into()))?;
                
                if bytes.len() != 32 {
                    return Err(crate::SprawlError::Other("Master key size mismatch".into()));
                }
                
                let mut out = [0u8; 32];
                out.copy_from_slice(&bytes);
                Ok(out)
            }
            Err(keyring::Error::NoEntry) => {
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                let key_b64 = base64::engine::general_purpose::STANDARD.encode(&key);
                
                entry.set_password(&key_b64)
                    .map_err(|e| crate::SprawlError::Other(format!("Failed to save master key to OS keyring: {}", e)))?;
                    
                Ok(key)
            }
            Err(e) => Err(crate::SprawlError::Other(format!("Cannot access OS credential store: {}. Sprawl requires a working keyring.", e)))
        }
    }

    /// Vault a discovered secret's raw value to the OS keyring.
    /// Returns a reference ID for later retrieval.
    /// The raw value is zeroized from caller memory after this call.
    pub fn vault_secret(&self, secret_id: &str, mut raw_value: String) -> Result<String> {
        let ref_id = format!("secret-{}", secret_id);
        let entry = Entry::new(&self.service_name, &ref_id)
            .map_err(|e| crate::SprawlError::Other(format!("Keyring error: {}", e)))?;
            
        entry.set_password(&raw_value)
            .map_err(|e| crate::SprawlError::Other(format!("Failed to save secret to OS keyring: {}", e)))?;
            
        raw_value.zeroize();
        Ok(ref_id)
    }

    /// Retrieve a vaulted secret by reference ID.
    pub fn retrieve_secret(&self, ref_id: &str) -> Result<String> {
        let entry = Entry::new(&self.service_name, ref_id)
            .map_err(|e| crate::SprawlError::Other(format!("Keyring error: {}", e)))?;
            
        entry.get_password()
            .map_err(|e| crate::SprawlError::Other(format!("Failed to retrieve secret from OS keyring: {}", e)))
    }
}
