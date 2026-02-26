# Security Policy / 安全策略

## Table of Contents / 目录

- [Supported Versions / 支持版本](#supported-versions--支持版本)
- [Reporting a Vulnerability / 漏洞报告方式](#reporting-a-vulnerability--漏洞报告方式)
- [Response SLA / 响应时效](#response-sla--响应时效)
- [Disclosure Process / 披露流程](#disclosure-process--披露流程)
- [Security Baselines / 安全基线](#security-baselines--安全基线)
- [Safe Harbor / 安全港](#safe-harbor--安全港)

## Supported Versions / 支持版本

| Version | Supported |
| --- | --- |
| 0.x | Yes |

## Reporting a Vulnerability / 漏洞报告方式

Please do **not** report security vulnerabilities through public GitHub issues.

请不要通过公开 GitHub Issue 报告安全漏洞。

Use one of the private channels below:

请使用以下私有渠道提交：

- Email: `security@nexis.ai`
- GitHub Security Advisory: <https://github.com/schorsch888/Nexis/security/advisories/new>

What to include / 建议包含信息：

- Affected component/version / 受影响组件与版本
- Reproduction steps / 复现步骤
- Impact assessment / 影响评估
- Suggested remediation (if available) / 可选修复建议

## Response SLA / 响应时效

On business days:

工作日响应目标：

- Acknowledgement within 24 hours / 24 小时内确认收到
- Initial triage within 72 hours / 72 小时内完成初步分级
- Coordinated remediation timeline based on severity / 按严重级别协同修复时间线

## Disclosure Process / 披露流程

1. Report received and tracked with internal incident ID.
2. Validate and assess severity (CVSS + business impact).
3. Mitigate and patch.
4. Coordinate release and advisory publication.
5. Close incident with postmortem improvements.

1. 接收报告并分配事件编号。
2. 复现并评估严重级别（CVSS + 业务影响）。
3. 实施缓解与修复。
4. 协调发布补丁与安全公告。
5. 完成复盘并跟踪改进项。

## Security Baselines / 安全基线

Detailed controls are documented in:

详细控制基线见：

- [docs/security/baseline.md](docs/security/baseline.md)
- [docs/security/enterprise.md](docs/security/enterprise.md)
- [docs/security/key-management.md](docs/security/key-management.md)

## Safe Harbor / 安全港

We support good-faith security research. If you avoid privacy violations,
service disruption, and data destruction, and comply with applicable laws,
we will treat your report as responsible disclosure.

我们支持善意安全研究。若你避免隐私侵害、服务破坏、数据损毁，并遵守相关法律法规，
我们将按负责任披露流程处理并与你协同修复。
