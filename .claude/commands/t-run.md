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
---

# 任务执行

## Purpose
- 读取 `.ai/task/[feature]/.state.json`。
- 按当前 phase 的 item DAG 计算执行顺序，但全程串行调度 item 对应 sub agent。
- `/t-run` 任意时刻最多只能有一个 item 处于 `running`，不得并发执行多个 item。
- `/t-run` 的最小执行单元是 item 文件：
  - `dev/*.md`
  - `test/*.md`
  - `accept/*.md`
- `index.md`、`dev.md`、`test.md`、`accept.md` 只作为上下文和导航，不作为直接执行输入。
- backend 的 `finalize.md` 不由 `/t-run` 执行；其入口固定为 `/t-backend-finalize [feature]`。

## Args
| 参数 | 说明 |
|---|---|
| `[feature]` | 功能名 |
| `--phase <backend\|frontend\|demo>` | 仅执行指定阶段；未指定时执行 `.state.json` 的当前阶段 |

## Preconditions
- `.ai/task/[feature]/.state.json` 必须存在且可解析。
- `.state.json` 不得包含旧状态字段或 `agents` 根字段。
- 目标阶段必须已生成，且 `phases[phase].generated_at` 非空。
- 前置阶段必须已完成：
  - frontend 依赖 backend completed
  - demo 依赖 frontend completed
- 当前阶段目录必须存在。
- 当前阶段必须包含：
  - `index.md`
  - 对应 slot manifest：backend/frontend 为 `dev.md`, `test.md`, `accept.md`；demo 为 `dev.md`, `accept.md`
  - 对应 item 目录和 item 文件
  - backend 阶段额外要求 `finalize.md`

## Execution Unit
`/t-run` 只执行 item 文件。

| phase | slot 顺序 | item 来源 |
|---|---|---|
| backend | `dev -> test -> accept` | `backend/dev/*.md`, `backend/test/*.md`, `backend/accept/*.md` |
| frontend | `dev -> test -> accept` | `frontend/dev/*.md`, `frontend/test/*.md`, `frontend/accept/*.md` |
| demo | `dev -> accept` | `demo/dev/*.md`, `demo/accept/*.md` |

执行策略：
- 全局单 item 串行：同一 phase 内任意时刻只允许启动一个 item agent。
- DAG 仅用于依赖校验和确定可执行顺序；即使同一 DAG 层存在多个已满足依赖的 item，也必须逐个执行。
- 当前 item 完成或失败并写回 `.state.json` 后，才允许选择下一个 item。

slot 状态由其 items 聚合：
- 任一 item `running` => slot `running`
- 任一 item `failed` => slot `failed`
- 全部 items `completed` => slot `completed`
- 否则 slot `pending`

phase 状态由 slot 状态聚合：
- 任一 slot `running` => phase `running`
- 任一 slot `failed` => phase `failed`
- backend 的 `dev/test/accept` 全部 completed 且 `finalize` 未 completed => phase `awaiting_finalize`
- 非 backend 阶段全部 slot completed => phase `completed`
- backend 含 `finalize` completed => phase `completed`
- 其他情况 => phase `pending`

## Item Selection
1. 读取目标 phase 的 slots。
2. 按 slot 顺序扫描 items。
3. 在同一 slot 内按 DAG 拓扑顺序选择 item，但每次只选择一个 item 执行。
4. 同一轮存在多个可执行 item 时，按 slot manifest 中的 item 顺序选择；manifest 缺失顺序时按 item ID 字典序选择。
5. 仅执行 `pending` 或 `failed` item。
6. `failed` item 重试前必须确认其依赖均为 `completed`。
7. 依赖未完成的 item 保持 `pending`，不得跳过依赖执行。
8. 当前 item 完成或失败并重新聚合状态前，不得启动任何其他 item。
9. 若 DAG 成环、依赖缺失或 item 文件缺失，立即终止并提示重新运行 `/t-task-check` 或重建任务。

## Sub Agent Context Contract
调用 item 对应 agent 时，prompt 必须至少包含：
1. agent 自身规范文件（如 `.claude/agents/backend-dev.md`）。
2. 当前 `feature`、`phase`、`slot`、`item_id`、目标 agent。
3. 当前 item 文件全文。
4. 当前 slot manifest 全文。
5. 当前阶段 `index.md`。
6. 由 `context-isolator` 提取的阶段设计摘要。
7. `.state.json` 中目标 phase 的最小必要状态切片。
8. 直接依赖 item 的 handoff 摘要和文件路径。
9. 当前 item 的 completion criteria 和 validation 要求。

对 `backend-test` item 额外要求：
10. 必须显式加载 `.claude/skills/backend-test-run/SKILL.md`。
11. prompt 必须包含：
   - 相关 backend-dev item handoff 摘要
   - 当前改动文件列表或实现影响点
   - 建议定向测试范围
   - 允许升级为全量测试的条件
12. 默认运行顺序必须是：change analysis -> targeted tests -> auto-fix/retest -> optional full-suite escalation。
13. 默认成功标准是“受影响测试通过且无未处理错误”，不是默认全量测试通过。

## State Transition
1. 读取状态并确定执行范围。
2. 校验阶段生成状态和前置阶段状态。
3. 校验当前 phase 的 item DAG。
4. 执行 item 前检查当前 phase 是否已有任何 item 为 `running`：
   - 若存在，立即终止，不启动新 agent。
   - 提示先确认该 item 的真实执行结果，并恢复或修正 `.state.json` 后再重试。
5. 执行 item 前写入：
   - `tasks[phase][slot].items[item_id].status = running`
   - `tasks[phase][slot].items[item_id].started_at = <timestamp>`
   - `tasks[phase][slot].status = running`
   - `phases[phase].status = running`
6. item 成功后写入：
   - `tasks[phase][slot].items[item_id].status = completed`
   - `tasks[phase][slot].items[item_id].completed_at = <timestamp>`
   - `tasks[phase][slot].items[item_id].handoff_summary = <summary>`
7. item 失败后写入：
   - `tasks[phase][slot].items[item_id].status = failed`
   - `tasks[phase][slot].items[item_id].last_error = <summary>`
   - `tasks[phase][slot].status = failed`
   - `phases[phase].status = failed`
   - 停止依赖该 item 的后续执行
8. 每个 item 完成或失败后重新聚合 slot 和 phase 状态。
9. 若当前 item 成功且仍有可执行 item，则返回 Item Selection，继续串行选择下一个 item。
10. backend 阶段在 `accept` slot 全部 completed 后停止，并提示执行 `/t-backend-finalize [feature]`。

## Forbidden
- 生成或依赖旧状态字段。
- 生成或依赖 `agents` 根字段。
- 直接执行 `dev.md`、`test.md`、`accept.md`。
- 只传 `index.md` 或 slot manifest 就开始执行。
- 忽略 item DAG，按文件名随意执行。
- 依赖未完成时执行下游 item。
- 同一 DAG 层并发执行多个 item。
- 一次启动多个 sub agents 或批量下发多个 item。
- 当前 item 未完成并写回状态时，预取、提前执行或跨 slot 执行其他 item。
- 对 `backend-test` 直接下发“先跑全量 `uv run scripts/backend-test.py`”而不做变更分析。
- backend `accept` 完成后自动执行 `finalize.md`。

## Failure
- 状态文件缺失/损坏：终止并提示先运行 `/t-task [feature] --phase [phase]`。
- 状态文件包含旧结构字段：终止并提示删除旧任务目录后重新运行 `/t-task`。
- 依赖不满足：阻塞后续依赖 item。
- 状态写入失败：重试一次，失败则终止。
- 阶段校验失败：提示先完成前置阶段。
- 阶段未生成：提示先运行 `/t-task [feature] --phase [phase]`。
- item 文件缺失或 DAG 非法：提示重建该阶段任务目录。

## Examples
```bash
# 按阶段执行
/t-run sample-feature --phase backend
```

```text
# 调用 backend-dev item 时的最小上下文
feature: sample-feature
phase: backend
slot: dev
item_id: BE-D02
agent: backend-dev
item_file: .ai/task/sample-feature/backend/dev/BE-D02-domain-models.md
slot_manifest: .ai/task/sample-feature/backend/dev.md
phase_index: .ai/task/sample-feature/backend/index.md
dependencies:
  BE-D01: completed, handoff=<summary>
```

## 相关引用
- `.claude/commands/t-task.md`
- `.claude/commands/t-backend-finalize.md`
- `.claude/commands/t-task-check.md`
- `.claude/skills/task-planner/references/phase-validator.md`
