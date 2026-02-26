# Nexis 核心能力设计

## 0. 目标与范围
- 目标：构建企业级 AI 工作平台四大核心能力，覆盖高频办公场景、Agent 长期记忆、自主问题解决与全链路安全。
- 范围：Web/Mobile 客户端、Gateway、Runtime、Memory 与 Security 控制面。
- 设计原则：高可用（HA）、可观测（Observability）、最小权限（Least Privilege）、默认安全（Secure by Default）、可灰度发布（Progressive Delivery）。

## 1. 工作场景功能设计

### 1.1 总体架构
- 客户端层：`apps/web`、`apps/mobile` 提供统一工作台入口。
- 协作服务层：会议、文档、项目、沟通四类域服务。
- AI 能力层：摘要、写作、提醒、问答、智能回复（由 `nexis-runtime` 统一调度）。
- 数据层：业务库（PostgreSQL）+ 检索索引（OpenSearch）+ 向量库（Qdrant / `nexis-vector`）。
- 事件层：事件总线（NATS/Kafka）驱动异步任务和自动化。

### 1.2 会议场景
#### 功能清单
- 日程安排：日历聚合、冲突检测、空闲时间推荐。
- 会议预约：一键邀请、会议室/链接自动分配。
- 会议纪要自动生成：ASR + Speaker Diarization + Action Item 抽取。
- AI 参会助手：会中问答、议题提醒、会后待办自动入库。

#### 技术方案
- 会中音频流 -> 实时转写服务 -> LLM 结构化总结（决策/风险/待办）。
- 纪要结构化模型：`title, decisions[], action_items[], risks[], owners[], due_dates[]`。
- 与项目系统打通：`action_items` 自动映射为任务。

#### 关键 API
- `POST /api/meetings`
- `POST /api/meetings/{id}/transcript:ingest`
- `POST /api/meetings/{id}/minutes:generate`
- `POST /api/meetings/{id}/assistant:query`

### 1.3 文档场景
#### 功能清单
- 文档创建/编辑：富文本 + Markdown 双模式。
- AI 辅助写作：提纲、扩写、改写、语气调整、术语统一。
- 文档摘要：按角色（管理层/研发/销售）生成差异化摘要。
- 多人协作：CRDT 实时协同、评论、版本回溯。

#### 技术方案
- 协同引擎：Yjs/Automerge + WebSocket。
- 内容智能：段落级 embedding，支持语义检索和引用回链。
- 版本治理：快照 + 增量日志，支持审计追踪。

#### 关键 API
- `POST /api/docs`
- `PATCH /api/docs/{id}`
- `POST /api/docs/{id}/ai:assist`
- `POST /api/docs/{id}/summary`

### 1.4 项目管理场景
#### 功能清单
- 任务创建/分配：模板化任务、优先级、SLA。
- 进度跟踪：燃尽图、关键路径、阻塞状态。
- AI 自动提醒：延期风险预测、依赖冲突预警。
- 报告生成：周报/月报/里程碑复盘自动生成。

#### 技术方案
- 任务状态机：`todo -> in_progress -> blocked -> review -> done`。
- 风险预测：基于历史周期、负责人负载、依赖密度构建规则+模型混合引擎。
- 报告生成：从任务事件流自动汇总 KPI 和异常。

#### 关键 API
- `POST /api/projects/{id}/tasks`
- `PATCH /api/tasks/{id}`
- `POST /api/projects/{id}/reminders:run`
- `GET /api/projects/{id}/reports?period=weekly`

### 1.5 沟通场景
#### 功能清单
- 即时消息：1:1、群组、线程回复。
- 群组讨论：话题标签、可追踪决议。
- AI 智能回复：上下文感知草稿、语气与长度控制。
- 信息检索：跨会话、跨文档、跨任务统一搜索。

#### 技术方案
- 实时通道：WebSocket + 断线重连 + 消息幂等。
- RAG 检索：消息、文档、任务统一索引；Top-K + 重排。
- 回复安全：敏感词与泄密策略先审后发。

#### 关键 API
- `POST /api/rooms/{id}/messages`
- `POST /api/rooms/{id}/ai:reply`
- `GET /api/search?q=...&scope=messages,docs,tasks`

## 2. Agent 长期记忆系统设计

### 2.1 记忆模型
```rust
struct AgentMemory {
    agent_id: AgentId,
    memories: Vec<MemoryEntry>,
    embeddings: VectorStore,
    index: MemoryIndex,
}

struct MemoryEntry {
    id: Uuid,
    content: String,
    memory_type: MemoryType,
    importance: f32,
    created_at: DateTime,
    last_accessed: DateTime,
    embedding: Option<Vec<f32>>,
}
```

### 2.2 记忆类型与策略
- 对话记忆：保留最近窗口 + 高重要片段长期化。
- 事件记忆：里程碑、决策、异常、承诺，默认高优先级。
- 语义记忆：概念、知识图谱节点、术语映射。
- 程序记忆：可复用流程、操作脚本、工具使用偏好。

### 2.3 存储与索引设计
- 热存储：PostgreSQL（元数据、权限、生命周期）。
- 向量存储：Qdrant（语义召回）。
- 倒排索引：OpenSearch（关键词精确检索）。
- 混合检索：`keyword + vector + recency + importance` 融合评分。

### 2.4 记忆操作
- `store`：写入时计算 `importance` 与 `embedding`，并打标签（tenant/user/project）。
- `recall`：多路召回 + 重排，输出引用来源和置信度。
- `forget`：按低重要性、低访问频率和过期策略软删除，再异步硬删除。
- `consolidate`：周期性把碎片记忆合并为摘要记忆，降低上下文 token 成本。

### 2.5 关键接口
- `POST /api/agents/{id}/memory:store`
- `POST /api/agents/{id}/memory:recall`
- `POST /api/agents/{id}/memory:forget`
- `POST /api/agents/{id}/memory:consolidate`

### 2.6 大厂对标实践
- 类 OpenAI Memory：用户可见、可控、可删除。
- 类 Google Vertex AI RAG：检索与生成解耦，支持可替换检索器。
- 类 Microsoft Copilot：租户边界隔离与审计追踪默认开启。

## 3. Agent 自主问题解决设计

### 3.1 目标模型
```rust
struct Goal {
    id: GoalId,
    description: String,
    priority: Priority,
    sub_goals: Vec<Goal>,
    status: GoalStatus,
}

enum GoalStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Failed,
}
```

### 3.2 自主执行引擎（Planner-Executor-Critic）
1. 理解问题：意图识别、约束提取、成功条件建模。
2. 分解目标：生成 DAG 子目标，标注依赖和优先级。
3. 制定计划：为每个子目标绑定工具、预算（token/time/cost）和回退策略。
4. 执行计划：串并行混合执行，实时更新状态。
5. 评估结果：自动验收（规则+LLM Judge+单测/校验脚本）。
6. 调整策略：失败重试、路径改写、必要时升级人工审批。

### 3.3 决策与学习
- 环境感知：读取系统状态、历史记忆、权限上下文。
- 状态评估：计算进度、风险、剩余预算。
- 行动选择：基于策略函数 `argmax(ExpectedUtility)`。
- 结果反馈：写入事件日志和记忆系统。
- 学习改进：从成功轨迹提取“程序记忆”，用于后续 few-shot。

### 3.4 失效保护
- 连续失败阈值触发熔断。
- 高风险动作（删库/外发敏感数据）强制人工确认。
- 超预算自动降级（小模型或只读模式）。

### 3.5 关键接口
- `POST /api/agents/{id}/goals`
- `POST /api/agents/{id}/goals/{goalId}:plan`
- `POST /api/agents/{id}/goals/{goalId}:execute`
- `POST /api/agents/{id}/goals/{goalId}:evaluate`

## 4. 信息安全框架设计

### 4.1 安全基线（Zero Trust）
- 默认不信任：每次请求强认证、强授权、全量审计。
- 多租户隔离：Tenant ID 强制贯穿 API、缓存、索引与日志。
- 策略中心化：统一 PDP（Policy Decision Point）和 PEP（Policy Enforcement Point）。

### 4.2 数据安全
- 端到端加密：传输 TLS 1.3，存储 AES-256（KMS 托管密钥）。
- 敏感数据脱敏：PII/密钥/财务字段写入前分类与掩码。
- 访问控制：RBAC + ABAC 组合；支持字段级权限。
- 审计日志：不可篡改（WORM）+ 哈希链。

### 4.3 Agent 安全
- 权限隔离：Agent 运行沙箱（FS/Network/Process 白名单）。
- 行为监控：工具调用频率、异常参数、越权意图检测。
- 异常检测：规则+统计双引擎，触发自动隔离。
- 安全沙箱：高风险任务在隔离执行池运行。

### 4.4 通信安全
- TLS 双向认证（服务间 mTLS）。
- 身份认证：OIDC/SAML + 短时令牌。
- 消息签名：HMAC/Ed25519，防篡改。
- 防重放：Nonce + Timestamp + 窗口校验。

### 4.5 隐私保护
- 数据最小化：只采集任务所需最小字段。
- 目的限制：数据使用绑定用途标签，越权访问阻断。
- 用户授权：细粒度同意管理与可撤销授权。
- 数据删除：支持按用户/租户一键删除与合规证明。

### 4.6 合规与对标
- 对标标准：ISO 27001、SOC 2 Type II、GDPR、CCPA。
- 大厂同级能力：密钥轮换、审计追踪、最小权限默认开启、分级告警。

## 5. 实施路线图（可执行）

### P0（2-4 周）
- 打通四场景最小闭环：会议纪要、文档摘要、任务提醒、消息智能回复。
- 建立 Memory MVP：`store/recall`。
- 上线安全基线：TLS、RBAC、审计日志。

### P1（4-8 周）
- 记忆系统补齐：`forget/consolidate` + 混合检索。
- 自主执行引擎上线：`plan/execute/evaluate`。
- 风险防护：异常检测 + 人工审批流。

### P2（8-12 周）
- 全场景联动自动化（会议待办 -> 项目任务 -> 沟通提醒）。
- 成本与性能优化：缓存、批处理、模型路由。
- 合规增强：数据主权策略与自动化合规报告。

## 6. 验收指标（SLO/KPI）
- 可用性：核心 API 月可用性 >= 99.9%。
- 性能：AI 辅助写作 P95 < 3s，纪要生成（60min 会议）< 2min。
- 准确性：任务抽取 F1 >= 0.85，检索命中率@5 >= 0.8。
- 安全性：高危漏洞修复 SLA <= 24h，审计覆盖率 100%。
- 运营性：关键链路可观测覆盖率 100%（日志/指标/追踪）。

## 7. 默认假设与边界
- 默认使用 PostgreSQL + Qdrant + OpenSearch 组合。
- 高风险自动化操作必须有人工兜底。
- 涉及外部系统集成（日历/IM/会议）优先采用标准 OAuth2/OIDC。
