# Herald Skills 使用指南

## 核心工作流

```
用户故事 → /t-design → /t-task → /t-run
```

**豁免前缀**: `bugfix-`, `refactor-`, `doc-`, `test-`, `style-`

## Slash Commands

### DDD 流程

| 命令 | 功能 | 输出位置 |
|------|------|----------|
| `/t-prd [feature]` | 创建或更新产品需求文档（可选） | `docs/prd/<domain>/<feature>.md` |
| `/t-design [feature]` | 生成技术设计文档 | `.ai/design/[feature].md` |
| `/t-task [feature]` | 生成任务规划 | `.ai/task/[feature]/` |
| `/t-run [feature]` | 执行任务，生成代码 | 后端/前端代码 |

### 质量检查

| 命令 | 功能 | 输出位置 |
|------|------|----------|
| `/t-task-check [feature]` | 任务规划质量评分（100分制） | `.ai/quality/` |
| `/t-ddd-check [feature]` | DDD 合规性检查 | `.ai/quality/` |

### Demo 测试

| 命令 | 功能 | 说明 |
|------|------|------|
| `/t-demo-run [文件路径]` | 运行演示测试 | headless 模式 |
| `/t-demo-run-all [mode]` | 运行所有演示测试 | fast/slow 模式 |
| `/t-demo-accept <file>` | Demo 测试验收 | 代码质量验证 |

### 其他工具

| 命令 | 功能 |
|------|------|
| `/t-consistency-check` | 后端一致性检查（PRD vs 代码） |

## Subagents

### 后端

| Agent | 角色 |
|-------|------|
| `backend-dev` | Rust API 功能实现 |
| `backend-test` | 场景测试开发 |
| `backend-accept` | 后端验收（只读） |
| `backend-consistency` | 后端一致性检查（只读） |

### 前端

| Agent | 角色 |
|-------|------|
| `frontend-dev` | React 功能实现 |
| `frontend-test` | 集成测试开发 |
| `frontend-accept` | 前端验收（只读） |

### Demo

| Agent | 角色 |
|-------|------|
| `demo-dev` | E2E 演示测试开发 |
| `demo-accept` | Demo 测试验收（只读） |
| `demo-diagnose` | Demo 测试诊断（只读） |

## 环境脚本

| 脚本 | 功能 |
|------|------|
| `scripts/demo-test-runner.py` | 运行 Demo 测试⭐推荐 |
| `scripts/test-start.py` / `scripts/test-stop.py` | 后端测试环境 |


## 文档位置

| 类型 | 位置 |
|------|------|
| 用户故事 | `docs/user-stories/*.md` |
| 技术设计 | `.ai/design/[feature].md` |
| 任务规划 | `.ai/task/[feature]/` |
| 执行记录 | `.ai/execution/[feature]/` |
| 质量报告 | `.ai/quality/` |
