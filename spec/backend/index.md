# Backend 规范入口

backend 规范入口，按“先定位问题，再读对应页面”使用。

## 先读哪一页

| 目标 | 文档 |
| --- | --- |
| 确认当前 backend 架构事实、crate 边界、HTTP/OpenAPI 约束 | [development.md](/spec/backend/development.md) |
| 确认测试位置、统一测试入口、`SchemaTestContext` 用法 | [testing.md](/spec/backend/testing.md) |
| 任务完成前跑什么检查 | [../agents/backend/validation.md](/spec/agents/backend/validation.md) |
| 需要完整验收、环境启动、OpenAPI 完整性检查 | [../agents/backend/quality.md](/spec/agents/backend/quality.md) |

## 使用规则

- `development.md` 是 backend 的事实型主规范，其他 backend 文档不应重写其中的架构事实。
- `testing.md` 只定义测试策略与入口，不定义第二套架构规则。
- `validation.md` 和 `quality.md` 只定义门禁与验收，不扩展业务模板。
- 若发现子页与 `development.md` 冲突，以 `development.md` 为准，并回收冲突内容。
