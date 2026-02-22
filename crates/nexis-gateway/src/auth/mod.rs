//! Authentication module for Nexis Gateway
//!
//! This module provides JWT-based authentication for WebSocket connections.
//! Currently implements token generation/verification; full integration pending.

#![allow(dead_code)]

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[cfg(feature = "multi-tenant")]
mod tenant;
#[cfg(feature = "multi-tenant")]
pub use tenant::{TenantContext, TenantError, TenantExtractor, TenantStore};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub aud: String,
    pub member_type: String,
    #[cfg(feature = "multi-tenant")]
    #[serde(default)]
    pub tenant_id: Option<String>,
}

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
    #[cfg(feature = "multi-tenant")]
    #[error("Tenant context required")]
    TenantRequired,
}

#[derive(Clone)]
pub struct JwtConfig {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    pub issuer: String,
    pub audience: String,
    pub expiry_seconds: u64,
}

impl JwtConfig {
    pub fn new(secret: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            issuer,
            audience,
            expiry_seconds: 3600,
        }
    }

    pub fn generate_token(&self, member_id: &str, member_type: &str) -> Result<String, AuthError> {
        self.generate_token_with_tenant(member_id, member_type, None)
    }

    #[cfg(feature = "multi-tenant")]
    pub fn generate_token_with_tenant(
        &self,
        member_id: &str,
        member_type: &str,
        tenant_id: Option<&str>,
    ) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: member_id.to_string(),
            exp: now + self.expiry_seconds as usize,
            iat: now,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            member_type: member_type.to_string(),
            tenant_id: tenant_id.map(|s| s.to_string()),
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|_| AuthError::InvalidToken)
    }

    #[cfg(not(feature = "multi-tenant"))]
    pub fn generate_token_with_tenant(
        &self,
        member_id: &str,
        member_type: &str,
        _tenant_id: Option<&str>,
    ) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: member_id.to_string(),
            exp: now + self.expiry_seconds as usize,
            iat: now,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            member_type: member_type.to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|_| AuthError::InvalidToken)
    }

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

pub struct AuthenticatedUser {
    pub member_id: String,
    pub member_type: String,
    pub claims: Claims,
    #[cfg(feature = "multi-tenant")]
    pub tenant_context: Option<TenantContext>,
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(feature = "multi-tenant")]
pub fn extract_tenant_from_claims(claims: &Claims) -> Option<TenantContext> {
    claims.tenant_id.as_ref().map(|tid| TenantContext {
        tenant_id: tid.clone(),
    })
}

#[cfg(feature = "multi-tenant")]
pub fn check_tenant_access(
    user_tenant: &TenantContext,
    resource_tenant: &str,
) -> Result<(), TenantError> {
    if user_tenant.tenant_id == resource_tenant {
        Ok(())
    } else {
        Err(TenantError::CrossTenantAccess {
            user_tenant: user_tenant.tenant_id.clone(),
            resource_tenant: resource_tenant.to_string(),
        })
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

        let token = config
            .generate_token("nexis:human:alice@example.com", "human")
            .unwrap();
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

    #[cfg(feature = "multi-tenant")]
    mod multi_tenant_tests {
        use super::*;

        #[test]
        fn jwt_config_generates_token_with_tenant() {
            let config = JwtConfig::new(
                "test_secret_key_that_is_long_enough",
                "nexis-test".to_string(),
                "nexis".to_string(),
            );

            let token = config
                .generate_token_with_tenant(
                    "nexis:human:alice@example.com",
                    "human",
                    Some("tenant_acme"),
                )
                .unwrap();
            let claims = config.verify_token(&token).unwrap();

            assert_eq!(claims.sub, "nexis:human:alice@example.com");
            assert_eq!(claims.member_type, "human");
            assert_eq!(claims.tenant_id, Some("tenant_acme".to_string()));
        }

        #[test]
        fn jwt_config_generates_token_without_tenant() {
            let config = JwtConfig::new(
                "test_secret_key_that_is_long_enough",
                "nexis-test".to_string(),
                "nexis".to_string(),
            );

            let token = config
                .generate_token_with_tenant("nexis:human:alice@example.com", "human", None)
                .unwrap();
            let claims = config.verify_token(&token).unwrap();

            assert_eq!(claims.tenant_id, None);
        }

        #[test]
        fn extract_tenant_from_claims_returns_context() {
            let config = JwtConfig::new(
                "test_secret_key_that_is_long_enough",
                "nexis-test".to_string(),
                "nexis".to_string(),
            );

            let token = config
                .generate_token_with_tenant("user1", "human", Some("tenant_123"))
                .unwrap();
            let claims = config.verify_token(&token).unwrap();

            let tenant_ctx = extract_tenant_from_claims(&claims);
            assert!(tenant_ctx.is_some());
            assert_eq!(tenant_ctx.unwrap().tenant_id, "tenant_123");
        }

        #[test]
        fn extract_tenant_from_claims_returns_none_when_missing() {
            let config = JwtConfig::new(
                "test_secret_key_that_is_long_enough",
                "nexis-test".to_string(),
                "nexis".to_string(),
            );

            let token = config.generate_token("user1", "human").unwrap();
            let claims = config.verify_token(&token).unwrap();

            let tenant_ctx = extract_tenant_from_claims(&claims);
            assert!(tenant_ctx.is_none());
        }

        #[test]
        fn check_tenant_access_allows_same_tenant() {
            let user_tenant = TenantContext {
                tenant_id: "tenant_123".to_string(),
            };
            let result = check_tenant_access(&user_tenant, "tenant_123");
            assert!(result.is_ok());
        }

        #[test]
        fn check_tenant_access_rejects_cross_tenant() {
            let user_tenant = TenantContext {
                tenant_id: "tenant_123".to_string(),
            };
            let result = check_tenant_access(&user_tenant, "tenant_456");
            assert!(result.is_err());
            match result {
                Err(TenantError::CrossTenantAccess {
                    user_tenant,
                    resource_tenant,
                }) => {
                    assert_eq!(user_tenant, "tenant_123");
                    assert_eq!(resource_tenant, "tenant_456");
                }
                _ => panic!("Expected CrossTenantAccess error"),
            }
        }
    }
}
