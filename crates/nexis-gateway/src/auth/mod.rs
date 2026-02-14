//! Authentication module for Nexis Gateway

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Subject (Member ID)
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at
    pub iss: String,        // Issuer
    pub aud: String,        // Audience
    pub member_type: String, // human, ai, agent, system
}

/// Authentication error
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Missing authorization header")]
    MissingHeader,
    #[error("Invalid header format")]
    InvalidHeaderFormat,
}

/// JWT configuration
#[derive(Clone)]
pub struct JwtConfig {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    pub issuer: String,
    pub audience: String,
    pub expiry_seconds: u64,
}

impl JwtConfig {
    /// Create a new JWT config with a secret
    pub fn new(secret: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            issuer,
            audience,
            expiry_seconds: 3600, // 1 hour default
        }
    }

    /// Generate a JWT token
    pub fn generate_token(&self, member_id: &str, member_type: &str) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: member_id.to_string(),
            exp: now + self.expiry_seconds as usize,
            iat: now,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            member_type: member_type.to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| AuthError::InvalidToken)
    }

    /// Verify a JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| {
                if e.kind() == &jsonwebtoken::errors::ErrorKind::ExpiredSignature {
                    AuthError::TokenExpired
                } else {
                    AuthError::InvalidToken
                }
            })
    }
}

/// Authenticated user extractor
pub struct AuthenticatedUser {
    pub member_id: String,
    pub member_type: String,
    pub claims: Claims,
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // TODO: Extract JWT from Authorization header and validate
        // For now, return a placeholder
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_config_generates_and_verifies_token() {
        let config = JwtConfig::new(
            "test_secret_key_that_is_long_enough",
            "nexis-test".to_string(),
            "nexis".to_string(),
        );

        let token = config.generate_token("nexis:human:alice@example.com", "human").unwrap();
        let claims = config.verify_token(&token).unwrap();

        assert_eq!(claims.sub, "nexis:human:alice@example.com");
        assert_eq!(claims.member_type, "human");
        assert_eq!(claims.iss, "nexis-test");
    }

    #[test]
    fn invalid_token_is_rejected() {
        let config = JwtConfig::new(
            "test_secret_key",
            "nexis-test".to_string(),
            "nexis".to_string(),
        );

        let result = config.verify_token("invalid_token");
        assert!(result.is_err());
    }
}
