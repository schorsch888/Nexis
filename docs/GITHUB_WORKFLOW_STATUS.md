# GitHub Workflow 状态记录

## 最后更新: 2026-02-26 13:17 GMT+8

---

## Workflow 列表

| Workflow | 文件 | 触发条件 |
|----------|------|----------|
| CI | ci.yml | push/PR to main, develop |
| Benchmark | benchmark.yml | push to main |
| Docs | docs.yml | push to main |
| Release | release.yml | tag push |
| Security | security.yml | schedule + push |

---

## 最近问题修复

| 日期 | 问题 | 修复 |
|------|------|------|
| 2026-02-26 | clippy map_entry warning | ✅ 已修复 (84e688a) |
| 2026-02-25 | rustfmt.toml nightly 配置 | ✅ 已修复 |
| 2026-02-25 | benchmark 空结果 | ✅ 已修复 |

---

## Dependabot 警告

| 状态 | 数量 | 说明 |
|------|------|------|
| Moderate | 2 | 间接依赖，已在 deny.toml 忽略 |

---

## CI 检查项

| 检查 | 状态 |
|------|------|
| cargo fmt --check | ✅ 通过 |
| cargo clippy -D warnings | ✅ 通过 |
| cargo test --workspace | ✅ 通过 |
| cargo doc | ✅ 通过 |
| cargo deny check | ✅ 通过 |

---

## 下次检查

推送代码后，GitHub Actions 会自动运行。
查看地址: https://github.com/schorsch888/Nexis/actions
