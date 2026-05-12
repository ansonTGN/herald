# 第三方 API 产品需求文档 (PRD)

**创建时间**: 2025-02-04
**状态**: Partially Implemented

---

## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `docs/user-stories/` 目录中的对应文件。

### 1.1 第三方应用用户故事
- 📄 [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)
  - **故事 2: 验证用户登录状态** [US-TP-002] (P0): 作为第三方应用,我想要验证用户的登录状态和身份,从而保护应用资源
  - **故事 3: 检查用户权限** [US-TP-003] (P0): 作为第三方应用,我想要检查用户是否有权限访问特定资源,从而实现细粒度的访问控制
  - **故事 8: 第三方 API 认证** [US-TP-008] (P0): 作为第三方应用,我想要使用 API Key 认证调用 Herald 第三方接口,从而安全地集成 Herald 系统
  - **故事 9: 查询订阅状态** [US-TP-009] (P0): 作为第三方应用,我想要能够查询客户端应用的订阅状态,从而根据订阅状态提供相应的功能和体验

### 1.2 主管理员用户故事
- 📄 [docs/user-stories/01-admin-realm-user-stories.md](/docs/user-stories/01-admin-realm-user-stories.md)
  - **API Key 管理** (P0): 作为主管理员,我想要创建和管理第三方 API Keys,从而控制第三方访问

### 1.3 用户故事优先级汇总
| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 4 | API Key 认证 (US-TP-008)、权限检查 (US-TP-003)、订阅查询 (US-TP-009) |
| P1 | 1 | API Key 管理 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能
- ✅ 第三方 API 认证系统（X-API-Key header）
- ✅ API Key 数据模型（client_api_keys 表）
- ✅ API Key 使用统计（last_used_at, usage_count）
- ✅ Realm 隔离（API Key 绑定到 realm）
- ✅ OpenAPI 文档集成（third tag）

### 2.2 不包含功能 (Out of Scope)
- ❌ API Key 管理界面 (原因: 后续优化 Phase 1 功能)
- ❌ 速率限制 (原因: 后续优化 Phase 1 功能)
- ❌ 审计日志 (原因: 后续优化 Phase 1 功能)
- ❌ Scope 验证 (原因: 后续优化 Phase 2 功能)
- ❌ API Key 轮换 (原因: 后续优化 Phase 2 功能)
- ❌ Webhooks 支持 (原因: 后续优化 Phase 3 功能)
- ❌ GraphQL 支持 (原因: 后续优化 Phase 3 功能)

### 2.3 依赖项
- ✅ [用户认证系统] (状态: 已实现)
- ✅ [权限系统] (状态: 已实现)
- ✅ [订阅系统] (状态: 已实现)
- ✅ [Realm 系统] (状态: 已实现)
- ✅ [Session Token 验证] (状态: 已实现)

---

## 3. 需求概述

### 3.1 功能描述

### 3.2 关键特性
- **专用认证**: 使用 `X-API-Key` header 认证，而非 session token
- **Realm 隔离**: API Key 绑定到特定 realm，防止跨租户访问
- **使用统计**: 记录 API Key 的使用次数和最后使用时间
- **OpenAPI 文档**: 新增 `third` tag 标识第三方接口

### 3.3 当前问题
1. **缺少统一前缀**: 第三方接口没有统一的 URL 前缀标识
2. **缺少专门认证**: 现有接口使用 session token 认证，不适合第三方集成
3. **缺少订阅查询**: 没有公开的 API 供第三方查询用户订阅状态
4. **OpenAPI 文档**: 缺少 `third` tag 标识第三方接口

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|

| API Key 认证中间件 | ❌ 待实施 | 中间件文件待创建 |
| OpenAPI 文档更新 (third tag) | ❌ 待实施 | 需添加第三方接口的 OpenAPI tag |
| 场景测试 | ❌ 待实施 | 需编写完整测试用例 |

---

## 5. 功能需求

### 5.1 API Key 认证系统

#### 功能描述
提取并验证 `X-API-Key` header，查询数据库验证 API Key 有效性，更新使用统计，注入 `ThirdPartyIdentity` 到 request extensions。

#### 验收标准
- ✅ 提取 `X-API-Key` header
- ✅ 哈希并查询数据库验证
- ✅ 检查 API Key 是否启用和未过期
- ✅ 更新使用统计（last_used_at, usage_count）
- ✅ 注入 `ThirdPartyIdentity` 到 request extensions
- ✅ 无效/缺失 API Key 返回 401 Unauthorized
- ✅ 过期/禁用 API Key 返回 401 Unauthorized
- ✅ 支持 Realm 隔离（API Key 只能访问所属 realm 的资源）

### 5.2 权限检查 API

#### 功能描述
第三方应用使用 API Key 和用户 session token 验证用户是否有权限访问特定资源。

#### 验收标准
- ✅ 接受 API Key 认证（`X-API-Key` header）
- ✅ 验证 session token 有效性
- ✅ 检查用户是否有指定权限（基于 `rules` 数组）
- ✅ 返回权限检查结果（`allowed`, `user_id`）
- ✅ 支持 batch 权限检查（多个 rules）
- ✅ 无效 session token 返回 `{"allowed": false}`
- ✅ API Key 无效返回 401 Unauthorized

### 5.3 订阅状态查询 API

#### 功能描述
第三方应用使用 API Key 查询指定客户端应用的订阅状态。

#### 验收标准
- ✅ 接受 API Key 认证（`X-API-Key` header）
- ✅ 接受 client_app_id（URL 参数）
- ✅ 验证客户端应用存在
- ✅ 查询订阅状态
- ✅ 返回订阅信息（status, tier, plan_name）
- ✅ 无订阅时返回 free tier 信息
- ✅ 客户端应用不存在返回 404 Not Found
- ✅ API Key 无效返回 401 Unauthorized

### 5.4 数据模型

#### client_api_keys 表
- ✅ `id` (UUID): API Key 唯一标识
- ✅ `name` (VARCHAR(255)): API Key 名称
- ✅ `api_key_hash` (VARCHAR(255)): API Key 哈希值（不存储明文）
- ✅ `realm_id` (VARCHAR(36)): 所属租户 ID
- ✅ `enabled` (BOOLEAN): 是否启用
- ✅ `expires_at` (TIMESTAMP): 过期时间（NULL 表示永不过期）
- ✅ `created_at` (TIMESTAMP): 创建时间
- ✅ `last_used_at` (TIMESTAMP): 最后使用时间
- ✅ `usage_count` (INTEGER): 使用次数

#### 索引
- ✅ `idx_client_api_keys_realm` on `realm_id`
- ✅ `idx_client_api_keys_key` on `api_key_hash`

---

## 6. API 相关约束

**状态**: 必填

- 仅说明第三方接入、Client App、API Key、回调或外部能力暴露的边界，不在 PRD 中展开端点、schema、签名格式或 SDK 类型定义。
- 必须遵守 realm 隔离、Client App/第三方身份校验、凭证脱敏、回调安全和可观测性要求。
- 详细接入契约、认证方式和错误模型应下沉到技术设计、接口说明或 SDK 文档。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留接入配置入口、凭证展示规则、状态反馈和帮助说明，不写 SDK 调用示例、前端实现代码或接口调试步骤。
- 涉及第三方接入时，需明确哪些流程由 Herald 后台完成，哪些流程在第三方应用或外部平台完成。

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

### 9.1 后端文件

  - 状态: ✅ 已创建

  - 状态: ❌ 待创建
  - 说明: 路由模块导出

  - 状态: ❌ 待创建
  - 说明: API Key 认证中间件

  - 状态: ❌ 待创建
  - 说明: 权限检查 API

  - 状态: ❌ 待创建
  - 说明: 订阅状态 API

  - 状态: ❌ 待修改
  - 说明: 添加 `pub mod third;`

  - 状态: ❌ 待修改
  - 说明: 集成 third 路由、添加 OpenAPI 配置

  - 状态: ❌ 待创建
  - 说明: 场景测试

### 9.2 前端文件
暂不涉及前端界面。

### 9.3 测试文件
  - 状态: ❌ 待创建
  - 说明: API Key 认证、权限检查、订阅查询场景测试

---

## 10. 参考资料

### 13.1 相关文档
- **设计文档**: `../../.ai/future/third.md`（待补充）
- **用户故事**: [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)
- **后端开发指南**: [spec/backend/development.md](/spec/backend/development.md)
- **测试指南**: [spec/backend/testing.md](/spec/backend/testing.md)

### 13.2 类似系统
- Keycloak Admin API: https://www.keycloak.org/docs-api/latest/rest_api/
- Stripe API: https://stripe.com/docs/api

