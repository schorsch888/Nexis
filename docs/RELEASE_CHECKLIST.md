# Release Checklist

**版本：** vX.X.X  
**发布日期：** YYYY-MM-DD  
**发布负责人：** @username  

---

## Pre-Release Preparation (发布前准备)

### 1. 代码完成度

- [ ] 所有计划功能已实现
- [ ] 所有 P0/P1 Bug 已修复
- [ ] 代码已合并到 develop 分支
- [ ] 代码审查已完成
- [ ] 无未解决的 PR comments

### 2. 测试完成度

- [ ] 单元测试全部通过（`cargo test --all`）
- [ ] 集成测试全部通过
- [ ] E2E 测试全部通过
- [ ] 手动测试已完成
- [ ] 测试覆盖率 ≥ 80%
- [ ] 性能测试无回退

### 3. 文档更新

- [ ] API 文档已更新（`cargo doc`）
- [ ] README.md 已更新
- [ ] CHANGELOG.md 已更新
- [ ] Migration Guide 已编写（如有 Breaking Changes）
- [ ] 架构文档已更新（如有重大变更）

### 4. 依赖管理

- [ ] 依赖已锁定（Cargo.lock）
- [ ] 安全审计通过（`cargo audit`）
- [ ] 无已知漏洞依赖

### 5. 版本号更新

- [ ] Cargo.toml 版本号已更新
- [ ] Cargo.lock 已更新
- [ ] 其他配置文件版本号已更新（如有）

---

## Release Process (发布流程)

### 1. Create Release Branch (创建发布分支)

```bash
git checkout develop
git pull origin develop
git checkout -b release/vX.X.X
```

- [ ] Release branch 创建完成
- [ ] Branch 名称符合规范：`release/vX.X.X`

### 2. Final Testing (最终测试)

- [ ] 在 release branch 上运行完整测试套件
- [ ] 手动 Smoke Test
- [ ] 性能基准测试

```bash
cargo test --all --release
cargo bench
```

### 3. Merge to Main (合并到主分支)

```bash
git checkout main
git pull origin main
git merge --no-ff release/vX.X.X
```

- [ ] Merge 完成，无冲突
- [ ] CI 检查全部通过

### 4. Create Git Tag (创建标签)

```bash
git tag -a vX.X.X -m "Release vX.X.X: Brief description"
git push origin vX.X.X
```

- [ ] Tag 创建完成
- [ ] Tag 已推送到远程

### 5. Build Release Artifacts (构建发布产物)

```bash
# Build binaries
cargo build --release

# Create archives
tar -czf nexis-vX.X.X-linux-x86_64.tar.gz target/release/nexis-*
```

- [ ] Linux x86_64 binary 构建完成
- [ ] Docker image 构建完成
- [ ] 产物已上传到 Release Page

### 6. Update Changelog (更新变更日志)

- [ ] CHANGELOG.md 已更新
- [ ] 变更分类正确（Added/Changed/Fixed/Security）
- [ ] Breaking Changes 已标注

### 7. Create GitHub Release (创建 GitHub Release)

- [ ] GitHub Release 创建完成
- [ ] Release Notes 填写完整
- [ ] 产物已附加

---

## Deployment (部署)

### 1. Staging Deployment (预发布环境部署)

```bash
# Deploy to staging
kubectl apply -f k8s/staging/
```

- [ ] Staging 环境部署完成
- [ ] Smoke Test 通过
- [ ] 功能验证完成

### 2. Production Deployment (生产环境部署)

**部署窗口：** YYYY-MM-DD HH:MM - HH:MM (时区)

```bash
# Deploy to production
kubectl apply -f k8s/production/
```

#### Pre-deployment

- [ ] 数据库备份完成
- [ ] 监控大盘准备好
- [ ] 回滚方案确认
- [ ] Stakeholder 已通知

#### Deployment

- [ ] 生产环境部署完成
- [ ] 健康检查通过
- [ ] 服务可用性确认

#### Post-deployment

- [ ] Smoke Test 通过
- [ ] 关键功能验证
- [ ] 监控指标正常
- [ ] 错误率正常
- [ ] 性能指标正常

### 3. Rollback Plan (回滚计划)

**回滚命令：**
```bash
kubectl rollout undo deployment/nexis-gateway
```

**回滚条件：**
- 错误率 > 1%
- P95 延迟 > 阈值
- 核心功能不可用

- [ ] 回滚步骤已测试
- [ ] 回滚命令可用

---

## Post-Release (发布后)

### 1. Monitoring (监控)

**监控时长：** 发布后 24 小时

- [ ] 错误率监控（前 1 小时每 5 分钟检查）
- [ ] 性能监控（P95 延迟）
- [ ] 资源使用监控（CPU/Memory）
- [ ] 业务指标监控

### 2. Communication (沟通)

- [ ] 发布公告已发送（Discord/Email）
- [ ] 团队已通知
- [ ] 用户文档已更新

### 3. Cleanup (清理)

- [ ] Release branch 已删除
- [ ] 临时资源已清理
- [ ] 发布经验总结已完成

### 4. Metrics (指标收集)

- [ ] 发布时长记录
- [ ] 问题数量统计
- [ ] 用户反馈收集

---

## Incident Response (应急响应)

### Severity Levels

| 级别 | 响应时间 | 处理时间 | 通知范围 |
|------|----------|----------|----------|
| P0 | 15 分钟 | 4 小时 | 全员 + Stakeholder |
| P1 | 1 小时 | 24 小时 | Tech Lead + Stakeholder |
| P2 | 4 小时 | 3 天 | 开发团队 |
| P3 | 1 工作日 | 下个版本 | 开发团队 |

### On-call Contact

**Primary:** @username (Discord: xxx)  
**Secondary:** @username (Discord: xxx)  

### Escalation Path

1. On-call Engineer (15 分钟内)
2. Tech Lead (30 分钟内)
3. Engineering Manager (1 小时内)

---

## Sign-off (签字确认)

**Pre-release checklist completed:**  
- [ ] @username (日期: YYYY-MM-DD)

**Release approved:**  
- [ ] Tech Lead (日期: YYYY-MM-DD)

**Production deployment completed:**  
- [ ] DevOps (日期: YYYY-MM-DD)

**Post-release monitoring completed:**  
- [ ] On-call Engineer (日期: YYYY-MM-DD)

---

## Notes (备注)

<!-- 任何额外的注意事项或经验教训 -->

