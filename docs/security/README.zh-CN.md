# 安全体系

> 适用范围：Nexis 自托管与企业私有化部署
> 分层策略：`Baseline Profile` + `Enterprise Profile`

## 1. 分层定位

### [Baseline] 开源自托管
- 目标对象：单组织 / 中小规模团队
- 安全目标：在不依赖第三方云安全服务的前提下，建立可运行、可审计、可维护的基础安全体系
- 设计原则：最小权限、默认拒绝、可观测、自动化检查优先

### [Enterprise] 企业私有化
- 目标对象：多租户、合规驱动、审计要求高的企业环境
- 安全目标：满足 SOC2 / ISO27001 控制域落地证据要求，强化租户隔离、审计追踪、访问治理
- 设计原则：分层隔离、集中审计、强认证、可证明合规

## 2. 控制域映射

| 控制域 | Baseline | Enterprise |
|--------|----------|------------|
| 身份认证 | OIDC/JWT 或内部 IdP | 企业 IdP + MFA + 条件访问 |
| 访问控制 | RBAC 最小权限 | RBAC + ABAC + 租户策略引擎 |
| 机密管理 | 本地 Vault/KMS 或环境变量 | 专用 HSM/KMS + 双人审批 + 自动轮转 |
| 网络安全 | TLS、内网分段 | 零信任分段、mTLS 全链路、东西向策略 |
| 审计日志 | 结构化日志、篡改防护存储 | 不可变审计链、集中 SIEM、保留策略 |
| 漏洞管理 | SAST + 依赖审计 + Secrets 扫描 | 增加镜像签名、SBOM、基线差异审计 |
| 合规治理 | 安全基线与事件响应 | SOC2/ISO27001 控制映射与证据自动化 |

## 3. 落地顺序

1. 建立仓库安全闸门：pre-commit + CI 安全扫描
2. 完成 Baseline 配置并通过检查清单
3. 引入多租户隔离与企业级审计能力
4. 将 Enterprise 控制项映射到 SOC2/ISO27001 证据库

## 4. 关键配置文件

- Baseline 细则：`docs/security/baseline.en.md`
- Enterprise 细则：`docs/security/enterprise.en.md`
- 环境变量模板：`.env.example`
- 本地提交检查：`.pre-commit-config.yaml`
- CI 安全扫描：`.github/workflows/security.yml`
- 对外安全政策：`SECURITY.md`

## 5. 快速检查清单

### [Baseline] 快速验收
- [ ] 所有密钥均通过环境变量注入，无明文入库
- [ ] pre-commit 启用 secrets 扫描与基础 SAST
- [ ] CI 安全扫描通过（gitleaks、trivy、audit）
- [ ] 所有外部端点启用 TLS
- [ ] 审计日志已启用
