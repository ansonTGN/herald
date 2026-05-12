# 环境与测试总览

本文档是 Herald 项目环境与测试体系的统一入口，负责回答两个问题：

- 当前任务应该使用哪个环境？
- 当前需求应该写哪一层测试？

执行细节请跳转到对应专项文档，避免多处重复维护。

## 环境选择决策

| 任务 | 推荐环境 | 入口命令 | 详细说明 |
|---|---|---|---|
| 人类开发者日常开发 | 开发环境 | `uv run scripts/dev-start.py` | [AGENTS.md 环境规则](/AGENTS.md) |
| AI 运行 Demo/E2E | Demo 环境 | `uv run scripts/demo-test-runner.py demo/e2e/[test].ts` | [Demo 测试指南](/spec/demo/e2e-testing.md#4-标准命令) |
| 后端场景测试 | 后端测试环境 | `uv run scripts/backend-test.py` | [后端测试指南](/spec/backend/testing.md#环境启动) |
| 前端组件测试 | 无需后端环境 | `cd frontend && npm run test:run` | [前端测试指南](/spec/frontend/testing.md) |

说明：
- `uv run scripts/backend-test.py` 是后端测试主入口；如需显式管理测试环境，使用 `uv run scripts/test-start.py` / `uv run scripts/test-stop.py`。
- Demo 环境与开发环境端口冲突，不可同时运行。

## 测试策略决策

Herald 采用“以 Demo 为主”的分层测试策略。

| 需求类型 | 首选测试层级 | 位置 | 详细说明 |
|---|---|---|---|
| 完整用户故事、跨模块流程、产品展示路径 | Demo 测试 | `demo/e2e/` | [Demo 测试指南](/spec/demo/e2e-testing.md) |
| 表单验证、状态切换、边界场景、MSW Mock | 前端组件测试 | `frontend/src/**/__tests__/` | [前端测试指南](/spec/frontend/testing.md) |
| API、业务流程、权限与数据库交互 | 后端场景测试 | `backend/*/tests/scenarios/` | [后端测试指南](/spec/backend/testing.md) |

禁止重复：
- 已由 Demo 覆盖的完整用户故事，不再重复编写组件级“同路径”测试。

## 全局约束

- 环境必须通过脚本管理，禁止直接以 `docker run`、`cargo run`、`npm run dev` 替代项目脚本。
- AI 默认不启动开发环境，除非用户明确要求。
- Agent 执行规则以 `AGENTS.md` 为准；本文档只做决策导流。

## 详细指南入口

- 环境与执行总规则：[`AGENTS.md`](/AGENTS.md)
- Demo 测试与 Demo 环境：[`../demo/e2e-testing.md`](/spec/demo/e2e-testing.md)
- 后端场景测试与测试环境：[`../backend/testing.md`](/spec/backend/testing.md)
- 前端组件测试：[`../frontend/testing.md`](/spec/frontend/testing.md)
- 质量总规范：[`quality.md`](quality.md)

## 快速 FAQ

### 后端测试前是否必须手动执行 `test-start.py`？

不必须。优先使用 `uv run scripts/backend-test.py`。当需要长时间复用测试环境或排查环境问题时，再显式使用 `test-start.py` / `test-stop.py`。

### Demo 环境与开发环境能否并行运行？

不能。两者存在端口冲突。

### 为什么入口文档不再放完整命令大全？

为了降低漂移风险。入口文档负责“选路径”，专项文档负责“怎么做”。

---

## 任务规划与执行

Herald 支持两种模式：
- 分阶段模式（推荐）：`backend -> frontend -> demo`
- 兼容模式：一次性全量生成并执行

本文件仅做导流，命令细节以以下文档为准：
- 任务生成：[`../../.claude/commands/t-task.md`](/.claude/commands/t-task.md)
- 任务执行：[`../../.claude/commands/t-run.md`](/.claude/commands/t-run.md)
- 后端收口：[`../../.claude/commands/t-backend-finalize.md`](/.claude/commands/t-backend-finalize.md)

最小命令集：
```bash
/t-task [feature] --phase backend
/t-run [feature] --phase backend
/t-backend-finalize [feature]
```

状态文件（最小结构）：
```json
{
  "version": "2.0",
  "feature": "[feature]",
  "phase": "backend",
  "phases": {
    "backend": {"status": "awaiting_finalize", "generated_at": "..."},
    "frontend": {"status": "pending", "generated_at": null},
    "demo": {"status": "pending", "generated_at": null}
  },
  "agents": {},
  "dependencies": {}
}
```
