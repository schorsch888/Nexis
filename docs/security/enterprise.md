# Enterprise Profile（企业私有化）

> 标签：`Enterprise`  
> 场景：多租户 / 合规驱动（SOC2、ISO27001）/ 高等级审计与隔离

## 1. 安全目标与边界

- 多租户隔离覆盖：身份域、数据域、网络域、缓存域、日志域。
- 合规目标：支持 SOC2（Security/Availability/Confidentiality）和 ISO27001 控制项映射。
- 关键要求：可追踪证据、不可抵赖审计、审批化高风险操作。

## 2. 实施步骤

### 步骤 1：多租户安全模型
1. 租户唯一标识全链路透传（Token、DB、消息、日志）。
2. 策略引擎实施 `tenant_id` 强约束（读写/管理操作）。
3. 缓存与队列按租户分区，禁止跨租户键复用。

### 步骤 2：强化身份治理
1. 接入企业 IdP（OIDC/SAML）并强制 MFA。
2. 高风险操作采用审批流（四眼原则）。
3. 建立 Break Glass 账号，限定时效并全量审计。

### 步骤 3：数据与密钥保护
1. 关键数据分级（P0/P1/P2）并实施差异加密策略。
2. 使用专用 KMS/HSM 托管根密钥。
3. 密钥轮转自动化并保留轮转证据。

### 步骤 4：审计与合规
1. 审计日志写入不可变存储（WORM/对象锁）。
2. 集中汇聚到 SIEM，建立检测规则与处置流程。
3. 维护控制项矩阵：控制目标 -> 技术措施 -> 证据链接。

### 步骤 5：供应链与发布治理
1. 生成 SBOM（SPDX/CycloneDX）。
2. 镜像签名与验签（如 cosign）。
3. 发布需通过安全 Gate：扫描通过 + 审批通过 + 变更单闭环。

## 3. 配置示例

### 3.1 多租户策略示例
```yaml
profile: enterprise
tenant_enforcement: strict
policy:
  deny_cross_tenant: true
  require_tenant_context: true
  break_glass:
    enabled: true
    ttl_minutes: 30
    require_ticket: true
```

### 3.2 合规证据台账示例
```yaml
controls:
  - id: SOC2-CC6.1
    owner: security-team
    evidence:
      - type: ci-artifact
        path: artifacts/security-scan-report.json
      - type: audit-log
        path: logs/immutable/audit-2026-02.ndjson
  - id: ISO27001-A.8.2
    owner: platform-team
    evidence:
      - type: data-classification
        path: docs/compliance/data-classification.md
```

### 3.3 审计日志不可变策略示例
```json
{
  "storage": "object-lock",
  "mode": "compliance",
  "retention_days": 365,
  "legal_hold": true
}
```

## 4. 合规检查清单

### [Enterprise] 日检
- [ ] 关键租户隔离告警为 0（跨租户访问阻断）
- [ ] 高风险操作审批记录完整
- [ ] SIEM 规则命中事件均已分级处置

### [Enterprise] 月检
- [ ] 控制项证据库更新并可追踪
- [ ] Break Glass 演练与回收记录完整
- [ ] 密钥轮转报告与异常处理闭环

### [Enterprise] 审计前
- [ ] SOC2/ISO27001 控制矩阵与实际配置一致
- [ ] 抽样日志可证明操作主体与审批链
- [ ] 发布链路具备 SBOM、签名、扫描报告三联证据
