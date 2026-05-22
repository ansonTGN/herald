---
description: 执行由 /t-task 生成的 item 任务计划，按依赖 DAG 排序并串行调度单个 sub agent。
argument-hint: [任务名称] [--phase <backend|frontend|demo>]
allowed-tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Task
  - TaskCreate
  - TaskUpdate
  - TaskList
  - TaskGet
  - Write
  - Bash
  - Agent
---

# 任务执行

共享约定：`spec/core/agent-conventions.md`

## 使用方式

```bash
/t-run [feature] [--phase <backend|frontend|demo>]
```

未指定 `--phase` 时使用 `.state.json` 当前阶段。

## 执行单元

- `/t-run` 只执行 item 文件：`dev/*.md`、`test/*.md`、`accept/*.md`。
- `index.md`、`dev.md`、`test.md`、`accept.md` 只作为上下文和导航。
- backend `finalize.md` 不由 `/t-run` 执行；入口固定为 `/t-backend-finalize [feature]`。

## 串行策略

- 同一 phase 内任意时刻最多一个 item agent 运行。
- DAG 只用于依赖校验和确定可执行顺序；即使同层多个 item 可执行，也逐个执行。
- 当前 item 完成或失败并写回 `.state.json` 后，才选择下一个 item。
- 依赖未完成的 item 保持 `pending`，不得跳过依赖。

## 最小上下文

每个 item 必须通过 `Agent` tool 启动，`subagent_type` 为 item 文件中的 `agent` 字段值。调用 item 对应 agent 时，prompt 至少包含：

- agent 规范文件路径。
- `feature`、`phase`、`slot`、`item_id`、目标 agent。
- 当前 item 文件全文。
- 当前 slot manifest 与阶段 `index.md`。
- 阶段设计摘要。
- `.state.json` 中目标 phase 的必要切片。
- 直接依赖 item 的 handoff 摘要和文件路径。
- 当前 item 的 completion criteria 与 validation。

## Backend Test 特例

- 必须读取 `test_item_type`，只允许 `authoring` 或 `runner`。
- 缺少 `test_item_type` 时拒绝执行，提示先运行 `/t-task-check` 或重建/修正 item。
- `authoring`：不加载 `backend-test-run`，只编写/调整场景测试并做编译验证。
- `runner`：加载 `.claude/skills/backend-test-run/SKILL.md`，执行定向测试、失败分类、生产代码修复委派和重测。
- 同一 backend-test item 同时包含“写新场景测试”和“修复生产代码直到通过”时拒绝执行。
- `backend-test-run` 是 skill，不是 agent；runner item 的 `agent` 仍是 `backend-test`。

## 状态写回

- 执行前写入 item `running`、`started_at`，并聚合 slot/phase 为 `running`。
- 成功后写入 item `completed`、`completed_at`、`handoff_summary`，再聚合 slot/phase。
- 失败后写入 item `failed`、`last_error`，停止依赖该 item 的后续执行。
- backend 阶段 `dev/test/accept` 全部完成后停止，并提示执行 `/t-backend-finalize [feature]`。

## 禁止事项

- 直接执行 slot manifest 或只传 `index.md`。
- 忽略 DAG、跳过依赖或并发执行多个 item。
- 当前 item 未完成并写回状态时，预取或执行其他 item。
- 对 backend-test runner 默认先跑全量后端测试；必须先做影响范围分析。
- backend accept 完成后自动执行 finalize。

## 失败处理

- 状态文件缺失/损坏：提示先运行 `/t-task [feature] --phase [phase]`。
- 旧结构字段、依赖缺失、DAG 成环或 item 文件缺失：终止并提示运行 `/t-task-check` 或重建任务。
- 当前 phase 已有 item 为 `running`：终止，要求先确认真实执行结果并修正状态。
- 状态写入失败：重试一次；仍失败则终止并报告。

## 相关引用

- `.claude/commands/t-task.md`
- `.claude/commands/t-task-check.md`
- `.claude/commands/t-backend-finalize.md`
- `.claude/skills/backend-test-run/SKILL.md`
