# Nexis 开发流程 - 大厂标准

## 工作流程

```
┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
│  设计   │ → │  Review │ → │  实现   │ → │  Test   │ → │  Deploy │
└─────────┘   └─────────┘   └─────────┘   └─────────┘   └─────────┘
     ↓             ↓             ↓             ↓             ↓
   Codex        Codex        OpenCode       CI/CD        GitHub
   设计        代码审查      实现          自动测试       Actions
```

## 阶段详解

### 1. 设计阶段 (Codex)
- 输出：设计文档 (docs/en/design/)
- 要求：
  - 技术方案
  - API 设计
  - 数据模型
  - 风险评估

### 2. Review 阶段 (Codex)
- 输出：审查报告
- 要求：
  - 代码质量
  - 架构一致性
  - 安全性
  - 性能

### 3. 实现阶段 (OpenCode)
- 输出：代码 + 测试
- 要求：
  - 测试覆盖 80%+
  - Clippy 0 warnings
  - 文档完整

### 4. Test 阶段 (CI/CD)
- 输出：测试报告
- 要求：
  - 单元测试通过
  - 集成测试通过
  - E2E 测试通过

### 5. Deploy 阶段 (GitHub Actions)
- 输出：部署结果
- 要求：
  - 文档部署
  - Docker 镜像
  - 发布说明

## 代码审查标准

### 必须通过
- [ ] cargo fmt --check
- [ ] cargo clippy -D warnings
- [ ] cargo test --workspace
- [ ] cargo deny check

### 审查要点
- [ ] 代码质量
- [ ] 测试覆盖
- [ ] 文档完整
- [ ] 安全合规

## PR 模板

```markdown
## 变更说明
[描述变更内容]

## 测试
- [ ] 单元测试
- [ ] 集成测试
- [ ] E2E 测试

## 文档
- [ ] API 文档更新
- [ ] 用户文档更新

## 检查清单
- [ ] cargo fmt
- [ ] cargo clippy
- [ ] cargo test
```

## 发布流程

### 版本号
- 遵循 SemVer
- MAJOR.MINOR.PATCH

### 发布步骤
1. 创建 release 分支
2. 更新 CHANGELOG
3. 创建 tag
4. CI/CD 自动部署
5. 发布 GitHub Release

## 工具角色

| 工具 | 角色 |
|------|------|
| Codex | 设计、Review、规划 |
| OpenCode | 实现、测试 |
| GitHub Actions | CI/CD |
| Telegram | 通知 |
