# Herald 质量总规范（Core）

## 目标

本文件定义跨领域质量总原则与门禁框架。

具体验收细则已下沉到：
- `spec/agents/backend/quality.md`
- `spec/agents/frontend/quality.md`
- `spec/agents/demo/quality.md`

## 1. DDD 合规总纲

默认流程：
`User Story -> PRD(可选) -> 技术设计 -> 任务拆解 -> 代码实现 -> Demo 测试 -> DDD 合规检查`

豁免前缀：
- `bugfix-`
- `refactor-`
- `doc-`
- `test-`
- `style-`

DDD 检查最小要求：
- 需求来源可追溯（`docs/user-stories` / `docs/prd`）
- 设计输入可追溯（`.ai/design`）
- 任务拆解可追溯（`.ai/task`）
- 实现与设计语义一致

## 2. 通用质量门禁

### P0（阻塞）
- 编译/类型检查失败
- 核心测试失败
- 关键验收流程缺失

### P1（重要）
- 关键一致性偏差（接口/文档/用户故事）
- 明显质量风险（高复杂度、关键重复逻辑）

### P2（一般）
- 可维护性优化项

### P3（优化）
- 体验或表达层改进

## 3. 证据与报告要求

质量结论必须：
- 给出文件来源（必要时附行号）
- 给出可执行修复建议
- 按 P0/P1/P2/P3 分类

报告目录：`.ai/quality/`

## 4. 领域细则入口

- 后端验收细则：`spec/agents/backend/quality.md`
- 前端验收细则：`spec/agents/frontend/quality.md`
- Demo 验收细则：`spec/agents/demo/quality.md`

## 5. 执行约束

- 需求语义冲突时，以 `docs/`（PRD + User Stories）为准
- 执行流程与测试约束，以 `spec/` 与 `AGENTS.md` 为准
- 未经授权不得修改与当前任务无关的文件

## 6. Agent 共享约定

- 共享优先级与禁止规则：`spec/core/agent-conventions.md`
- 任务输出结构：`.claude/protocols/task-output-contract.md`
