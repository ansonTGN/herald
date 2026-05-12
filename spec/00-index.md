# Spec Index

Herald 执行规范索引，按领域组织。

使用原则：

- `development.md` 是各领域的事实型主规范，只回答“当前仓库实际上怎么组织、哪些稳定约束必须遵守”。
- `testing.md` 只回答测试策略、入口和边界，不再重复第二套架构真相。
- demo 主指南是入口页，不再承担排障大全或教学手册职责。
- `index.md` 负责导航与分流，agent 文档负责执行门禁与验收。

## Core
- [环境与测试总览](/spec/core/environment-and-testing-guide.md)
- [质量总规范](/spec/core/quality.md)

## Backend
- [后端规范入口](/spec/backend/index.md)
- [后端开发规范](/spec/backend/development.md)
- [后端测试规范](/spec/backend/testing.md)

## Frontend
- [前端规范入口](/spec/frontend/index.md)
- [前端开发规范](/spec/frontend/development.md)
- [前端测试规范](/spec/frontend/testing.md)
- [data-testid 规范](/spec/agents/frontend/testid-standards.md)

## Demo
- [Demo E2E 测试规范](/spec/demo/e2e-testing.md)
- [Demo POM 指南](/spec/demo/pom-guide.md)
- [Demo 选择器策略](/spec/agents/demo/selector-strategy.md)
- [Demo 测试维护指南](/spec/demo/test-maintenance.md)
- [Demo 诊断指南](/spec/demo/diagnose-guide.md)
- [诊断报告模板](/spec/demo/templates/diagnose-report-template-v3-minimal.md)

## Product
- [PRD 编写规范](/spec/product/prd.md)
- [用户故事编写规范](/spec/product/user-story.md)

## Agents
- [Agent 专项规范目录](/spec/agents/)
- [Backend Agent 质量验收规范](/spec/agents/backend/quality.md)
- [Frontend Agent 质量验收规范](/spec/agents/frontend/quality.md)
- [Demo Agent 质量验收规范](/spec/agents/demo/quality.md)
