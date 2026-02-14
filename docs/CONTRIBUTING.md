# Nexis 贡献指南

## 代码规范

### Commit 规范 (Conventional Commits)

```
<type>(<scope>): <subject>

<body>

<footer>
```

#### Type 类型

| Type | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(core): add member identity system` |
| `fix` | Bug 修复 | `fix(gateway): resolve websocket connection leak` |
| `docs` | 文档更新 | `docs: update API documentation` |
| `style` | 代码格式（不影响逻辑） | `style(cli): format with rustfmt` |
| `refactor` | 重构 | `refactor(core): simplify permission check` |
| `perf` | 性能优化 | `perf(gateway): optimize message routing` |
| `test` | 测试 | `test(core): add unit tests for MemberId` |
| `chore` | 构建/工具 | `chore: update dependencies` |
| `ci` | CI 配置 | `ci: add github actions workflow` |

#### Scope 范围

- `core` - nexis-core 核心库
- `cli` - nexis-cli 命令行
- `gateway` - nexis-gateway 网关
- `web` - Web 前端
- `protocol` - 协议规范
- `docs` - 文档

#### 示例

```bash
# 新功能
git commit -m "feat(core): implement Member identity protocol (NIP-001)

- Add MemberId parsing and validation
- Support human/ai/agent/system types
- Include comprehensive unit tests

Closes #12"

# Bug 修复
git commit -m "fix(gateway): prevent message duplication in broadcast

The previous implementation could send duplicate messages when
multiple clients reconnect simultaneously.

Fixes #45"
```

### Rust 代码规范

```bash
# 格式化
cargo fmt

# Lint
cargo clippy -- -D warnings

# 测试
cargo test

# 文档
cargo doc --no-deps
```

### TypeScript 代码规范

```bash
# 格式化
pnpm format

# Lint
pnpm lint

# 测试
pnpm test
```

## 分支策略

```
main           # 生产分支，受保护
├── develop    # 开发分支
│   ├── feat/xxx
│   ├── fix/xxx
│   └── refactor/xxx
└── release/x.x.x
```

## PR 规范

### 标题格式

```
<type>(<scope>): <subject>
```

### 描述模板

```markdown
## 变更类型
- [ ] feat: 新功能
- [ ] fix: Bug 修复
- [ ] refactor: 重构
- [ ] docs: 文档
- [ ] test: 测试
- [ ] chore: 其他

## 变更说明
<!-- 描述这个 PR 做了什么 -->

## 测试
<!-- 如何测试这些变更 -->

## 相关 Issue
<!-- Closes #xxx -->
```

## 代码审查

- 所有 PR 必须经过至少一人审查
- CI 检查必须全部通过
- 代码覆盖率不得降低

## 发布流程

1. 从 `develop` 创建 `release/x.x.x` 分支
2. 更新版本号和 CHANGELOG
3. 合并到 `main` 并打 tag
4. 自动触发发布流程
