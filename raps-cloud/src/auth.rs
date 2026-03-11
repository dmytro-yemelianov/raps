// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {e}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid hash format: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn encode_jwt(secret: &str, claims: &Claims) -> anyhow::Result<String> {
    let token = encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn decode_jwt(secret: &str, token: &str) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "correct-horse-battery-staple";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_jwt_roundtrip() {
        let secret = "test-secret-key-that-is-long-enough";
        let claims = Claims {
            sub: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            role: "admin".to_string(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        let token = encode_jwt(secret, &claims).unwrap();
        let decoded = decode_jwt(secret, &token).unwrap();
        assert_eq!(claims.sub, decoded.sub);
        assert_eq!(claims.tenant_id, decoded.tenant_id);
        assert_eq!(claims.role, decoded.role);
    }

    #[test]
    fn test_expired_jwt_rejected() {
        let secret = "test-secret";
        let claims = Claims {
            sub: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            role: "member".to_string(),
            exp: (Utc::now() - chrono::Duration::hours(1)).timestamp() as usize,
        };
        let token = encode_jwt(secret, &claims).unwrap();
        let result = decode_jwt(secret, &token);
        assert!(result.is_err());
    }
}
