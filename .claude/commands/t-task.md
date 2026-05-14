---
description: >
  基于设计文档生成实施任务。固定使用分阶段 + 子任务模型，不保留旧模式。
argument-hint: [任务名称] [--phase <backend|frontend|demo>]
allowed-tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Task
  - Write
  - Bash
---

# 任务规划生成

## Purpose
- 从 `.ai/design/[feature].md` 生成 `.ai/task/[feature]/` 任务目录和 `.state.json`。
- 固定使用 `phase -> slot -> item` 模型：
  - `phase`: `backend | frontend | demo`
  - `slot`: `dev | test | accept`，backend 额外有 `finalize`
  - `item`: slot 下的最小可执行子任务文件
- `index.md` 是阶段总览。
- `dev.md`、`test.md`、`accept.md` 是 slot manifest，只做导航、依赖和完成标准。
- `dev/*.md`、`test/*.md`、`accept/*.md` 是唯一可由 `/t-run` 直接执行的任务输入。
- backend 的 `finalize.md` 是验收后的固定收口流程，由 `/t-backend-finalize [feature]` 执行。

## Args
| 参数 | 说明 |
|---|---|
| `[feature]` | 功能名（必填） |
| `--phase <backend\|frontend\|demo>` | 指定阶段生成；未指定时自动选择第一未完成阶段 |

## Preconditions
- `.ai/design/[feature].md` 必须存在。
- 阶段前置必须完成：

| 目标阶段 | 前置阶段要求 |
|---|---|
| `backend` | 无 |
| `frontend` | `backend == completed` |
| `demo` | `frontend == completed` |

- `frontend` 阶段生成前必须先执行：
  - `cd frontend && npm run generate-api && cd ../`
  - 命令失败时立即终止，不生成当前阶段任务文件，也不调度 `frontend-dev/test/accept`。

## Output Layout
backend 阶段：
```text
.ai/task/[feature]/backend/
├── index.md
├── dev.md
├── dev/
│   ├── BE-D01-*.md
│   └── ...
├── test.md
├── test/
│   ├── BE-T01-*.md
│   └── ...
├── accept.md
├── accept/
│   ├── BE-A01-*.md
│   └── ...
└── finalize.md
```

frontend 阶段：
```text
.ai/task/[feature]/frontend/
├── index.md
├── dev.md
├── dev/FE-D01-*.md
├── test.md
├── test/FE-T01-*.md
├── accept.md
└── accept/FE-A01-*.md
```

demo 阶段：
```text
.ai/task/[feature]/demo/
├── index.md
├── dev.md
├── dev/DE-D01-*.md
├── accept.md
└── accept/DE-A01-*.md
```

## State Shape
`.state.json` 必须使用当前唯一结构，不包含旧状态字段或 `agents` 根字段：

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
  "metadata": {
    "design_document": ".ai/design/sample-feature.md",
    "created_at": "<timestamp>",
    "updated_at": "<timestamp>"
  }
}
```

## Generation Flow
1. 校验 `.ai/design/[feature].md` 存在。
2. 解析 `[feature]` 和 `--phase`；未传 `--phase` 时自动选择第一未完成阶段。
3. 校验阶段前置状态。
4. 若目标阶段为 `frontend`，先执行 `cd frontend && npm run generate-api && cd ../`。
5. 按当前阶段 slot 串行调度 agent：
   - backend/frontend: `dev -> test -> accept`
   - demo: `dev -> accept`
6. 每个 slot agent 必须返回：
   - slot manifest 正文
   - item 文件集合
   - item DAG
   - slot completion criteria
   - handoff summary
7. 主流程在每个 slot 返回后立即写盘：
   - `<phase>/<slot>.md`
   - `<phase>/<slot>/<ITEM-ID>-*.md`
8. 下游 slot prompt 必须包含：
   - 上游 slot manifest 路径
   - 上游 item 清单与 DAG
   - 上游 handoff summary
   - 已写入的上游 item 文件路径
9. 当前阶段所有 slot 齐备后生成 `<phase>/index.md`。
10. 写入或更新 `.state.json`。
11. 返回下一步建议：`/t-run [feature] --phase [phase]`。

## Item Contract
每个 item 文件必须包含：
- `id`: 稳定 ID，例如 `BE-D01`、`FE-T02`、`DE-A01`
- `title`: 子任务标题
- `agent`: 执行 agent
- `scope`: 本 item 的明确边界
- `inputs`: 必读设计、规范、上游 handoff 和相关文件
- `steps`: 可执行步骤
- `expected_files`: 预计新增或修改的文件/目录
- `validation`: 该 item 的最小验证命令或检查方式
- `depends_on`: 依赖的 item ID 列表
- `handoff_summary`: 完成后传给下游 item/slot 的摘要要求
- `completion_criteria`: 完成标准

## Splitting Rules
必须拆分 item，如果任一条件成立：
- 预计超过 1 天才能完成。
- 预计修改超过 5 个核心文件。
- 跨越超过 2 个领域模块或页面域。
- 超过 8 个主要步骤。
- 单个 item 文件预计超过 12KB 且不是验收清单。
- scope 中包含两个可独立交付、独立验证的主交付物（例如 `A + B`、两个页面、页面 + 弹窗、helper + 场景测试）。
- 单个 HTTP/API item 同时包含 5 个以上 endpoint、DTO、路由注册和 OpenAPI/schema 更新。
- 单个 demo item 同时创建复用 helper 并覆盖多个完整用户故事或多个业务状态流。

推荐拆分方式：
- backend dev：数据库/实体、domain、repository、service/use case、HTTP/OpenAPI、外部集成、SDK/API 影响点。
- backend HTTP/API：DTO 与路由骨架、读模型/list/detail、写操作/create/update、状态操作、配置类接口分别拆分；每个 item 必须能用定向 `cargo check` 或场景测试验证。
- backend test：domain/unit、repository/integration、API scenario、regression、高风险业务规则。
- frontend dev：API/type 适配、schema/query/store、页面主流程、状态与错误处理、权限与空态。
- frontend dev：一个 item 默认只交付一个页面域或一个可复用组件族；seller config、user page、admin page、dialog 等可独立验证的 UI 不应合并。
- demo dev：先拆 fixtures/helpers，再拆主流程、异常/校验场景、权限场景；不要把 helper 和完整业务流放在同一个 item。
- accept：design consistency、public API contract、business rules、permission/security、test evidence、demo readiness。

## Backend Finalize
- backend 阶段必须额外生成 `<phase>/finalize.md`。
- `finalize.md` 必须明确：
  - `/simplify` 目标范围
  - `cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features`
  - `cargo fmt --all`
  - 全量 `uv run scripts/backend-test.py`
  - OpenAPI 导出与前端 API 生成
  - 失败后从失败步骤恢复
- `finalize.md` 不拆 item，不由 `/t-run` 执行。

## Forbidden
- 生成或依赖旧状态字段。
- 生成或依赖 `agents` 根字段。
- 支持旧参数。
- 生成根级 `backend-dev.md`、`backend-test.md`、`frontend-dev.md`、`agents.json` 等旧结构文件。
- 把 `dev.md`、`test.md`、`accept.md` 当作 `/t-run` 的直接执行输入。
- 在单个 item 中塞入跨多模块、多天或不可恢复的大任务。
- 当前阶段 slot 并行生成；slot 必须按依赖串行。
- 未写入上游 manifest 和 item 文件就调用下游 slot agent。
- backend 阶段遗漏 `finalize.md`。

## Failure
- 设计文档不存在：提示先运行 `/t-design [feature]`。
- 前置阶段未完成：返回阻塞阶段与阻塞 items。
- `frontend` 阶段 `npm run generate-api` 失败：立即终止，并返回失败命令与错误摘要。
- 任一 slot agent 生成失败：终止本次任务生成，不写入该 slot 的成功状态，并返回失败 agent 与失败原因。
- slot agent 返回 item 缺少必填字段、依赖不存在或形成环：拒绝写入成功状态，要求重新生成该 slot。

## Examples
```bash
# 生成 backend 阶段任务
/t-task realm-user-rbac --phase backend

# 未指定 phase 时自动选择第一未完成阶段
/t-task realm-user-rbac
```

期望响应：
```text
已生成 backend 阶段任务：
- index.md
- dev.md + dev/*.md
- test.md + test/*.md
- accept.md + accept/*.md
- finalize.md

状态已更新：phase=backend, phases.backend.generated_at=<timestamp>
下一步: /t-run realm-user-rbac --phase backend
```

## 相关引用
- `.claude/skills/task-planner/SKILL.md`
- `.claude/skills/task-planner/references/context-isolator.md`
- `.claude/skills/task-planner/references/phase-validator.md`
- `.claude/skills/task-planner/references/phase-index-generator.md`
