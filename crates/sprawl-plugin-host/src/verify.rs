use ed25519_dalek::{Signature, VerifyingKey, Verifier, PUBLIC_KEY_LENGTH};
use serde::{Deserialize, Serialize};
use std::path::Path;
use sprawl_core::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub signature_hex: String, // Hex-encoded signature of the WASM file
}

/// The default community signing key for Sprawl Plugins
pub const COMMUNITY_SIGNING_KEY: [u8; 32] = [
    0xB7, 0x19, 0x3E, 0x87, 0x63, 0x58, 0x90, 0x60, 
    0xD0, 0x33, 0x1F, 0x52, 0xCD, 0x74, 0xEF, 0x0C, 
    0x3C, 0x3F, 0xF4, 0xB1, 0x48, 0xFF, 0x9B, 0x93, 
    0x47, 0x4C, 0xE1, 0x76, 0x29, 0x99, 0x3A, 0x76, 
];

pub struct Ed25519Verifier {
    public_key: VerifyingKey,
}

impl Ed25519Verifier {
    /// Initialize with a known trusted public key for Sprawl Community Plugins
    pub fn new(public_key_bytes: &[u8; PUBLIC_KEY_LENGTH]) -> Result<Self> {
        let public_key = VerifyingKey::from_bytes(public_key_bytes)
            .map_err(|e| sprawl_core::SprawlError::Other(format!("Invalid public key: {}", e)))?;
        Ok(Self { public_key })
    }

    /// Verify a `.wasm` file against a signature
    pub fn verify_file(&self, wasm_path: &Path, signature_hex: &str) -> Result<bool> {
        let wasm_bytes = std::fs::read(wasm_path)
            .map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to read WASM file: {} - {}", wasm_path.display(), e)))?;
            
        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| sprawl_core::SprawlError::Other(format!("Invalid hex signature: {}", e)))?;
            
        if sig_bytes.len() != ed25519_dalek::SIGNATURE_LENGTH {
            return Ok(false);
        }
        
        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|e| sprawl_core::SprawlError::Other(format!("Invalid signature format: {}", e)))?;
            
        Ok(self.public_key.verify(&wasm_bytes, &signature).is_ok())
    }
}
