# backend-test-run Skill

这是 backend/test `test_item_type: runner` 的执行 skill 索引。

详细规则只维护在：

- `.claude/skills/backend-test-run/SKILL.md`

核心边界：

- 运行定向后端测试、解析失败、分类归因、编排 production-code 修复和重测。
- 不编写新场景测试；场景测试 authoring 属于 `backend-test` authoring item。
- 不修改断言、状态码预期、权限预期或业务规则预期。
- 生产代码问题委派 `backend-dev`，并要求不得修改 `backend/**/tests/scenarios/**` 或任何 `*_scenarios.rs`。
- 测试语义可能错误时停止并输出诊断报告，由用户决定。
