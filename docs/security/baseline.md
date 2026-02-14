# Baseline Profile（开源自托管）

> 标签：`Baseline`  
> 场景：单组织 / 中小规模 / 不依赖第三方云安全服务

## 1. 安全目标与边界

- 单租户部署，服务默认运行在私有网络。
- 保护对象：源代码、配置、数据库、消息流、审计日志。
- 威胁重点：密钥泄露、依赖漏洞、未授权访问、供应链污染。

## 2. 实施步骤

### 步骤 1：身份与访问控制
1. 使用 OIDC/JWT（RS256）或自建 IdP。
2. 定义最小权限角色（`admin`/`operator`/`viewer`）。
3. 默认拒绝策略：未匹配规则即拒绝。

### 步骤 2：机密与配置管理
1. 所有凭证仅通过环境变量注入，严禁硬编码。
2. 使用自托管密钥库（如 Vault）或加密配置文件。
3. 设定轮转周期：API Key 90 天、证书 365 天。

### 步骤 3：传输与网络防护
1. 外部入口强制 TLS 1.2+（建议 TLS 1.3）。
2. 仅暴露必需端口；数据库和内部服务不暴露公网。
3. 服务间启用 mTLS（可分阶段推进）。

### 步骤 4：日志与可观测
1. 结构化日志（JSON）统一字段：时间、主体、动作、结果、资源。
2. 审计日志与应用日志分离存储。
3. 设置异常告警：认证失败激增、权限拒绝激增、关键表异常访问。

### 步骤 5：开发与发布安全
1. 本地启用 pre-commit（secret/sast/格式检查）。
2. CI 启用依赖漏洞扫描、secret 扫描、镜像扫描。
3. 发布前执行最小安全回归测试清单。

## 3. 配置示例

### 3.1 访问控制策略示例
```yaml
profile: baseline
default_effect: deny
roles:
  admin:
    - resource: "*"
      actions: ["*"]
  operator:
    - resource: "project/*"
      actions: ["read", "deploy", "rollback"]
  viewer:
    - resource: "project/*"
      actions: ["read"]
```

### 3.2 日志字段约定示例
```json
{
  "ts": "2026-02-14T10:00:00Z",
  "actor": "user:alice",
  "tenant": "default",
  "action": "project.deploy",
  "resource": "project/api-gateway",
  "result": "allowed",
  "trace_id": "a1b2c3"
}
```

### 3.3 最小化容器运行参数示例
```bash
docker run --read-only \
  --cap-drop=ALL \
  --security-opt=no-new-privileges:true \
  --pids-limit=256 \
  --memory=512m \
  nexis:baseline
```

## 4. 运维检查清单

### [Baseline] 日检
- [ ] 前 24 小时高危告警是否处理完成
- [ ] 密钥泄露扫描是否为 0
- [ ] 审计日志是否完整写入

### [Baseline] 周检
- [ ] 依赖漏洞扫描（含 Rust crates）是否无未豁免高危
- [ ] 权限角色是否存在越权配置
- [ ] 备份恢复抽样验证是否通过

### [Baseline] 发布前
- [ ] 关键环境变量完整且非默认值
- [ ] `security.yml` 全部 job 通过
- [ ] 变更记录包含安全影响评估
