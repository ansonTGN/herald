# Realm Settings 产品需求文档 (PRD)

**创建时间**: 2025-01-05
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

- `[US-RA-008]` 配置 Realm 设置 (P0)，来源 `docs/user-stories/core/realm-admin.md`
  - 角色：Realm Admin
  - 摘要：作为 Realm Admin，配置 Realm 设置（Turnstile、注册策略、OAuth Provider），管理本 Realm 的安全和访问控制

- `[US-RA-013]` 配置 Realm 邮件服务 (P0)，来源 `docs/user-stories/core/realm-admin.md`
  - 角色：Realm Admin
  - 摘要：配置邮件发送方式（Resend API 或 SMTP），让本 Realm 独立发送系统邮件

- `[US-RA-014]` 发送测试邮件 (P1)，来源 `docs/user-stories/core/realm-admin.md`
  - 角色：Realm Admin
  - 摘要：发送测试邮件验证配置正确性

- `[US-RA-015]` 邮件依赖功能开关前置验证 (P0)，来源 `docs/user-stories/core/realm-admin.md`
  - 角色：Realm Admin
  - 摘要：未配置邮件时无法开启邮箱验证等邮件依赖功能

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 3 | 配置 Realm 设置、配置邮件服务、功能开关前置验证 |
| P1 | 1 | 发送测试邮件 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- Realm Config 管理（Turnstile、Registration）
- OAuth Provider 配置管理（独立系统，不在 Realm Config 中管理）
- Email 邮件服务配置（Per-Realm，支持 Resend / SMTP）
- 邮件依赖功能开关前置验证
- 前端 Settings 页面（多 Tab 布局）

### 2.2 不包含功能 (Out of Scope)

- 端到端测试
- 更多配置类型（当前仅支持 Turnstile、Registration、OAuth、Email）
- 配置模板功能（无预定义配置模板）
- 会话配置、密码策略配置（暂不在 Realm Config 中管理）

### 2.3 依赖项

- **Realm 系统** — Config 属于 Realm 级别，依赖 Realm 基础设施
- **权限管理系统** — Realm Admin 权限检查
- **OAuth Provider 系统** — OAuth Provider 配置管理

---

## 3. 需求概述

### 3.1 功能描述

在 Herald 管理后台提供 Realm Settings 功能，允许 Realm Admin 管理本 Realm 的各类配置项，包括 Turnstile 验证码配置、用户注册配置、OAuth Provider 配置和邮件服务配置。Settings 页面通过多 Tab 布局组织不同配置类型，每个 Tab 包含独立的配置表单。

### 3.2 关键特性

- 分 Tab 布局管理多种配置类型（Turnstile、Registration、OAuth、Email）
- 每种配置类型独立启用/禁用，支持保存和重置
- 邮件服务支持 Resend API 和 SMTP 两种 Provider
- 邮件依赖的功能开关（如邮箱验证）需前置校验邮件配置完整性
- OAuth 配置通过独立系统管理，不在 Realm Config 中

---

## 4. 业务规则与状态

### 4.1 业务规则

- **Realm 隔离**：所有配置项属于 Realm 级别，不同 Realm 的配置相互独立
- **权限要求**：仅 Realm Admin 角色可查看和修改 Realm Settings
- **敏感信息脱敏**：密码、密钥类字段（Turnstile site_secret、Resend API Key、SMTP 密码）在展示时必须脱敏，编辑时才暴露为输入框
- **邮件配置完整性定义**：provider + from_address + 对应 provider 的必填字段均已填写且 enabled=true
- **功能开关前置验证**：`require_email_verification` 开关仅在邮件配置完整时可开启；未配置邮件时，该开关显示为禁用状态，提示 "Email verification requires email configuration"
- **密码策略强制生效**：所有密码策略字段（最小长度、大小写、数字、特殊字符）用于密码强度校验，系统默认最小长度 8，必须包含大小写字母、数字和特殊字符
- **OAuth 配置独立**：OAuth Provider 有独立配置系统，不在 Realm Config 中管理

### 4.2 关键状态与异常

- **未配置邮件 + 尝试开启邮箱验证**：开关禁用，显示提示信息，阻止开启
- **Provider 切换**：切换邮件 Provider 时，隐藏/显示对应字段（Resend 显示 API Key；SMTP 显示 Host/Port/Username/Password）
- **测试邮件**：保存配置后可通过 "Send Test Email" 验证配置正确性
- **新用户默认状态**：Registration 配置中 default_user_status 取值范围 0-3

---

## 5. 功能需求

### 5.1 核心需求

- **Turnstile 配置**：管理 Turnstile 验证码的 site_secret，用于人机验证
- **Registration 配置**：管理用户注册策略，包括是否开放注册、是否需要邮箱验证、新用户默认状态、密码强度策略
- **Email 配置**：管理邮件服务（Resend 或 SMTP），包括发件人地址、Provider 特定参数
- **OAuth 配置**：通过独立系统管理 OAuth Provider（不在本页面详细定义）
- **Settings 页面**：多 Tab 布局，每个配置类型对应一个 Tab，包含启用/禁用开关、配置表单、保存/重置按钮

### 5.2 验收目标

- Realm Admin 能通过 Settings 页面成功配置 Turnstile、Registration、Email 各项参数
- 邮件服务配置保存后，可通过测试邮件功能验证配置正确性
- 未配置邮件时，邮箱验证开关处于禁用状态并有明确提示
- 密码策略配置生效，注册时按配置规则校验密码强度
- 敏感字段在页面展示时脱敏，仅在编辑时可见
- 不同 Realm 的配置相互隔离

---

## 6. API 相关约束

**适用性**: 适用

- 接口能力范围：Realm Config 的查询和更新（涵盖 turnstile、registration、email 配置类型），以及 OAuth Provider 的独立配置管理
- 访问控制原则：所有接口要求 Realm Admin 权限，操作需通过 Realm 归属校验
- 数据边界原则：配置数据按 Realm 隔离，不同 Realm 之间不可交叉访问
- 敏感信息处理：密码、密钥等敏感字段在读取时脱敏返回，仅在写入时接受明文
- 审计要求：关键配置变更应记录审计日志
- 详细接口契约、验证规则和错误模型在技术设计文档中维护

---

## 7. 前端/交互约束

**适用性**: 适用

- **页面入口**：管理后台左侧导航栏 Settings 菜单项，realmId 从 UI 上下文获取
- **页面布局**：多 Tab 布局，每个配置类型对应一个 Tab（Turnstile、Registration、OAuth、Email）
- **每个 Tab 包含**：配置标题、启用/禁用开关、配置项表单、保存/重置按钮
- **敏感字段交互**：密码/密钥类字段展示脱敏占位符，点击编辑后变为输入框
- **Email Provider 切换**：切换 Provider 时动态隐藏/显示对应字段
- **功能开关联动**：未配置邮件时，Registration Tab 中邮箱验证开关显示为禁用状态，并提示原因
- **操作反馈**：保存成功/失败有明确反馈，测试邮件发送有结果反馈
- **角色差异**：仅 Realm Admin 可见和操作 Settings 入口

---

## 8. 已确认决策

### 8.1 已确认决策

- OAuth 配置使用独立系统管理，不纳入 Realm Config 存储结构
- 邮件服务配置纳入 Realm Config 管理，使用 `email` 配置类型
- 会话配置、密码策略配置暂不纳入 Realm Config 管理
- Settings 页面使用多 Tab 布局而非分组卡片布局

---

## 9. 参考资料

- 用户故事：`docs/user-stories/core/realm-admin.md`
- 相关 PRD：`docs/prd/core/realm.md`
- 相关 PRD：`docs/prd/core/users.md`
- 相关 PRD：`docs/prd/integration/client-app.md`
