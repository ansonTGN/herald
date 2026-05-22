---
name: prd-manager
description: 创建或更新 PRD 文档，并在缺失或过时时先同步相关 user story。仅在用户明确执行 `/t-prd [feature]` 或明确要求维护 PRD 时使用。
argument-hint: [feature-name]
disable-model-invocation: true
allowed-tools:
  - AskUserQuestion
  - Read
  - Glob
  - Grep
  - Write
  - Bash
---

# PRD 创建 / 更新

## 优先级

共享约定：`spec/core/agent-conventions.md`

## 适用范围

这是一个有副作用的任务型 skill，负责先补或修 user story，再创建或更新 PRD 文档。

不要用它做：
- PRD 完整性检查
- 用户故事质量检查

这些工作统一交给 `/t-prd-check` 或其他专用命令。

## 目标

基于现有 user story、PRD 索引和用户补充信息，先补齐或修订必要的 user story，再创建或更新一份 PRD，供后续 `/t-design` 使用。

## 参数要求

命令格式：
```bash
/t-prd [feature-name]
```

其中：
- `$0` 必须是 feature 名称

如果参数不合法，立即终止并提示正确用法。

## 核心约束

- PRD 必须写入 `docs/prd/<domain>/$0.md`
- `<domain>` 只能是现有一级目录：`auth`、`billing`、`core`、`integration`
- 不写入 `docs/prd/` 根目录
- user story 优先追加到现有角色文件；只有现有分组明显不适合时才新增单独文件
- PRD 只引用相关用户故事，不复制完整验收文本
- PRD 聚焦产品边界与规则，不承载接口 schema、数据库建表或技术方案
- 如果已有同名 PRD，进入更新模式，而不是整文覆盖式重建
- 更新模式允许同步修订“当前实现状态”、需求语义、范围、约束和引用
- 更新 PRD 时按模板重整关键章节，保留仍然有效的业务语义和交叉引用
- 完成后，建议用户运行 `/t-prd-check $0`

## 工作流程

### 1. 校验参数

- 检查 `$0` 非空
- 文件名仅允许英文、数字、空格、下划线、连字符
- 拒绝 `..`, `/`, `\`

### 2. 选择目标域

先读取：
- `docs/prd/00-index.md`
- `docs/user-stories/00-index.md`

然后确定目标域：
- `auth`
- `billing`
- `core`
- `integration`

优先根据 feature 名、用户故事和需求语义推断。
如果无法可靠推断，再用 `AskUserQuestion` 询问一次目标域。

### 3. 判定创建或更新

检查目标文件：
- `docs/prd/<domain>/$0.md`

执行规则：
- 文件不存在：进入创建模式
- 文件已存在：进入更新模式
- 更新模式下先读取现有 PRD，提取仍然有效的业务规则、交叉引用、实现状态和结构差异
- 不以“是否覆盖整文”作为默认问题，只有 feature 指向多个候选 PRD 时才提问澄清

### 4. 收集最小必要信息

如果当前上下文不够，使用 `AskUserQuestion` 只补齐这些信息：
1. 功能目标
2. 相关角色
3. 范围边界
4. 是否需要后端 API
5. 是否需要前端实现
6. 关键依赖或前置能力

其中：
- “是否需要后端 API” 只用于判断是否写入 API 能力边界、权限原则和接口约束，不展开端点清单
- “是否需要前端实现” 只用于判断是否写入页面入口、关键交互和状态反馈约束，不展开组件实现方案

角色名称应优先使用仓库既有体系，例如：
- `Admin Realm`
- `Realm Admin`
- `Regular User`
- `Third-Party App`
- `TOTP User`
- `Billing User`
- `Points Admin`
- `Points User`

如果需要新建或补充 user story，还必须拿到：
7. 目标用户价值
8. 至少 1 个主验收场景
9. 默认优先级（P0/P1/P2）

### 5. 检查并补齐 user story

先读取：
- `docs/user-stories/00-index.md`
- `docs/user-stories/_README.md`
- `docs/user-stories/_roles.md`
- `spec/product/user-story.md`

然后搜索真实目录：
- `docs/user-stories/**/*.md`

执行规则：
- 如果已存在足够覆盖该功能的 user story，直接引用，不重复创建
- 如果只缺少少量场景，优先在对应角色现有文件中追加故事
- 如果现有 user story 语义过时或无法支撑当前 PRD，允许同步修订
- 如果现有角色文件都不适合，才创建新的 user story 文件

优先复用现有角色文件，例如：
- `01-admin-realm-user-stories.md`
- `02-realm-admin-user-stories.md`
- `03-regular-user-user-stories.md`
- `04-third-party-app-user-stories.md`
- 以及仓库内已有的专项 story 文件

新增或补充 user story 时必须遵循：
- 使用 `spec/product/user-story.md` 的结构和约束
- 引用 `docs/user-stories/_roles.md`
- 聚焦用户行为和价值，不写 API、数据表、实现细节
- 使用 GWT 风格验收标准

如需新增/补充，使用 [user-story-template.md](user-story-template.md) 作为结构模板。

### 6. 关联用户故事与现有文档

搜索真实目录：
- `docs/user-stories/**/*.md`
- `docs/prd/**/*.md`

优先从索引定位，再读取相关明细。

至少提取：
- 用户故事 ID、标题、优先级、来源文件
- 相关业务规则或现有 PRD 交叉引用
- 已有能力边界，避免重复定义
- 现有 PRD 中仍然成立的范围、当前实现状态和待补充缺口

如果补齐后仍没有足够用户故事：
- 继续生成 PRD
- 在文档中显式标记“待补充用户故事”

### 7. 创建或更新 PRD

使用 [template.md](template.md) 作为模板，写入：
- `docs/prd/<domain>/$0.md`

文档至少包含：
- 相关用户故事
- 范围界定
- 需求概述
- 当前实现状态
- 功能需求
- API 相关约束（如适用）
- 前端/交互约束（如适用）
- 相关文件索引
- 参考资料

不适用的章节保留并标记“不适用”。

创建或更新 PRD 时必须遵循：
- 可以写接口能力范围、访问控制原则、租户/realm 边界、兼容性要求、相关接口说明位置
- 可以写前端页面入口、关键交互、可见性、反馈约束
- 禁止写具体端点、请求参数表、响应字段表、HTTP 状态码列表
- 禁止写数据库表结构、迁移方案、DDL、Rust/TypeScript 类型定义
- 需要技术细节时，引用或建议补充 `/t-design` 产出的技术设计文档
- 更新模式下优先按模板重整关键章节，而不是只做零散 patch
- 更新模式必须清理已过时的结构、错误范围界定和失效引用
- 更新模式必须保留仍然有效的人工补充和业务语义，避免机械覆盖

### 8. 收尾输出

完成后明确说明：
- 本次动作（创建或更新）
- user story 文件路径和变更方式（新增/追加/修订/仅引用）
- 文档路径
- 所属域
- 需要重点补充或确认的部分
- 下一步：`/t-prd-check $0`
- 如需进入设计：`/t-design $0`

## 失败处理

- 参数缺失：终止并给出 `/t-prd [feature-name]`
- 目标域无法判断且用户未提供：终止并要求选择域
- 定位到多个候选 PRD：终止并要求澄清 feature 或域
- 文件无法写入：终止并报告
- user story 信息不足：先补问；仍不足则继续，但在 PRD 中写出缺口

## 附加资源

- PRD 模板：`template.md`
- User Story 模板：`user-story-template.md`
