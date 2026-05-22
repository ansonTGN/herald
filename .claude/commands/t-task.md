---
description: >
  基于设计文档生成实施任务。固定使用 phase -> slot -> item 模型，不保留旧模式。
argument-hint: [任务名称] [--phase <backend|frontend|demo>]
allowed-tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Task
  - Write
  - Bash
  - Agent
---

# 任务规划生成

共享约定：`spec/core/agent-conventions.md`

## 使用方式

```bash
/t-task [feature] [--phase <backend|frontend|demo>]
```

未指定 `--phase` 时，选择第一未完成阶段。

## 职责

- `/t-task` 是薄入口，负责调用 `.claude/skills/task-planner/SKILL.md` 生成任务目录。
- 详细生成流程、拆分阈值、item 字段、backend test authoring/runner 规则和错误码以 `task-planner` skill 为准。
- 本命令不直接执行 item；执行入口是 `/t-run`。

## 固定模型

- `phase`: `backend | frontend | demo`
- `slot`: `dev | test | accept`，backend 额外有 `finalize`
- `item`: slot 子目录下的最小可执行任务文件

可由 `/t-run` 执行的最小单元只包括：

```text
<phase>/dev/*.md
<phase>/test/*.md
<phase>/accept/*.md
```

`index.md` 与 slot manifest 只做导航和上下文；backend `finalize.md` 由 `/t-backend-finalize [feature]` 执行。

## 前置条件

- `.ai/design/[feature].md` 必须存在。
- 阶段顺序为 `backend -> frontend -> demo`，不得跳过未完成前置阶段。
- 生成 frontend 阶段前执行 `cd frontend && npm run generate-api && cd ../`；失败则停止，不生成该阶段任务。

## 输出契约

生成 `.ai/task/[feature]/` 下当前阶段文件：

- `<phase>/index.md`
- `<phase>/dev.md` 与 `<phase>/dev/*.md`
- `<phase>/test.md` 与 `<phase>/test/*.md`，demo 阶段无 test slot
- `<phase>/accept.md` 与 `<phase>/accept/*.md`
- backend 阶段额外生成 `<phase>/finalize.md`
- `.state.json`

`.state.json` 使用当前结构，不包含旧状态字段或 `agents` 根字段。

## 关键约束

- slot 串行生成：backend/frontend 为 `dev -> test -> accept`，demo 为 `dev -> accept`。
- 每个 slot agent 必须通过 `Agent` tool 启动，`subagent_type` 按 `.claude/skills/task-planner/SKILL.md` 的 Agent Dispatch Mapping 映射。
- 先写入上游 slot manifest 和 item 文件，再生成下游 slot。
- 单个 item 必须足够小，能由一次 agent 调用完成、验证并写 handoff。
- backend/test 必须拆成 `test_item_type: authoring` 与 `test_item_type: runner`：
  - authoring item 只创建/修改场景测试、helper 和模块注册，只做编译验证。
  - runner item 使用 `.claude/skills/backend-test-run/SKILL.md` 执行定向测试、分类失败、委派生产代码修复和重测。
  - `backend-test-run` 是 skill，不是 agent；runner item 的 `agent` 仍是 `backend-test`。
- 不生成同时“写场景测试”和“修复生产代码直到通过”的 backend test item。
- 不生成“为新增 struct/DTO/builder/getter/常量补单测”这类低价值 backend unit test item。

## 失败处理

- 设计文档不存在：提示先运行 `/t-design [feature]`。
- 前置阶段未完成：返回阻塞阶段。
- frontend API 生成失败：返回失败命令和错误摘要。
- slot agent 输出缺字段、依赖缺失或 DAG 成环：拒绝写入成功状态，要求重新生成该 slot。

## 相关引用

- `.claude/skills/task-planner/SKILL.md`
- `.claude/commands/t-run.md`
- `.claude/commands/t-task-check.md`
- `.claude/commands/t-backend-finalize.md`
