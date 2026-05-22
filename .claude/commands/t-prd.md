---
description: >
  PRD 统一写入口。先补齐相关 user story，再创建或更新指定 feature 的 PRD 文档。
argument-hint: [feature-name]
allowed-tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Write
  - Bash
---

# PRD 管理

共享约定：`spec/core/agent-conventions.md`

## 使用方式

```bash
/t-prd [feature]
```

## 职责

- `/t-prd` 是入口，只负责调用 `prd-manager` skill 完成 PRD 创建或更新。
- `/t-prd-check` 负责质量检查；本命令不做检查流程。
- PRD 只承载产品语义、范围、规则和引用，不承载接口 schema、数据库设计或技术实现方案。

## 执行要求

- 参数缺失或非法时直接失败并提示用法。
- 详细流程、目标域选择、user story 补齐、PRD 分层和禁止内容以 `.claude/skills/prd-manager/SKILL.md` 为准。
- 完成后输出：本次动作、user story 路径和变更方式、PRD 路径、所属域、待确认点、建议下一步。

## 失败处理

- 目标业务域无法可靠判断时，只询问一次 `auth|billing|core|integration`。
- 定位到多个候选 PRD 时停止并要求澄清 feature 或域。
- 信息不足但不阻塞 PRD 草案时，在 PRD 中显式记录缺口，不静默补假设。

## 相关引用

- `.claude/skills/prd-manager/SKILL.md`
- `.claude/commands/t-prd-check.md`
