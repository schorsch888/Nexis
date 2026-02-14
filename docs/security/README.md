# Security

> Scope: Nexis self-hosted and enterprise deployments
> Layered strategy: `Baseline Profile` + `Enterprise Profile`

## 1. Layered Positioning

### [Baseline] Open-source self-hosted
- Target: Single organization / Small to medium teams
- Goal: Build runnable, auditable, maintainable security without third-party cloud services
- Principles: Least privilege, deny by default, observable, automated checks

### [Enterprise] Enterprise private deployment
- Target: Multi-tenant, compliance-driven, high audit requirements
- Goal: Meet SOC2 / ISO27001 control requirements with tenant isolation, audit trails, access governance
- Principles: Layered isolation, centralized audit, strong authentication, provable compliance

## 2. Control Domain Mapping

| Control Domain | Baseline | Enterprise |
|----------------|----------|------------|
| Authentication | OIDC/JWT or internal IdP | Enterprise IdP + MFA + conditional access |
| Access Control | RBAC least privilege | RBAC + ABAC + tenant policy engine |
| Secret Management | Local Vault/KMS or env vars | Dedicated HSM/KMS + dual approval + auto rotation |
| Network Security | TLS, internal segmentation | Zero-trust segmentation, mTLS, east-west policies |
| Audit Logs | Structured logs, tamper-proof storage | Immutable audit chain, centralized SIEM, retention policies |
| Vulnerability Management | SAST + dependency audit + secrets scanning | Add image signing, SBOM, baseline diff audit |
| Compliance Governance | Security baseline and incident response | SOC2/ISO27001 control mapping and evidence automation |

## 3. Implementation Order

1. Establish repository security gates: pre-commit + CI security scanning
2. Complete Baseline configuration and pass checklists
3. Introduce multi-tenant isolation and enterprise audit capabilities
4. Map Enterprise controls to SOC2/ISO27001 evidence library

## 4. Key Configuration Files

- Baseline details: `docs/security/baseline.md`
- Enterprise details: `docs/security/enterprise.md`
- Environment template: `.env.example`
- Local commit checks: `.pre-commit-config.yaml`
- CI security scanning: `.github/workflows/security.yml`
- Public security policy: `SECURITY.md`

## 5. Quick Checklist

### [Baseline] Quick Acceptance
- [ ] All secrets injected via environment variables, no plaintext in repo
- [ ] pre-commit enabled with secrets scanning and basic SAST
- [ ] CI security scanning passes (gitleaks, trivy, audit)
- [ ] TLS enabled for all external endpoints
- [ ] Audit logging enabled
