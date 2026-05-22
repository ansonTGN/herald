# Agent 与 Command 共享约定

## 优先级层级

`AGENTS.md` 是最高约束。每个 agent/command 只定义自身执行边界。若任务、spec、User Story/PRD 或测试语义冲突，停止并说明。

## 共享禁止

- 不把 agent/command 文档当作架构规范第二真相（真相在 `spec/`）
- 不引用不存在的文档段落或伪造行号
- 不绕过 `spec/*/index.md` 的导航关系
- 不在没有证据时凭印象重写项目模式
- 不在完成报告中忽略失败的类型检查、构建或必要测试
- 不静默跳过验证步骤或将未验证代码标记为完成

## Accept Agent 共享限制

所有 accept agent：
- 未经授权不得修改代码
- 每条结论必须标明文件来源
- 禁止空泛建议
