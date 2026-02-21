//! Multi-tenant domain model for Nexis.
//!
//! This module defines tenant boundaries and cross-module entity mapping.
//! Enabled via the `multi-tenant` feature flag.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TenantError {
    #[error("tenant name cannot be empty")]
    EmptyName,
    #[error("tenant slug cannot be empty")]
    EmptySlug,
    #[error("tenant slug contains invalid characters: {0}")]
    InvalidSlug(String),
    #[error("tenant not found: {0}")]
    NotFound(TenantId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantId(Uuid);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn parse(s: &str) -> Result<Self, TenantError> {
        let uuid = Uuid::parse_str(s).map_err(|_| TenantError::NotFound(TenantId(Uuid::nil())))?;
        Ok(Self(uuid))
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub is_active: bool,
}

impl Tenant {
    pub fn new(name: String, slug: String) -> Result<Self, TenantError> {
        Self::validate_name(&name)?;
        Self::validate_slug(&slug)?;
        Ok(Self {
            id: TenantId::new(),
            name,
            slug,
            is_active: true,
        })
    }

    pub fn with_id(mut self, id: TenantId) -> Self {
        self.id = id;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    fn validate_name(name: &str) -> Result<(), TenantError> {
        if name.trim().is_empty() {
            return Err(TenantError::EmptyName);
        }
        Ok(())
    }

    fn validate_slug(slug: &str) -> Result<(), TenantError> {
        if slug.trim().is_empty() {
            return Err(TenantError::EmptySlug);
        }
        let valid = slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid {
            return Err(TenantError::InvalidSlug(slug.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_id_generates_uuid_v7() {
        let id = TenantId::new();
        let uuid = id.as_uuid();
        assert!(uuid.get_version() == Some(uuid::Version::SortRand));
    }

    #[test]
    fn tenant_id_parses_valid_uuid_string() {
        let original = TenantId::new();
        let parsed = TenantId::parse(&original.to_string()).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn tenant_id_rejects_invalid_string() {
        let result = TenantId::parse("not-a-uuid");
        assert!(matches!(result, Err(TenantError::NotFound(_))));
    }

    #[test]
    fn tenant_creates_with_valid_name_and_slug() {
        let tenant = Tenant::new("Acme Corp".to_string(), "acme-corp".to_string()).unwrap();
        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.slug, "acme-corp");
        assert!(tenant.is_active);
    }

    #[test]
    fn tenant_rejects_empty_name() {
        let result = Tenant::new("".to_string(), "slug".to_string());
        assert_eq!(result, Err(TenantError::EmptyName));
    }

    #[test]
    fn tenant_rejects_empty_slug() {
        let result = Tenant::new("Name".to_string(), "".to_string());
        assert_eq!(result, Err(TenantError::EmptySlug));
    }

    #[test]
    fn tenant_rejects_invalid_slug_characters() {
        let result = Tenant::new("Name".to_string(), "Invalid Slug!".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[test]
    fn tenant_serializes_to_camel_case() {
        let tenant = Tenant::new("Test".to_string(), "test".to_string()).unwrap();
        let json = serde_json::to_value(&tenant).unwrap();
        assert!(json.get("id").is_some());
        assert_eq!(json["name"], "Test");
        assert_eq!(json["slug"], "test");
        assert_eq!(json["isActive"], true);
    }
}
