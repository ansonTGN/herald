# 微信 OAuth 集成产品需求文档 (PRD)

**创建时间**: 2026-03-03
**状态**: Active
**优先级**: P1

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `docs/user-stories/` 目录中的对应文件。

### 1.1 租户管理员用户故事
- 📄 `docs/user-stories/auth/oauth-extension.md`
  - **[US-RA-010] OAuth Provider 配置管理** (P0): 作为 Realm Admin,我想要管理 OAuth Provider 配置（Google、GitHub、Facebook、Apple、WeChat）,以便用户可以使用第三方登录

### 1.2 租户用户用户故事
- 📄 `docs/user-stories/core/regular-user.md`
  - **[US-RU-003] OAuth 第三方登录** (P1): 作为普通用户,我想要使用第三方账号（Google、GitHub、Facebook、Apple、WeChat）登录,以便无需记忆额外密码

### 1.3 微信专属用户故事
- 📄 `docs/user-stories/auth/wechat-oauth.md`
  - **[US-RA-011] WeChat OAuth Provider 配置** (P1): 作为 Realm Admin,我想要配置 WeChat OAuth Provider,以便用户可以使用微信登录
  - **[US-RA-012] WeChat Mini Program Provider 配置** (P1): 作为 Realm Admin,我想要配置 WeChat Mini Program Provider,以便小程序用户可以登录
  - **[US-RU-010] 微信网站应用登录** (P1): 作为普通用户,我想要使用微信扫码登录,以便快速访问系统
  - **[US-RU-011] 微信小程序登录** (P1): 作为小程序用户,我想要使用微信账号登录,以便在小程序内访问 Herald 服务

### 1.4 用户故事优先级汇总
| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 1 | OAuth Provider 配置管理 |
| P1 | 4 | WeChat OAuth Provider 配置、WeChat Mini Program Provider 配置、微信网站应用登录、微信小程序登录 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能
- ✅ 网站应用微信登录（QRconnect，PC 网站扫码登录）
- ✅ 微信小程序登录（code2session）
- ✅ WeChat OAuth Provider 配置管理
- ✅ WeChat Mini Program Provider 配置管理
- ✅ UnionID 机制支持（跨应用用户匹配）
- ✅ Placeholder 邮箱生成（微信不提供邮箱）

### 2.2 不包含功能 (Out of Scope)
- ❌ 微信公众号登录（需要不同的 OAuth 流程）
- ❌ 微信支付集成（不涉及支付功能）
- ❌ 微信社交分享功能
- ❌ 微信用户信息解密（如需敏感信息，需要额外开发）

### 2.3 依赖项
- ✅ OAuth 2.0 框架（状态: 已实现，支持 Google、GitHub、Facebook、Apple）
- ✅ Realm 隔离机制（状态: 已实现）

---

## 3. 需求概述

### 3.1 功能描述
Herald 项目需要接入微信账号体系，支持两种登录方式：
1. **网站应用微信登录** - PC 网站用户扫码登录
2. **微信小程序登录** - 小程序内使用微信账号登录

这两种登录方式使用不同的 API 流程，但需要统一的用户管理和 UnionID 匹配机制。

### 3.2 关键特性
- **UnionID 机制**: 支持跨应用用户匹配，同一开放平台账号下的用户使用相同的 UnionID
- **Placeholder 邮箱**: 微信不提供邮箱，自动生成占位符邮箱
- **配置灵活**: Realm Admin 可以独立配置 WeChat 和 WeChat Mini Program Provider

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| ProviderType 枚举扩展 | ❌ | 待添加 WeChat、WeChatMiniProgram |
| WeChat OAuth Provider 实现 | ❌ | 待实现网站应用微信登录 |
| WeChat Mini Program Provider 实现 | ❌ | 待实现小程序登录 |
| OAuthUserInfo 更新 | ❌ | 待添加 openid、unionid 字段 |
| OAuth Services 更新 | ❌ | 待添加 ProviderHandler 变体 |
| UnionID 匹配逻辑 | ❌ | 待实现跨应用用户匹配 |

| API 端点（网站应用） | ❌ | 待实现登录和回调端点 |
| API 端点（小程序） | ❌ | 待实现 code2session 端点 |
| 前端集成 | ❌ | 不适用（用户选择不需要前端实现） |

---

## 5. 功能需求

### 4.1 UnionID 机制
- 同一开放平台账号下的所有应用，用户的 UnionID 相同
- 优先使用 UnionID 进行用户匹配
- UnionID 获取条件：应用必须绑定到微信开放平台
- 支持网站应用和小程序之间的跨应用用户匹配

### 4.2 Scope 配置
- 网站应用微信登录的 scope **必须**是 `snsapi_login`（固定值）
- WeChat Mini Program Provider 不需要配置 scope
- UnionID 通过访问用户信息接口获取（网站应用），不是在 scope 中指定

### 4.3 Email 处理
- 微信不提供邮箱地址
- Placeholder 邮箱生成策略：
  - 优先使用: `{unionid}@wechat.placeholder`（如果 unionid 可用）
  - 降级使用: `{openid}@wechat.placeholder`（如果 unionid 不可用）
- 邮箱标记为可选（verified: false）
- 在需要邮箱的场景提示用户补充真实邮箱

### 4.4 用户匹配优先级
1. 优先通过 unionid 查找已存在的用户
2. 如果找不到，通过 email 查找
3. 如果还找不到，创建新用户

### 4.5 跨应用匹配
- 支持网站应用和小程序之间的用户匹配
- 使用 UnionID 作为跨应用匹配的唯一标识
- 同一用户可以有多个 Provider 记录（wechat、wechat-miniprogram），UnionID 相同

### 4.6 安全要求
- state 参数防止 CSRF 攻击（网站应用）
- code 只能使用一次，10分钟内有效（网站应用）
- js_code 只能使用一次，5分钟内有效（小程序）
- Client Secret 不在 GET 响应中返回
- 编辑模式下 Client Secret 为可选（留空表示不更新）
- 所有 Provider 配置操作记录审计日志

### 4.7 与其他 OAuth Provider 的区别

| 特性 | Google/GitHub/Facebook/Apple | WeChat (网站应用) | WeChat Mini Program |
|------|----------------------------|-------------------|---------------------|
| OAuth 流程 | 标准 OAuth 2.0 | 三步法流程 | code2session（非标准） |
| Scope | 可配置多种 scopes | 固定 `snsapi_login` | 不需要 scope |
| UnionID | 无 | 支持（需绑定开放平台） | 支持（需绑定开放平台） |
| Email 提供 | ✅ 提供真实邮箱 | ❌ Placeholder 邮箱 | ❌ Placeholder 邮箱 |
| QR Code | 不需要 | ✅ 需要扫码 | 不需要 |

### 6.1 网站应用微信登录

**授权流程**：
1. 用户点击"微信登录"按钮
2. 系统生成授权 URL 并重定向到微信授权页面
3. 用户扫码并授权
4. 微信回调到 Herald 系统并携带 code
5. 系统使用 code 换取 access_token
6. 系统使用 access_token 获取用户信息（含 unionid）
7. 系统创建或匹配用户并设置 session

**三步法流程**：
1. 用户扫码授权获取授权码
2. 使用授权码换取访问令牌
3. 使用访问令牌获取用户信息（含 UnionID）

### 6.2 微信小程序登录

**授权流程**：
1. 小程序用户触发微信登录
2. 系统接收小程序发送的授权码
3. 系统验证授权码并获取用户信息
4. 系统创建或匹配用户
5. 返回访问令牌给小程序用户

### 6.3 Realm Admin 配置管理

**WeChat OAuth Provider 配置**：
- 配置项：Client ID（AppID）、Client Secret（AppSecret）、Redirect URI、Scope（固定 `snsapi_login`）、Enabled
- 操作：创建、查看、编辑、删除、启用/禁用
- Scope 为固定值，不可修改

**WeChat Mini Program Provider 配置**：
- 配置项：Client ID（AppID）、Client Secret（AppSecret）、Enabled
- 操作：创建、查看、编辑、删除、启用/禁用
- 不需要配置 scope 和 redirect_uri

### 6.4 数据库要求

- Provider 表需添加 `union_id` 索引用于跨应用用户匹配
- 现有 `provider` 表已包含 `open_id`、`union_id`、`email` 字段，无需添加新字段

---

## 6. API 相关约束

**状态**: 必填

- 仅说明认证、授权、验证、回调或账号绑定等能力边界，不在 PRD 中展开端点、请求响应 schema、状态码矩阵。
- 必须遵守 realm 隔离、权限边界、凭证脱敏和幂等要求；涉及回调时需满足回调来源校验、重放防护和错误可恢复性。
- 若存在第三方身份提供商或支付/消息回调，应在技术设计或接口说明中维护详细契约，PRD 只保留业务约束和兼容性要求。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留页面入口、关键用户路径、状态反馈、权限可见性和异常提示要求，不写组件实现步骤或前端类型定义。
- 认证相关流程应优先保证成功/失败状态清晰、回跳路径明确、敏感信息不回显，并对首次配置、失效、锁定、重试等场景提供稳定反馈。


## 8. 相关文件索引

### 9.1 后端文件
- `backend/core/src/domain/oauth/entities.rs` - ❌ 待添加 WeChat ProviderType
- `backend/core/src/domain/oauth/providers/wechat.rs` - ❌ 待创建（网站应用微信登录实现）
- `backend/core/src/domain/oauth/providers/wechat_miniprogram.rs` - ❌ 待创建（小程序登录实现）
- `backend/core/src/domain/oauth/providers.rs` - ❌ 待导出新的提供者
- `backend/core/src/domain/oauth/services.rs` - ❌ 待更新 ProviderHandler
- `backend/core/src/domain/oauth/ports.rs` - ❌ 待添加 find_by_union_id
- `backend/core/src/domain/oauth/value_objects.rs` - ❌ 待更新 OAuthUserInfo

### 9.2 前端文件
- `frontend/src/routes/$realmId/auth/login.tsx` - ❌ 不适用（用户选择不需要前端实现）

---

## 9. 参考资料

- 相关用户故事:
  - [oauth-extension.md](/docs/user-stories/auth/oauth-extension.md)
  - [regular-user.md](/docs/user-stories/core/regular-user.md)
  - [wechat-oauth.md](/docs/user-stories/auth/wechat-oauth.md)
- 微信开放平台文档: https://open.weixin.qq.com/cgi-bin/showdocument?action=dir_list&t=resource/res_list&verify=1&id=open1419316505&token=&lang=zh_CN
- 微信小程序登录文档: https://developers.weixin.qq.com/miniprogram/dev/OpenApiDoc/user-info/phone-number/getPhoneNumber.html


