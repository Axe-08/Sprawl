use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use rand::RngCore;
use crate::Result;

pub struct FieldEncryptor {
    cipher: Aes256Gcm,
}

impl FieldEncryptor {
    /// Create from a 256-bit key retrieved from the OS keyring.
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(key)
            .expect("Valid AES-256 key size");
        Self { cipher }
    }

    /// Encrypt a field value. Returns base64(nonce || ciphertext).
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| crate::SprawlError::Other(format!("Encryption error: {}", e)))?;

        let mut out = nonce_bytes.to_vec();
        out.extend(ciphertext);
        
        Ok(base64::engine::general_purpose::STANDARD.encode(&out))
    }

    /// Decrypt a field value. Input is base64(nonce || ciphertext).
    pub fn decrypt(&self, encoded: &str) -> Result<String> {
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded)
            .map_err(|e| crate::SprawlError::Other(format!("Decryption (base64) error: {}", e)))?;
        
        if decoded.len() < 12 {
            return Err(crate::SprawlError::Other("Invalid encrypted payload length".into()));
        }
        
        let nonce = Nonce::from_slice(&decoded[..12]);
        let ciphertext = &decoded[12..];

        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| crate::SprawlError::Other(format!("Decryption error: {}", e)))?;
            
        String::from_utf8(plaintext)
            .map_err(|_| crate::SprawlError::Other("Invalid UTF-8 in decrypted string".into()))
    }
}
