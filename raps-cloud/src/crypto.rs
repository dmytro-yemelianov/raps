// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};

pub struct MasterKey {
    key: [u8; 32],
}

pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

impl MasterKey {
    pub fn from_hex(hex_str: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 32 {
            anyhow::bail!("Master key must be exactly 32 bytes (64 hex chars)");
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(Self { key })
    }

    #[cfg(test)]
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }
}

pub fn encrypt(master_key: &MasterKey, plaintext: &[u8]) -> anyhow::Result<EncryptedData> {
    let cipher = Aes256Gcm::new_from_slice(&master_key.key)?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {e}"))?;

    Ok(EncryptedData {
        ciphertext,
        nonce: nonce_bytes.to_vec(),
    })
}

pub fn decrypt(master_key: &MasterKey, ciphertext: &[u8], nonce: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(&master_key.key)?;
    let nonce = Nonce::from_slice(nonce);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {e}"))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = MasterKey::generate();
        let plaintext = br#"{"client_id":"abc","client_secret":"xyz"}"#;

        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted.ciphertext, &encrypted.nonce).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = MasterKey::generate();
        let key2 = MasterKey::generate();
        let plaintext = b"secret data";

        let encrypted = encrypt(&key1, plaintext).unwrap();
        let result = decrypt(&key2, &encrypted.ciphertext, &encrypted.nonce);

        assert!(result.is_err());
    }
}
