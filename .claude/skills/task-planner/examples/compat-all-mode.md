# Example: Compatibility All Mode

## User Input
```bash
/t-task sample-feature --all
```

## Expected Response
```text
已按兼容模式生成全链路任务。
输出目录: .ai/task/sample-feature/
输出文件: index.md, backend-dev.md, ..., demo-accept.md, .state.json
下一步: /t-run sample-feature
```

## State Delta
```json
{
  "phase": "backend",
  "agents": {
    "backend-dev": {"status": "pending"},
    "frontend-dev": {"status": "pending"},
    "demo-dev": {"status": "pending"}
  }
}
```
