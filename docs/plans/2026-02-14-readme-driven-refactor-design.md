# README-Driven Refactor Design

## 背景
当前仓库与 README 描述存在明显落差：README 描述了完整的 AI-Native 协作平台架构，但代码层面仅有协议文档与 `packages/nexis-core/Cargo.toml`。本次重构目标是让项目结构、模块入口、构建路径和文档索引与 README 一致，并提供“最小可运行实现”。

## 目标
- 按 README 建立可验证的工程骨架：`nexis-core`、`nexis-cli`、`nexis-gateway`、`apps/web`、`docs` 子目录。
- 实现可构建、可测试、可运行的最小闭环，而非空目录。
- 对未实现高级能力明确标记 `stub/planned`，避免文档与实现再次偏离。

## 范围与非目标
### 范围
- Rust workspace 重构。
- Core 协议模型最小实现（NIP-001/002/003 对应的基础类型与边界）。
- CLI 与 Gateway 最小可运行入口。
- Web 前端基础骨架与启动能力。
- 文档目录与状态说明补全。

### 非目标
- 完整向量存储/知识图谱生产级实现。
- 全量 MCP Provider 集成。
- 复杂多 AI 编排策略的完整落地。

## 架构设计
- `packages/nexis-core` 作为唯一协议/领域模型来源，定义身份、消息、权限、上下文与统一错误类型。
- `packages/nexis-cli` 仅做命令编排与 I/O，依赖 core。
- `servers/nexis-gateway` 负责接入、路由、鉴权与 MCP 适配边界，依赖 core。
- `apps/web` 作为独立前端壳层，通过 API/WS 对接 gateway。

## 数据流
1. CLI/Web 生成请求。
2. Gateway 解析身份并调用 core 权限检查。
3. Gateway 执行消息路由与房间广播。
4. AI 相关请求通过 `mcp` 适配层（本轮为最小 stub）。
5. 响应统一回传客户端。

## 错误处理
- core 暴露统一 `NexisError`。
- gateway 以结构化 JSON 错误返回（参数、鉴权、权限、路由、MCP 不可用）。
- cli 将错误映射为可读输出并保留错误码。

## 测试策略
- core: MemberId 解析、Message 校验、Permission 检查单测。
- cli: 命令解析与关键命令冒烟测试。
- gateway: 健康检查、基础鉴权、消息路由测试。
- web: 至少保障 build 通过。

## 验收标准
- README 中声明的关键目录与入口命令均存在且可验证。
- `cargo check/test` 覆盖 workspace。
- 前端 `build` 成功。
- 所有 stub 在代码与文档中显式标注。
