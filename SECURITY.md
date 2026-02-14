# Security Policy

> 本策略适用于 Nexis 的分层安全模型：`Baseline Profile` 与 `Enterprise Profile`。

## 1. Profile 分类

### [Baseline] 开源自托管
- 适用：单组织 / 中小规模部署
- 特点：不依赖第三方云安全服务，强调可落地、低耦合、自动化扫描
- 基准文档：`docs/security/baseline.md`

### [Enterprise] 企业私有化
- 适用：多租户、强审计、合规驱动部署
- 特点：租户隔离、不可变审计、审批治理、SOC2/ISO27001 证据管理
- 基准文档：`docs/security/enterprise.md`

## 2. Vulnerability Reporting

**禁止通过公开 issue 报告安全漏洞。**

请通过以下渠道私下提交：
- Email: `security@nexis.ai`
- GitHub Security Advisory: <https://github.com/schorsch888/Nexis/security/advisories/new>

响应 SLA（工作日）：
- 24 小时内确认收到
- 72 小时内给出分级和初步缓解建议

## 3. Supported Versions

| Version | Supported |
| --- | --- |
| 0.x | Yes |

## 4. Security Controls

### [Baseline] 必要控制
- 认证：OIDC/JWT（RS256）或等价机制
- 授权：最小权限 RBAC，默认拒绝
- 传输：TLS 1.2+（建议 1.3）
- 密钥：环境变量或自建密钥库，禁止硬编码
- 扫描：`gitleaks` + `cargo audit` + `trivy`
- 日志：审计日志与应用日志分离

### [Enterprise] 增强控制
- 认证：企业 SSO + MFA + 条件访问
- 授权：RBAC + ABAC + tenant 强制策略
- 数据：租户级隔离与关键数据分级加密
- 审计：不可变存储 + SIEM 汇聚 + 长周期保留
- 供应链：SBOM、制品签名、发布审批 Gate
- 合规：SOC2/ISO27001 控制项与证据映射

## 5. Implementation Steps

1. 采用 `.pre-commit-config.yaml` 启用本地安全门禁。
2. 采用 `.github/workflows/security.yml` 启用 PR 安全扫描。
3. 依据 `docs/security/baseline.md` 完成 Baseline 落地。
4. 企业部署按 `docs/security/enterprise.md` 启用增强控制。
5. 每次发布前完成安全检查清单并存档证据。

## 6. Configuration Examples

### Baseline 示例
```env
NEXIS_PROFILE=baseline
NEXIS_DEFAULT_DENY=true
NEXIS_AUDIT_LOG_ENABLED=true
```

### Enterprise 示例
```env
NEXIS_PROFILE=enterprise
NEXIS_MULTI_TENANT_ENABLED=true
NEXIS_AUDIT_IMMUTABLE_STORAGE=true
NEXIS_REQUIRE_MFA=true
```

## 7. Incident Response

### 分级
- `Critical`：远程未授权访问、密钥泄露、跨租户数据泄露
- `High`：权限提升、认证绕过、可利用高危依赖漏洞
- `Medium/Low`：需结合上下文评估

### 响应流程
1. 受理并确认（分配事件 ID）
2. 快速分级（CVSS + 业务影响）
3. 遏制（隔离、禁用密钥、回滚）
4. 修复与验证
5. 发布公告与复盘（含时间线与改进项）

## 8. Security Checklist

### [Baseline] 发布前
- [ ] pre-commit 全部通过
- [ ] CI `Security` 工作流通过
- [ ] 无未豁免高危漏洞
- [ ] 密钥与证书在有效轮转周期内

### [Enterprise] 发布前
- [ ] 多租户隔离策略审计通过
- [ ] 审计日志不可变策略有效
- [ ] 高风险操作审批链完整
- [ ] 合规证据（SBOM/扫描/审计记录）已归档

## 9. Safe Harbor

只要你本着善意、避免数据破坏、并遵守法律法规进行安全研究，Nexis 将把你的研究视为负责任披露，并与你合作完成修复。
