# TOTP 二次认证产品需求文档 (PRD)

**创建时间**: 2025-01-31
**状态**: ✅ Implemented

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Realm Admin 用户故事

- 📄 [docs/user-stories/05-totp-user-stories.md](/docs/user-stories/05-totp-user-stories.md)
  - **启用/禁用 TOTP 功能** (P0): 作为 Realm 管理员，我想要为本 Realm 启用或禁用 TOTP 二次认证功能，以便提升本 Realm 用户账户的安全性
  - **强制启用 TOTP** (P1): 作为 Realm 管理员，我想要设置本 Realm 强制启用 TOTP，以便确保所有用户都必须使用二次认证
  - **查看 TOTP 统计** (P2): 作为 Realm 管理员，我想要查看本 Realm 的 TOTP 启用率，以便了解安全状态

### 1.2 Regular User 用户故事

- 📄 [docs/user-stories/05-totp-user-stories.md](/docs/user-stories/05-totp-user-stories.md)
  - **[US-TO-001] 启用 TOTP 二次认证** (P0): 作为普通用户，我想要为我的账户启用 TOTP 二次认证，以便提升账户安全性，防止密码泄露导致账户被盗
  - **[US-TO-003] 使用 TOTP 登录** (P0): 作为普通用户，我想要在登录时能够通过 TOTP 验证码进行二次认证，以便确保即使密码泄露，账户仍受保护
  - **[US-TO-002] 禁用 TOTP** (P0): 作为普通用户，我想要能够禁用我的 TOTP 二次认证，以便在更换设备或不再需要时可以关闭此功能
  - **[US-TO-006] 重置 TOTP 恢复码** (P1): 作为普通用户，我想要能够重新生成我的 TOTP 密钥和备份恢复码，以便在丢失验证器应用或备份码时恢复访问
  - **[US-TO-004] 查看 TOTP 状态** (P2): 作为普通用户，我想要能够查看我的 TOTP 设置和使用情况，以便了解我的账户安全状态
  - **[US-TO-005] 查看 TOTP 恢复码** (P1): 作为普通用户，我想要查看我的 TOTP 恢复码，以便在无法使用验证器时可以登录

### 1.3 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 4 | 启用/禁用 TOTP 功能、用户启用 TOTP、使用 TOTP 登录、用户禁用 TOTP |
| P1 | 2 | 强制启用 TOTP、重新生成 TOTP 密钥 |
| P2 | 2 | 查看 TOTP 统计、查看 TOTP 使用情况 |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ Realm 级别 TOTP 开关（管理员可启用/禁用）
- ✅ 强制 TOTP 模式（所有用户必须启用）
- ✅ 用户启用 TOTP（生成密钥、二维码、备份恢复码）
- ✅ TOTP 二次认证登录流程
- ✅ 备份恢复码登录（10 个 6 位数字，一次性使用）
- ✅ 用户禁用 TOTP（需验证当前密码）
- ✅ 重新生成 TOTP 密钥和备份码（需验证当前密码）
- ✅ TOTP 使用情况查看（启用时间、最近验证、剩余备份码）
- ✅ TOTP 验证码防暴力破解（连续错误 5 次锁定 15 分钟）

### 2.2 不包含功能 (Out of Scope)

- ❌ **WebAuthn/FIDO2** (原因: 不同类型的二次认证，后续版本考虑)
- ❌ **SMS 二次认证** (原因: 需要短信服务支持，优先级较低)
- ❌ **邮箱验证码二次认证** (原因: 安全性较低，优先级较低)
- ❌ **TOTP 设备管理** (原因: 当前版本不支持多个验证器设备)
- ❌ **TOTP 验证历史记录** (原因: 需要审计日志系统支持)

### 2.3 依赖项

- ✅ **用户认证系统** (状态: 已实现) - 提供登录和会话管理
- ✅ **Realm 配置系统** (状态: 已实现) - 存储 Realm 级别 TOTP 开关
- ✅ **用户密码系统** (状态: 已实现) - 禁用/重新生成 TOTP 时验证密码
- ✅ **Session 管理** (状态: 已实现) - TOTP 验证通过后创建 Session

---

## 3. 需求概述

### 3.1 功能描述

TOTP（Time-based One-Time Password，基于时间的一次性密码）是一种二次认证（2FA）机制，通过支持 RFC 6238 标准的验证器应用（如 Google Authenticator、Authy、Microsoft Authenticator）生成 6 位动态验证码。

Herald 系统支持在 Realm 级别和用户级别配置 TOTP：
- **Realm 级别**：管理员可决定是否启用 TOTP 功能，以及是否强制所有用户启用
- **用户级别**：用户可选择是否启用 TOTP（若 Realm 允许），并管理自己的 TOTP 配置

### 3.2 关键特性

- **RFC 6238 标准**：使用 SHA-256 哈希算法，6 位数字码，30 秒周期
- **备份恢复机制**：启用 TOTP 时生成 10 个备份恢复码，防止无法访问验证器应用
- **安全存储**：TOTP 密钥使用 AES-256-GCM 加密存储，备份码使用 bcrypt hash
- **防暴力破解**：连续错误 5 次后锁定 TOTP 验证 15 分钟
- **时间漂移支持**：支持 ±1 个周期（30 秒）的时间漂移

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| Realm TOTP 配置 | ✅ | 已完成（Realm TOTP 配置 API + 前端表单） |
| 用户 TOTP 启用/禁用 | ✅ | 已完成（用户 TOTP 管理 API + 前端组件） |
| TOTP 登录流程 | ✅ | 已完成（TOTP 验证 API + 前端验证表单） |
| 备份恢复码管理 | ✅ | 已完成（生成、存储、验证备份码） |
| TOTP 统计查看 | ✅ | 已完成（Realm TOTP 统计 API） |
| 前端 TOTP 设置页面 | ✅ | 已完成（Settings TOTP 配置表单） |
| 前端 TOTP 登录页面 | ✅ | 已完成（Profile Security 页面 + TOTP 验证） |
| 演示测试 | ✅ | 已完成（Realm Admin + Regular User TOTP 测试） |
- ✅ 后端 Service：`UserTotpService`（密钥生成、验证、加密/解密）
- ✅ 后端 API：Realm TOTP 配置、用户 TOTP 管理、TOTP 登录验证
- ✅ 前端组件：`TotpConfigForm`, `TotpManagement`, `TotpSetupForm`, `TotpStatusCard`, `TotpVerificationForm`
- ✅ 演示测试：`realm-admin-totp-config-demo.e2e.ts`, `regular-user-totp-comprehensive-demo.e2e.ts`, `totp-user-stories.e2e.ts`

---

## 5. 功能需求

### 5.1 Realm 级别 TOTP 配置

**管理员功能**：

1. **启用/禁用 Realm TOTP 功能**
   - 在 Settings -> Security 页面添加 TOTP 开关
   - 禁用后，新用户无法启用 TOTP，已启用用户仍需验证（平滑降级）

2. **强制 TOTP 模式**
   - 启用后，所有用户必须启用 TOTP
   - 未启用用户下次登录时被要求设置
   - 已启用用户无法禁用 TOTP

3. **查看 TOTP 统计**
   - 显示已启用 TOTP 的用户数量
   - 显示未启用 TOTP 的用户数量
   - 显示 TOTP 启用率
### 5.2 用户 TOTP 管理

**用户功能**：

1. **启用 TOTP**
   - 访问个人资料 -> Security 页面
   - 点击 "Enable TOTP" 按钮
   - 系统生成 TOTP 密钥并显示二维码（QR Code）
   - 用户扫描二维码添加到验证器应用
   - 输入 6 位验证码验证
   - 系统显示 10 个备份恢复码（仅显示一次）
   - 用户确认"已保存备份码"后完成设置

2. **禁用 TOTP**
   - 在 Security 页面点击 "Disable TOTP" 按钮
   - 输入当前密码进行确认
   - TOTP 被禁用，下次登录无需 TOTP 验证
   - 若 Realm 强制启用 TOTP，则不允许禁用

3. **重新生成 TOTP 密钥**
   - 在 Security 页面点击 "Regenerate TOTP Secret" 按钮
   - 输入当前密码进行确认
   - 旧 TOTP 密钥失效，生成新密钥和二维码
   - 生成新的 10 个备份恢复码
   - 必须输入新验证码验证成功（验证失败时保留旧密钥）

4. **查看 TOTP 使用情况**
   - 显示 TOTP 启用状态（已启用/未启用）
   - 显示启用时间
   - 显示最近一次 TOTP 验证时间
   - 显示剩余可用备份恢复码数量
### 5.3 TOTP 登录流程

**登录步骤**：

1. **第一步：密码验证**
   - 用户输入邮箱和密码
   - 系统验证密码正确性

2. **第二步：TOTP 验证**（若用户已启用 TOTP）
   - 显示 TOTP 验证码输入页面
   - 用户输入 6 位 TOTP 验证码或备份恢复码
   - 验证码有效期 30 秒（支持 ±1 个周期的时间漂移）
   - 连续错误 5 次后锁定 15 分钟

3. **验证通过**
   - 创建 Session Cookie
   - 跳转到管理后台首页

**失败场景**：

- 验证码错误：提示"验证码错误"，可重新输入
- 验证码过期：提示"验证码已过期"，可输入新验证码
- 备份码耗尽：提示"备份恢复码已耗尽，请联系管理员"
- TOTP 锁定：提示"验证码错误次数过多，请 15 分钟后再试"

### 5.4 业务规则

1. **验证码规则**
   - TOTP 验证码为 6 位数字
   - 有效期为 30 秒（支持时间漂移 ±1 个周期）
   - 连续错误 5 次后锁定 TOTP 验证 15 分钟

2. **备份恢复码规则**
   - 启用 TOTP 时生成 10 个备份恢复码
   - 每个备份码为 6 位数字
   - 使用后立即失效（不可重复使用）
   - 重新生成 TOTP 密钥时同步重新生成备份码

3. **强制 TOTP 模式规则**
   - Realm 管理员可强制所有用户启用 TOTP
   - 强制模式下，用户无法禁用 TOTP
   - 未启用 TOTP 的用户在下次登录时被要求设置

4. **安全规则**
   - TOTP 密钥使用 AES-256-GCM 加密存储
   - 禁用 TOTP 需验证当前密码
   - 重新生成 TOTP 密钥需验证当前密码
   - TOTP 验证失败不暴露具体错误（统一提示"验证码错误"）

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

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

### 9.1 后端文件

- `backend/core/src/entity/user_totp_config.rs` - TOTP 实体定义 ✅
- `backend/core/src/entity/user_totp_backup_codes.rs` - 备份码实体定义 ✅
- `backend/core/src/domain/user_totp/entities.rs` - TOTP 领域实体 ✅
- `backend/core/src/domain/user_totp/ports.rs` - TOTP Repository 接口 ✅
- `backend/core/src/domain/user_totp/service.rs` - TOTP Service 层 ✅
- `backend/core/src/infrastructure/user_totp/repositories.rs` - TOTP Repository 实现 ✅
- **状态**: ✅ 已实施

### 9.2 前端文件

- `frontend/src/lib/types/totp.ts` - TOTP 类型定义 ✅
- `frontend/src/lib/api.ts` - TOTP API 调用 ✅
- `frontend/src/features/settings/totp-config-form.tsx` - Realm TOTP 配置表单 ✅
- `frontend/src/features/profile/totp-management.tsx` - 用户 TOTP 管理页面 ✅
- `frontend/src/components/totp/totp-setup-form.tsx` - TOTP 设置表单 ✅
- `frontend/src/components/totp/totp-status-card.tsx` - TOTP 状态卡片 ✅
- `frontend/src/components/totp/totp-verification-form.tsx` - TOTP 验证表单 ✅
- `frontend/src/routes/$realm_id.profile/security.tsx` - Security 页面 ✅
- **状态**: ✅ 已实施

- **状态**: ✅ 已实施

### 9.4 演示测试

- `demo/e2e/realm-admin/realm-admin-totp-config-demo.e2e.ts` - Realm Admin TOTP 配置测试 ✅
- `demo/e2e/regular-user/regular-user-totp-comprehensive-demo.e2e.ts` - 普通 TOTP 综合测试 ✅
- `demo/e2e/regular-user/totp-user-management-demo.e2e.ts` - TOTP 用户故事测试 ✅
- **状态**: ✅ 已实施

---

## 10. 参考资料

- **相关用户故事**: [docs/user-stories/05-totp-user-stories.md](/docs/user-stories/05-totp-user-stories.md)
- **RFC 6238**: TOTP 标准规范
- **Google Authenticator**: 验证器应用参考
- **与 Realm 设置的关系**: TOTP 配置属于 Realm Settings 的一部分
- **与登录流程的关系**: TOTP 验证是登录的第二步

