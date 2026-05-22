# TOTP 二次认证用户故事

**角色代码**: TO
**角色定义**: TOTP 用户包括 Realm Admin（管理 TOTP 功能）和 Regular User（使用 TOTP 验证）。

**故事范围**: US-TO-001 ~ US-TO-007
**创建时间**: 2025-02-01
**状态**: Active

---

## 用户故事

### 故事 1：Realm 管理员启用/禁用 TOTP 功能 [US-TO-001]

**【用户故事】**
**作为**：Realm 管理员（详见 [`docs/user-stories/_roles.md`](/docs/user-stories/_roles.md)）
**我希望**：能够为本 Realm 启用或禁用 TOTP 二次认证功能
**从而**：提升本 Realm 用户账户的安全性

**【验收标准】**

**场景 1：启用 TOTP 功能**
Given Realm 管理员 admin-realm1 属于 realm-1
When 管理员在"Settings" -> "Security"页面中启用 TOTP
Then Realm realm-1 的 TOTP 功能被启用
And 本 Realm 用户可以设置和使用 TOTP

**场景 2：禁用 TOTP 功能**
Given Realm realm-1 已启用 TOTP 功能
When 管理员在"Settings" -> "Security"页面中禁用 TOTP
Then Realm realm-1 的 TOTP 功能被禁用
And 现有已启用 TOTP 的用户仍需验证（平滑降级）
And 新用户无法设置 TOTP

**场景 3：查看 TOTP 状态**
Given 管理员在"Settings" -> "Security"页面
Then 显示当前 TOTP 功能的启用状态（启用/禁用）

**场景 4：无法修改其他 Realm 设置（失败场景）**
Given 次管理员属于 realm-1
When 尝试访问 realm-2 的 TOTP 设置
Then 系统提示"权限不足"并返回 403 错误

---

### 故事 2：用户启用 TOTP 二次认证 [US-TO-002]

**【用户故事】**
**作为**：普通用户（详见 [`docs/user-stories/_roles.md`](/docs/user-stories/_roles.md)）
**我希望**：能够为我的账户启用 TOTP 二次认证
**从而**：提升账户安全性，防止密码泄露导致账户被盗

**【验收标准】**

**场景 1：正常启用 TOTP**
Given Realm realm-1 已启用 TOTP 功能
When 用户访问个人资料 -> "Security"页面
And 点击"Enable TOTP"按钮
Then 系统生成 TOTP 密钥并显示二维码
And 显示备份恢复码（10个6位数字）
And 用户输入验证器应用（如 Google Authenticator）中的6位验证码
And 验证码验证通过
Then TOTP 启用成功
And 下次登录需输入 TOTP 验证码

**场景 2：Realm 未启用 TOTP（失败场景）**
Given Realm realm-2 未启用 TOTP 功能
When 用户访问"Security"页面
Then 不显示"Enable TOTP"选项

**场景 3：验证码错误（失败场景）**
Given 用户在设置 TOTP 过程中
When 输入错误的 TOTP 验证码
Then 系统提示"验证码错误，请重试"
And 用户可重新输入验证码

**场景 4：验证码超时（失败场景）**
Given TOTP 验证码有效期为 30 秒
When 用户输入的验证码已过期
Then 系统提示"验证码已过期，请重新输入"
And 用户可输入新的验证码

**场景 5：保存备份恢复码**
Given 用户成功启用 TOTP
When 系统显示 10 个备份恢复码
Then 用户必须确认"已保存备份码"才能完成设置
And 备份恢复码仅显示一次

**场景 6：重复启用 TOTP（失败场景）**
Given 用户已启用 TOTP
When 再次访问"Security"页面
Then 显示"Disable TOTP"按钮而非"Enable TOTP"

---

### 故事 3：用户使用 TOTP 登录 [US-TO-003]

**【用户故事】**
**作为**：普通用户
**我希望**：在登录时能够通过 TOTP 验证码进行二次认证
**从而**：确保即使密码泄露，账户仍受保护

**【验收标准】**

**场景 1：正常 TOTP 登录流程**
Given 用户 user@example.com 已启用 TOTP
When 用户访问登录页面
And 输入正确的邮箱和密码
Then 登录第一步验证通过
And 系统显示 TOTP 验证码输入页面
And 用户输入 6 位 TOTP 验证码
And 验证码正确
Then 登录成功，系统设置 Session Cookie

**场景 2：TOTP 验证码错误（失败场景）**
Given 用户已启用 TOTP
When 在 TOTP 验证页面输入错误的 6 位验证码
Then 系统提示"验证码错误"
And 用户可重新输入
And 连续错误 5 次后返回登录页面

**场景 3：TOTP 验证码过期（失败场景）**
Given TOTP 验证码有效期为 30 秒
When 用户输入的验证码已过期
Then 系统提示"验证码已过期"
And 用户可输入新的验证码

**场景 4：使用备份恢复码登录**
Given 用户已启用 TOTP 但无法访问验证器应用
When 在 TOTP 验证页面点击"Use backup code"
And 输入一个有效的备份恢复码
Then 验证通过，登录成功
And 该备份恢复码被标记为已使用（不可重复使用）

**场景 5：备份恢复码耗尽（失败场景）**
Given 用户的所有 10 个备份恢复码都已使用
When 尝试使用备份恢复码登录
Then 系统提示"备份恢复码已耗尽，请联系管理员"

**场景 6：未启用 TOTP 的用户直接登录**
Given 用户 user@example.com 未启用 TOTP
When 输入正确的邮箱和密码
Then 直接登录成功，无需 TOTP 验证

---

### 故事 4：用户禁用 TOTP [US-TO-004]

**【用户故事】**
**作为**：普通用户
**我希望**：能够禁用我的 TOTP 二次认证
**从而**：在更换设备或不再需要时可以关闭此功能

**【验收标准】**

**场景 1：正常禁用 TOTP**
Given 用户已启用 TOTP
When 用户访问"Security"页面
And 点击"Disable TOTP"按钮
And 输入当前密码进行确认
Then TOTP 被禁用
And 下次登录无需 TOTP 验证码

**场景 2：密码验证失败（失败场景）**
Given 用户在禁用 TOTP 过程中
When 输入错误的当前密码
Then 系统提示"密码错误"
And TOTP 保持启用状态

**场景 3：Realm 强制启用 TOTP（失败场景）**
Given Realm realm-1 设置为"强制启用 TOTP"
When 用户尝试禁用 TOTP
Then 系统提示"本 Realm 要求必须启用 TOTP"
And TOTP 保持启用状态

---

### 故事 5：用户重新生成 TOTP 密钥 [US-TO-005]

**【用户故事】**
**作为**：普通用户
**我希望**：能够重新生成我的 TOTP 密钥和备份恢复码
**从而**：在丢失验证器应用或备份码时恢复访问

**【验收标准】**

**场景 1：正常重新生成 TOTP 密钥**
Given 用户已启用 TOTP
When 用户访问"Security"页面
And 点击"Regenerate TOTP Secret"按钮
And 输入当前密码进行确认
Then 旧 TOTP 密钥失效
And 生成新的 TOTP 密钥和二维码
And 生成新的 10 个备份恢复码
And 用户需重新配置验证器应用

**场景 2：密码验证失败（失败场景）**
Given 用户在重新生成 TOTP 密钥过程中
When 输入错误的当前密码
Then 系统提示"密码错误"
And 原有 TOTP 密钥保持有效

**场景 3：重新生成后需立即验证**
Given 用户重新生成 TOTP 密钥
When 显示新的二维码
Then 必须输入新验证码验证成功
And 验证失败时保留旧密钥（回滚机制）

---

### 故事 6：Realm 管理员强制启用 TOTP [US-TO-006]

**【用户故事】**
**作为**：Realm 管理员
**我希望**：能够设置本 Realm 强制启用 TOTP
**从而**：确保所有用户都必须使用二次认证

**【验收标准】**

**场景 1：启用强制 TOTP 模式**
Given Realm 管理员在"Settings" -> "Security"页面
When 启用"Force TOTP"选项
Then 所有本 Realm 用户必须启用 TOTP
And 未启用 TOTP 的用户下次登录时被要求设置
And 已启用 TOTP 的用户无法禁用

**场景 2：禁用强制 TOTP 模式**
Given Realm realm-1 处于"强制 TOTP"模式
When 管理员禁用"Force TOTP"选项
Then 用户可以选择禁用 TOTP
And 新用户可以选择不启用 TOTP

**场景 3：查看强制 TOTP 统计**
Given 管理员在"Settings" -> "Security"页面
Then 显示已启用 TOTP 的用户数量
And 显示未启用 TOTP 的用户数量
And 显示 TOTP 启用率

---

### 故事 7：用户查看 TOTP 使用情况 [US-TO-007]

**【用户故事】**
**作为**：普通用户
**我希望**：能够查看我的 TOTP 设置和使用情况
**从而**：了解我的账户安全状态

**【验收标准】**

**场景 1：查看 TOTP 状态**
Given 用户已启用 TOTP
When 用户访问"Security"页面
Then 显示 TOTP 启用状态（已启用）
And 显示启用时间
And 显示最近一次 TOTP 验证时间

**场景 2：查看备份恢复码使用情况**
Given 用户已启用 TOTP
When 用户访问"Security"页面
Then 显示剩余可用备份恢复码数量
And 显示已使用的备份恢复码数量

**场景 3：未启用 TOTP 时显示提示**
Given 用户未启用 TOTP
When 用户访问"Security"页面
Then 显示"启用 TOTP 可提升账户安全性"提示
And 显示"Enable TOTP"按钮

---

## 备注

### 业务规则

1. **TOTP 配置级别**：
   - **Realm 级别**：管理员可启用/禁用整个 Realm 的 TOTP 功能
   - **用户级别**：用户可选择是否启用 TOTP（若 Realm 允许）

2. **验证码规则**：
   - TOTP 验证码为 6 位数字
   - 有效期为 30 秒（支持时间漂移 ±1 个周期）
   - 连续错误 5 次后锁定 TOTP 验证 15 分钟

3. **备份恢复码**：
   - 启用 TOTP 时生成 10 个备份恢复码
   - 每个备份码为 6 位数字
   - 使用后立即失效（不可重复使用）
   - 重新生成 TOTP 密钥时同步重新生成备份码

4. **强制 TOTP 模式**：
   - Realm 管理员可强制所有用户启用 TOTP
   - 强制模式下，用户无法禁用 TOTP
   - 未启用 TOTP 的用户在下次登录时被要求设置

5. **安全规则**：
   - TOTP 密钥使用 AES-256-GCM 加密存储
   - 禁用 TOTP 需验证当前密码
   - 重新生成 TOTP 密钥需验证当前密码
   - TOTP 验证失败不暴露具体错误（统一提示"验证码错误"）

### 技术实现要点

1. **TOTP 算法**：
   - 使用 RFC 6238 标准（基于时间的一次性密码）
   - SHA-256 哈希算法
   - 6 位数字码，30 秒周期

2. **存储方案**：
   - 后端：`user_totp_config` 表存储用户 TOTP 配置
   - `secret_hash`：使用 AES-256-GCM 加密的 TOTP secret
   - `backup_codes`：使用 bcrypt hash 存储备份恢复码

3. **前端库**：
   - 二维码生成：`qrcode` 或 `qrcode.react`
   - 验证码输入：`react-otp-input`

4. **与现有功能集成**：
   - 扩展登录流程：登录成功后检查用户是否启用 TOTP
   - 扩展 Realm 配置：添加 `totp` 配置类型
   - 扩展 Session：记录 TOTP 验证状态

### 与其他功能的关系

| 功能 | 关系 |
|------|------|
| Realm 设置 | Realm 可启用/禁用 TOTP 功能 |
| 用户登录 | 启用 TOTP 后需二次验证 |
| 密码管理 | 禁用/重新生成 TOTP 需验证密码 |
| Session 管理 | TOTP 验证通过后创建 Session |
| OAuth 登录 | OAuth 用户也可启用 TOTP |

### 用户体验流程

```
首次启用 TOTP：
1. 用户访问 Security 页面
2. 点击"Enable TOTP"
3. 系统生成密钥并显示二维码
4. 用户扫描二维码添加到验证器应用
5. 输入验证码验证
6. 保存备份恢复码（仅显示一次）
7. TOTP 启用成功

使用 TOTP 登录：
1. 输入邮箱和密码
2. 第一步验证通过
3. 显示 TOTP 验证页面
4. 输入 6 位验证码或备份恢复码
5. 验证通过，登录成功
```

---

## 优先级

**故事 1-4**: P0（关键）- TOTP 核心功能（启用、登录、禁用）
**故事 5**: P1（重要）- 密钥恢复功能
**故事 6**: P1（重要）- 强制安全策略
**故事 7**: P2（一般）- 查看功能，增强用户体验

---

## 📖 相关PRD

- **TOTP 二次认证**: [docs/prd/auth/totp.md](/docs/prd/auth/totp.md)

---
