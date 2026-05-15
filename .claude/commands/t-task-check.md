---
description: >
  任务规划质量检查。对 .ai/task/[feature]/ 下的新 item 任务结构进行量化评分并输出修复清单。
argument-hint: [任务名称] [--phase <backend|frontend|demo>]
allowed-tools:
  - Read
  - Glob
  - Grep
  - Task
  - Write
---

# 任务规划质量检查

## 优先级

`AGENTS.md` 是最高约束。本命令只读检查任务规划；若任务文档、设计文档、spec 或仓库事实冲突，停止并说明，不要把规范冲突伪装成单方错误。

## 目标
- 评估任务文档可执行性与一致性。
- 验证 `phase -> slot -> item` 结构。
- 给出可复查的 100 分量化结果。
- 输出 P0/P1/P2 修复清单。
- 必须按当前阶段调度对应 sub agent 做专业校验，再由主流程聚合结论。

## 事实优先级（强制）
- 最终结论的证据优先级必须为：`docs/` 与 `spec/` 与仓库实际文件 > 当前 phase 任务文档 > sub agent 评审意见。
- sub agent 只能产出候选问题，不能直接充当最终裁决。
- 若 agent 结论与仓库事实、规范文本或现有目录结构冲突，必须以仓库事实和规范为准。
- 对路径、命令、阶段状态、测试位置、职责边界等可从仓库发现的事实，主流程必须再次自行核验。
- 若规范之间存在冲突，必须在报告中标记为“规范冲突/待澄清”，不得直接记为 P0。

## 使用方式
```bash
/t-task-check [feature] [--phase <backend|frontend|demo>]
```

| 参数 | 说明 |
|---|---|
| `[feature]` | 功能名（必填） |
| `--phase <phase>` | 指定阶段检查；未指定时检查 `.state.json` 当前阶段 |

## 输入范围
- 设计文档：`.ai/design/[feature].md`
- 状态文件：`.ai/task/[feature]/.state.json`
- 阶段目录：`.ai/task/[feature]/[phase]/`
- 阶段索引：`index.md`
- slot manifest：
  - backend/frontend: `dev.md`、`test.md`、`accept.md`
  - demo: `dev.md`、`accept.md`
- item 文件：
  - backend/frontend: `dev/*.md`、`test/*.md`、`accept/*.md`
  - demo: `dev/*.md`、`accept/*.md`
- backend 额外文件：`finalize.md`

## Schema 校验
`.state.json` 必须满足：
- 不包含旧状态字段或 `agents` 根字段。
- `feature` 必须存在。
- `phase` 必须存在且为 `backend|frontend|demo`。
- `phases` 必须包含 `backend/frontend/demo`。
- `phases[*].status` 必须存在。
- `tasks[phase]` 必须存在。
- backend/frontend 必须包含 `dev/test/accept` slot；demo 必须包含 `dev/accept` slot。
- 每个 slot 必须包含：
  - `status`
  - `manifest`
  - `items`
- 每个 item 必须包含：
  - `status`
  - `file`
  - `agent`
  - `depends_on`
- backend 必须包含 `tasks.backend.finalize.file` 和 `tasks.backend.finalize.status`。

任一项缺失或非法即返回 `TASK_SCHEMA_INVALID`。

## 执行流程
1. 校验设计文档是否存在。
2. 读取 `.state.json` 并验证 schema。
3. 若指定 `--phase`，仅检查该阶段；否则检查当前阶段。
4. 校验阶段依赖正确性。
5. 读取阶段目录下的 `index.md`、slot manifest 和 item 文件。
6. 校验 item DAG：
   - item ID 唯一
   - `depends_on` 指向存在 item
   - 无依赖环
   - item 文件路径与 state 一致
   - manifest 覆盖全部 items
7. 检查旧结构残留：
   - 根级 `backend-dev.md`
   - 根级 `backend-test.md`
   - 根级 `frontend-dev.md`
   - 根级 `demo-dev.md`
   - 根级 `README.md`
   - 根级 `agents.json`
   - 其他混用旧结构的任务文件
8. 验证 item 文件结构与内容：
   - 必须包含 `id/title/agent/scope/inputs/steps/expected_files/validation/depends_on/handoff_summary/completion_criteria`
   - 不得把完整 slot 内容塞进一个 item
   - 超过拆分阈值必须有合理说明，否则记 P1
   - scope 中包含两个可独立交付、独立验证的主交付物时，必须拆分，否则记 P1
   - 单个 HTTP/API item 同时包含 5 个以上 endpoint、DTO、路由注册和 OpenAPI/schema 更新时，必须拆分，否则记 P1
   - 单个 demo item 同时创建复用 helper 并覆盖多个完整用户故事或多个业务状态流时，必须拆分，否则记 P1
   - backend/test item 必须声明 `test_item_type: authoring|runner`，缺失直接记 P0，不做 `steps` fallback
   - `test_item_type: authoring` 必须声明 `uses_skill: none` 或省略 `uses_skill`
   - `test_item_type: runner` 必须声明 `uses_skill: .claude/skills/backend-test-run/SKILL.md`
   - backend/test item 的 `steps`、`validation`、`completion_criteria` 必须与显式 `test_item_type` 自洽
9. 核对设计文档与任务文档的一致性。
10. 调用当前阶段对应 agent 进行专业校验：
   - backend: `backend-dev`, `backend-test`, `backend-accept`
   - frontend: `frontend-dev`, `frontend-test`, `frontend-accept`
   - demo: `demo-dev`, `demo-accept`
11. 聚合 agent 结果并进行主流程复核。
12. 按评分体系生成评分与问题清单。
13. 执行报告一致性自检。
14. 写入报告：`.ai/quality/task-check-[feature]-[YYYYMMDD-HHMMSS].md`。

## Agent Review Contract
任务检查时，当前阶段每个被调度的 agent 输出必须至少包含：
- `score`：该 agent 对当前阶段任务文档的评分
- `findings`：问题列表，包含 `severity`、`file`、`evidence`、`why_it_matters`
- `fixes`：可执行修复建议
- `summary`：1-2 句专业结论

主流程负责：
- 去重和合并 findings。
- 将每条 finding 补全为最终证据记录，至少包含：
  - `status`: `confirmed | disputed | assumption`
  - `task_file`: 任务文档文件与行号
  - `source_of_truth`: 对应规范、设计文档或仓库文件与行号
  - `repo_evidence`: 实际目录、文件或命令证据；若无，则明确写“未找到仓库证据”
  - `why_blocking`: 为什么阻塞执行；若不能证明阻塞，则不得定为 P0
  - `fix`: 最小修复动作
- 按评分体系折算总分。
- 生成统一的 P0/P1/P2 清单。
- 输出 `confirmed`、`disputed`、`assumption` 分类结果。
- 写入最终质量报告。

## 评分体系（总分 100）
| 维度 | 分值 | 说明 |
|---|---:|---|
| 状态文件结构 | 15 | `.state.json` 的 `phase/phases/tasks/slot/items` 结构完整性 |
| 文档完整性 | 15 | `index.md`、slot manifest、item 文件和 backend `finalize.md` |
| Item 可执行性 | 20 | item 足够小、步骤明确、验证命令明确、边界清晰 |
| 内容一致性 | 20 | 与设计文档、PRD、用户故事、仓库路径和术语一致 |
| 依赖与恢复 | 15 | item DAG 合法、handoff 可追溯、失败可恢复 |
| 文档规范 | 10 | Markdown 结构和格式规范 |
| 代码示例质量 | 5 | 示例可读、可执行、不误导 |

## P0（阻塞）
- `.state.json` 缺失或格式错误。
- `.state.json` 包含旧状态字段。
- `.state.json` 包含 `agents` 根字段。
- 缺少 `phase/phases/tasks/status/manifest/items` 必需字段。
- `phase` 字段指向不存在的阶段。
- backend/frontend 阶段缺少 `dev/test/accept` slot。
- demo 阶段缺少 `dev/accept` slot。
- backend 阶段缺少 `finalize.md` 或 `tasks.backend.finalize`。
- 阶段目录缺少 `index.md`、slot manifest 或 item 目录。
- state 声明的 item 文件不存在。
- item 依赖不存在或形成依赖环。
- manifest 未覆盖全部 items。
- 出现旧结构残留并与新结构混用。
- 阶段依赖关系错误。
- 任务文档中的命令、路径、阶段链路经仓库和规范双重验证后，确认会直接导致 `/t-run` 无法执行。
- backend test item 缺少 `test_item_type: authoring|runner`。
- backend test item 同时包含“写/新增场景测试”和“运行失败后修生产代码/委派 backend-dev 直到通过”的混合职责。
- authoring item 的 completion criteria 要求“所有目标测试必须通过”。
- runner item 缺少对应 authoring item 依赖，却执行新测试。
- runner item 的 `agent` 写成 `backend-test-run`；`backend-test-run` 是 skill，不是 agent。
- backend test item 显式字段与 `steps`、`validation` 或 `completion_criteria` 冲突。
- 最终报告中出现 `confirmed P0` 时，必须拒绝进入 `/t-run`。

## P1（重要）
- slot 状态与 item 聚合状态不匹配。
- item 缺少 `scope/inputs/steps/expected_files/validation/handoff_summary/completion_criteria` 任一关键章节。
- item 超过拆分阈值且没有合理说明。
- item 职责混杂，导致一次 agent 调用高概率无法完成。
- item 合并多个可独立交付、独立验证的主交付物。
- HTTP/API item 同时覆盖 5 个以上 endpoint、DTO、路由注册和 OpenAPI/schema 更新。
- demo item 同时创建复用 helper 并覆盖多个完整用户故事或多个业务状态流。
- 下游 item 缺少对上游 handoff 的追溯。
- backend 阶段缺少 `awaiting_finalize` 收口语义。
- `finalize.md` 未限制 `/simplify` 目标范围，或未声明全量测试后的修复/重试规则。
- API 路径、业务规则或测试范围与设计文档严重不一致，但不直接阻塞执行。
- authoring item 缺少 User Story/PRD 输入。
- authoring item 缺少测试追溯要求：`User Story`、`Covers`。
- runner item 未声明 `.claude/skills/backend-test-run/SKILL.md`。
- runner item 未声明 backend-dev 不得修改 `backend/**/tests/scenarios/**` 或任何 `*_scenarios.rs`。
- accept item 只依赖 authoring item，不依赖 runner item。

## P2（优化）
- 代码示例可读性差。
- Markdown 结构可优化。
- 表达不够具体但不影响执行。
- item 命名可读性不足。
- 补充示例、说明、顺序提示即可改进。
- 场景测试模板包含大量 `println!("[Step N]")` 或长分割线注释。
- 新场景测试文件命名不是 `*_scenarios.rs` 且没有合理说明。

## Backend Test Item 识别规则

- backend/test item 必须通过显式 `test_item_type` 判断 authoring vs runner。
- 不允许通过 `steps` 内容推断类型来兼容旧 item。
- `steps` 内容只用于校验显式字段是否自洽。
- `test_item_type: authoring` 表示只写场景测试、helper 和模块注册，只做编译验证。
- `test_item_type: runner` 表示加载 `backend-test-run` skill，运行定向测试、诊断、委派 production fix、重测。
- 同时包含两类动作时按 P0 “混合职责”处理。

## Backend Test Style Checks

新增或修改场景测试时检查：
- 文件默认命名为 `<feature>_scenarios.rs`。
- 测试函数默认命名为 `test_scenario_<feature>_<scenario>_<outcome>`。
- 每个核心测试包含 `User Story` 和 `Covers` 注释。
- 使用简洁 Given/When/Then 注释。
- 不写 `println!("[Step 1] ...")` 或长 banner。
- 优先使用 `backend/api/src/tests/helpers/` 和 `backend/test-support` helper。

## 报告要求
报告必须包含：
- 总分、等级、是否可进入 `/t-run`
- 状态文件验证结果
- 阶段依赖验证结果
- item DAG 验证结果
- 旧结构残留检查结果
- 每个维度得分与扣分证据
- 实际调用的 agent 集合
- `confirmed` / `disputed` / `assumption` 分类摘要
- P0/P1/P2 问题列表（含文件定位、真源定位、仓库证据）
- 明确修复步骤
- “已排除的误报/争议项”小节（如有）

等级建议：
- 90-100：优秀，可进入实施
- 75-89：良好；仅在无 `confirmed P0` 时可进入实施，建议先修 P1
- 60-74：需改进；仅在无 `confirmed P0` 时可进入实施，否则必须先修 P0
- <60：不合格，建议重新规划

## 错误处理
| 错误码 | 触发条件 | 用户可见提示 | 恢复动作 |
|---|---|---|---|
| `DESIGN_DOC_MISSING` | 设计文档不存在 | 未找到设计文档 | 先运行 `/t-design [feature]` |
| `STATE_FILE_MISSING` | 任务目录或 `.state.json` 缺失 | 状态文件不存在 | 运行 `/t-task [feature] --phase backend` 重建 |
| `STATE_JSON_INVALID` | `.state.json` 格式错误 | 状态文件解析失败 | 修复 JSON 后重试；或重建任务目录 |
| `TASK_SCHEMA_INVALID` | 缺少 `phase/phases/tasks/status/manifest/items` 字段 | 任务状态结构不完整 | 运行 `/t-task [feature] --phase [phase]` 重建 |
| `LEGACY_STRUCTURE_FOUND` | 发现旧结构字段或旧任务文件 | 旧结构残留 | 删除旧任务目录后重新运行 `/t-task` |
| `PHASE_INVALID` | `--phase` 不是 `backend|frontend|demo` | 非法阶段，仅支持 backend/frontend/demo | 使用合法参数后重试 |
| `PHASE_DIR_MISSING` | 阶段目录不存在 | 找不到阶段目录 | 运行 `/t-task [feature] --phase [phase]` 生成 |
| `ITEM_DAG_INVALID` | item 依赖缺失或成环 | 子任务依赖非法 | 修复或重新生成该阶段 |
| `REPORT_INCONSISTENT` | 报告中的严重度、总分、准入结论或问题数量互相冲突 | 报告自检失败 | 重新聚合证据并重生成报告 |

信息提示（不阻断）：
- `PHASE_NOT_CURRENT`：指定 `--phase` 非当前阶段时提示“当前阶段为 [state.phase]，继续检查指定阶段”。
- `PHASE_CHECK_AGENT_SET`：展示本次实际调用的 phase agent 集合，便于复查。

## 示例
```bash
/t-task-check sample-feature --phase backend
```

输出：
```text
总分: 92/100 (优秀，可进入实施)

状态文件验证: 通过
Item DAG 验证: 通过
旧结构残留检查: 通过

状态文件结构: 15/15
文档完整性: 14/15
Item 可执行性: 18/20
内容一致性: 19/20
依赖与恢复: 15/15
文档规范: 8/10
代码示例质量: 3/5

Agent 集合: backend-dev, backend-test, backend-accept
问题分类摘要: confirmed=2, disputed=0, assumption=0

P1 问题:
1. BE-D03 超过拆分阈值，建议拆为 repository trait 与 repository implementation 两个 item

下一步: /t-run sample-feature --phase backend
```

## 质量门禁
- 分项分值之和必须等于 100。
- 每个扣分项必须有文件定位。
- 每个 P0/P1 必须同时有任务文档证据和真源证据。
- `confirmed P0 > 0` 时，“是否可进入 /t-run”必须为“否”。
- `disputed` 或 `assumption` 不得计入 P0。
- 结论必须可追溯到证据，且报告内部不得自相矛盾。

## 相关引用
- `.claude/commands/t-task.md`
- `.claude/commands/t-backend-finalize.md`
- `.claude/commands/t-run.md`
- `.claude/skills/task-planner/SKILL.md`
- `.claude/skills/task-planner/references/phase-validator.md`
- `.claude/skills/task-planner/references/phase-index-generator.md`
- `.claude/agents/backend-dev.md`
- `.claude/agents/frontend-dev.md`
- `.claude/agents/demo-accept.md`
