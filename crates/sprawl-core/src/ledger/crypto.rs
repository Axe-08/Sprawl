use crate::Result;
use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;

pub struct FieldEncryptor {
    cipher: Aes256Gcm,
}

impl FieldEncryptor {
    pub fn new(key: &[u8; 32]) -> Result<Self> {
        let cipher = Aes256Gcm::new(key.into());
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| crate::SprawlError::Other(format!("Encryption failed: {:?}", e)))?;

        let mut payload = nonce_bytes.to_vec();
        payload.extend(ciphertext);

        Ok(base64::engine::general_purpose::STANDARD.encode(&payload))
    }

    pub fn decrypt(&self, payload_b64: &str) -> Result<String> {
        let payload = base64::engine::general_purpose::STANDARD
            .decode(payload_b64)
            .map_err(|e| crate::SprawlError::Other(format!("Base64 decode failed: {:?}", e)))?;

        if payload.len() < 12 {
            return Err(crate::SprawlError::Other("Invalid payload length".into()));
        }

        let nonce = Nonce::from_slice(&payload[..12]);
        let ciphertext = &payload[12..];

        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| crate::SprawlError::Other(format!("Decryption failed: {:?}", e)))?;

        String::from_utf8(plaintext_bytes)
            .map_err(|e| crate::SprawlError::Other(format!("UTF-8 decode failed: {:?}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let encryptor = FieldEncryptor::new(&key).unwrap();
        let plaintext = "secret_provider_string_12345!@#";

        let ciphertext = encryptor.encrypt(plaintext).unwrap();
        assert_ne!(plaintext, ciphertext, "Ciphertext should not be plaintext");
        assert!(
            ciphertext.len() > plaintext.len(),
            "Ciphertext should include nonce and MAC"
        );

        let decrypted = encryptor.decrypt(&ciphertext).unwrap();
        assert_eq!(
            plaintext, decrypted,
            "Decrypted text should match original plaintext"
        );
    }

    #[test]
    fn test_decryption_fails_on_tamper() {
        let key = [42u8; 32];
        let encryptor = FieldEncryptor::new(&key).unwrap();
        let ciphertext = encryptor.encrypt("data").unwrap();

        // Tamper with the base64 string
        let tampered = ciphertext[..ciphertext.len() - 4].to_string() + "AAAA";
        assert!(
            encryptor.decrypt(&tampered).is_err(),
            "Decryption must fail if ciphertext or MAC is tampered"
        );
    }
}
