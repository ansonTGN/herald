# Task Output Contract

定义所有 dev/test agent 完成或失败时返回的结构化输出。

## 成功输出

```json
{
  "task_completion": {
    "status": "success|partial|failed",
    "summary": "简要说明",
    "files_modified": ["path/a.tsx"],
    "files_created": [],
    "change_scope": { "backend": false, "frontend": true, "demo": false },
    "tests_to_run": [
      { "layer": "frontend", "command": "...", "reason": "...", "required": true }
    ],
    "next_steps": []
  }
}
```

## 必填字段

- `status`: success | partial | failed
- `change_scope`: 标记影响层
- `tests_to_run`: 修复循环中的 agent 必须返回

## tests_to_run 契约

- `layer`: backend | frontend | demo
- `command`: 可直接执行的命令
- `reason`: 为何需要此回归
- `required`: 是否必须通过，默认 true

允许的命令：
- backend: `uv run scripts/backend-test.py -- [filter]`
- frontend: `cd frontend && npm run test:run -- [pattern]`
- demo: `uv run scripts/demo-test-runner.py "[file]" --run-id [ID] --grep "[title]"`

## 错误输出

```json
{
  "task_completion": {
    "status": "failed",
    "error": {
      "severity": "P0|P1|P2|P3",
      "type": "compilation_error|type_check_error|build_error|runtime_error|logic_error",
      "message": "错误描述",
      "location": "文件路径:行号",
      "suggested_fix": "建议修复方案"
    }
  }
}
```

## 角色扩展字段

- backend-dev: 可加 `compilation`、`tests_written`
- frontend-dev: 可加 `validation_results`、`components_added`、`components_modified`
- backend-test: 用 `traceability` + `suggested_runner_command` 替代 `tests_to_run`
- demo-dev: 保持最小字段
