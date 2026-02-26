# Contributing to Nexis / 参与贡献

Thanks for your interest in contributing to Nexis.

感谢你为 Nexis 做出贡献。

## Table of Contents / 目录

- [Development Environment / 开发环境](#development-environment--开发环境)
- [Workflow / 开发流程](#workflow--开发流程)
- [Code Standards / 代码规范](#code-standards--代码规范)
- [Testing Requirements / 测试要求](#testing-requirements--测试要求)
- [Pull Request Process / PR 流程](#pull-request-process--pr-流程)
- [Commit Convention / 提交规范](#commit-convention--提交规范)

## Development Environment / 开发环境

### Prerequisites / 前置要求

- Rust `1.75+`
- Git `2.30+`
- Optional: Docker / 可选：Docker

### Setup / 初始化

```bash
git clone https://github.com/schorsch888/Nexis.git
cd Nexis

cargo build --workspace
cargo test --workspace
```

## Workflow / 开发流程

1. Create a branch from `main`: `feat/<topic>` or `fix/<topic>`.
2. Make focused changes with clear commit messages.
3. Run format/lint/test locally before pushing.
4. Open PR and complete template/checklist.

1. 从 `main` 创建功能分支：`feat/<topic>` 或 `fix/<topic>`。
2. 保持改动聚焦，提交信息清晰。
3. 推送前完成本地格式化、静态检查、测试。
4. 提交 PR 并完成模板与检查项。

## Code Standards / 代码规范

### Rust

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

### Documentation

- Keep docs bilingual where required.
- Keep links relative and valid.
- Update docs when behavior changes.

- 需要双语的文档请同步更新中英文。
- 优先使用相对链接并保持可访问。
- 行为变更必须同步更新文档。

## Testing Requirements / 测试要求

```bash
cargo test --workspace
```

If your change impacts runtime behavior, include new or updated tests.

若改动影响运行逻辑，请补充或更新对应测试。

## Pull Request Process / PR 流程

1. Use the PR template and describe user impact.
2. Link related issue(s).
3. Ensure CI is green.
4. Request at least one reviewer.

1. 使用 PR 模板并说明用户影响。
2. 关联相关 Issue。
3. 确保 CI 通过。
4. 至少邀请一位 reviewer。

## Commit Convention / 提交规范

We use Conventional Commits:

```text
feat(scope): short summary
fix(scope): short summary
docs: short summary
refactor(scope): short summary
test(scope): short summary
chore: short summary
```

Examples:

```bash
git commit -m "feat(gateway): add room-level permission guard"
git commit -m "docs: refresh security reporting process"
```

## Community Rules / 社区规范

By participating, you agree to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

参与贡献即表示你同意遵守 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。
