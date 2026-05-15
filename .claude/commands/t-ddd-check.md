---
description: >
  项目 DDD 合规检查。验证 PRD、用户故事、Demo 测试和实现一致性；可选后端深度一致性检查。
argument-hint: [feature] [--deep] [--backend-only]
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
  - Task
  - Write
---

# DDD 合规检查

## 优先级

`AGENTS.md` 是最高约束。本命令只读检查；若规范、设计或代码事实冲突，停止并说明。

## 目标
- 检查文档完整性与测试覆盖。
- 检查 PRD 与实现的一致性。
- 在深度模式下调用 `backend-consistency` 做模块级评估。

## 使用方式
```bash
/t-ddd-check
/t-ddd-check realm
/t-ddd-check --deep
/t-ddd-check realm --backend-only
```

## 执行流程
1. 扫描 PRD：`docs/prd/**/*.md`（排除模板）。
2. 扫描用户故事：`docs/user-stories/**/*.md`。
3. 扫描 Demo 测试：`demo/e2e/**/*.e2e.ts`。
4. 如果传入 `feature`，聚焦该功能相关文档与测试。
5. 如果传入 `--deep` 或 `--backend-only`：
- 识别模块列表。
- 对每个模块调用 `backend-consistency`。
- 汇总模块评分与问题。
6. 生成报告：`.ai/quality/ddd-check-[YYYYMMDD-HHMMSS].md`。

## 评分体系
默认模式（不含深度检查）：总分 100。
- PRD 完整性：30
- 用户故事完整性：30
- Demo 测试覆盖与质量：30
- 代码实现基本一致性：10

深度模式（含 `--deep` 或 `--backend-only`）：总分 100。
- PRD 完整性：20
- 用户故事完整性：20
- Demo 测试覆盖与质量：20
- 代码实现基本一致性：20
- 后端深度一致性：20

后端深度一致性（20）内部权重：
- API 一致性：30%
- 数据模型一致性：25%
- 验证规则一致性：20%
- 权限一致性：15%
- 业务逻辑一致性：10%

## 报告要求
报告必须包含：
- 执行摘要与总分
- 分项得分
- P0/P1/P2 问题列表
- 深度模式下的模块评分表
- 下一步修复建议

## 失败处理
- 找不到 PRD：标记 P1 并继续。
- 找不到用户故事：标记 P1 并继续。
- 深度检查 agent 失败：标记 P1 并记录失败模块。

## 质量门禁
- 所有统计项必须有数据来源。
- 评分公式必须可复算。
- 报告必须落盘。

## 相关引用
- `.claude/commands/t-consistency-check.md`
- `.claude/agents/backend-consistency.md`
