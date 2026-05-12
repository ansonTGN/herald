# Realm Settings 菜单产品需求文档 (PRD)

**创建时间**: 2025-01-05
**状态**: Implemented
**最后更新**: 2026-03-31

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Realm Admin 用户故事

- 📄 [docs/user-stories/02-realm-admin-user-stories.md](/docs/user-stories/02-realm-admin-user-stories.md)
  - **[US-RA-008] 配置 Realm 设置** (P0): 作为 Realm Admin，我想要配置 Realm 设置（Turnstile、注册策略、OAuth Provider），以便管理本 Realm 的安全和访问控制

### 1.2 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 3 | 配置 Turnstile 验证码、配置注册策略、配置 OAuth Provider |
| P1 | 0 | - |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ Realm Config 管理（Turnstile、Registration）
- ✅ OAuth Provider 配置管理（独立系统）
- ✅ 前端 Settings 页面（多 Tab 布局）
- ✅ Turnstile 配置表单
- ✅ Registration 配置表单
- ✅ OAuth 配置表单
- ✅ 后端 Realm Config API
- ✅ 后端 OAuth Config API（部分实现）

### 2.2 不包含功能 (Out of Scope)

- ❌ **端到端测试** (原因: 待实施)
- ❌ **更多配置类型** (原因: 当前只支持 Turnstile、Registration、OAuth)
- ❌ **配置模板功能** (原因: 没有预定义配置模板)

### 2.3 依赖项

- ✅ **Realm 系统** (状态: 已实现) - Config 属于 Realm 级别
- ✅ **权限管理系统** (状态: 已实现) - Realm Admin 权限检查
- ✅ **OAuth Provider 系统** (状态: 部分实现) - OAuth Provider 配置管理

---

## 3. 需求概述

在 Herald 管理后台的左侧导航栏添加 **Settings** 菜单项，用于管理 Realm 的各类配置项，包括 Turnstile 验证码配置、用户注册配置等。

**注意**:
- OAuth 配置有独立的配置系统，不在 Realm Config 中管理
- 邮件服务配置、会话配置、密码策略配置暂不在 Realm Config 中管理

**导航菜单配置**（已完成，参考 `frontend/src/data/navigation.ts`）：

---

## 4. 当前实现状态

### ✅ 已完整实现 (2026-03-31 更新)

**后端实现**：
- [x] Realm Config 后端接口实现
- [x] 后端数据库 Repository 实现 (`PostgresRealmConfigRepository`)
- [x] 后端 Service 实现 (`RealmConfigServiceImpl`)
- [x] 后端 Service 与 HTTP 层集成
- [x] 路由注册到 HTTP Server
- [x] OAuth Provider 配置系统（独立系统）
- [x] Stripe 支付配置系统

**前端实现**：
- [x] Settings 页面：`frontend/src/routes/$realmId/manage/settings.tsx`
- [x] TOTP 配置表单：`frontend/src/components/realm-config/totp-config-form.tsx`
- [x] Registration 配置表单：`frontend/src/components/realm-config/registration-config-form.tsx`
- [x] OAuth Provider 配置页面：`frontend/src/components/oauth-config/provider-config-page.tsx`
- [x] Stripe 配置表单：`frontend/src/components/stripe-config/stripe-config-form.tsx`
- [x] 前端数据层：`frontend/src/lib/api-generated`（Realm Config API）
- [x] 完整的权限检查（settings.view、settings.manage）
- [x] 多 Tab 布局（TOTP、Registration、OAuth、Stripe）

**已实现功能**：
- ✅ TOTP 二次认证配置（启用/禁用、issuer、算法、digits、period）
- ✅ 用户注册配置（注册开关、邮箱验证、默认用户状态、密码策略）
- ✅ OAuth Provider 配置（Google、GitHub、Facebook、Apple 等）
- ✅ Stripe 支付配置（API Key、Webhook Secret 等）
- ✅ 配置的批量保存和验证
- ✅ 完整的错误处理和用户反馈

**实现说明**：
- Settings 页面使用 `/$realmId/manage/settings` 路由
- 使用 Tabs 布局组织多个配置类型
- 所有配置类型均已实现，包括 PRD 中描述的 Turnstile（现为 TOTP）和 Registration
- 实际实现比 PRD 原始设计更全面，增加了 OAuth、Stripe 和 TOTP 配置

**测试覆盖**：
- 组件测试：`frontend/src/components/stripe-config/__tests__/`
- E2E 测试：通过其他 Demo 测试覆盖（OAuth、TOTP 等）

### 📝 架构说明

#### OAuth 配置系统 (独立)

**适用场景**: 管理第三方登录提供商的完整配置

**后端实现**:
- 实体: `core/src/domain/oauth/entities.rs` - `OAuthProviderConfig`
- API: `api/src/application/http/oauth/`
- Repository: `PostgresOAuthConfigRepository`

**前端实现**:
- API: `oauthConfigApi` in `frontend/src/lib/api.ts`
- 组件: `frontend/src/features/settings/oauth-config-form.tsx` (独立使用)

**数据结构**:

**优点**:
- ✅ 类型安全 - 强类型实体，编译期检查
- ✅ 业务逻辑清晰 - 独立的登录流程、provider 验证
- ✅ 易于扩展 - Provider 特定的配置和验证逻辑
- ✅ 代码可读性高 - `OAuthProviderConfig` 比 `RealmConfig { config_type: "oauth" }` 清晰

#### Realm Config 系统 (简单配置)

**适用场景**: 简单的 key-value 配置

**后端实现**:
- 实体: `core/src/domain/realm_config/entities.rs` - `RealmConfig`
- API: `api/src/application/http/realm_config/`
- Repository: `PostgresRealmConfigRepository`

**前端实现**:
- API: `realmConfigApi` in `frontend/src/lib/api.ts`
- 组件: Settings 页面中的 Turnstile 和 Registration 配置

**支持类型**:

**数据结构**:

**适用配置项**:
- ✅ 注册开关: `enabled: "true"`
- ✅ 会话超时: `session_timeout: "3600"`
- ✅ Turnstile site_secret: `site_secret: "0x4AAA..."`
- ❌ OAuth provider 配置 (使用独立系统)
- ❌ 邮件服务配置 (未来可能需要独立系统)

#### 关键区别

| 特性 | OAuth 系统 | Realm Config 系统 |
|------|-----------|------------------|
| 数据模型 | 强类型结构体 | Key-Value 泛化 |
| 复杂度 | 复杂 (scopes, provider 验证) | 简单 (开关、字符串) |
| 扩展性 | Provider 特定逻辑 | 统一存储格式 |
| 类型安全 | 编译期检查 | 运行时解析 |
| 使用场景 | OAuth 第三方登录 | 简单配置项 |

---

## 5. 功能需求

### 2.1 Settings 页面布局

#### 2.1.1 路由定义

创建 `frontend/src/routes/admin/(dashboard)/settings.tsx` 路由文件：

**架构说明**:
- **固定路由**: `/admin/settings`（不包含 realmId 参数）
- **realmId 获取**: 从 UI 上下文获取（从 Realms Table 选择后通过导航传递）
- **当前实现**: 临时硬编码 `realmId = 'admin'`
- **未来实现**: 从 Realms Table 页面选择一行，点击 "Settings" 按钮，跳转到 Settings 页面并传递 realmId

**注意**: 文件名使用括号分组 `(dashboard)`，不使用点号或斜杠

#### 2.1.2 页面结构

Settings 页面采用分组卡片布局，每个配置类型对应一个独立的卡片区域：

1. **页面标题区域**
   - 标题: "Settings"
   - 描述: "Manage realm configuration"

2. **配置分组**（使用 Tabs 或垂直布局）
   - Turnstile 配置
   - 用户注册配置 (Registration)

**注意**:
- OAuth 配置应该通过独立的 OAuth 配置管理页面或 API 进行管理
- 邮件服务、会话配置、密码策略配置暂不在此页面管理

3. **每个配置卡片包含**:
   - 配置标题
   - 启用/禁用开关
   - 配置项表单
   - 保存/重置按钮

### 2.2 配置类型详细设计

#### 2.2.1 Turnstile 配置

**配置类型**: `turnstile`

**存储结构**: 单个配置项，site_key 存储在 metadata 中

**配置项**:

| 配置键 | 显示名称 | 类型 | 是否敏感 | 说明 |
|--------|---------|------|----------|------|
| `site_secret` | Site Secret | string | 是 | Turnstile Secret Key |

**存储结构**:

**表单定义**:

#### 2.2.3 用户注册配置

**配置类型**: `registration`

**配置项**:

| 配置键 | 显示名称 | 类型 | 是否敏感 | 说明 |
|--------|---------|------|----------|------|
| `enabled` | Allow Registration | boolean | 否 | 是否开放用户注册 |
| `require_email_verification` | Require Email Verification | boolean | 否 | 是否需要邮箱验证 |
| `default_user_status` | Default User Status | number | 否 | 新用户默认状态 (0-3) |
| `password_min_length` | Password Min Length | number | 否 | 密码最小长度（默认 8） |
| `password_require_uppercase` | Password Require Uppercase | boolean | 否 | 密码是否需要大写字母（默认 true） |
| `password_require_lowercase` | Password Require Lowercase | boolean | 否 | 密码是否需要小写字母（默认 true） |
| `password_require_number` | Password Require Number | boolean | 否 | 密码是否需要数字（默认 true） |
| `password_require_special_char` | Password Require Special Char | boolean | 否 | 密码是否需要特殊字符（默认 true） |

**说明**：
- 所有密码策略字段强制生效，用于密码强度校验
- 系统默认：最小长度 8，必须包含大小写字母、数字和特殊字符

**表单定义**:

---

## 6. API 相关约束

**状态**: 必填

- 仅说明 Realm、用户、设置等核心管理能力的访问边界、角色要求和数据隔离原则，不在 PRD 中维护端点列表、请求响应字段或实现细节。
- 必须遵守 realm 隔离、权限校验、敏感信息脱敏和关键操作审计要求。
- 详细接口契约、验证规则和错误模型应在技术设计、接口说明或代码中维护。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留菜单入口、页面可见性、表单校验期望、操作反馈和角色差异，不写路由代码、组件实现或类型定义。
- 核心管理流程需确保敏感操作有明确确认与结果反馈，且不同角色看到的操作入口和数据范围保持一致。

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

- 相关实现文件请以本功能对应的 `backend/`、`frontend/`、`demo/` 目录和现有设计文档为准。
- 若需补充精确文件清单，应在技术设计文档中维护，避免在 PRD 中混入实现级细节。

---

## 10. 参考资料

- 前端开发指南: `../../spec/frontend/development.md`
- Realm Config 实体定义: `core/src/domain/realm_config/entities.rs`
- Realm Config API 定义: `api/src/application/http/realm_config/mod.rs`
- Realm Config Service: `core/src/domain/realm_config/service.rs`
- Users 功能参考: `docs/frontend/users.md`
- Client Apps 功能参考: `docs/frontend/client_app.md`
- 现有代码参考:
  - 数据层: `frontend/src/data/users.ts`
  - 工具层: `frontend/src/utils/users.ts`
  - 页面组件: `frontend/src/features/users/index.tsx`
  - 表单组件: `frontend/src/features/users/user-form.tsx`
  - 路由配置: `frontend/src/routes/$realmId/users.tsx`

