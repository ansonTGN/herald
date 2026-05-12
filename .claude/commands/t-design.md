---
description: >
  生成技术设计文档。基于仓库内需求资料与现有代码调用 design-doc-generator skill 产出实施方案。
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

## 目标
- 为新功能产出可实施的技术设计文档。
- 明确 API、数据库设计、前端方案、约束和实施边界。
- 对前端相关方案，页面设计聚焦交互、状态与依赖，不单列 API 契约描述。

## 使用方式
```bash
/t-design [方案名称]
```

## 执行流程
1. 参数校验：方案名称不能为空。
2. 收集上下文：读取人类已准备的仓库内需求资料、相关设计资料与代码目录。
3. 调用 `design-doc-generator` skill 生成文档。
4. 输出到 `.ai/design/[方案名称].md`。
5. 显示摘要：设计范围、关键接口、数据库影响、风险点。

## 失败处理
- 名称缺失：提示补充参数。
- 相关 PRD 缺失：继续生成但在文档中标记假设。
- skill 执行失败：返回失败原因与重试建议。

## 质量门禁
- 输出文档必须包含：目标、范围、API 接口设计、数据库设计、测试策略、风险。
- 涉及前端时，输出文档必须包含页面/组件说明与页面线框说明；前端部分不要求专门的 API 契约块。
- 默认不额外搜索资料；如确需外部参考，必须由人类显式要求或提供来源。

## 相关引用
- `.claude/skills/design-doc-generator/SKILL.md`
- `.claude/commands/t-task.md`
