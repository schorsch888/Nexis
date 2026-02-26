# A2A (Agent-to-Agent) 协议设计

## 1. 协议概述

### 1.1 目标
设计一套企业级 Agent 协作协议，能力对标 Google A2A、OpenAI Swarm、Anthropic MCP、Amazon Bedrock Agent，支持跨团队、跨运行时、跨部署域的 Agent 互联与协同。

### 1.2 设计原则
- 标准化: 统一身份、能力、消息、任务与安全语义。
- 可发现: Agent 能被注册、检索、筛选、健康检查。
- 可组合: 支持点对点、编排式、群体协作三种工作模式。
- 可治理: 内建审计、租户隔离、权限边界、策略执行。
- 向后兼容: 协议版本化，支持能力协商和渐进升级。

### 1.3 对标映射
- Google A2A: 参考其 Agent 间标准通信与互操作目标。
- OpenAI Swarm: 参考其“handoff/角色分工/任务移交”协作模式。
- Anthropic MCP: 参考其工具与上下文标准化暴露方式。
- Amazon Bedrock Agent: 参考其企业编排、可观测、安全治理和集成能力。

### 1.4 协议分层
- 传输层: HTTP/2 + JSON（必选），WebSocket/gRPC（可选扩展）。
- 协议层: 能力声明、发现注册、消息交换、任务编排、上下文同步。
- 治理层: 认证鉴权、策略、审计、配额、SLA。

## 2. 核心概念

### 2.1 Agent 身份与能力声明
- `agent_id`: 全局唯一标识（推荐 URI 风格，如 `agent://org/team/name`）。
- `identity`: 所属组织、环境、版本、签名公钥、信任域。
- `capabilities`: 结构化能力列表，包含输入/输出模式、工具依赖、SLA、成本档位。
- `policies`: 可执行策略约束（数据域、可调用对象、时限、预算）。

### 2.2 Agent 发现与注册
- Registry 支持主动注册、心跳续租、被动发现、标签检索。
- 支持多维过滤: 能力标签、版本、地理域、延迟、成功率、成本。
- 支持健康状态: `healthy/degraded/unavailable`。

### 2.3 点对点通信
- 基础通信单元为 `Envelope`（统一元数据）+ `Payload`（业务语义）。
- 通信语义支持: 请求-响应、异步事件、流式分片、确认回执。
- 每条消息必须具备可追踪 ID 与幂等键。

### 2.4 多 Agent 协作
- 协作角色: `orchestrator`（编排者）、`specialist`（专家）、`executor`（执行者）、`reviewer`（校验者）。
- 协作模式:
  - Handoff: 当前 Agent 将任务与上下文移交下一 Agent。
  - Fan-out/Fan-in: 并行子任务执行后聚合。
  - Negotiation: 多 Agent 基于约束协商最优执行路径。

### 2.5 上下文共享
- 上下文分层: `conversation`（会话级）、`task`（任务级）、`working_memory`（临时态）。
- 上下文通过引用传递优先，避免大对象全量复制。
- 支持可见性级别: `private/shared/redacted`。

### 2.6 任务编排
- 任务生命周期: `created -> planned -> running -> blocked -> completed/failed/cancelled`。
- 支持 DAG 编排、重试策略、超时、补偿动作、人工审批节点。
- 任务结果包含结构化产物、证据链、质量评分。

## 3. 消息格式

### 3.1 统一 Envelope
必填字段:
- `protocol_version`: 协议版本（如 `a2a.v1`）。
- `message_id`: 全局唯一消息 ID。
- `correlation_id`: 跨消息链路追踪 ID。
- `idempotency_key`: 幂等键。
- `timestamp`: RFC3339 时间戳。
- `sender`: 发送方 `agent_id`。
- `receiver`: 接收方 `agent_id` 或逻辑组。
- `message_type`: 消息类型。
- `auth_context`: 签名摘要、令牌声明、租户标识。

### 3.2 消息类型
- `CAPABILITY_ANNOUNCE`: 能力声明/更新。
- `DISCOVERY_QUERY` / `DISCOVERY_RESULT`: 发现请求与响应。
- `TASK_CREATE` / `TASK_UPDATE` / `TASK_CANCEL` / `TASK_RESULT`。
- `HANDOFF_REQUEST` / `HANDOFF_ACCEPT` / `HANDOFF_REJECT`。
- `CONTEXT_SYNC` / `CONTEXT_PATCH`。
- `EVENT_EMIT`: 异步事件广播。
- `ERROR`: 结构化错误。

### 3.3 Payload 约定
- `schema_ref`: 指向 JSON Schema 或契约版本。
- `content`: 业务内容。
- `attachments`: 外部对象引用（对象存储 URI、知识库文档 ID）。
- `constraints`: 时间、预算、合规、数据边界。

### 3.4 错误模型
- `error_code`: 稳定错误码（如 `AUTH_DENIED`、`CONTEXT_TOO_LARGE`）。
- `error_class`: `transient` | `permanent` | `policy`。
- `retry_hint`: 重试建议（间隔、最大次数）。
- `diagnostics`: 最小必要诊断信息（禁止泄露敏感数据）。

## 4. API 设计

### 4.1 Registry API
- `POST /v1/agents/register`: Agent 注册。
- `POST /v1/agents/heartbeat`: 心跳与健康上报。
- `POST /v1/agents/unregister`: 下线。
- `GET /v1/agents/discover`: 按能力与策略筛选 Agent。
- `GET /v1/agents/{agent_id}`: 查询 Agent 元数据。

### 4.2 Messaging API
- `POST /v1/messages/send`: 同步发送消息。
- `POST /v1/messages/publish`: 异步发布事件。
- `GET /v1/messages/stream`: 流式订阅消息。
- `POST /v1/messages/ack`: 消息确认。

### 4.3 Orchestration API
- `POST /v1/tasks`: 创建任务（支持 DAG 描述）。
- `GET /v1/tasks/{task_id}`: 查询状态。
- `POST /v1/tasks/{task_id}/control`: 暂停/恢复/取消。
- `POST /v1/tasks/{task_id}/handoff`: 任务移交。
- `GET /v1/tasks/{task_id}/trace`: 获取执行链路与证据。

### 4.4 Context API
- `POST /v1/context/snapshot`: 创建上下文快照。
- `POST /v1/context/patch`: 增量更新上下文。
- `GET /v1/context/{context_id}`: 按权限读取上下文。
- `POST /v1/context/share`: 以策略控制方式共享上下文。

### 4.5 版本与兼容
- Header: `A2A-Version`, `A2A-Capabilities`。
- 不兼容变更通过主版本升级；可选字段新增保持向后兼容。
- 注册与握手阶段完成能力协商，不支持能力必须显式拒绝并返回替代建议。

## 5. 数据模型

### 5.1 AgentProfile
- 主键: `agent_id`。
- 字段: `name`, `org`, `environment`, `version`, `endpoint`, `public_key`, `capabilities[]`, `policy_refs[]`, `sla`, `cost_profile`, `status`, `last_seen_at`。

### 5.2 Capability
- 字段: `capability_id`, `name`, `input_schema_ref`, `output_schema_ref`, `tool_requirements[]`, `latency_p50/p95`, `throughput`, `quality_score`, `compliance_tags[]`。

### 5.3 Task
- 字段: `task_id`, `parent_task_id`, `objective`, `plan_graph`, `owner_agent_id`, `assignee_agent_id`, `state`, `priority`, `deadline`, `budget`, `constraints`, `result_ref`, `created_at`, `updated_at`。

### 5.4 ContextObject
- 字段: `context_id`, `scope`, `visibility`, `encryption_class`, `token_count`, `summary_ref`, `artifact_refs[]`, `retention_policy`, `lineage`。

### 5.5 MessageRecord
- 字段: `message_id`, `correlation_id`, `from`, `to`, `type`, `payload_hash`, `signature`, `delivery_state`, `retry_count`, `occurred_at`, `acked_at`。

### 5.6 AuditEvent
- 字段: `event_id`, `actor`, `action`, `resource`, `decision`, `policy_id`, `reason`, `timestamp`, `trace_id`。

## 6. 安全考虑

### 6.1 身份认证与信任
- 使用 mTLS + 短期令牌（JWT/OAuth2）双重认证。
- Agent 注册必须携带可验证签名与证书链。
- 支持租户级信任域与跨域信任桥接。

### 6.2 授权与最小权限
- 基于策略引擎执行 ABAC/RBAC 混合授权。
- 权限粒度覆盖: API、能力、上下文对象、工具调用。
- 默认拒绝，按需授权，带有效期。

### 6.3 数据安全与隐私
- 传输全链路加密，敏感字段应用层加密。
- Context 共享支持脱敏与最小披露。
- 支持数据驻留与合规标签（如 PII/金融/医疗）。

### 6.4 运行安全
- 防重放: 时间窗 + nonce + 幂等键。
- 防滥用: 限流、配额、预算阈值、策略熔断。
- 沙箱执行: 外部工具调用需隔离与审计。

### 6.5 可审计与可追溯
- 所有关键动作产生日志与不可篡改审计事件。
- 统一 `trace_id` 贯穿 Agent、任务、消息、上下文。
- 支持事后取证与责任归因。

## 7. 实现路线图

### 阶段 0: 协议基线（2-3 周）
- 定稿 `a2a.v1` 消息 Envelope、错误码、核心数据模型。
- 完成 Registry + Messaging 最小闭环（注册、发现、发送、回执）。
- 交付兼容性测试样例与 conformance checklist。

### 阶段 1: 协作能力（3-4 周）
- 增加任务模型、handoff、并行协作（fan-out/fan-in）。
- 增加 Context 快照/补丁与可见性控制。
- 建立链路追踪与基础 SLA 指标。

### 阶段 2: 企业治理（3-4 周）
- 接入统一策略引擎、审计中心、密钥管理。
- 增加预算/配额/风控策略与异常处置流程。
- 建立多租户隔离与跨域信任管理。

### 阶段 3: 生态互操作（4-6 周）
- 提供 MCP 适配层（工具与上下文桥接）。
- 提供 Swarm 风格编排接口（handoff/role routing）。
- 提供云厂商集成适配（Bedrock/其他托管 Agent 平台）。

### 验收标准
- 互操作: 至少 3 类不同 Agent 运行时可互通。
- 可靠性: 消息送达成功率 >= 99.9%，幂等一致性可验证。
- 安全性: 关键 API 全量鉴权与审计覆盖率 100%。
- 可运维: 可观测指标、告警、回放与追踪链路完整。

