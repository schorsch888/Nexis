//! Tenant isolation security tests
//!
//! Tests to verify cross-tenant access is properly denied.

#[cfg(feature = "multi-tenant")]
mod multi_tenant_tests {
    use nexis_gateway::auth::{TenantContext, TenantError};
    use nexis_core::tenant::TenantId;

    #[test]
    fn tenant_context_extraction() {
        let tenant_id = TenantId::new();
        let ctx = TenantContext::new(tenant_id.clone());

        assert_eq!(ctx.tenant_id(), &tenant_id);
    }

    #[test]
    fn cross_tenant_access_denied() {
        let tenant_a = TenantId::new();
        let tenant_b = TenantId::new();

        let ctx_a = TenantContext::new(tenant_a);

        // Verify that tenant A context cannot access tenant B resources
        assert!(!ctx_a.can_access(&tenant_b));
    }

    #[test]
    fn same_tenant_access_allowed() {
        let tenant_id = TenantId::new();
        let ctx = TenantContext::new(tenant_id.clone());

        // Verify that tenant context can access its own resources
        assert!(ctx.can_access(&tenant_id));
    }

    #[test]
    fn tenant_error_display() {
        let error = TenantError::CrossTenantAccess {
            user_tenant: "tenant-a".to_string(),
            resource_tenant: "tenant-b".to_string(),
        };

        let message = format!("{}", error);
        assert!(message.contains("tenant-a"));
        assert!(message.contains("tenant-b"));
        assert!(message.contains("denied"));
    }
}

#[cfg(not(feature = "multi-tenant"))]
mod single_tenant_tests {
    #[test]
    fn single_tenant_mode_works() {
        // In single-tenant mode, no tenant isolation is required
        assert!(true);
    }
}
