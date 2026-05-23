# 微信支付集成产品需求文档 (PRD)

**创建时间**: 2026-04-04
**优先级**: P1

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

**微信支付用户故事**
- `[US-WP-001]` 配置微信支付平台，优先级 P0，来源 `docs/user-stories/billing/wechat-pay.md`
- `[US-WP-002]` 查看微信支付平台配置，优先级 P0，来源 `docs/user-stories/billing/wechat-pay.md`
- `[US-WP-003]` 编辑微信支付平台配置，优先级 P1，来源 `docs/user-stories/billing/wechat-pay.md`
- `[US-WP-004]` 删除微信支付平台配置，优先级 P1，来源 `docs/user-stories/billing/wechat-pay.md`
- `[US-WP-005]` 用户通过微信扫码支付，优先级 P0，来源 `docs/user-stories/billing/wechat-pay.md`
- `[US-WP-006]` 微信支付 Webhook 回调处理，优先级 P0，来源 `docs/user-stories/billing/wechat-pay.md`
- `[US-WP-007]` 主动查询支付状态，优先级 P0，来源 `docs/user-stories/billing/wechat-pay.md`
- `[US-WP-008]` 关闭过期支付订单，优先级 P1，来源 `docs/user-stories/billing/wechat-pay.md`

**通用支付平台配置用户故事（部分复用）**
- `[US-PP-001]` 配置支付平台，优先级 P0，来源 `docs/user-stories/billing/payment-provider.md`
- `[US-PP-002]` 查看支付平台配置，优先级 P0，来源 `docs/user-stories/billing/payment-provider.md`

### 1.2 优先级汇总表

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 5 | 配置微信支付、查看配置、用户扫码支付、Webhook 处理、主动查询状态 |
| P1 | 3 | 编辑配置、删除配置、关闭过期订单 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 微信支付作为支付平台选项之一（与 Creem、Stripe、Shopify 并列）
- 微信支付配置管理（App ID、Merchant ID、私钥、序列号、API v3 Key、Notify URL）
- Native 支付下单（后端调用统一下单 API，前端生成二维码）
- Webhook 回调处理（SHA256-RSA 签名验证 + AEAD_AES_256_GCM 数据解密）
- 支付状态主动查询（Webhook 补充）
- 过期订单自动关闭
- 与现有 Billing/Subscription/Points 系统集成

### 2.2 不包含功能

- H5 支付（v1 仅支持 Native 扫码支付）
- JSAPI 支付（需在微信浏览器内完成）
- App 支付 / 小程序支付（Herald 无原生 App 或小程序）
- 微信退款 API（v1 通过管理后台人工处理退款）
- 多商户模式（一个 Realm 绑定一个微信商户号）
- 平台证书自动更新（v1 手动更新）
- 微信支付分账
- 对账文件下载

### 2.3 依赖项

- 通用支付平台配置系统（见 Billing PRD）
- Billing 订阅计费系统（`docs/prd/billing/subscription.md`）
- Points 积分系统（`docs/prd/billing/points.md`）
- Subscription History 订阅历史（`docs/prd/billing/subscription.md`）
- Realm 管理系统
- 用户管理系统
- 微信支付商户号和 API 凭据（需配置）
- 微信支付沙箱环境（需配置）

---

## 3. 需求概述

### 3.1 功能描述

微信支付集成是 Herald 系统支付平台选项之一，与 Creem（模拟平台）、Stripe 和 Shopify 并列。Realm Admin 可以选择使用微信支付作为订阅支付的处理平台，面向中国市场的用户提供 Native 扫码支付体验。

### 3.2 关键特性

- **Native 扫码支付**：后端调用统一下单 API 返回 code_url，前端生成二维码供用户扫描
- **回调加密处理**：Webhook 回调使用 AEAD_AES_256_GCM 加密，需解密后处理
- **双重状态确认**：Webhook 回调为主，前端轮询查询为辅，确保支付状态一致性
- **多租户支持**：每个 Realm 配置独立的微信支付商户号
- **过期订单管理**：二维码 2 小时过期，自动关闭过期订单

---

## 4. 业务规则与状态

### 4.1 业务规则

- **配置管理规则**：每个 Realm 可配置一个微信支付商户号；配置项包括 App ID、Merchant ID、商户 RSA 私钥（PEM 格式）、证书序列号、API v3 Key（32 字节）、Notify URL（必须 HTTPS）；敏感信息（私钥、API v3 Key）加密存储；不允许修改平台类型（创建后不可变）；编辑时敏感字段留空则保留现有值
- **权限控制**：只有 Realm Admin 可以查看和更新配置；敏感信息查看时显示脱敏信息
- **双重确认**：Webhook 回调 + 主动查询双重保障支付状态一致性
- **幂等处理**：同一商户订单号（out_trade_no）不重复发放积分
- **金额校验**：回调金额与订单金额必须一致才确认支付
- **安全加密**：私钥和 API v3 Key 加密存储，回调数据解密验证
- **数据隔离**：不同 Realm 的支付数据完全隔离；一个 Realm 绑定一个微信支付商户号
- **金额单位**：微信支付金额单位为分（整数），需与 Herald 内部金额表示转换
- **二维码有效期**：订单创建后 2 小时内有效

### 4.2 关键状态与异常

- **trade_state 处理**：SUCCESS 确认支付并发放积分；NOTPAY 保持待支付；CLOSED 更新为已关闭；REFUND 记录退款事件
- **签名验证失败**：返回 401，记录审计日志
- **金额不一致**：记录告警，不执行业务逻辑
- **商户响应时限**：需在 5 秒内返回 200 响应，否则微信会重试
- **过期订单处理**：创建超过 2 小时仍未支付的订单，关单前先查询微信侧实际状态，确认未支付才执行关单；如果实际已支付则按支付成功处理

---

## 5. 功能需求

### 5.1 核心需求

- **微信支付配置管理**：支持创建、查看（脱敏）、更新、删除配置
- **Native 支付下单**：使用 out_trade_no 调用微信统一下单 API → 获取 code_url → 前端渲染二维码 → 用户扫码支付
- **Webhook 回调处理**：验证 SHA256-RSA 签名 → 解密回调数据（AEAD_AES_256_GCM）→ 校验金额 → 根据 trade_state 执行业务逻辑 → 5 秒内返回 200
- **支付状态主动查询**：作为 Webhook 补充，前端轮询触发后端查询微信订单状态；系统定时任务补偿查询超过 5 分钟未收到回调的订单
- **过期订单管理**：定时扫描创建超过 2 小时仍未支付的订单 → 查询微信侧实际状态 → 确认未支付后关单
- **系统集成**：复用现有 Subscription 领域模型、Points 系统、Subscription History、Payment Event；微信支付订单号与 Herald 订单建立映射

### 5.2 验收目标

- Realm Admin 可以配置和管理微信支付
- Native 扫码支付流程正常工作
- Webhook 回调正确处理支付成功/失败/关闭/退款
- 支付状态主动查询正确返回实际状态
- 过期订单自动关闭
- 不同 Realm 的数据完全隔离
- 所有支付操作记录审计日志

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力范围**：微信支付集成的能力边界；不在 PRD 中列出端点、schema 或状态码细节
- **访问控制原则**：必须遵守 realm 隔离、Webhook 签名验证（SHA256-RSA）、回调数据解密（AEAD_AES_256_GCM）、幂等处理和金额校验约束
- **Webhook 回调 trade_state**：SUCCESS、NOTPAY、CLOSED、REFUND
- **幂等键**：Native 支付下单使用 out_trade_no 作为幂等键
- **Notify URL 隔离**：按 realm 隔离
- **兼容性要求**：与微信支付 API 的详细契约应下沉到技术设计或接口说明

---

## 7. 前端/交互约束

**适用性**: 适用

- **管理入口**：支付平台配置管理页面，包含微信支付配置列表、创建表单（App ID、Merchant ID、Private Key 文件上传、Serial No、API v3 Key、Notify URL）、编辑（密钥轮换）、删除
- **用户扫码支付流程**：
  - 选择套餐后显示支付方式选项
  - 选择"微信支付"后页面展示二维码和倒计时
  - 二维码下方提示"请使用微信扫描二维码完成支付"
  - 支付状态自动轮询（2-3 秒间隔）
  - 支付成功显示成功页面和订阅信息
  - 支付失败提供"重新支付"按钮
  - 二维码过期后提示并提供"重新获取"按钮
  - 未配置微信支付时禁用该支付选项
- **关键交互约束**：二维码有效期 2 小时，需展示倒计时；支付过程中用户可"取消支付"返回套餐选择；金额单位转换为用户友好的显示格式（分 → 元）
- **状态反馈**：敏感信息脱敏显示；配置状态展示；操作成功/失败反馈
- **权限可见性**：仅 Realm Admin 可访问配置管理页面

---

## 8. 已确认决策

### 8.1 已确认决策

- v1 仅支持 Native 扫码支付，不支持 H5/JSAPI/App/小程序支付
- 复用现有 Billing/Subscription/Points 领域模型，仅新增支付通道
- 一个 Realm 绑定一个微信支付商户号
- 采用双重确认机制（Webhook + 主动查询）

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/wechat-pay.md`、`docs/user-stories/billing/payment-provider.md`
- 相关 PRD：`docs/prd/billing/subscription.md`、`docs/prd/billing/points.md`、`docs/prd/billing/stripe-payment.md`、`docs/prd/billing/shopify-pay.md`
- 微信支付官方文档：[API v3](https://pay.weixin.qq.com/wiki/doc/apiv3/wxpay/pages/index.shtml)、[Native 支付](https://pay.weixin.qq.com/wiki/doc/apiv3/open/pay/chapter2_7_2.shtml)、[签名验证](https://pay.weixin.qq.com/wiki/doc/apiv3/wechatpay/wechatpay4_1.shtml)、[回调通知](https://pay.weixin.qq.com/wiki/doc/apiv3/wechatpay/wechatpay4_5.shtml)
