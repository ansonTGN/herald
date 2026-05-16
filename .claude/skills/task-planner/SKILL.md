---
name: task-planner
description: >
  基于设计文档生成任务规划。固定使用分阶段 + 子任务模型，
  输出可执行 item 文件、slot manifest、阶段索引和可恢复状态。
tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Task
  - Write
  - Bash
  - TaskCreate
  - TaskUpdate
  - TaskList
  - TaskGet
  - Agent
---

# 任务规划生成助手

## 优先级

`AGENTS.md` 是最高约束。本 skill 是 `/t-task` 的详细事实源，但任务拆分必须服务于简单、外科式、可验证的执行；若与用户当前指令、设计文档、spec 或代码事实冲突，停止并说明冲突。

## 概述
- 作用：把 `.ai/design/[feature].md` 转换为可执行任务目录。
- 固定模型：`phase -> slot -> item`。
- `index.md` 是阶段入口和总览。
- `dev.md`、`test.md`、`accept.md` 是 slot manifest，不是执行输入。
- `dev/*.md`、`test/*.md`、`accept/*.md` 是 `/t-run` 的最小执行输入。
- 每个 item 必须足够小，保证一次 agent 调用可以完成、验证、写 handoff。

## 适用场景
- 用户要求“拆解任务、生成执行计划、输出下一步命令”。
- 使用 `/t-task [feature] --phase ...` 分阶段推进。
- 使用 `/t-task [feature]` 自动选择第一未完成阶段。

## 先决条件
| 类别 | 要求 |
|---|---|
| 文件 | `.ai/design/[feature].md` 必须存在 |
| 阶段 | `backend -> frontend -> demo`，不可跳跃 |
| agent 来源 | 仅允许设计文档 `Sub Agent 参考` 明确列出的 agent |
| 输出一致性 | 任务文件与 `.state.json` 必须同轮更新 |
| 执行粒度 | `/t-run` 只能执行 item 文件，不能直接执行 slot manifest |

## 输入参数
| 参数 | 说明 |
|---|---|
| `$1` | `feature`（必填） |
| `--phase <backend\|frontend\|demo>` | 指定阶段生成；未指定时自动选择第一未完成阶段 |

## 决策规则
| 条件 | 动作 |
|---|---|
| 未传 `--phase` | 自动选择第一未完成阶段 |
| 传入 `--phase` | 先做前置阶段校验 |
| 生成 `frontend` 阶段任意任务前 | 必须先执行 `cd frontend && npm run generate-api && cd ../`，失败则终止 |
| 生成 `frontend/test` | 默认只规划高价值逻辑型 Vitest；页面 happy-path 与完整故事交给 Demo |
| 生成 `backend/test` | 必须拆成 `test_item_type: authoring` 与 `test_item_type: runner` 两类 item |

## 状态契约
`.state.json` 不包含旧状态字段或 `agents` 根字段。

最小主结构：
```json
{
  "feature": "sample-feature",
  "phase": "backend",
  "phases": {
    "backend": {"status": "pending", "generated_at": null},
    "frontend": {"status": "pending", "generated_at": null},
    "demo": {"status": "pending", "generated_at": null}
  },
  "tasks": {
    "backend": {
      "dev": {
        "status": "pending",
        "manifest": ".ai/task/sample-feature/backend/dev.md",
        "items": {
          "BE-D01": {
            "status": "pending",
            "file": ".ai/task/sample-feature/backend/dev/BE-D01-database-foundation.md",
            "agent": "backend-dev",
            "depends_on": []
          }
        }
      },
      "test": {
        "status": "pending",
        "manifest": ".ai/task/sample-feature/backend/test.md",
        "items": {}
      },
      "accept": {
        "status": "pending",
        "manifest": ".ai/task/sample-feature/backend/accept.md",
        "items": {}
      },
      "finalize": {
        "status": "pending",
        "file": ".ai/task/sample-feature/backend/finalize.md"
      }
    }
  },
  "metadata": {"design_document": ".ai/design/sample-feature.md"}
}
```

## 执行流程
1. 校验设计文档存在。
2. 解析参数并确定目标阶段。
3. 校验阶段前置状态。
4. 提取并过滤阶段 agents。
5. 若目标阶段为 `frontend`，先执行 `cd frontend && npm run generate-api && cd ../`。
6. 按 slot 串行调度当前阶段 agents。每个 slot agent 必须通过 `Agent` tool 启动，`subagent_type` 按 Agent Dispatch Mapping 映射。传入 prompt 必须包含：设计文档相关节、上游 slot handoff（如有）、`.claude/guides/` 路径、Agent Output Contract 要求的字段列表。
   - backend/frontend: `dev -> test -> accept`
   - demo: `dev -> accept`
7. 每个 slot agent 返回 manifest 和 item 集合。
8. 主流程立即写入：
   - `<phase>/<slot>.md`
   - `<phase>/<slot>/<ITEM-ID>-*.md`
9. 主流程校验 item DAG：
   - item ID 唯一
   - `depends_on` 指向已存在 item
   - 无依赖环
   - item 文件路径与 state 一致
10. 下游 slot prompt 注入上游 manifest、item 清单、item 文件路径和 handoff 摘要。
11. backend 阶段在 `accept` 生成后写入 `<phase>/finalize.md`。
12. 当前阶段 slot 齐备后生成 `<phase>/index.md`。
13. 更新 `.state.json`。
14. 返回下一步建议：`/t-run [feature] --phase [phase]`。

## Agent Dispatch Mapping

| phase | slot | subagent_type |
|-------|------|---------------|
| backend | dev | backend-dev |
| backend | test | backend-test |
| backend | accept | backend-accept |
| frontend | dev | frontend-dev |
| frontend | test | frontend-test |
| frontend | accept | frontend-accept |
| demo | dev | demo-dev |
| demo | accept | demo-accept |

## Slot Manifest Contract
每个 slot manifest 必须包含：
- slot 目标和边界
- item 表格：`id | title | agent | file | depends_on | status`
- item DAG 或执行顺序
- 上游输入和下游 handoff
- slot 级完成标准
- 测试或验收策略摘要

manifest 不得包含完整实现步骤；完整步骤必须写入 item 文件。

## Item Contract
每个 item 文件必须包含：
- `id`
- `title`
- `agent`
- backend/test item 必须额外包含 `test_item_type: authoring|runner`
- backend/test runner item 必须包含 `uses_skill: .claude/skills/backend-test-run/SKILL.md`；authoring item 必须为 `uses_skill: none` 或省略
- `scope`
- `inputs`
- `steps`
- `expected_files`
- `validation`
- `depends_on`
- `handoff_summary`
- `completion_criteria`

## 拆分阈值
命中任一条件必须继续拆分：
- 预计超过 1 天。
- 预计修改超过 5 个核心文件。
- 跨越超过 2 个领域模块或页面域。
- 超过 8 个主要步骤。
- 单个 item 文件预计超过 12KB，且不是验收清单。
- scope 中包含两个可独立交付、独立验证的主交付物（例如 `A + B`、两个页面、页面 + 弹窗、helper + 场景测试）。
- 单个 HTTP/API item 同时包含 5 个以上 endpoint、DTO、路由注册和 OpenAPI/schema 更新。
- 单个 demo item 同时创建复用 helper 并覆盖多个完整用户故事或多个业务状态流。

## 拆分建议
- backend dev：数据库/实体、domain、repository、service/use case、HTTP/OpenAPI、外部集成、SDK/API 影响点。
- backend HTTP/API：DTO 与路由骨架、读模型/list/detail、写操作/create/update、状态操作、配置类接口分别拆分；每个 item 必须能用定向 `cargo check` 或场景测试验证。
- backend test：按场景测试 authoring 与测试执行 runner 拆分；不要把创建场景测试和修复实现直到测试通过放在同一个 item。
- backend unit test：不得规划“为新增 struct/DTO/builder/getter/常量补单测”这类低价值 item。
- frontend dev：API/type 适配、schema/query/store、页面主流程、状态与错误处理、权限与空态。
- frontend dev：一个 item 默认只交付一个页面域或一个可复用组件族；seller config、user page、admin page、dialog 等可独立验证的 UI 不应合并。
- frontend test：schema、query options、store/state machine、数据转换、异常边界。
- demo dev：fixtures/helpers、主流程场景、错误/权限场景、可视化演示流分别拆分；不要把 helper 和完整业务流放在同一个 item。
- accept：design consistency、public API contract、business rules、permission/security、test evidence、demo readiness。

## Agent Output Contract
slot agent 输出必须至少包含：
- `slot`: `dev|test|accept`
- `manifest_target_file`
- `manifest_content`
- `items`: item 对象列表，每个 item 包含 `id/file/agent/depends_on/content`
- `item_dag`
- `completion_criteria`
- `handoff_summary`

主流程必须：
- 校验 `slot` 与被调度 agent 是否匹配。
- 校验 item 依赖合法且无环。
- 先写入当前 slot manifest 和 item 文件，再继续调用下游 slot。
- 在当前阶段要求的 slot 结果齐备后再生成 `index.md`。
- 文档写入与 `.state.json` 更新保持同轮完成。

## Backend Finalize
- backend 阶段额外生成 `backend/finalize.md`。
- `finalize.md` 必须固定声明：
  - `/simplify`
  - `cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features`
  - `cargo fmt --all`
  - `uv run scripts/backend-test.py`
  - OpenAPI 导出
  - 前端 API 生成
  - 失败后从失败步骤恢复
- `finalize.md` 不拆 item。

## Backend Test Planning Rules

backend/test slot 必须按新契约生成，不做旧格式兼容：

| 类型 | agent | test_item_type | uses_skill | depends_on |
|---|---|---|---|---|
| authoring | backend-test | authoring | none | 对应 backend-dev item |
| runner | backend-test | runner | `.claude/skills/backend-test-run/SKILL.md` | 对应 authoring item |

authoring item：
- 只创建/修改 `*_scenarios.rs`、测试 helper、模块注册。
- inputs 必须包含 User Story/PRD 和上游 dev handoff。
- steps 必须包含测试追溯要求：`User Story`、`Covers`。
- validation 只能是 `cargo check --tests`、`cargo test --no-run` 或建议 runner 命令。
- completion criteria 不得要求目标测试全部通过。

runner item：
- 必须加载 `.claude/skills/backend-test-run/SKILL.md`。
- 只执行定向测试、分析失败、委派 production-code 修复、重测。
- 必须声明 backend-dev 不得修改 `backend/**/tests/scenarios/**` 或任何 `*_scenarios.rs`。
- 测试语义可能错误时，停止并输出诊断报告，由用户决定。

backend/test slot 不规划源文件内单元测试；确有必要的高价值单元测试归入对应 backend/dev item。

依赖规则：
- runner item 必须依赖对应 authoring item。
- accept item 必须依赖 runner item，不能只依赖 authoring item。
- runner item 的 `agent` 必须仍是 `backend-test`；`backend-test-run` 只作为 `uses_skill`。

禁止：
- backend test item 同时包含“写场景测试”和“修复生产代码直到通过”。
- authoring item 的完成标准要求“所有测试通过”。
- runner item 缺少对应 authoring item 依赖。

## 错误处理
| 错误码 | 触发条件 | 用户可见提示 | 恢复动作 | 可重试 |
|---|---|---|---|---|
| `DESIGN_DOC_MISSING` | `.ai/design/[feature].md` 不存在 | 未找到设计文档 | 先运行 `/t-design [feature]` | 是 |
| `STATE_JSON_INVALID` | `.state.json` 格式损坏 | 状态文件解析失败 | 修复 JSON 后重试；或重建任务目录 | 是 |
| `PHASE_INVALID` | `--phase` 非法值 | 非法阶段，仅支持 backend/frontend/demo | 改为合法阶段参数 | 是 |
| `PHASE_BLOCKED` | 前置阶段未完成 | 当前阶段被阻塞 | 先执行 `/t-run [feature] --phase [pre_phase]` | 是 |
| `GENERATE_API_FAILED` | `frontend` 阶段 `npm run generate-api` 失败 | 前端 API 生成失败，当前阶段未生成 | 修复后重试 | 是 |
| `AGENT_OUTPUT_INVALID` | slot agent 返回缺少 manifest/items 或 item 必填字段 | agent 输出格式非法 | 重新调用该 agent 或终止生成 | 是 |
| `ITEM_DAG_INVALID` | item 依赖缺失或成环 | 子任务依赖非法 | 修复依赖后重试该 slot | 是 |

## 示例
```text
用户输入:
/t-task sample-feature --phase backend

期望响应:
已生成 backend 阶段任务：
- backend/index.md
- backend/dev.md + backend/dev/*.md
- backend/test.md + backend/test/*.md
- backend/accept.md + backend/accept/*.md
- backend/finalize.md

下一步: /t-run sample-feature --phase backend
```

## 禁止事项
- 生成或依赖旧状态字段。
- 生成或依赖 `agents` 根字段。
- 支持旧参数。
- 生成旧结构文件，如 `backend-dev.md`、`backend-test.md`、`README.md`、`agents.json`。
- 用 `index.md` 或 slot manifest 代替 item 文件执行。
- 一个 agent 一次性返回整阶段可执行正文而不拆 item。
- 单个 item 超过拆分阈值。
- 生成缺少 `test_item_type: authoring|runner` 的 backend test item。
- 生成 `agent: backend-test-run` 的 item。

## 引用
- `.claude/commands/t-task.md`
- `.claude/commands/t-run.md`
- `.claude/commands/t-backend-finalize.md`
- `.claude/skills/task-planner/references/context-isolator.md`
- `.claude/skills/task-planner/references/phase-validator.md`
- `.claude/skills/task-planner/references/phase-index-generator.md`
