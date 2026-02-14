# Nexis 架构设计（Split Deployment: Control Plane + Agent Runtime）

**日期：** 2026-02-14  
**决策基线：** 方案 B（Control Plane + Agent Runtime 拆分部署）  
**核心目标：** AI 协作执行 + 完整通讯能力（人-人、人-AI、AI-AI）

---

## 0. 设计原则与边界

1. **执行优先**：优先保证任务执行闭环与交互体验，拆分不牺牲交付速度。
2. **AI 一等公民**：AI 与人类统一身份、统一消息协议、统一权限框架。
3. **AI 负责人级权限**：AI 默认具备负责人级操作能力，但受策略引擎和审计约束。
4. **控制面与执行面分离**：Control Plane 负责治理与编排，Agent Runtime 负责 AI 执行与工具调用。
5. **协议先行**：统一交互模型 + 统一 AI 网关协议 + 控制面/执行面契约。

---

## 1. 系统架构图

### 1.1 逻辑架构（双组件）

```mermaid
flowchart LR
    subgraph Clients[Clients]
      H[Human Client\nWeb/CLI]
      A[AI Agent Client\nMCP-compatible]
    end

    H -->|WS/HTTP| CP
    A -->|MCP/HTTP| CP

    subgraph CPN[Control Plane]
      CP[Unified Gateway\nAuth + Routing + Session]
      IM[Interaction Engine\nConversation/Thread/State]
      PE[Policy Engine\nRBAC+ABAC+Guardrails]
      MSG[Messaging Core\nRoom/Presence/Delivery]
      AUD[Observability & Audit]
      SCHED[Task Scheduler & Dispatch]
    end

    subgraph ARN[Agent Runtime]
      OR[AI Orchestrator\nCollaboration Modes]
      AGW[AI Gateway\nProvider Adapters]
      TOOL[Tool Runner\nSandbox/Connector]
      KB[Semantic Backbone\nVector + Graph + Memory]
    end

    CP --> IM
    CP --> MSG
    IM --> SCHED
    SCHED -->|Dispatch API| OR
    OR --> AGW
    OR --> TOOL
    OR --> KB
    OR -->|Status/Events| SCHED
    PE --> SCHED
    MSG --> KB

    SCHED --> AUD
    OR --> AUD
    TOOL --> AUD

    AGW -->|Adapter Protocol| EXT

    subgraph EXT[External AI Providers]
      OAI[OpenAI]
      ANT[Anthropic]
      GEM[Gemini]
      LOC[Local Models]
    end
```

### 1.2 部署架构（独立部署 + API 通信）

```mermaid
flowchart TB
    subgraph Edge[Ingress]
      GW[API Gateway / LB]
    end

    subgraph CPCluster[Control Plane Cluster]
      CPAPI[Control Plane API\nAuth/Routing/Policy]
      CPMSG[Messaging + Interaction]
      CPSCH[Scheduler + Dispatch]
      CPAUD[Audit/Observability]
      CPDB[(PostgreSQL-CP)]
      CPCA[(Redis-CP)]
    end

    subgraph ARCluster[Agent Runtime Cluster]
      ARAPI[Runtime API\nTask/Tool/Callback]
      ARO[Orchestrator Workers]
      ARGW[AI Gateway + Adapters]
      ARSEM[Semantic Service]
      ARDB[(PostgreSQL-AR)]
      ARV[(Vector DB)]
      ARCA[(Redis-AR Queue/Cache)]
    end

    GW --> CPAPI
    CPAPI --> CPMSG
    CPAPI --> CPSCH
    CPSCH --> CPDB
    CPMSG --> CPDB
    CPAPI --> CPCA
    CPSCH --> CPCA

    CPSCH <-->|mTLS HTTPS + Signed Events| ARAPI
    ARAPI --> ARO
    ARO --> ARGW
    ARO --> ARSEM
    ARSEM --> ARV
    ARO --> ARDB
    ARAPI --> ARCA

    ARGW --> EXT
```

### 1.3 容量与扩缩边界

- **Control Plane 扩容触发**：并发连接与消息扇出增长（> 5k 并发连接持续 2 周）。
- **Agent Runtime 扩容触发**：AI 调用复杂度增长（> 4 provider，> 30 AI 实例）。
- **语义组件扩容触发**：检索延迟或语义任务占用 > 30% CPU 时间。

### 1.4 Control Plane 与 Agent Runtime 职责划分

**Control Plane（治理与调度中枢）**
- 接入层：统一认证鉴权（OIDC/JWT、API Key）、会话路由、连接管理。
- 交互层：Room/Thread/Message 生命周期管理，回执与在线状态。
- 权限层：RBAC + ABAC 策略判定、风险评分、审批门控。
- 调度层：任务创建、优先级队列、运行实例分派、重试与取消。
- 审计层：策略决策日志、操作审计、trace 聚合与检索。

**Agent Runtime（执行与智能中枢）**
- 执行层：接收任务租约（lease），执行 AI 协作工作流（parallel/sequential/debate/vote）。
- AI 网关层：Provider 适配、模型路由、fallback、流式输出。
- 工具层：工具调用、连接器访问、沙箱执行、结果标准化。
- 语义层：向量索引、语义召回、上下文组装。
- 结果层：状态回调、增量事件回传、成本/质量指标上报。

**职责边界原则**
- Control Plane 不直接调用外部模型和工具。
- Agent Runtime 不做最终权限裁决与审批决策。
- 用户可见状态以 Control Plane 为准，执行细节以 Agent Runtime 上报为准。

---

## 2. 核心模块设计

### 2.1 Interaction Model（AI 原生交互范式）

定义统一的 `Interaction` 作为顶层对象：

```text
Interaction
├── Context (workspace/room/thread/task)
├── Participants (human/ai/agent/system)
├── Intent (question/execute/review/debate/vote)
├── Messages (event stream)
├── Artifacts (docs/code/links/results)
└── State (running/waiting/completed/blocked)
```

关键能力：
- **执行型对话**：每次会话可绑定任务状态与可执行动作。
- **多 AI 协作模式**：`parallel` / `sequential` / `debate` / `vote`。
- **结果可追溯**：消息、工具调用、决策链可审计。

### 2.2 Messaging Core

- 房间与线程模型：`Room -> Thread -> Message`。
- 实时传输：WebSocket 双向推送，支持流式 chunk。
- 投递语义：`sent -> delivered -> read -> acknowledged`。
- 可恢复会话：断线重连后按 `cursor` 回放事件。

### 2.3 AI Orchestrator

- 调度策略：按任务类型、成本预算、能力标签选择 AI。
- 协作编排：支持单 AI、并行多 AI、辩论与投票聚合。
- 失败回退：超时、限流、质量不足时自动 fallback。

### 2.4 Semantic Backbone（语义化知识中枢）

- **统一语义入口**：消息、文档、任务、代码产出统一向量化。
- **双存储策略**：关系存储（事实）+ 向量存储（语义）+ 图关系（关联）。
- **上下文组装**：按 `intent + role + policy` 动态构建 AI 输入上下文。

### 2.5 Commercial & Ecosystem Layer（商业化与生态）

- 计量：token、调用次数、协作任务耗时。
- 计费：workspace/seat/model-tier 分层。
- 生态：Provider 插件、Agent 模板、第三方工具市场（后续阶段）。

---

## 3. 通讯协议设计（人-人 / 人-AI / AI-AI）

### 3.1 统一消息信封（Envelope）

```json
{
  "version": "nmp/1.0",
  "messageId": "msg_01J...",
  "interactionId": "int_01J...",
  "roomId": "room_general",
  "threadId": "th_01J...",
  "sender": "nexis:human:alice@example.com",
  "receiver": ["nexis:ai:openai/gpt-4.1"],
  "mode": "human_ai",
  "intent": "execute",
  "content": {
    "type": "markdown",
    "text": "请给出发布方案并执行检查项"
  },
  "policy": {
    "classification": "internal",
    "requiresApproval": false
  },
  "trace": {
    "traceId": "tr_01J...",
    "parentMessageId": null
  },
  "timestamp": "2026-02-14T00:00:00Z"
}
```

### 3.2 人与人（Human-Human）

- 模式：`mode=human_human`
- 语义：即时沟通、任务协同、线程讨论。
- 保障：回执、已读、编辑历史、审计日志。

事件流：
1. Client 发送 `MESSAGE_CREATE`
2. Gateway 鉴权 + 房间权限校验
3. Messaging Core 持久化并广播
4. 订阅者收到 `MESSAGE_DELIVERED` / `MESSAGE_READ`

### 3.3 人与 AI（Human-AI）

- 模式：`mode=human_ai`
- 扩展字段：`toolPolicy`、`budget`、`expectedOutputSchema`
- 支持：流式响应、工具调用、结构化输出。

事件流：
1. 用户发送执行请求（intent: `execute`）
2. Orchestrator 选择 AI + 组装上下文
3. AI Gateway 发起 provider 调用并流式回传 chunk
4. 完成后回写 `usage/cost/quality` 指标

### 3.4 AI 与 AI（AI-AI）

- 模式：`mode=ai_ai`
- 约束：默认 `internal=true`，可配置是否对人类可见。
- 协作语义：质检、辩论、投票、角色分工。

协作模式定义：
- `parallel`: 多 AI 同步输出，聚合器合并。
- `sequential`: 上一个 AI 产出作为下一个输入。
- `debate`: 至少 2 轮反驳，最终裁决模型输出结论。
- `vote`: 多模型打分，按权重投票。

---

## 4. AI 接入网关设计（AI Gateway & Protocol）

### 4.1 网关职责

1. **统一接入**：屏蔽 provider 差异，暴露一致 API。
2. **协议转换**：Nexis 协议 <-> Provider/MCP 协议。
3. **能力发现**：模型能力、上下文窗口、工具支持动态注册。
4. **策略执行**：限流、预算、内容安全、降级回退。
5. **可观测性**：请求链路、token、成本、错误码标准化。

### 4.2 Gateway 内部组件

```text
AI Gateway
├── Provider Registry
├── Capability Catalog
├── Request Normalizer
├── Policy & Guardrails
├── Routing & Fallback Engine
├── Streaming Multiplexer
└── Usage/Billing Reporter
```

### 4.3 统一调用接口（建议）

- `POST /v1/ai/execute`：同步/异步执行入口
- `POST /v1/ai/stream`：SSE/WS 流式入口
- `POST /v1/ai/collaboration`：AI-AI 协作入口
- `GET /v1/ai/models`：能力与可用性发现

### 4.4 Provider 适配协议（抽象）

```json
{
  "provider": "openai",
  "model": "gpt-4.1",
  "capabilities": ["text", "code", "tool_call", "streaming"],
  "limits": {
    "contextWindow": 128000,
    "rpm": 120,
    "tpm": 200000
  },
  "health": "healthy"
}
```

### 4.5 失败策略

- 429/5xx：自动 fallback 到同能力模型。
- 超时：截断上下文重试 1 次。
- 工具调用失败：回退纯文本策略并记录错误。

### 4.6 Control Plane <-> Agent Runtime 通信协议

**传输与安全**
- 协议：`HTTPS (REST) + Webhook Callback + 事件总线（可选）`。
- 双向认证：mTLS（服务身份证书）+ HMAC 签名（消息级）。
- 重放防护：`timestamp + nonce + signature`，默认 5 分钟有效期。

**核心 API（Control Plane -> Agent Runtime）**
- `POST /runtime/v1/tasks/dispatch`：下发任务（含 lease、priority、budget、toolPolicy）。
- `POST /runtime/v1/tasks/{taskId}/cancel`：取消任务。
- `GET /runtime/v1/tasks/{taskId}`：查询执行状态。

**核心 Callback（Agent Runtime -> Control Plane）**
- `POST /control/v1/runtime/events`：回传状态与增量结果。
- 事件类型：`TASK_ACCEPTED | TASK_PROGRESS | TOOL_CALL | TASK_COMPLETED | TASK_FAILED | TASK_TIMEOUT`。

**消息契约（示例）**
```json
{
  "eventId": "evt_01J...",
  "taskId": "task_01J...",
  "interactionId": "int_01J...",
  "eventType": "TASK_PROGRESS",
  "status": "running",
  "payload": {
    "chunk": "正在执行依赖检查...",
    "usage": {"inputTokens": 1200, "outputTokens": 80}
  },
  "trace": {"traceId": "tr_01J...", "spanId": "sp_01J..."},
  "idempotencyKey": "task_01J...:17",
  "emittedAt": "2026-02-14T00:00:00Z",
  "signature": "sha256=..."
}
```

**一致性与幂等**
- 交付语义：至少一次（at-least-once），Control Plane 基于 `idempotencyKey` 去重。
- 状态机约束：`queued -> accepted -> running -> completed|failed|timeout|canceled`。
- 超时恢复：Control Plane 在 lease 过期后可重派；Agent Runtime 必须上报最终终态。

---

## 5. 权限与安全机制

### 5.1 身份与权限模型

基于 NIP-001，统一四类主体：`human` / `ai` / `agent` / `system`。

权限分层：
1. **Platform Role**（平台）
2. **Workspace Role**（工作空间）
3. **Room Role**（房间）
4. **Action Policy**（操作级）

### 5.2 角色设计（含 AI 负责人）

- `owner`：全量管理权限（可包含受信 AI）。
- `lead`：负责人级（本项目 AI 默认角色）。
- `member`：普通执行权限。
- `observer`：只读。

AI 默认 `lead`，但以下动作需二次策略：
- 删除资源
- 对外发送敏感信息
- 高成本调用（超预算阈值）

### 5.3 安全控制

- **认证**：Human(OIDC/JWT)、AI(API Key + 签名/MCP Auth)、Service(mTLS)。
- **授权**：RBAC + ABAC（上下文属性 + 风险评分）。
- **数据分级**：`public/internal/confidential/restricted`。
- **加密**：传输 TLS1.3，静态 AES-256（由 KMS 管理密钥）。
- **审计**：所有 AI 执行动作必须可追踪到 `traceId` 与策略决策记录。

### 5.4 安全事件与响应

- 实时检测：异常调用频率、越权访问、敏感数据外泄。
- 自动处置：降权、熔断、隔离 AI 实例、告警通知。
- 取证保留：关键日志保留 180 天（MVP 可先 30 天）。

---

## 6. MVP 实现路径

### 6.1 目标定义（8-10 周）

- 可运行的拆分系统：Control Plane + Agent Runtime 独立部署。
- 支持三种通讯：人-人、人-AI、AI-AI。
- 支持 AI 负责人级协作执行（含策略护栏）。
- 接入 2 家以上模型 Provider，具备 fallback。

### 6.2 里程碑

**M1（第 1-2 周）：Control Plane 基础闭环**
- 完成统一身份、房间、消息、线程、回执。
- 建立调度器与任务状态机（仅 mock Runtime）。

**M2（第 3-4 周）：Agent Runtime 执行闭环**
- 实现 Runtime Task API、Orchestrator、Tool Runner 基础能力。
- 打通 Control Plane -> Runtime dispatch 与 Runtime -> Control Plane callback。

**M3（第 5-6 周）：AI 网关与人-AI执行**
- 实现 Provider Registry、统一调用接口、流式输出。
- 接入至少 2 个 AI Provider，支持 fallback。

**M4（第 7-8 周）：AI-AI 协作与语义中枢 MVP**
- 实现 `parallel/debate/vote` 三种协作模式。
- 建立最小语义检索（向量索引 + 上下文组装）。

**M5（第 9-10 周）：安全/计量/可观测收敛**
- 权限护栏、预算限制、审计日志全链路贯通。
- token/cost 指标与看板，补齐重试、超时、幂等等可靠性机制。

### 6.3 MVP 验收标准

1. 三种通讯模式端到端可演示（含流式）。
2. AI 可在负责人权限下执行任务并被审计追踪。
3. 任一 Provider 故障时可自动回退。
4. 关键接口 p95 < 300ms（不含模型推理时延）。
5. Control Plane 与 Agent Runtime 断连/重连场景下，任务状态最终一致。

### 6.4 技术债与后续拆分

- 技术债：跨组件领域模型仍存在一定重复定义。
- 后续演进：按领域继续细分为 `Messaging`、`AI Gateway`、`Semantic` 独立服务。
- 演进原则：先稳固契约与状态机，再做存储与流量拆分。

---

## 附录 A：关键接口契约（Split-Ready）

- `InteractionService`：创建/推进/终止交互生命周期。
- `MessagingService`：消息投递、回执、订阅。
- `AIGatewayService`：模型发现、执行、流式回传、回退。
- `PolicyService`：权限与风险判定。
- `SemanticService`：索引写入、召回、上下文拼装。

这些接口在拆分架构下以跨服务 API/事件契约存在，并保持可版本化演进。

## 附录 B：商业化最小闭环

- Workspace 套餐：基础版（单 AI）/ 团队版（多 AI 协作）/ 企业版（私有模型接入）。
- 计费维度：seat + token + 高级协作模式。
- 生态入口：Provider Adapter SDK + Agent Template Registry。
