# Discord OAuth 登录 产品需求文档 (PRD)

**创建时间**: 2026-10-01
**优先级**: P1
**所属域**: auth

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/auth/support-discord.md`。

### 1.1 相关故事

| US-ID | 标题 | 角色 | 优先级 | 来源 |
|-------|------|------|--------|------|
| US-SD-001 | Discord OAuth Provider 配置 | Realm Admin | P1 | `docs/user-stories/auth/support-discord.md` |
| US-SD-002 | 使用 Discord 账号登录 | Regular User | P1 | `docs/user-stories/auth/support-discord.md` |

表达既有通用行为、不复制验收标准的既有故事：

- `docs/user-stories/auth/oauth-extension.md`（US-OE-001 OAuth Provider 配置管理，P0 — Provider 配置增删改查与启停的通用形态）
- `docs/user-stories/core/regular-user.md`（US-RU-003 OAuth 第三方登录，P1 — 用户侧第三方登录的通用价值）
- `docs/user-stories/auth/third-party-app.md`（US-TP-001 / US-TP-015 — 下游应用授权码流程与 SPA 发起 SSO；Discord 登录在下游授权上下文中同样适用）

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 0 | —（通用 Provider 配置管理已由已发布 US-OE-001 覆盖） |
| P1 | 2 | US-SD-001 Discord Provider 配置、US-SD-002 Discord 账号登录 |
| P2 | 0 | — |

---

## 2. 范围界定

### 2.1 包含功能

- **Discord 作为通用跳转式 OAuth Provider**：Realm Admin 为 Realm 配置并启用 Discord Provider（Client ID、Client Secret、Scopes、Enabled）后，用户在 Herald 登录页看到 "Discord" 登录按钮，经标准跳转式授权码流程完成 SSO 登录，与现有 Google/GitHub/Facebook/Apple 链路同构
- **Discord 身份映射**：以 Discord 用户唯一标识作为 provider 身份关联键；Discord 无跨应用统一标识，不参与 union 层匹配；显示名取 Discord 的全局显示名（缺失时回退用户名）；头像按 Discord 返回的头像资料展示，无头像时不显示头像
- **邮箱验证语义透传**：Discord 返回的邮箱验证状态如实采信——已验证邮箱可用于关联既有账号与自动建号；未验证邮箱不得关联既有账号、不得自动建号（防接管门控对 Discord 同样生效）
- **email 缺失显式拒绝**：Discord 凭据未携带邮箱时（如 Scopes 配置丢失 email 授权范围）明确拒绝登录并说明原因，不生成占位邮箱
- **既有通用机制全量适用**：四层用户匹配、注册政策门控（注册关闭不自动建号）、注册邮箱域白名单、登录同意闸门、下游应用 brokered 授权分支、state 一次性校验、per-IP 限流——均与 provider 无关，Discord 自动获得
- **默认 Scopes**：`identify` 与 `email`（Realm Admin 可调整）

### 2.2 不包含功能 (Out of Scope)

- **Discord 专属流程**：不做扫码、Bot、guild/服务器维度身份、Discord 客户端内嵌授权等非标准跳转式能力（如需各自独立立项）
- **新路由 / 新依赖 / 数据库变更**：Discord 完全走现有通用 Provider 登录/回调链路与现有配置存储，不新增端点、外部依赖或数据迁移
- **Scopes 白名单强校验**：不限制 Discord Provider 的 Scopes 可配置范围；误配（丢失 identify 或 email）以明确错误失败，可诊断可自愈（修正配置即恢复），与现有 provider 误配失败同级
- **占位邮箱与验证标志硬编码**：不生成占位邮箱；不沿用 Facebook provider 无可信逐邮箱验证标志时的硬编码处理（Discord 有真实验证标志，必须透传）
- **登录页/管理端前端改版**：登录按钮、配置表单、Provider 列表均由现有服务端配置与常量驱动自动纳入 Discord，无前端结构改动，不新增 provider 图标或专属词条

### 2.3 依赖项

- **Realm 与 Provider 配置管理**：Discord Provider 配置是 Realm 级资源，依赖现有 Settings → Providers 配置能力（US-OE-001）
- **现有通用 OAuth 登录/回调链路**：授权发起、回调处理、一次性 state 管理均复用现有能力
- **外部前提（不在本仓库范围）**：Realm Admin 须在 Discord Developer Portal 创建 Application、获取 Client ID/Secret，并按本 Realm 的公网回调地址注册 Redirect URI（Discord 要求精确匹配）；与其他 provider 接入前提一致

---

## 3. 需求概述

### 3.1 功能描述

为 Herald 增加第 5 个通用跳转式社交登录 Provider：Discord。Realm Admin 在后台为 Realm 配置 Discord OAuth 凭证并启用后，用户即可在登录页通过 Discord 账号完成 SSO 登录；凭据按既有身份匹配与安全门控规则关联既有账号或创建新账号。该扩展完全落在已发布 OAuth PRD（[oauth.md](oauth.md)）定义的通用跳转式链路之内，不引入新的产品流程、角色或业务状态。

### 3.2 关键特性

- Discord 与现有 Google/GitHub/Facebook/Apple 同构：跳转式授权码流程，无专属路由或专属交互
- 登录页按钮由服务端配置驱动自动出现/消失，前端零结构改动
- Discord 的邮箱验证标志是真实可信信号，如实透传参与防接管门控——相对 Facebook 的不可信标志处理是安全语义收紧而非放宽
- 默认 Scopes 为 identify + email，保证基本资料与邮箱可用；丢失关键 scope 的误配以明确错误暴露而非静默降级

---

## 4. 业务规则与状态

### 4.1 业务规则

**Provider 管理:**
- Discord 加入可配置 Provider 类型清单，配置字段与其他 provider 一致（Provider Type、Client ID、Client Secret、Scopes、Enabled）
- Discord Provider 为 Realm 级资源，仅 Realm Admin 可管理；可独立启用/禁用，禁用后登录页不显示 Discord 按钮、不参与授权
- 默认 Scopes 为 identify 与 email：identify 是获取 Discord 基本资料的最低要求，email 使 Discord 返回邮箱与验证标志

**登录与身份匹配:**
- Discord 登录走现有通用跳转式 Provider 登录/回调链路；回调是经一次性 state 约束的未认证入口，不依赖浏览器既有会话作为关联依据
- 身份匹配按既有顺序：provider 身份（Discord 用户唯一标识）→ 邮箱 → 建号；Discord 无跨应用统一标识，不参与 union 层匹配
- 邮箱命中既有账号时，要求 Discord 返回的邮箱已验证；未验证邮箱不得关联既有账号（防止经 provider 未验证邮箱接管既有密码账号）
- Discord 邮箱验证状态如实采信，不做硬编码改写

**建号门控:**
- 自动建号受 Realm 注册政策门控：注册关闭时 OAuth 路径不绕过政策自动建号，返回注册未开放提示，引导用户走显式注册入口；已命中既有用户的关联登录不受此门控影响
- 注册邮箱域白名单配置后，Discord 邮箱域名不在白名单内时建号同样被拒；白名单为空表示不限
- 邮箱未验证的 Discord 凭据同样不得自动建号

**同意闸门与下游授权:**
- Discord 登录的第一方直登与下游 brokered 授权分支均执行登录同意闸门，规则与现有 provider 一致：同意缺失或版本过期时不签发会话/授权码，按既有补全路径处理
- 下游应用经 Herald 发起授权时携带的下游事务标识在 Discord 登录中同样适用：认证结果转换为下游授权码，Herald 不为该用户建立自身会话

**异常处理（用户可见语义，非后端错误消息逐字契约）:**
- Discord 凭据未携带邮箱：登录被明确拒绝，错误信息说明邮箱缺失原因（常见于 Scopes 误配丢失 email），不生成占位邮箱
- Scopes 误配丢失 identify：Discord 侧资料获取失败，按既有"无法获取用户信息"口径呈现
- 用户在 Discord 授权页拒绝授权：按既有回调错误分支处理（下游分支携带错误重定向回下游，第一方分支返回统一拒绝体，不签发任何会话或授权码）
- Discord API 不可达/限流：按既有 provider 故障口径处理，不引入额外重试
- State 校验失败/过期：提示登录链接已过期，请重新发起登录（与现有 provider 一致）

### 4.2 关键状态与异常

- **Provider 状态**: Enabled / Disabled — 与其他 provider 一致，禁用后不在登录页展示、不参与授权流程
- **无新增业务状态**：Discord 登录不引入新的账号状态或流程状态；账号状态流转遵循既有规则
- **头像展示异常**: Discord 未提供头像时不显示头像（不物化默认头像）；头像仅为展示字段，异常不影响登录主流程

---

## 5. 功能需求

### 5.1 核心需求

- Provider 类型管理：Provider Type 下拉新增 "Discord" 选项，配置表单字段与既有 provider 完全一致（编辑时 Client Secret 留空保持不变）
- 配置默认值：选择 Discord 时 Scopes 默认填充 identify 与 email，可调整
- 登录页按钮：Discord Provider 配置存在且启用时，登录页自动渲染 "Discord" 登录按钮（显示名称 "Discord"，文本按钮，无图标）
- 登录流程：点击按钮 → 跳转 Discord 授权页 → 用户同意 → 回调完成身份匹配与门控 → 建立第一方会话（或按下游上下文签发授权码）
- 用户资料：首次建号时采用 Discord 返回的显示名与头像；显示名优先全局显示名，缺失时回退用户名

### 5.2 验收目标

- Realm Admin 可完成 Discord Provider 的完整增删改查与启停，行为与其他 provider 一致
- 配置并启用后登录页出现 Discord 按钮；禁用或删除后按钮消失
- 用户可经 Discord 完成首次登录（自动建号，受注册政策与域白名单门控）与既有账号关联登录（要求已验证邮箱）
- 未验证邮箱、注册关闭、邮箱缺失三类拒绝路径均返回明确的用户可见提示，不静默降级、不生成占位数据
- 下游应用可经 Discord 完成 brokered SSO 授权，与现有 provider 行为一致
- Discord 登录复用既有一次性 state、per-IP 限流与同意闸门，无安全边界放宽

---

## 6. API 相关约束

**适用性**: 适用（仅约束边界，无新端点）

- Discord 登录完全复用现有通用 Provider 登录/回调能力，**不新增任何端点**；Discord 作为 provider 类型值进入既有通用路径的合法取值集
- Provider 配置管理访问控制与其他 provider 一致：Realm 级资源，仅 Realm Admin 可访问
- 回调入口安全属性不变：未认证入口、一次性 state 校验、per-IP 限流；Discord 不引入新的安全边界，也不放宽既有边界
- 公开配置能力（登录页 provider 列表）自动包含已启用的 Discord Provider，前端据此时渲染按钮
- 详细接口契约与错误模型下沉技术设计；与 Discord 侧端点、客户端认证方式、用户字段映射的技术对接细节见技术预研（参考资料）

---

## 7. 前端/交互约束

**适用性**: 适用

- 配置入口：Settings → Providers Tab，Provider Type 下拉新增 "Discord"；表单字段（Client ID、Client Secret、Scopes、Enabled）与既有 provider 一致，编辑时 Secret 留空保持不变、前端不回显已存储 Secret
- 登录页：按钮由服务端返回的已启用 provider 列表驱动，Discord 启用后自动出现，显示名称 "Discord"（文本按钮，无图标映射）
- 管理端列表与表单由既有常量表驱动自动纳入 Discord，无新增 i18n 词条、无 provider 专属 UI 组件
- 交互反馈遵循既有 provider 行为：授权跳转、回调落地、错误提示口径一致

---

## 8. 已确认决策

| Decision ID | 状态 | 决策项 | 结论 | PRD 落点 | 来源 |
|---|---|---|---|---|---|
| `DEC-support-discord-001` | Applied | scope.discord-provider | 支持 Discord OAuth 登录：作为第 5 个通用跳转式 OAuth Provider 接入既有链路，复用配置管理、四层身份匹配与全部门控；不新增专属流程、路由、依赖或迁移 | §2 / §3 / §4 | `.ai/decision-log/support-discord.md` |
| `DEC-openid-connect-001` | Not Applicable | Herald 定位边界（SaaS 底座 vs 完整身份平台的 OIDC 能力范围） | 该决策不约束社交登录 provider 数量与接入；新增跳转式 provider 属既有能力范围内的按需扩展，不触及定位边界 | — | `.ai/decision-log/openid-connect.md` |

> 邮箱验证透传、email 缺失显式拒绝、默认 Scopes、不做 scope 白名单校验等为已发布规则（[oauth.md](oauth.md) §4.1）与技术预研结论在 Discord 上的落实，属 agent 授权的工程取舍，未达账本入账门槛，记录于 §2 / §4。

---

## 9. 参考资料

- 用户故事：`docs/user-stories/auth/support-discord.md`（US-SD-001/002）
- 既有用户故事：`docs/user-stories/auth/oauth-extension.md`、`docs/user-stories/core/regular-user.md`、`docs/user-stories/auth/third-party-app.md`
- 正式 PRD 基线：[oauth.md](oauth.md)（通用跳转式 Provider 链路、四层匹配、门控语义）
- 技术预研：`.ai/tech-research/support-discord.md`（端点对接、scope 语义、身份/邮箱/头像映射、email 拒绝、测试影响）
- 决策账本：`.ai/decision-log/support-discord.md`
- 外部参考：Discord Developer Documentation — OAuth2 与 Users Resource（授权/token 端点、客户端认证、scope 与用户字段语义）
