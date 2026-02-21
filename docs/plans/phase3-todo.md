# Phase 3 TODO List

## Task 5: 索引管线（Gateway -> 向量入库）

### 5.1 基础结构
- [ ] 5.1.1 创建 `crates/nexis-gateway/src/indexing/mod.rs`
- [ ] 5.1.2 定义 `IndexingService` trait
- [ ] 5.1.3 添加依赖到 Cargo.toml

### 5.2 消息索引
- [ ] 5.2.1 实现 `MessageIndexer` 结构
- [ ] 5.2.2 添加 `index_message()` 方法
- [ ] 5.2.3 集成 EmbeddingProvider 调用
- [ ] 5.2.4 集成 VectorStore 写入

### 5.3 异步处理
- [ ] 5.3.1 添加后台任务队列
- [ ] 5.3.2 实现失败重试逻辑
- [ ] 5.3.3 添加索引状态追踪

### 5.4 测试
- [ ] 5.4.1 添加单元测试
- [ ] 5.4.2 验证 cargo test 通过

---

## Task 6: 上下文窗口强化

### 6.1 Token 计数
- [ ] 6.1.1 添加 tokenizers 依赖 (feature-gated)
- [ ] 6.1.2 实现 `TokenCounter` trait
- [ ] 6.1.3 实现简单的字符估算器

### 6.2 窗口策略
- [ ] 6.2.1 实现优先级保留策略
- [ ] 6.2.2 添加 system message 保护
- [ ] 6.2.3 实现上下文快照接口

### 6.3 测试
- [ ] 6.3.1 添加单元测试
- [ ] 6.3.2 验证窗口溢出行为

---

## Task 7: 摘要回收链路

### 7.1 摘要接口
- [ ] 7.1.1 创建 `crates/nexis-context/src/summary/mod.rs`
- [ ] 7.1.2 定义 `Summarizer` trait
- [ ] 7.1.3 实现 `AISummarizer` (调用 runtime)

### 7.2 摘要策略
- [ ] 7.2.1 实现摘要触发条件
- [ ] 7.2.2 实现摘要结果缓存
- [ ] 7.2.3 集成到 ContextManager

### 7.3 测试
- [ ] 7.3.1 添加单元测试

---

## Task 8: 语义检索服务

### 8.1 服务层
- [ ] 8.1.1 创建 `crates/nexis-gateway/src/search/mod.rs`
- [ ] 8.1.2 实现 `SemanticSearchService`
- [ ] 8.1.3 融合 vector + context 检索

### 8.2 混合搜索
- [ ] 8.2.1 实现关键词预处理
- [ ] 8.2.2 实现向量相似度排序
- [ ] 8.2.3 实现结果合并策略

### 8.3 测试
- [ ] 8.3.1 添加单元测试

---

## Task 9: Gateway Phase 3 API

### 9.1 搜索 API
- [ ] 9.1.1 添加 `/api/v1/search` 路由
- [ ] 9.1.2 实现搜索请求/响应模型
- [ ] 9.1.3 集成 SemanticSearchService

### 9.2 上下文 API
- [ ] 9.2.1 添加 `/api/v1/context/:id` 路由
- [ ] 9.2.2 实现上下文查询接口

### 9.3 测试
- [ ] 9.3.1 添加 API 集成测试

---

## Task 10: CLI 智能命令

### 10.1 搜索命令
- [ ] 10.1.1 添加 `search <query>` 命令
- [ ] 10.1.2 实现结果展示格式化

### 10.2 上下文命令
- [ ] 10.2.1 添加 `context show` 命令
- [ ] 10.2.2 添加 `context clear` 命令

### 10.3 测试
- [ ] 10.3.1 添加命令测试

---

## Task 11: 知识图谱 MVP

### 11.1 基础结构
- [ ] 11.1.1 创建 `crates/nexis-graph/Cargo.toml`
- [ ] 11.1.2 定义 `Entity` 和 `Relation` 类型
- [ ] 11.1.3 实现 `GraphStore` trait

### 11.2 实体提取
- [ ] 11.2.1 定义 `EntityExtractor` trait
- [ ] 11.2.2 实现简单的规则提取器

### 11.3 测试
- [ ] 11.3.1 添加单元测试

---

## Task 12: 可观测性与发布

### 12.1 Metrics
- [ ] 12.1.1 添加向量操作 metrics
- [ ] 12.1.2 添加上下文操作 metrics
- [ ] 12.1.3 添加搜索性能 metrics

### 12.2 文档
- [ ] 12.2.1 更新 README Runtime Status
- [ ] 12.2.2 更新 CHANGELOG

### 12.3 CI/CD
- [ ] 12.3.1 验证所有测试通过
- [ ] 12.3.2 验证 clippy 无警告

---

## 进度追踪

| TODO 项 | 完成数 | 总数 | 进度 |
|---------|--------|------|------|
| Task 5 | 0 | 10 | 0% |
| Task 6 | 0 | 8 | 0% |
| Task 7 | 0 | 7 | 0% |
| Task 8 | 0 | 7 | 0% |
| Task 9 | 0 | 6 | 0% |
| Task 10 | 0 | 5 | 0% |
| Task 11 | 0 | 6 | 0% |
| Task 12 | 0 | 6 | 0% |
| **总计** | **0** | **55** | **0%** |
