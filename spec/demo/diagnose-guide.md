# Demo-Diagnose Guide

## Overview

`demo-diagnose` 是只读诊断代理，用于分析 Demo 测试失败并输出结构化诊断报告。

职责：
- 提取失败上下文
- 先验证测试代码与用户故事是否一致
- 分类问题来源
- 在 API 类失败时生成复现命令
- 输出统一格式的诊断报告

限制：
- 不修改业务代码
- 不负责重启环境或执行修复
- 不依赖历史错题库才能完成诊断

## Workflow

诊断流程固定为 5 步：

1. **收集失败证据**
   - `demo/test-results/runs/<runId>/playwright-output.log`
   - `demo/test-results/unified-logs/*`
   - `log/backend-demo.log`
   - 失败测试文件、相关 page object、必要的前端组件与用户故事

2. **优先检查测试本身**
   - 测试场景与用户故事是否一致
   - 选择器是否有效
   - 测试数据是否满足约束
   - 断言和等待是否合理
   - 流程是否缺少登录、导航、准备或清理步骤

3. **分类问题来源**
   - `TEST`
   - `DATA`
   - `AUTH`
   - `FRONTEND`
   - `BACKEND`
   - `ENV`

4. **API 失败时生成复现命令**
   - 从 `*-network.json` 提取请求信息
   - 必要时生成 curl 命令
   - 写入报告的 `API复现命令` 章节

5. **输出诊断报告**
   - 写入 `.ai/diagnose/`
   - 使用 v3.0 极简协议

## Report Contract

诊断报告的唯一格式标准是：
- `.claude/protocols/diagnostic-report-v3-minimal.md`

示例模板：
- `spec/demo/templates/diagnose-report-template-v3-minimal.md`

每份报告必须包含固定元字段：
- `problem_code`
- `severity`
- `recommended_agent`
- `confidence`

固定章节顺序：
1. `失败上下文`
2. `问题分类`
3. `根本原因分析`
4. `修复建议`
5. `相关文件`
6. `API复现命令（来自 network.json）`，可选
7. `快速验证命令`

## Classification Rules

### TEST

适用场景：
- 选择器拼写错误
- 断言错误
- 等待条件不正确
- 测试流程缺步骤

推荐处理方：
- `demo-dev`

### DATA

适用场景：
- 测试数据格式不合法
- 数据唯一性冲突
- 缺少必要前置数据

推荐处理方：
- `demo-dev`

### AUTH

适用场景：
- 401 / 403
- 登录状态异常
- 权限配置不满足场景预期

推荐处理方：
- `backend-dev`

### FRONTEND

适用场景：
- API 正常但页面渲染失败
- 元素存在但不可见/被遮挡
- 路由、交互或前端状态处理错误

推荐处理方：
- `frontend-dev`

### BACKEND

适用场景：
- API 500 / 502 / 503
- 后端日志明确报错
- 查询逻辑或序列化异常

推荐处理方：
- `backend-dev`

### ENV

适用场景：
- `ECONNREFUSED`
- 服务未启动
- 端口冲突
- 基础依赖不可达

推荐处理方：
- `manual`

## Best Practices

1. 先验证测试代码，再判断是不是运行时问题。
2. 每个结论都必须能回指到日志、代码或请求证据。
3. 只输出一个主问题类型，避免模糊归因。
4. API 复现命令只在 API 类问题时输出。
5. 用离散等级表达结论强度：`high`、`medium`、`low`。
6. 如果证据不足以得出稳定结论，降低 `confidence`，不要虚构细节。

## Related Documentation

- [Demo E2E Testing Guide](e2e-testing.md)
- [Environment and Testing Guide](/spec/core/environment-and-testing-guide.md)
- [Diagnostic Report Template](templates/diagnose-report-template-v3-minimal.md)
