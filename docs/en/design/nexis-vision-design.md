# Nexis 愿景对齐设计 - AI 原生沟通协作平台

## 1. 愿景陈述

Nexis 的目标是成为对标 Slack + Notion + Zoom + AI 的一体化平台，在同一套协作空间内实现：
- 即时沟通（消息、频道、线程）
- 文档协作（实时编辑、知识沉淀、可追溯版本）
- 音视频会议（WebRTC、录制、总结）
- AI 原生协作（AI 与人平等身份、平等权限、平等参与）

平台核心原则：
- 平等性：AI 与人都以 Member 身份参与，具备统一协作语义。
- 可组合：消息、文档、会议、知识库、任务可相互引用与联动。
- 可治理：多租户隔离、审计可追踪、策略可执行。
- 可扩展：技能系统与 A2A 协议支持多 AI 协同与生态扩展。

## 2. 核心功能设计

### 2.1 AI 身份与平等性

目标：让 AI 成为“平台原生成员”，而非外挂机器人。

关键设计：
- AI 独立 `MemberId`：在 `members` 表中新增 `member_type`（`human` | `ai`），并为 AI 绑定 `agent_profile_id`。
- 房间成员一致模型：`room_members` 不区分人/AI，统一成员权限模型（Owner/Admin/Member/Guest）。
- 统一消息协议：AI 通过与人相同的消息 API/WS 发送 `message`、`thread_reply`、`reaction`、`mention`。
- 会议参与：AI 可作为“虚拟参会者”加入 WebRTC 房间，支持听/说/文本转语音。

数据模型建议：
- `members(id, tenant_id, display_name, member_type, status, created_at)`
- `ai_profiles(member_id, provider_id, model, system_prompt, skill_bindings, policy_id)`
- `room_members(room_id, member_id, role, joined_at)`

### 2.2 AI 技能系统

目标：每个 AI 可按场景配置能力，支持动态演进与技能市场。

关键设计：
- 多技能绑定：`ai_skill_bindings` 支持一个 AI 绑定多个技能（含版本、优先级、启用状态）。
- 动态增删：通过 `PATCH /ai/{member_id}/skills` 实现热更新，运行时重载不重启服务。
- 能力分类：文档处理、代码审查、翻译、摘要、会议助手、知识问答等。
- 技能市场：支持租户私有市场 + 官方市场，包含评分、版本、权限声明、签名校验。

运行机制：
- 技能执行走沙箱（权限白名单：FS/Network/Process）。
- 每次技能调用生成审计事件（trace_id + skill_version + decision）。
- 技能路由可基于意图识别与上下文（如“总结会议纪要”自动路由 summary skill）。

### 2.3 知识库集成

目标：让 AI “基于组织知识协作”，而不是仅基于会话上下文。

关键设计：
- 数据源接入：文档（PDF/Docx/Markdown）、网页、代码仓库。
- 索引链路：解析 -> 分块 -> embedding -> 向量索引 + 关键词倒排双索引。
- 语义搜索：Hybrid Search（BM25 + Vector）+ rerank。
- 实时更新：通过事件总线监听文档变更，增量重建索引。

能力接口：
- `POST /knowledge/sources`：注册知识源
- `POST /knowledge/index/jobs`：触发索引任务
- `GET /knowledge/search?q=...`：语义检索
- `POST /knowledge/refresh`：增量刷新

### 2.4 视频会议（AI 可参与）

目标：在 Zoom 级会议体验中引入 AI 协作能力。

关键设计：
- WebRTC SFU 架构：媒体服务采用 SFU，支持多人会议、屏幕共享、录制。
- AI 参会链路：会议音轨 -> ASR -> LLM 推理 -> TTS -> AI 音轨回注。
- 会议产物：自动生成摘要、行动项、决策记录，并回写文档与任务。
- 会议控制：AI 可被邀请/静音/移出，默认需房主授权发言。

### 2.5 文档协作

目标：实现 Notion 级协作体验 + AI 辅助编辑。

关键设计：
- 实时协同：基于 CRDT（如 Yjs）支持多人并发编辑。
- AI 辅助：段落润色、结构重写、引用补全、长文摘要、风险提示。
- 版本控制：文档快照 + diff + 回滚；关键版本可签名归档。
- 评论批注：评论线程、@提及、AI 建议可一键转任务。

### 2.6 A2A 协议（AI 与 AI 协作）

目标：让多个 AI 在同一任务中可通信、协商、分工、汇总。

关键设计：
- 通信协议：复用 Nexis A2A 设计（Envelope + Payload + Trace + Auth）。
- 协作模式：`handoff`、`fan-out/fan-in`、`debate/vote`。
- 决策机制：支持规则投票与策略约束（预算、时限、数据域）。
- 任务分配：编排器根据技能、成本、SLA 自动分配给最优 AI。

## 3. 技术架构

### 3.1 分层架构（对标 Slack + Notion + Zoom + AI）

- 协作层：频道消息、文档、会议、任务。
- AI 层：AI Member、技能运行时、A2A 编排、上下文管理。
- 知识层：多源采集、索引、语义检索、知识权限。
- 平台层：身份权限、多租户、审计、计费、可观测。

### 3.2 核心服务拆分

- `gateway-service`：统一 API/WS 入口、租户鉴权、限流。
- `realtime-service`：消息通道、在线状态、线程与通知。
- `doc-service`：CRDT 同步、版本管理、评论批注。
- `meeting-service`：WebRTC 信令、录制控制、转写任务触发。
- `ai-runtime-service`：AI 会话执行、技能编排、工具调用。
- `a2a-hub-service`：AI 间协议通信、任务编排、协作状态。
- `knowledge-service`：索引管道、检索 API、权限过滤。
- `audit-policy-service`：策略执行、审计归档、合规检查。

### 3.3 关键技术方案

- 实时通信：WebSocket（消息）+ WebRTC（音视频）。
- 文档协作：CRDT（Yjs）+ Postgres 快照存储。
- 搜索与知识：Qdrant/pgvector + BM25 + reranker。
- 任务编排：事件总线（Kafka/NATS）+ 状态机（task lifecycle）。
- AI 接入：Provider 抽象（OpenAI/Anthropic/Gemini/本地模型）。
- 安全治理：mTLS + JWT，RBAC/ABAC，租户级隔离。
- 可观测：OpenTelemetry + Prometheus + Grafana + 审计链路。

### 3.4 多租户与权限模型

- 资源主键统一带 `tenant_id`。
- AI 与人统一受策略控制（谁可调用哪些技能/知识源/会议能力）。
- 知识库检索强制权限过滤（先鉴权后召回）。
- 高风险操作（外发、删除、管理员动作）支持 Human-in-the-loop 审批。

## 4. 实现路线图

### Phase 1（0-2 个月）：AI 成员化与基础协作闭环
- 交付：
  - AI `MemberId` 与成员模型统一
  - AI 收发消息、参与房间
  - 基础技能绑定（文档摘要/翻译/问答）
  - 知识库最小可用（文档上传 + 检索）
- 验收指标：
  - AI 消息投递成功率 >= 99.9%
  - 知识检索 P95 < 800ms

### Phase 2（2-4 个月）：文档与会议 AI 化
- 交付：
  - CRDT 实时文档协作上线
  - AI 辅助编辑（润色、摘要、评论建议）
  - WebRTC 会议 + AI 参会（ASR/TTS）
  - 会议录制与自动纪要
- 验收指标：
  - 文档协同冲突恢复成功率 >= 99.99%
  - 会议纪要生成时延 <= 3 分钟（会后）

### Phase 3（4-6 个月）：A2A 协作与技能市场
- 交付：
  - A2A 协议闭环（发现、通信、任务分配）
  - 多 AI 协作模式（并行、辩论、投票）
  - 技能市场（安装、版本、签名校验、评分）
  - 组织级策略中心（预算、权限、合规）
- 验收指标：
  - 多 AI 任务自动闭环率 >= 70%
  - 技能执行审计覆盖率 = 100%

### Phase 4（6-9 个月）：企业级治理与规模化
- 交付：
  - 全链路审计与回放
  - 成本与 SLA 路由（模型/技能调度优化）
  - 高并发优化（100K 连接目标对齐现有路线图）
  - 企业安全能力（密钥托管、数据驻留、DLP）
- 验收指标：
  - 平台可用性 >= 99.95%
  - 消息 P95 < 80ms（Region 内）

## 5. 优先级排序（P0/P1/P2）

### P0（必须先做）
- AI 身份与成员平等模型
- AI 消息参与与房间权限
- 基础知识库检索（文档 + 语义搜索）
- 基础技能系统（绑定/启用/禁用 + 审计）

原因：这是 “AI 原生协作” 的底座，没有 P0 就无法形成差异化产品闭环。

### P1（核心增强）
- 实时文档协作 + AI 辅助编辑
- WebRTC 会议 + AI 参会 + 自动纪要
- 技能市场（租户私有 + 官方）

原因：直接形成对标 Slack + Notion + Zoom + AI 的完整体验。

### P2（规模化与生态）
- A2A 多 AI 协作编排与协议生态
- 成本/SLA 智能路由
- 企业级治理与跨域协作

原因：用于企业扩张与平台生态阶段，构建长期竞争壁垒。

---

## 附：建议首批 API 与事件（便于立项拆解）

- API：
  - `POST /v1/ai-members`
  - `PATCH /v1/ai-members/{member_id}/skills`
  - `POST /v1/knowledge/sources`
  - `GET /v1/knowledge/search`
  - `POST /v1/meetings/{meeting_id}/ai-join`
  - `POST /v1/a2a/tasks`
- 事件：
  - `ai.member.joined_room`
  - `skill.execution.started|finished|failed`
  - `knowledge.index.updated`
  - `meeting.summary.generated`
  - `a2a.task.assigned|completed`
