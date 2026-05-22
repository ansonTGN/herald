# 微信支付集成产品需求文档 (PRD)

**创建时间**: 2026-04-04
**状态**: Partially Implemented
**优先级**: P1

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `docs/user-stories/` 目录中的对应文件。

### 1.1 微信支付用户故事

- 📄 [docs/user-stories/billing/wechat-pay.md](/docs/user-stories/billing/wechat-pay.md)
  - **[US-WP-001] 配置微信支付平台** (P0): 作为 Realm Admin，我想要配置微信支付作为支付平台，以便用户可以使用微信支付完成订阅购买
  - **[US-WP-002] 查看微信支付平台配置** (P0): 作为 Realm Admin，我想要查看微信支付配置和状态，以便管理支付集成
  - **[US-WP-003] 编辑微信支付平台配置** (P1): 作为 Realm Admin，我想要更新微信支付配置，以便进行密钥轮换和配置变更
  - **[US-WP-004] 删除微信支付平台配置** (P1): 作为 Realm Admin，我想要删除微信支付配置，以便移除不再使用的平台
  - **[US-WP-005] 用户通过微信扫码支付** (P0): 作为 Regular User，我想要在 Herald 订阅页面扫码完成微信支付，以便使用微信便捷地购买订阅套餐
  - **[US-WP-006] 微信支付 Webhook 回调处理** (P0): 作为 Herald 系统，我想要接收并处理微信支付回调通知，以便及时同步支付状态并触发积分发放
  - **[US-WP-007] 主动查询支付状态** (P0): 作为 Herald 系统，我想要主动查询微信支付订单状态，作为 Webhook 的补充手段确保一致性
  - **[US-WP-008] 关闭过期支付订单** (P1): 作为 Herald 系统，我想要自动关闭过期的微信支付订单，防止用户扫描过期二维码

### 1.2 通用支付平台配置用户故事（部分复用）

- 📄 [docs/user-stories/billing/payment-provider.md](/docs/user-stories/billing/payment-provider.md)
  - **[US-PP-001] 配置支付平台** (P0): 微信支付作为平台选项之一，复用通用配置流程
  - **[US-PP-002] 查看支付平台配置** (P0): 复用通用查看流程

### 1.3 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 5 | 配置微信支付、查看配置、用户扫码支付、Webhook 处理、主动查询状态 |
| P1 | 3 | 编辑配置、删除配置、关闭过期订单 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ 微信支付作为支付平台选项之一（与 Creem、Stripe、Shopify 并列）
- ✅ 微信支付配置管理（App ID、Merchant ID、私钥、序列号、API v3 Key、Notify URL）
- ✅ Native 支付下单（后端调用统一下单 API，前端生成二维码）
- ✅ Webhook 回调处理（SHA256-RSA 签名验证 + AEAD_AES_256_GCM 数据解密）
- ✅ 支付状态主动查询（Webhook 补充）
- ✅ 过期订单自动关闭
- ✅ 与现有 Billing/Subscription/Points 系统集成

### 2.2 不包含功能 (Out of Scope)

- ❌ **H5 支付**（原因：v1 仅支持 Native 扫码支付，H5 支付面向移动浏览器场景，后续迭代添加）
- ❌ **JSAPI 支付**（原因：JSAPI 需要在微信浏览器内完成，Herald 当前无微信内嵌页面场景）
- ❌ **App 支付 / 小程序支付**（原因：Herald 当前无原生 App 或小程序）
- ❌ **微信退款 API**（原因：v1 通过管理后台人工处理退款，不集成退款 API）
- ❌ **多商户模式**（原因：一个 Realm 绑定一个微信商户号）
- ❌ **平台证书自动更新**（原因：v1 手动更新平台证书，后续可添加定时拉取）
- ❌ **微信支付分账**（原因：v1 不涉及分账业务）
- ❌ **对账文件下载**（原因：v1 不支持自动对账，后续迭代添加）

### 2.3 依赖项

- ✅ 通用支付平台配置系统（状态: 待实现，见 Billing PRD）
- ✅ Billing 订阅计费系统（状态: 部分实现，docs/prd/billing/subscription.md）
- ✅ Points 积分系统（状态: 已实现，docs/prd/billing/points.md）
- ✅ Subscription History 订阅历史（状态: 部分实现，docs/prd/billing/subscription.md）
- ✅ Realm 管理系统（状态: 已实现）
- ✅ 用户管理系统（状态: 已实现）
- ❌ 微信支付商户号和 API 凭据（状态: 待配置）
- ❌ 微信支付沙箱环境（状态: 待配置）

### 2.4 核心约束

- ✅ **双重确认**: Webhook 回调 + 主动查询双重保障支付状态一致性
- ✅ **幂等处理**: 同一商户订单号（out_trade_no）不重复发放积分
- ✅ **金额校验**: 回调金额与订单金额必须一致才确认支付
- ✅ **安全加密**: 私钥和 API v3 Key 加密存储，回调数据解密验证

---

## 3. 需求概述

### 3.1 功能描述

微信支付集成是 Herald 系统支付平台选项之一，与 Creem（模拟平台）、Stripe 和 Shopify 并列。Realm Admin 可以选择使用微信支付作为订阅支付的处理平台，面向中国市场的用户提供 Native 扫码支付体验。

**关键特性**：
1. **Native 扫码支付**: 后端调用统一下单 API 返回 code_url，前端生成二维码供用户扫描
2. **回调加密处理**: Webhook 回调使用 AEAD_AES_256_GCM 加密，需解密后处理
3. **双重状态确认**: Webhook 回调为主，前端轮询查询为辅，确保支付状态一致性
4. **多租户支持**: 每个 Realm 配置独立的微信支付商户号
5. **过期订单管理**: 二维码 2 小时过期，自动关闭过期订单

### 3.2 与其他支付平台的对比

| 特性 | Creem | Stripe | Shopify | WeChat Pay |
|------|-------|--------|---------|------------|
| 类型 | 模拟平台 | 真实支付 | 真实支付 | 真实支付 |
| 目标市场 | 开发测试 | 国际市场 | 电商订阅 | 中国市场 |
| 支付模式 | 重定向 | 重定向 | 订阅合同 | 扫码（Native） |
| Webhook 签名 | HMAC-SHA256 | HMAC-SHA256 | HMAC-SHA256 | SHA256-RSA（非对称） |
| 回调数据 | 明文 | 明文 | 明文 | AEAD 加密 |
| 前端交互 | 跳转第三方 | 跳转第三方 | 跳转第三方 | 本页二维码 + 轮询 |
| 金额单位 | 各异 | 各异 | 各异 | 分（整数） |
| 证书管理 | 无 | Webhook Secret | HMAC Secret | 商户私钥 + 平台证书 |

### 3.3 业务价值

- **用户价值**: 为中国用户提供熟悉的微信支付体验，降低支付门槛
- **业务价值**: 覆盖中国市场，提高支付转化率（微信月活 13 亿+）
- **技术价值**: 复用现有 Billing/Subscription/Points 领域模型，仅新增支付通道

### 3.4 核心场景：Native 扫码支付流程

**业务背景**：
微信 Native 支付与 Stripe/Shopify 的重定向模式不同，用户无需离开 Herald 页面，直接在当前页面扫描二维码完成支付。

**支付流程**：
1. 用户在 Herald 选择套餐，点击 "微信支付"
2. Herald 后端调用微信统一下单 API，获取 code_url
3. Herald 前端将 code_url 渲染为二维码
4. 用户使用微信扫描二维码完成支付
5. 微信发送 Webhook 回调通知 Herald 支付结果
6. Herald 验证签名、解密数据、更新订阅状态、发放积分
7. 前端轮询支付状态，显示支付成功

**关键约束**：
- 二维码有效期 2 小时，过期需关单并重新下单
- 商户需在 5 秒内返回 200 响应，否则微信会重试
- 除 Webhook 外，建议前端轮询支付状态作为兜底

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 通用支付平台配置 API | ❌ | 待实现（见 Billing PRD） |
| 微信支付配置管理 | ❌ | 待实现 |
| Native 支付下单 | ❌ | 待实现 |
| Webhook 端点 | ❌ | 待实现 |
| Webhook 签名验证 + 数据解密 | ❌ | 待实现 |
| 支付状态主动查询 | ❌ | 待实现 |
| 过期订单自动关闭 | ❌ | 待实现 |
| 与 Billing 系统集成 | ❌ | 待实现 |
| 前端二维码组件 | ❌ | 待实现 |
| 前端支付状态轮询 | ❌ | 待实现 |

---

## 5. 功能需求

### 5.1 微信支付配置管理

- 每个 Realm 可以配置一个微信支付商户号（支持多租户）
- 配置项包括：
  - App ID（微信公众平台/开放平台应用 ID）
  - Merchant ID（商户号）
  - 商户 RSA 私钥（PEM 格式，用于请求签名）
  - 证书序列号（商户证书序列号）
  - API v3 Key（32 字节，用于回调数据解密）
  - Notify URL（回调通知地址，必须 HTTPS）
- 所有敏感信息（私钥、API v3 Key）必须加密存储
- 只有 Realm Admin 可以查看和更新配置
- 敏感信息查看时显示脱敏信息
- 不允许修改平台类型（创建后不可变）
- **编辑时密钥保留**：更新配置时，敏感字段（商户私钥、API v3 Key）为可选；留空则保留现有值。非敏感字段（App ID、Merchant ID、Notify URL 等）正常更新

### 5.2 Native 支付下单流程

**下单流程**：
1. 用户选择套餐并点击 "微信支付"
2. 后端使用 `out_trade_no`（Herald 订单号）调用微信统一下单 API
3. 微信返回 `code_url`（二维码链接）
4. 后端将 `code_url` 返回给前端
5. 前端将 `code_url` 渲染为二维码
6. 二维码下方显示 "请使用微信扫描二维码完成支付" 提示
7. 前端开始轮询支付状态（2-3 秒间隔）

**约束条件**：
- `out_trade_no` 在同一商户下唯一，用于幂等控制
- 金额单位为**分**（整数），需与 Herald 内部金额表示转换
- Notify URL 必须包含 realm_id 以支持多租户隔离
- 订单创建后 2 小时内有效

### 5.3 Webhook 回调处理

**回调处理流程**：
1. 微信发送 POST 请求到 Herald Notify URL
2. Herald 验证 SHA256-RSA 签名（使用微信平台证书公钥）
3. Herald 使用 API v3 Key 解密回调数据（AEAD_AES_256_GCM）
4. Herald 校验金额与本地订单金额一致
5. Herald 根据 `trade_state` 执行业务逻辑：
   - `SUCCESS`: 确认支付，创建订阅，发放积分
   - `NOTPAY`: 保持待支付状态
   - `CLOSED`: 更新为已关闭状态
   - `REFUND`: 记录退款事件
6. Herald 在 5 秒内返回 200 响应

**幂等处理**：
- 基于 `out_trade_no` 做幂等判断
- 重复回调直接返回 200，不重复执行业务逻辑

**安全要求**：
- 签名验证失败返回 401，记录审计日志
- 金额不一致记录告警，不执行业务逻辑
- 所有回调事件必须记录审计日志

### 5.4 支付状态主动查询

- 作为 Webhook 的补充手段，前端轮询触发后端查询微信订单状态
- 后端调用微信查询订单 API，返回当前实际支付状态
- 系统定时任务补偿查询：超过 5 分钟未收到回调的订单，主动查询微信侧状态
- 查询结果与 Webhook 处理逻辑一致（幂等）

### 5.5 过期订单管理

- 定时任务扫描创建超过 2 小时仍未支付的订单
- 关单前先查询微信侧实际状态，确认未支付才执行关单
- 如果实际已支付，按支付成功处理
- 关单后本地订单状态更新为 "已关闭"

### 5.6 与现有系统集成

- 复用现有 Subscription 领域模型创建订阅记录
- 复用 Points 系统发放积分
- 复用 Subscription History 记录订阅变更
- 复用 Payment Event 记录支付事件
- 微信支付订单号（transaction_id）与 Herald 订单（out_trade_no）建立映射关系

### 5.7 数据隔离

- 不同 Realm 的支付数据完全隔离（通过 realm 路径）
- Webhook 端点按 Realm 隔离
- 一个 Realm 绑定一个微信支付商户号

### 5.8 安全要求

- 所有 API 调用使用 HTTPS
- Webhook 回调 URL 必须为 HTTPS
- 商户私钥和 API v3 Key 加密存储
- 日志中不记录完整密钥、证书内容
- 签名验证防止伪造回调
- 幂等处理防止重复发放
- 金额校验防止金额篡改

---

## 6. API 相关约束

**状态**: 必填

- 仅说明微信支付集成的能力边界，不在 PRD 中列出端点、schema 或状态码细节。
- 必须遵守 realm 隔离、Webhook 签名验证（SHA256-RSA）、回调数据解密（AEAD_AES_256_GCM）、幂等处理和金额校验约束。
- Webhook 回调处理必须支持以下 trade_state：
  - `SUCCESS`: 支付成功
  - `NOTPAY`: 未支付
  - `CLOSED`: 已关闭
  - `REFUND`: 转入退款
- 与微信支付 API 的详细契约应下沉到技术设计、接口说明或实现代码。
- Native 支付下单使用 `out_trade_no` 作为幂等键。
- Notify URL 按 realm 隔离。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留管理入口、关键操作路径、支付交互流程和状态反馈，不写组件实现、数据层封装或代码结构。

**微信支付配置页面**：
- 配置列表展示（Platform、App ID、Merchant ID、Last Updated、Actions）
- 配置创建表单（App ID、Merchant ID、Private Key 文件上传、Serial No、API v3 Key、Notify URL）
- 配置编辑功能（支持密钥轮换）
- 配置删除功能（无活跃订阅时可删除）
- 敏感信息脱敏显示

**用户扫码支付流程**：
- 用户选择套餐后，显示支付方式选项
- 选择 "微信支付" 后，页面展示二维码和倒计时
- 二维码下方提示 "请使用微信扫描二维码完成支付"
- 支付状态自动轮询（2-3 秒间隔）
- 支付成功后显示成功页面和订阅信息
- 支付失败后提供 "重新支付" 按钮
- 二维码过期后提示并提供 "重新获取" 按钮
- 未配置微信支付时禁用该支付选项

**关键交互约束**：
- 二维码有效期 2 小时，需展示倒计时
- 支付过程中用户可 "取消支付" 返回套餐选择
- 金额单位转换为用户友好的显示格式（分 → 元）


## 8. 相关文件索引

### 8.1 后端文件

- 领域层：复用现有 Billing/Realm Config 实体
- 基础设施层：新增微信支付 API 客户端模块
- 应用层：新增微信支付 Webhook 处理器和路由
- 数据库迁移：待技术设计文档定义

### 8.2 前端文件

- 支付平台配置管理页面（修改现有页面）
- 微信支付二维码组件（新增）
- 支付平台配置表单（扩展微信支付支持）

### 8.3 测试文件

- 微信支付配置场景测试（新增）
- 微信支付 Webhook 场景测试（新增）
- 微信支付流程 E2E 测试（新增，可选）

---

## 9. 参考资料

### 9.1 微信支付官方文档
- [微信支付 API v3 文档](https://pay.weixin.qq.com/wiki/doc/apiv3/wxpay/pages/index.shtml)
- [Native 支付接入指南](https://pay.weixin.qq.com/wiki/doc/apiv3/open/pay/chapter2_7_2.shtml)
- [签名验证指南](https://pay.weixin.qq.com/wiki/doc/apiv3/wechatpay/wechatpay4_1.shtml)
- [回调通知处理](https://pay.weixin.qq.com/wiki/doc/apiv3/wechatpay/wechatpay4_5.shtml)

### 9.2 SDK 文档
- [wechat-pay-rust-sdk](https://github.com/dounine/wechat-pay-rust-sdk) - Rust 微信支付 SDK

### 9.3 相关用户故事
- 📄 [docs/user-stories/billing/wechat-pay.md](/docs/user-stories/billing/wechat-pay.md) - 微信支付用户故事
- 📄 [docs/user-stories/billing/payment-provider.md](/docs/user-stories/billing/payment-provider.md) - 通用支付平台配置用户故事

### 9.4 相关 PRD
- [Billing 订阅计费 PRD](/docs/prd/billing/subscription.md) - 现有订阅计费系统
- [Points 积分系统 PRD](/docs/prd/billing/points.md) - 积分发放和回收逻辑
- [Subscription History PRD](/docs/prd/billing/subscription.md) - 订阅变更历史
- [Stripe Payment PRD](/docs/prd/billing/stripe-payment.md) - Stripe 支付集成参考
- [Shopify Pay PRD](/docs/prd/billing/shopify-pay.md) - Shopify Pay 支付集成参考

### 9.5 技术资源
