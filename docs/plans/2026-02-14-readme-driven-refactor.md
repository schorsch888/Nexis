# README-Driven Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将仓库重构为与 README 对齐的可运行工程骨架，并提供最小可验证实现。

**Architecture:** 以 `nexis-core` 作为领域模型与协议核心，`nexis-cli` 与 `nexis-gateway` 依赖 core；`apps/web` 提供前端壳层并对接 gateway。所有高级能力以明确 `stub/planned` 方式落地，避免伪完成。

**Tech Stack:** Rust workspace (tokio, axum, clap, serde), TypeScript + React + Vite, Markdown docs

---

### Task 1: 建立 Rust Workspace 与 crate 骨架

**Files:**
- Create: `Cargo.toml`
- Create: `packages/nexis-cli/Cargo.toml`
- Create: `packages/nexis-cli/src/main.rs`
- Create: `servers/nexis-gateway/Cargo.toml`
- Create: `servers/nexis-gateway/src/main.rs`
- Modify: `packages/nexis-core/Cargo.toml`
- Create: `packages/nexis-core/src/lib.rs`

**Step 1: Write the failing test**
```rust
// packages/nexis-core/tests/workspace_smoke.rs
#[test]
fn core_crate_links() {
    assert_eq!(nexis_core::version(), "0.1.0");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p nexis-core core_crate_links -v`
Expected: FAIL with unresolved item or missing crate sources.

**Step 3: Write minimal implementation**
```rust
// packages/nexis-core/src/lib.rs
pub fn version() -> &'static str { "0.1.0" }
```

**Step 4: Run test to verify it passes**
Run: `cargo test -p nexis-core core_crate_links -v`
Expected: PASS

**Step 5: Commit**
```bash
git add Cargo.toml packages/nexis-core packages/nexis-cli servers/nexis-gateway
git commit -m "feat(core): initialize workspace and runnable crates"
```

### Task 2: 实现 NIP-001 身份模型（core）

**Files:**
- Create: `packages/nexis-core/src/identity/mod.rs`
- Create: `packages/nexis-core/tests/identity_member_id.rs`
- Modify: `packages/nexis-core/src/lib.rs`

**Step 1: Write the failing test**
```rust
#[test]
fn parse_valid_member_id() {
    let id: nexis_core::identity::MemberId = "nexis:ai:openai/gpt-4".parse().unwrap();
    assert_eq!(id.kind().as_str(), "ai");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p nexis-core parse_valid_member_id -v`
Expected: FAIL with missing `identity::MemberId`.

**Step 3: Write minimal implementation**
实现 `MemberType`、`MemberId`、`FromStr` 校验与 `Display`。

**Step 4: Run test to verify it passes**
Run: `cargo test -p nexis-core parse_valid_member_id -v`
Expected: PASS

**Step 5: Commit**
```bash
git add packages/nexis-core/src/identity packages/nexis-core/tests/identity_member_id.rs packages/nexis-core/src/lib.rs
git commit -m "feat(core): implement NIP-001 member identity parsing"
```

### Task 3: 实现 NIP-002 消息模型（core）

**Files:**
- Create: `packages/nexis-core/src/message/mod.rs`
- Create: `packages/nexis-core/tests/message_validation.rs`
- Modify: `packages/nexis-core/src/lib.rs`

**Step 1: Write the failing test**
```rust
#[test]
fn reject_empty_text_message() {
    let msg = nexis_core::message::Message::text("room1", "nexis:human:a@b.com", "");
    assert!(msg.validate().is_err());
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p nexis-core reject_empty_text_message -v`
Expected: FAIL with missing message APIs.

**Step 3: Write minimal implementation**
实现 `Message`、`Content`、`validate()`、最小流式事件类型。

**Step 4: Run test to verify it passes**
Run: `cargo test -p nexis-core reject_empty_text_message -v`
Expected: PASS

**Step 5: Commit**
```bash
git add packages/nexis-core/src/message packages/nexis-core/tests/message_validation.rs packages/nexis-core/src/lib.rs
git commit -m "feat(core): add NIP-002 message model and validation"
```

### Task 4: 实现 permission/context 与统一错误模型（core）

**Files:**
- Create: `packages/nexis-core/src/permission/mod.rs`
- Create: `packages/nexis-core/src/context/mod.rs`
- Create: `packages/nexis-core/src/error.rs`
- Create: `packages/nexis-core/tests/permission_checks.rs`
- Modify: `packages/nexis-core/src/lib.rs`

**Step 1: Write the failing test**
```rust
#[test]
fn deny_action_without_permission() {
    let decision = nexis_core::permission::can("read", &["write"]);
    assert!(!decision.allowed);
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p nexis-core deny_action_without_permission -v`
Expected: FAIL with missing permission module.

**Step 3: Write minimal implementation**
实现 `NexisError`、`PermissionDecision`、`can()` 与 `RoomContext` 基础结构。

**Step 4: Run test to verify it passes**
Run: `cargo test -p nexis-core deny_action_without_permission -v`
Expected: PASS

**Step 5: Commit**
```bash
git add packages/nexis-core/src packages/nexis-core/tests/permission_checks.rs
git commit -m "feat(core): add permission/context primitives and unified errors"
```

### Task 5: 实现 CLI 最小命令集（create-room/send/member parse）

**Files:**
- Create: `packages/nexis-cli/src/commands/mod.rs`
- Create: `packages/nexis-cli/src/commands/member.rs`
- Create: `packages/nexis-cli/tests/cli_smoke.rs`
- Modify: `packages/nexis-cli/src/main.rs`
- Modify: `packages/nexis-cli/Cargo.toml`

**Step 1: Write the failing test**
```rust
#[test]
fn member_parse_command_accepts_valid_id() {
    let out = assert_cmd::Command::cargo_bin("nexis-cli").unwrap()
        .args(["member", "parse", "nexis:ai:openai/gpt-4"])
        .assert();
    out.success();
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p nexis-cli member_parse_command_accepts_valid_id -v`
Expected: FAIL with missing command wiring.

**Step 3: Write minimal implementation**
使用 `clap` 实现命令树并调用 core。

**Step 4: Run test to verify it passes**
Run: `cargo test -p nexis-cli member_parse_command_accepts_valid_id -v`
Expected: PASS

**Step 5: Commit**
```bash
git add packages/nexis-cli
git commit -m "feat(cli): add minimal command set aligned with README"
```

### Task 6: 实现 Gateway 骨架（health/auth/router/mcp stub）

**Files:**
- Create: `servers/nexis-gateway/src/router/mod.rs`
- Create: `servers/nexis-gateway/src/auth/mod.rs`
- Create: `servers/nexis-gateway/src/mcp/mod.rs`
- Create: `servers/nexis-gateway/tests/healthcheck.rs`
- Modify: `servers/nexis-gateway/src/main.rs`
- Modify: `servers/nexis-gateway/Cargo.toml`

**Step 1: Write the failing test**
```rust
#[tokio::test]
async fn health_endpoint_returns_ok() {
    // call GET /health and assert 200
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p nexis-gateway health_endpoint_returns_ok -v`
Expected: FAIL with missing route.

**Step 3: Write minimal implementation**
使用 `axum` 提供 `/health`、消息路由入口、auth stub、mcp adapter stub。

**Step 4: Run test to verify it passes**
Run: `cargo test -p nexis-gateway health_endpoint_returns_ok -v`
Expected: PASS

**Step 5: Commit**
```bash
git add servers/nexis-gateway
git commit -m "feat(gateway): add runnable gateway skeleton with health/auth/mcp stubs"
```

### Task 7: 创建 Web 前端骨架并接入基本页面

**Files:**
- Create: `apps/web/package.json`
- Create: `apps/web/tsconfig.json`
- Create: `apps/web/vite.config.ts`
- Create: `apps/web/index.html`
- Create: `apps/web/src/main.tsx`
- Create: `apps/web/src/App.tsx`
- Create: `apps/web/src/styles.css`

**Step 1: Write the failing test**
使用构建测试替代单元测试。

**Step 2: Run test to verify it fails**
Run: `cd apps/web && npm run build`
Expected: FAIL with missing project files/scripts.

**Step 3: Write minimal implementation**
搭建 Vite React TS 骨架并展示 gateway 连接状态占位。

**Step 4: Run test to verify it passes**
Run: `cd apps/web && npm run build`
Expected: PASS

**Step 5: Commit**
```bash
git add apps/web
git commit -m "feat(web): scaffold React TypeScript shell app"
```

### Task 8: 文档与 README 对齐修正

**Files:**
- Create: `docs/architecture/overview.md`
- Create: `docs/api/gateway.md`
- Create: `docs/guides/local-development.md`
- Modify: `README.md`

**Step 1: Write the failing test**
定义文档一致性检查（脚本或人工 checklist）。

**Step 2: Run test to verify it fails**
Run: `find docs -maxdepth 3 -type f | grep -E 'architecture|api|guides'`
Expected: FAIL or missing required docs.

**Step 3: Write minimal implementation**
补齐文档与 README 状态矩阵（done/minimal/stub/planned）。

**Step 4: Run test to verify it passes**
Run: `find docs -maxdepth 3 -type f | grep -E 'architecture|api|guides'`
Expected: PASS

**Step 5: Commit**
```bash
git add README.md docs
git commit -m "docs: align README with implemented project skeleton and status"
```

### Task 9: 全仓验证与收尾

**Files:**
- Modify: `.github/workflows/ci.yml` (如需)

**Step 1: Write the failing test**
先运行全量验证，记录失败点。

**Step 2: Run test to verify it fails**
Run:
```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cd apps/web && npm run build
```
Expected: 初次可能 FAIL。

**Step 3: Write minimal implementation**
修复格式、lint、测试与构建错误，保持最小改动。

**Step 4: Run test to verify it passes**
Run same commands above。
Expected: 全部 PASS。

**Step 5: Commit**
```bash
git add .
git commit -m "chore: pass workspace verification and finalize README-driven refactor"
```
