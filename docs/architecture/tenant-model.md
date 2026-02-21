# Multi-Tenant Domain Model

## Overview

Nexis supports multi-tenancy for enterprise deployments, enabling isolated workspaces within a single deployment.

## Entity Hierarchy

```
Tenant (Organization)
├── Workspace (Team/Project)
│   ├── Members (Users)
│   │   └── Permissions
│   └── Rooms (Channels)
│       └── Messages
└── Settings
```

### Relationships

| Entity | Parent | Description |
|--------|--------|-------------|
| Tenant | - | Top-level organization (e.g., "Acme Corp") |
| Workspace | Tenant | Logical grouping (e.g., "Engineering", "Marketing") |
| Member | Workspace | User or agent with workspace access |
| Room | Workspace | Communication channel |

## ID Specification

All entity IDs use **UUID v7** format for time-ordered uniqueness:

```
Format: urn:uuid:<uuid-v7>
Example: urn:uuid:0194a2b8-7c2d-7d3e-8f4a-5b6c7d8e9f0a
```

### Why UUID v7?

- **Time-ordered**: Naturally sortable by creation time
- **Distributed**: No coordination needed for ID generation
- **Database-friendly**: Better index locality vs UUID v4
- **Standard**: RFC 9562 compliant

### ID Types

| Type | Prefix | Example |
|------|--------|---------|
| `TenantId` | - | `0194a2b8-7c2d-7d3e-8f4a-5b6c7d8e9f0a` |
| `WorkspaceId` | - | `0194a2b9-1a2b-3c4d-5e6f-7a8b9c0d1e2f` |
| `MemberId` | `nexis:` | `nexis:human:alice@acme.com` |
| `RoomId` | - | `0194a2ba-2b3c-4d5e-6f7a-8b9c0d1e2f3a` |

## Cross-Module Entity Mapping

### Tenant Context Flow

```
HTTP Request → Middleware → Tenant Resolution → Context Injection → Handler
```

1. **Tenant Resolution**: Extract tenant from subdomain, header, or path
2. **Context Injection**: Set tenant context for request lifetime
3. **Query Scoping**: All DB queries automatically scoped to tenant

### Module Integration

```
┌─────────────────────────────────────────────────────────────┐
│                         API Layer                            │
├─────────────────────────────────────────────────────────────┤
│                    nexis-core::tenant                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  TenantId, Tenant, TenantError                       │   │
│  └─────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  nexis-db (tenant-scoped queries)                           │
│  nexis-api (tenant-aware handlers)                          │
│  nexis-protocol (MemberId already supports tenancy)         │
└─────────────────────────────────────────────────────────────┘
```

### Feature Flag

Multi-tenant functionality is gated behind the `multi-tenant` feature flag:

```toml
# Cargo.toml
[features]
multi-tenant = []
```

```rust
// Conditional compilation
#[cfg(feature = "multi-tenant")]
use nexis_core::tenant::{TenantId, Tenant};
```

## Tenant Model

```rust
pub struct Tenant {
    pub id: TenantId,      // UUID v7
    pub name: String,      // Display name: "Acme Corporation"
    pub slug: String,      // URL-safe: "acme-corp"
    pub is_active: bool,   // Soft delete support
}
```

### Slug Validation

- Lowercase alphanumeric characters
- Hyphens allowed as separators
- No leading/trailing hyphens
- Length: 3-63 characters

## Database Schema (Future)

```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- All tenant-scoped tables include tenant_id
CREATE TABLE workspaces (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    name TEXT NOT NULL,
    -- ...
);
```

## Security Considerations

1. **Tenant Isolation**: Database queries must always include tenant_id filter
2. **Cross-Tenant Access**: Explicit validation required for any cross-tenant operations
3. **Audit Logging**: All tenant operations must be logged
4. **Rate Limiting**: Per-tenant rate limits

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| TenantId | ✅ Done | UUID v7, feature-gated |
| Tenant | ✅ Done | Core struct with validation |
| TenantError | ✅ Done | Error types |
| Database Schema | 📋 Planned | Phase 4 continuation |
| API Middleware | 📋 Planned | Phase 4 continuation |
| Tenant Resolution | 📋 Planned | Phase 4 continuation |
