# Provider System Design

## 1. 背景与目标

Nexis 当前 provider 能力分散在 `nexis-runtime` 与 `nexis-mcp`：
- `nexis-runtime`：OpenAI、Anthropic，接口以 `generate/generate_stream` 为主，embedding 独立在 `embedding` 模块。
- `nexis-mcp`：OpenAI、Anthropic、Gemini，含 provider factory，但与 runtime 抽象未完全统一。

本设计目标：
- 统一 Provider 接口（Completion/Stream/Embedding）。
- 支持可插拔 provider（内置 + 扩展）。
- 提供智能路由（成本/性能/容量/故障转移）。
- 在 CLI 与 Gateway 提供一致入口。
- 提供完整错误处理与可观测性。

## 2. 设计原则

- 向后兼容优先：保留现有 `generate/generate_stream` 调用链，通过适配层迁移到新接口。
- 能力显式声明：每个 provider 必须声明能力与模型元信息，路由不依赖硬编码。
- 失败可恢复：统一错误分类 + 重试 + 熔断 + 降级。
- 插件友好：新增 provider 不需要改动核心路由逻辑。
- 多租户可扩展：配置和路由策略支持 tenant 级覆盖。

## 3. 方案对比（2-3 个）

### 方案 A（推荐）：统一核心 trait + Provider Registry + Router Strategy

- 在 `nexis-runtime` 定义新一代 `AIProvider` trait 与能力模型。
- 所有 provider（OpenAI/Anthropic/Gemini/Deepseek/Moonshot/Ollama）实现统一 trait。
- 路由由 `ProviderRouter` 执行，策略可配置。

优点：
- 结构清晰，长期维护成本最低。
- 能力和策略解耦，易扩展。

缺点：
- 需要一次性定义较完整的能力模型。

### 方案 B：保留双接口（Runtime/MCP）并在 Gateway 做转换

- Runtime 维持现状，MCP 维持现状。
- 在 Gateway 层增加“超级适配器”统一 API。

优点：
- 短期改动小。

缺点：
- 技术债高，能力重复建模。
- 路由策略难以下沉复用。

### 方案 C：拆分 per-provider 微服务，Nexis 仅做路由

优点：
- Provider 隔离好。

缺点：
- 运维复杂度显著增加。
- 本地模型与内存内 fallback 路径变长，延迟高。

推荐采用方案 A。

## 4. 统一接口设计

```rust
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub supports_stream: bool,
    pub supports_embed: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    pub input_cost_per_1m_tokens_usd: Option<f64>,
    pub output_cost_per_1m_tokens_usd: Option<f64>,
    pub quality_tier: Option<u8>,
    pub latency_tier: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub messages: Vec<Message>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub id: Option<String>,
    pub content: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
    pub provider: String,
}

#[derive(Debug, Clone)]
pub struct EmbedRequest {
    pub input: Vec<String>,
    pub model: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct EmbedResponse {
    pub vectors: Vec<Vec<f32>>,
    pub model: String,
    pub usage: Option<TokenUsage>,
    pub provider: String,
}

pub type CompletionStream = std::pin::Pin<
    Box<dyn futures::Stream<Item = Result<StreamChunk, ProviderError>> + Send>
>;

#[async_trait]
pub trait AIProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn models(&self) -> Vec<ModelInfo>;

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, ProviderError>;
    async fn stream(&self, req: CompletionRequest) -> Result<CompletionStream, ProviderError>;
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ProviderError>;
}
```

### 4.1 兼容迁移层

- 保留旧接口 `generate/generate_stream`，通过 `LegacyProviderAdapter` 转换到 `complete/stream`。
- embedding 统一迁移到 provider trait，`EmbeddingProvider` 逐步废弃。
- CLI 与 Gateway 先接入新 router，老调用链保持可用。

## 5. 可插拔架构

### 5.1 组件结构

- `ProviderFactory`: 根据配置构建 provider 实例。
- `ProviderRegistry`: 管理 provider 生命周期、默认 provider、健康状态。
- `ProviderCapabilities`: 汇总 `models()` 与 provider 级能力。
- `ProviderRouter`: 根据请求目标和策略选择 provider+model。
- `ProviderHealthMonitor`: 周期探活与熔断恢复。

建议目录：
- `crates/nexis-runtime/src/provider/mod.rs`
- `crates/nexis-runtime/src/provider/types.rs`
- `crates/nexis-runtime/src/provider/registry.rs`
- `crates/nexis-runtime/src/provider/router.rs`
- `crates/nexis-runtime/src/provider/health.rs`
- `crates/nexis-runtime/src/provider/providers/{openai,anthropic,google,deepseek,moonshot,ollama}.rs`

### 5.2 内置 Providers（初版）

- OpenAI: `gpt-4o`, `gpt-4-turbo`（Chat/Stream/Embed）
- Anthropic: `claude-3-5-sonnet`, `claude-3`（Chat/Stream）
- Google: `gemini-2.0`, `gemini-1.5`（Chat/Stream/Vision）
- Deepseek: `deepseek-v3`, `deepseek-r1`（Chat/Reasoning）
- Moonshot: `kimi-k2`（Chat/Long Context）
- Ollama: 本地模型（Chat/Stream/可选 Embed）

## 6. 配置模型

```yaml
providers:
  openai:
    enabled: true
    api_key: ${OPENAI_API_KEY}
    base_url: https://api.openai.com/v1
    timeout_ms: 60000
    models:
      - gpt-4o
      - gpt-4-turbo
    default: gpt-4o

  anthropic:
    enabled: true
    api_key: ${ANTHROPIC_API_KEY}
    base_url: https://api.anthropic.com/v1
    models:
      - claude-3-5-sonnet-20241022
    default: claude-3-5-sonnet-20241022

  google:
    enabled: true
    api_key: ${GEMINI_API_KEY}
    models:
      - gemini-2.0-flash
      - gemini-1.5-pro
    default: gemini-2.0-flash

  deepseek:
    enabled: false
    api_key: ${DEEPSEEK_API_KEY}
    base_url: https://api.deepseek.com
    models:
      - deepseek-chat
      - deepseek-reasoner
    default: deepseek-chat

  moonshot:
    enabled: false
    api_key: ${MOONSHOT_API_KEY}
    base_url: https://api.moonshot.cn/v1
    models:
      - kimi-k2
    default: kimi-k2

  ollama:
    enabled: false
    base_url: http://localhost:11434
    models:
      - llama3
      - mistral
    default: llama3

routing:
  mode: balanced # cheapest | fastest | balanced | pinned
  fallback_order: [openai, anthropic, google, ollama]
  retry:
    max_attempts: 3
    backoff_ms: 200
    jitter: true
  circuit_breaker:
    failure_threshold: 5
    reset_timeout_sec: 30
```

配置规则：
- 缺失 `api_key` 或 provider 被禁用时，provider 不注册。
- `default` 必须存在于该 provider `models` 列表。
- 支持环境变量覆盖：`NEXIS_PROVIDER_DEFAULT`, `NEXIS_MODEL_DEFAULT`。

## 7. 智能路由策略

### 7.1 路由输入

- 请求约束：`required_capabilities`（stream/embed/vision/reasoning）、模型偏好、预算上限、延迟 SLO。
- 运行状态：provider 健康度、最近错误率、并发占用、RTT。
- 配置策略：`mode`、fallback 顺序、权重。

### 7.2 策略算法

1. 过滤：按能力、启用状态、健康状态过滤候选集。
2. 打分：
   - cheapest: `score = cost_weight * estimated_cost + penalty`
   - fastest: `score = latency_weight * p95_latency + penalty`
   - balanced: 成本/延迟/质量加权综合
3. 选择：取最低分 provider+model。
4. 执行：调用 provider。
5. 故障：若可重试错误，按策略重试；若 provider 熔断，切换下一个候选。

### 7.3 故障转移规则

- 超时、429、5xx、网络错误：可重试 + fallback。
- 4xx 参数错误、认证失败：不可重试，直接返回错误。
- 连续失败触发熔断，熔断期间不参与路由。

## 8. 错误处理设计

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider '{provider}' auth failed: {message}")]
    Auth { provider: String, message: String },

    #[error("provider '{provider}' rate limited: retry_after={retry_after_ms:?}")]
    RateLimited { provider: String, retry_after_ms: Option<u64> },

    #[error("provider '{provider}' timeout after {timeout_ms}ms")]
    Timeout { provider: String, timeout_ms: u64 },

    #[error("provider '{provider}' unavailable: {status}")]
    Unavailable { provider: String, status: u16 },

    #[error("provider '{provider}' invalid request: {message}")]
    InvalidRequest { provider: String, message: String },

    #[error("provider '{provider}' decode error: {message}")]
    Decode { provider: String, message: String },

    #[error("router: no eligible provider for capability '{capability}'")]
    NoEligibleProvider { capability: String },

    #[error("router: all providers failed: {summary}")]
    AllProvidersFailed { summary: String },

    #[error("configuration error: {message}")]
    Config { message: String },
}
```

错误处理约定：
- 对外 API 返回统一 JSON 错误结构：`code/message/provider/retriable/request_id`。
- 记录结构化日志字段：`provider`, `model`, `attempt`, `latency_ms`, `error_code`。
- 指标：`provider_requests_total`, `provider_errors_total`, `provider_latency_ms`, `provider_circuit_open`。

## 9. CLI 设计

新增命令：

```bash
nexis provider list
nexis provider add <name>
nexis provider test <name>
nexis provider models <name>
nexis chat --provider <name> --model <model>
```

行为定义：
- `provider list`: 输出 provider 名称、状态、默认模型、能力。
- `provider add`: 交互或参数模式写入配置（不回显密钥）。
- `provider test`: 发起最小 completion + 可选 stream/embedding 健康测试。
- `provider models`: 从 provider `models()` 拉取并展示。
- `chat --provider/--model`: 显式指定时跳过自动路由。

## 10. 统一 API 网关

### 10.1 Endpoint

`POST /v1/chat/completions`

Headers:
- `Authorization: Bearer <token>`
- `X-Provider: <provider>`（可选）
- `X-Model: <model>`（可选）

行为：
- 未指定 provider/model 时，走 `ProviderRouter` 自动选择。
- 指定 provider 时，若不可用则按 `strict_provider` 决定：
  - `strict_provider=true`：直接报错。
  - `strict_provider=false`：允许 fallback。

响应头：
- `X-Resolved-Provider`
- `X-Resolved-Model`
- `X-Request-Id`

### 10.2 OpenAI 兼容

- 请求/响应尽量兼容 OpenAI Chat Completions 结构。
- 对不支持字段做降级或返回 `unsupported_feature`。

## 11. 安全与合规

- API Key 使用环境变量或密钥管理系统，不写入日志。
- provider 级速率限制，防止单 key 被打爆。
- 本地 provider（Ollama）支持网络隔离策略与 allowlist。
- 审计日志记录路由结果，不记录原始敏感 prompt（可配置脱敏）。

## 12. 分阶段落地计划

### Phase 1: 抽象统一（runtime）

- 新增 `complete/stream/embed` 类型与 trait。
- 为 OpenAI/Anthropic 实现新 trait。
- 提供 legacy adapter 兼容旧调用。

### Phase 2: Provider 扩展与注册

- 接入 Google/Deepseek/Moonshot/Ollama。
- 重构 `nexis-mcp` 使用统一 factory 与模型元信息。

### Phase 3: Router 与 Gateway

- 引入 `ProviderRouter` 与健康监控。
- 增加 `/v1/chat/completions`，支持自动路由与 fallback。

### Phase 4: CLI 与运维

- 新增 `nexis provider *` 命令。
- 接入 metrics、trace、报警与熔断可视化。

## 13. 验收标准

- 至少 6 个 provider 可在配置层启用/禁用。
- Completion/Stream/Embedding 接口统一并通过集成测试。
- 路由在 429/5xx/timeout 场景下可自动切换。
- CLI 命令可列出、测试、查看模型。
- Gateway `/v1/chat/completions` 可兼容主流 OpenAI SDK 基本调用。

## 14. 与现状差异摘要

- 现有 `GenerateRequest/GenerateResponse` 将演进为更通用 completion schema。
- `nexis-runtime` 与 `nexis-mcp` provider 构造逻辑合并为同一配置驱动。
- embedding 从“独立 provider 接口”收敛为统一 `AIProvider::embed` 能力。

