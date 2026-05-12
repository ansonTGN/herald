---
description: >
  PRD 统一写入口。用于先补齐相关 user story，再根据现有文档状态创建或更新指定 feature 的 PRD 文档。
argument-hint: [feature-name]
allowed-tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Write
  - Bash
---

# PRD 管理

## 使用方式
```bash
/t-prd [feature]
```

## 职责边界
- `/t-prd` 负责先补齐相关 user story，再创建或更新 PRD
- `/t-prd-check` 负责检查 PRD 与用户故事质量
- `/t-prd` 不负责 `check`
- `/t-prd` 产出产品语义文档，不负责生成接口明细、数据库设计或技术实现方案

## 执行流程
1. 校验命令参数，要求使用 `[feature]`。
2. 调用 `prd-manager` skill 检查并补齐相关 user story。
3. 根据 feature、域推断和现有 PRD 状态，自动判定执行“创建”或“更新”。
4. 生成或重整 PRD 正文。
5. 输出结果摘要：本次动作、user story 路径、PRD 路径、所属域、后续建议。

## 失败处理
- 缺失 feature：直接失败并提示参数。
- 目标业务域无法判断：提示选择 `auth|billing|core|integration`。

## 质量门禁
- 创建或更新 PRD 前应尽量具备可引用的 user story；缺失或过时时由 `prd-manager` 先补齐。
- PRD 至少包含：相关用户故事、范围界定、需求概述、当前实现状态、功能需求、API 相关约束（如适用）、前端/交互约束（如适用）、相关文件索引、参考资料。
- PRD 禁止写入：`GET/POST /api/...` 端点清单、请求/响应 schema、HTTP 状态码矩阵、数据库建表/迁移细节、Rust/TypeScript 数据结构示例。
- PRD 完成后建议立即运行 `/t-prd-check [feature]`。

## 相关引用
- `.claude/skills/prd-manager/SKILL.md`
- `.claude/commands/t-prd-check.md`
