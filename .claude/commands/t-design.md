---
description: >
  技术设计文档生成入口。基于仓库内需求资料与现有代码调用 design-doc-generator skill 产出实施方案。
argument-hint: [方案名称]
allowed-tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Task
  - Write
  - Bash
---

# 技术设计生成

## 优先级

`AGENTS.md` 是最高约束。设计生成应保持简单、当前必需、可追溯；如果需求、spec、代码或本命令冲突，停止并说明冲突。

## 使用方式

```bash
/t-design [方案名称]
```

## 职责

- `/t-design` 是入口，只负责校验参数并调用 `design-doc-generator` skill。
- 详细信息收集、现有实现分析、文档模板和质量门禁以 `.claude/skills/design-doc-generator/SKILL.md` 为准。
- 默认不外查资料；只有用户明确要求或提供来源时才使用外部资料。

## 输出

- 写入 `.ai/design/[方案名称].md`。
- 响应摘要包含：文档路径、核心范围、关键风险或待确认点、下一步 `/t-task [方案名称]`。

## 失败处理

- 名称缺失或非法：停止并提示正确用法。
- PRD/User Story 不足：可继续生成，但必须在设计文档中记录显式假设。
- 代码分析不完整：可继续生成，但必须标记影响范围不完整。

## 相关引用

- `.claude/skills/design-doc-generator/SKILL.md`
- `.claude/commands/t-task.md`
