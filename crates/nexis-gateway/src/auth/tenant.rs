//! Tenant context extraction and cross-tenant access detection
//!
//! This module provides tenant-aware authentication for multi-tenant deployments.

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum TenantError {
    #[error("Missing tenant context")]
    MissingTenant,
    #[error("Invalid tenant header format")]
    InvalidHeaderFormat,
    #[error("Cross-tenant access denied: user '{user_tenant}' cannot access resource in '{resource_tenant}'")]
    CrossTenantAccess {
        user_tenant: String,
        resource_tenant: String,
    },
    #[error("Tenant ID is required for this operation")]
    TenantRequired,
}

impl IntoResponse for TenantError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            TenantError::MissingTenant => {
                (StatusCode::BAD_REQUEST, "Missing tenant context")
            }
            TenantError::InvalidHeaderFormat => {
                (StatusCode::BAD_REQUEST, "Invalid tenant header format")
            }
            TenantError::CrossTenantAccess { .. } => {
                (StatusCode::FORBIDDEN, "Cross-tenant access denied")
            }
            TenantError::TenantRequired => {
                (StatusCode::BAD_REQUEST, "Tenant ID is required")
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    pub tenant_id: String,
}

impl TenantContext {
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
        }
    }

    pub fn is_same_tenant(&self, other: &str) -> bool {
        self.tenant_id == other
    }
}

#[derive(Debug, Clone)]
pub struct TenantExtractor {
    pub tenant: TenantContext,
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for TenantExtractor
where
    S: Send + Sync,
{
    type Rejection = TenantError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        extract_tenant_from_headers(&parts.headers)
    }
}

pub fn extract_tenant_from_headers(headers: &HeaderMap) -> Result<TenantExtractor, TenantError> {
    headers
        .get("X-Tenant-ID")
        .or_else(|| headers.get("x-tenant-id"))
        .and_then(|value| value.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|tenant_id| TenantExtractor {
            tenant: TenantContext::new(tenant_id),
        })
        .ok_or(TenantError::MissingTenant)
}

pub fn tenant_extractor(claims: &crate::auth::Claims) -> Option<TenantContext> {
    claims.tenant_id.as_ref().map(|tid| TenantContext::new(tid))
}

#[derive(Debug, Clone)]
pub struct TenantGuard {
    current_tenant: TenantContext,
}

impl TenantGuard {
    pub fn new(tenant: TenantContext) -> Self {
        Self {
            current_tenant: tenant,
        }
    }

    pub fn check_access(&self, resource_tenant: &str) -> Result<(), TenantError> {
        if self.current_tenant.is_same_tenant(resource_tenant) {
            Ok(())
        } else {
            Err(TenantError::CrossTenantAccess {
                user_tenant: self.current_tenant.tenant_id.clone(),
                resource_tenant: resource_tenant.to_string(),
            })
        }
    }

    pub fn tenant_id(&self) -> &str {
        &self.current_tenant.tenant_id
    }
}

#[derive(Debug, Clone, Default)]
pub struct TenantStore {
    tenants: Arc<std::sync::RwLock<Vec<String>>>,
}

impl TenantStore {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    pub fn with_tenants(tenants: Vec<String>) -> Self {
        Self {
            tenants: Arc::new(std::sync::RwLock::new(tenants)),
        }
    }

    pub fn register_tenant(&self, tenant_id: String) {
        let mut tenants = self.tenants.write().unwrap();
        if !tenants.contains(&tenant_id) {
            tenants.push(tenant_id);
        }
    }

    pub fn tenant_exists(&self, tenant_id: &str) -> bool {
        let tenants = self.tenants.read().unwrap();
        tenants.iter().any(|t| t == tenant_id)
    }

    pub fn list_tenants(&self) -> Vec<String> {
        let tenants = self.tenants.read().unwrap();
        tenants.clone()
    }
}

impl FromRef<()> for TenantStore {
    fn from_ref(_: &()) -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_context_is_same_tenant_returns_true_for_match() {
        let ctx = TenantContext::new("tenant_123");
        assert!(ctx.is_same_tenant("tenant_123"));
    }

    #[test]
    fn tenant_context_is_same_tenant_returns_false_for_mismatch() {
        let ctx = TenantContext::new("tenant_123");
        assert!(!ctx.is_same_tenant("tenant_456"));
    }

    #[test]
    fn tenant_guard_allows_same_tenant() {
        let guard = TenantGuard::new(TenantContext::new("tenant_123"));
        let result = guard.check_access("tenant_123");
        assert!(result.is_ok());
    }

    #[test]
    fn tenant_guard_rejects_cross_tenant() {
        let guard = TenantGuard::new(TenantContext::new("tenant_123"));
        let result = guard.check_access("tenant_456");
        assert!(result.is_err());
    }

    #[test]
    fn tenant_store_registers_and_checks_tenants() {
        let store = TenantStore::new();
        assert!(!store.tenant_exists("tenant_123"));

        store.register_tenant("tenant_123".to_string());
        assert!(store.tenant_exists("tenant_123"));
    }

    #[test]
    fn tenant_store_list_tenants_returns_all() {
        let store = TenantStore::with_tenants(vec![
            "tenant_123".to_string(),
            "tenant_456".to_string(),
        ]);
        let tenants = store.list_tenants();
        assert_eq!(tenants.len(), 2);
        assert!(tenants.contains(&"tenant_123".to_string()));
        assert!(tenants.contains(&"tenant_456".to_string()));
    }

    #[test]
    fn tenant_store_prevents_duplicates() {
        let store = TenantStore::new();
        store.register_tenant("tenant_123".to_string());
        store.register_tenant("tenant_123".to_string());

        let tenants = store.list_tenants();
        assert_eq!(tenants.len(), 1);
    }
}
