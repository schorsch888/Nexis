# Nexis Skills 系统设计（对标 openclaw.ai）

## 1. 目标
- 设计一套可扩展、可插拔的 Skills 系统，对标 openclaw.ai 的三层技能来源（内置/全局/项目）与动态安装能力。
- 支持完整生命周期：`install -> enable -> execute -> disable -> uninstall`。
- 支持版本管理（语义化版本、依赖约束、锁定文件）与可复现执行。
- 提供默认安全沙箱，确保技能执行最小权限、可审计、可回放。

## 2. 对标 openclaw.ai 的映射

### 2.1 目录与来源映射
- openclaw `~/.config/opencode/superpowers/` -> Nexis `builtin://skills`（编译时内置，只读）。
- openclaw 项目 `.opencode/skills/` -> Nexis `.nexis/skills/`（项目级）。
- openclaw 动态安装 `npx skills add <url>` -> Nexis `nexis skill add <url>`（支持 git/http/file 源）。
- 增加全局目录：`~/.nexis/skills/`。

### 2.2 加载优先级
按优先级从高到低：
1. 项目级 `.nexis/skills/`
2. 全局 `~/.nexis/skills/`
3. 内置 `builtin://skills`

同名 skill 冲突时采用“高优先级覆盖低优先级”，并给出冲突告警（可通过 `--source` 指定来源绕过默认解析）。

## 3. Skill 模型设计

### 3.1 Skill 类型
- `tool`: 工具类，提供可执行动作（文件、命令、API）。
- `prompt`: 提示类，提供模板、角色、人设、策略。
- `workflow`: 工作流类，编排多步任务与条件分支。
- `integration`: 集成类，封装外部服务连接器。

### 3.2 Skill Manifest（`skill.yaml`）
```yaml
apiVersion: nexis.skills/v1
kind: Skill
name: code-review
displayName: Code Review
version: 1.2.0
description: 自动代码审查与风险提示
type: workflow
source:
  uri: https://example.com/skills/code-review.git
  revision: v1.2.0
triggers:
  - event: pr_opened
  - event: pre_commit
handlers:
  - id: analyze_code
    run: tool://ast.analyze
  - id: check_style
    run: tool://lint.check
  - id: security_scan
    run: tool://security.scan
dependencies:
  - name: git
    constraint: ">=2.40.0"
  - name: ast-parser
    constraint: "^3.1.0"
permissions:
  fs:
    read:
      - "."
    write:
      - "./.nexis/tmp"
  network:
    allow:
      - "api.github.com:443"
  process:
    allow:
      - "git"
timeout: 120s
resources:
  cpu: "1"
  memory: "512Mi"
entry:
  main: workflow/main.yaml
checksum:
  sha256: "<artifact-sha256>"
signature:
  issuer: "nexis-registry"
  sig: "<base64-signature>"
```

### 3.3 核心字段
- `name`/`version`: 唯一标识与版本。
- `triggers`: 触发条件（事件、命令、上下文匹配）。
- `handlers`: 执行单元与入口。
- `dependencies`: 运行依赖（本地工具/其他 skills/connector）。
- `permissions`: 权限声明（文件、网络、进程、环境变量）。

## 4. 运行时架构

### 4.1 组件
- `SkillRegistry`: 发现与索引技能元数据（多来源聚合）。
- `SkillResolver`: 解析名称、版本、依赖与冲突。
- `SkillInstaller`: 拉取、校验、解包、写入、锁定。
- `SkillRuntime`: 启动沙箱并执行 handler。
- `SkillPolicyEngine`: 权限评估、策略拒绝、审计输出。
- `SkillEventBus`: 事件触发执行（如 `pre_commit`/`pr_opened`）。

### 4.2 数据文件
- `~/.nexis/skills/<name>/<version>/...`：全局安装包。
- `.nexis/skills/<name>/<version>/...`：项目安装包。
- `.nexis/skills.lock`：项目锁文件（精确版本与校验）。
- `~/.nexis/skills/index.json`：全局索引缓存。
- `.nexis/state/skills-enabled.json`：启用状态。

## 5. 生命周期状态机

### 5.1 状态
- `installed`
- `enabled`
- `running`
- `disabled`
- `uninstalled`
- `failed`（异常中间态）

### 5.2 转换规则
- `install`: 拉取+校验后进入 `installed`。
- `enable`: 通过依赖与权限预检查后进入 `enabled`。
- `execute`: 从 `enabled` 进入 `running`，完成后回 `enabled`；失败进入 `failed`。
- `disable`: `enabled/failed` -> `disabled`。
- `uninstall`: `disabled/installed` -> `uninstalled`（仍保留审计记录）。

## 6. 版本与依赖管理

### 6.1 版本策略
- 使用 SemVer：`MAJOR.MINOR.PATCH`。
- 默认安装最新稳定版本；支持 `@1`, `@^1.2`, `@~1.2.3`。
- 允许多版本并存，通过解析器选择匹配版本。

### 6.2 锁文件
`.nexis/skills.lock` 示例：
```yaml
version: 1
skills:
  - name: code-review
    version: 1.2.0
    source: https://example.com/skills/code-review.git
    revision: v1.2.0
    checksum: sha256:abc123...
```

### 6.3 依赖解析
- 支持 skill 对 skill 依赖（DAG），禁止循环依赖。
- 版本冲突时策略：
  - 优先满足显式顶层依赖。
  - 无法满足则安装失败并输出冲突路径。

## 7. 安全沙箱设计

### 7.1 默认隔离
- 文件系统：默认只读项目目录，写入限制在 `.nexis/tmp`。
- 网络：默认拒绝，按 `permissions.network.allow` 白名单放行。
- 进程：仅允许 manifest 声明的可执行命令。
- 环境变量：默认不透传，仅注入显式白名单。

### 7.2 可信供应链
- 安装时执行 checksum 校验。
- 支持签名验证（可选强制策略：仅允许签名 skill）。
- 记录来源 URL、revision、digest，便于追溯。

### 7.3 审计与可观测
- 审计日志字段：`timestamp`、`skill`、`version`、`action`、`decision`、`duration_ms`、`trace_id`。
- 失败保留完整错误栈与策略拒绝原因。
- 提供 `nexis skill logs <name>` 查询最近执行记录。

## 8. CLI 设计

### 8.1 用户命令
```bash
nexis skill list
nexis skill add <url>
nexis skill enable <name>
nexis skill run <name> [args]
nexis skill info <name>
```

### 8.2 建议补充命令
```bash
nexis skill disable <name>
nexis skill remove <name>
nexis skill update <name>[@version]
nexis skill verify <name>
nexis skill logs <name>
```

### 8.3 命令语义
- `list`: 展示名称、版本、来源、状态、是否启用。
- `add`: 拉取并安装，可选 `--global` / `--project`。
- `enable`: 依赖检查 + 权限检查，失败返回阻断原因。
- `run`: 临时执行并输出结构化结果（JSON + 人类可读摘要）。
- `info`: 展示 manifest、权限、依赖树、最近执行摘要。

## 9. 触发系统
- 事件触发：`pre_commit`、`pr_opened`、`manual`、`schedule`。
- 上下文触发：根据用户意图/命令关键字匹配 `triggers`。
- 防抖与幂等：同一 `trace_id + trigger + skill@version` 在时间窗口内只执行一次。

## 10. 兼容性与扩展性
- `apiVersion` 支持多版本并行解析。
- 通过 `type` + `entry` 扩展新的 skill 类型，无需修改核心执行器。
- 集成类 skill 通过 connector 抽象接入外部系统（统一认证与速率限制）。

## 11. 方案对比与推荐

### 方案 A：本地目录优先（最小实现）
- 只支持本地与 git URL，弱版本管理。
- 优点：上线快。
- 缺点：可追溯与安全治理弱。

### 方案 B：目录 + 锁文件 + 沙箱（推荐）
- 支持多来源、SemVer、锁文件、权限模型、签名校验。
- 优点：工程复杂度可控，具备生产可用性。
- 缺点：实现工作量中等。

### 方案 C：中心化 Registry（长期）
- 引入官方 skill registry、评分、信任策略、策略下发。
- 优点：生态治理强。
- 缺点：系统复杂、需要服务端投入。

推荐先落地方案 B，并为方案 C 预留协议扩展位。

## 12. 分阶段实施计划

### Phase 1（MVP）
- Manifest v1 解析。
- `list/add/info/run`。
- 项目级与全局目录加载。

### Phase 2（生产可用）
- `enable/disable/remove/update`。
- 锁文件与依赖解析。
- 沙箱权限与审计日志。

### Phase 3（生态化）
- 签名强校验策略。
- 远程 registry 与评分/信任体系。
- 企业策略集成（组织级 allow/deny）。

## 13. 验收标准
- 安装与执行成功率 >= 99%。
- 同名冲突可解释且可控（来源优先级明确）。
- 未授权文件写入/网络访问拦截率 100%。
- 锁文件可复现同一项目技能环境。

## 14. 风险与缓解
- 风险：第三方 skill 供应链污染。
  - 缓解：签名校验 + checksum + 来源白名单。
- 风险：权限声明过宽导致越权。
  - 缓解：默认拒绝 + 最小权限模板 + 审计告警。
- 风险：版本冲突导致运行不稳定。
  - 缓解：锁文件 + 冲突路径可视化 + 显式覆盖策略。
