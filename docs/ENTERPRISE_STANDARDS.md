# Nexis 大厂标准检查清单

## 参考：Google / Microsoft / GitHub / Meta

---

## 1. 代码质量

| 检查项 | 标准 | 工具 |
|--------|------|------|
| 代码格式 | 100% 通过 | cargo fmt --check |
| 静态分析 | 0 warnings | cargo clippy -D warnings |
| 测试覆盖 | ≥80% | cargo tarpaulin |
| 依赖审计 | 0 漏洞 | cargo deny check |
| 文档生成 | 无警告 | cargo doc |

## 2. 测试标准

| 类型 | 要求 |
|------|------|
| 单元测试 | 每个公开 API |
| 集成测试 | 关键流程 |
| E2E 测试 | 用户场景 |
| 性能测试 | 基准对比 |
| 安全测试 | 渗透测试 |

## 3. 代码审查

### 必须通过
- [ ] 所有 CI 检查通过
- [ ] 至少 1 人 Approve
- [ ] 无未解决的 Comment
- [ ] 分支与 main 同步

### 审查要点
- [ ] 架构一致性
- [ ] 代码可读性
- [ ] 性能影响
- [ ] 安全风险
- [ ] 测试充分

## 4. 文档标准

### 必须包含
- [ ] README (EN + ZH-CN)
- [ ] API 文档
- [ ] 架构图
- [ ] 变更日志
- [ ] 贡献指南

### 结构要求
```
docs/
├── en/           # 英文（默认）
├── zh-CN/        # 中文
└── index.md      # 语言选择
```

## 5. 发布流程

### 版本管理
- 遵循 SemVer (MAJOR.MINOR.PATCH)
- Git Tag 格式: v1.0.0

### 发布检查
- [ ] CHANGELOG 更新
- [ ] 文档更新
- [ ] 测试全部通过
- [ ] 安全审计通过

### 发布步骤
1. 创建 release 分支
2. 更新版本号
3. 更新 CHANGELOG
4. 创建 PR → Review → Merge
5. 创建 Tag
6. CI/CD 自动部署
7. 发布 GitHub Release

## 6. 安全合规

### 代码安全
- [ ] 无硬编码密钥
- [ ] 输入验证
- [ ] 输出编码
- [ ] 权限最小化

### 依赖安全
- [ ] 依赖审计通过
- [ ] 无已知漏洞
- [ ] 许可证合规

### 数据安全
- [ ] 敏感数据加密
- [ ] 访问控制
- [ ] 审计日志

## 7. CI/CD 标准

### Pipeline 阶段
```yaml
stages:
  - lint        # 格式检查
  - test        # 测试
  - security    # 安全扫描
  - build       # 构建
  - deploy      # 部署
```

### 必须通过
- [ ] fmt / clippy / test
- [ ] security / audit
- [ ] docs build
- [ ] docker build

## 8. 监控告警

### 必须监控
- [ ] 服务健康
- [ ] 错误率
- [ ] 响应时间
- [ ] 资源使用

### 告警规则
- [ ] 错误率 > 1%
- [ ] P99 延迟 > 1s
- [ ] CPU > 80%
- [ ] 内存 > 80%

## 9. 工作流程

### 分支策略
```
main        # 稳定版本
develop     # 开发版本
feature/*   # 功能分支
release/*   # 发布分支
hotfix/*    # 紧急修复
```

### PR 流程
1. 创建 feature 分支
2. 开发 + 测试
3. 创建 PR
4. CI 检查
5. Code Review
6. Approve + Merge

## 10. 团队协作

### 工具分工
| 工具 | 职责 |
|------|------|
| Codex | 设计、Review、规划 |
| OpenCode | 实现、测试 |
| GitHub Actions | CI/CD |
| Telegram | 通知 |

### 会议节奏
- 每日站会 (异步)
- 每周回顾
- 每月规划

---

## 检查频率

| 检查项 | 频率 |
|--------|------|
| CI | 每次 commit |
| 依赖审计 | 每周 |
| 安全扫描 | 每周 |
| 文档更新 | 每次发布 |
| 回顾会议 | 每周 |
