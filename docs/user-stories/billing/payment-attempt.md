# PaymentAttempt 支付尝试用户故事

**角色代码**: PA (Payment Attempt)
**角色定义**：System 或 Third-party App 创建支付尝试；用户完成支付；System 处理支付结果和履约。

**故事范围**: US-PA-001 ~ US-PA-004
**创建时间**: 2026-04-08
**状态**: Active

**依赖关系**:
- US-PA-001 依赖 US-PP-001（需要积分包商品）和 US-PR-001（需要订阅套餐）
- US-PA-001 依赖支付平台配置（docs/user-stories/billing/payment-provider.md）
- US-PA-002 依赖 US-PA-001（需要先有支付尝试）
- US-PA-003 依赖 US-PA-001 和 US-PA-002（需要支付尝试和状态查询）
- US-PA-004 无前置依赖，可并行实现
- 依赖微信支付/Stripe/Creem 集成（docs/user-stories/billing/wechat-pay.md）

---

## 故事 1：创建支付尝试（订阅或积分包） [US-PA-001]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：为发起式支付平台（Wechat/Stripe/Creem）创建统一的支付尝试（PaymentAttempt）
**从而**：统一不同平台的支付流程，并提供一致的支付体验

**【验收标准】**

**场景 1：创建订阅购买的支付尝试**
```gherkin
Given 用户选择了订阅套餐 "pro-monthly"
And 用户选择了支付平台 "WeChat Pay"
And 套餐 "pro-monthly" 已配置微信支付映射
When 系统创建支付尝试
Then 支付尝试创建成功，包含以下信息：
  | 字段             | 值                       |
  | ID               | pa-uuid-001              |
  | Realm ID         | realm-1                  |
  | User ID          | user-1                   |
  | Payment Provider | wechat                   |
  | Target Type      | subscription_plan        |
  | Target ID        | plan-uuid-pro            |
  | Amount           | 5000（分）               |
  | Currency         | CNY                      |
  | Status           | Pending                  |
  | Expires At       | 2 小时后                 |
And 系统调用微信支付 API 创建订单
And 系统返回支付上下文给前端（如二维码 URL）
```

**场景 2：创建积分包购买的支付尝试**
```gherkin
Given 用户选择了积分包 "credits-500"
And 用户选择了支付平台 "WeChat Pay"
And 积分包 "credits-500" 已配置微信支付映射
When 系统创建支付尝试
Then 支付尝试创建成功，包含以下信息：
  | 字段             | 值                       |
  | ID               | pa-uuid-002              |
  | Realm ID         | realm-1                  |
  | User ID          | user-1                   |
  | Payment Provider | wechat                   |
  | Target Type      | points_package           |
  | Target ID        | package-uuid-500         |
  | Amount           | 5000（分）               |
  | Currency         | CNY                      |
  | Status           | Pending                  |
  | Expires At       | 2 小时后                 |
And 系统调用微信支付 API 创建订单
And 系统返回支付上下文给前端（如二维码 URL）
```

**场景 3：Stripe 支付的支付尝试**
```gherkin
Given 用户选择了订阅套餐 "pro-yearly"
And 用户选择了支付平台 "Stripe"
When 系统创建支付尝试
Then 支付尝试创建成功
And 系统调用 Stripe API 创建 Checkout Session 或 Payment Intent
And 系统返回 Stripe Checkout URL 或 client_secret 给前端
```

**场景 4：PaymentAttempt 唯一性约束**
```gherkin
Given 用户创建了一个支付尝试
And 支付尝试状态为 "Pending"
When 用户尝试为同一目标创建另一个支付尝试
Then 系统允许创建（不阻止）
And 两个支付尝试互不干扰
And 每个支付尝试有独立的过期时间
```

**场景 5：Creem 支付的支付尝试**
```gherkin
Given 用户选择了订阅套餐
And 用户选择了支付平台 "Creem"
When 系统创建支付尝试
Then 支付尝试创建成功
And 系统调用 Creem API 创建支付订单
And 系统返回 Creem 支付 URL 给前端
```

---

## 故事 2：查询支付尝试状态 [US-PA-002]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：查询支付尝试的状态
**从而**：前端可以轮询展示支付进度，并在支付成功后履约

**【验收标准】**

**场景 1：查询 Pending 状态的支付尝试**
```gherkin
Given 已存在支付尝试 pa-001，状态为 "Pending"
When 前端查询支付尝试状态
Then 系统返回支付尝试信息：
  | ID     | pa-001        |
  | Status | Pending       |
  | Expires At | 2026-04-08 12:00:00 |
And 前端继续轮询
```

**场景 2：查询 Succeeded 状态的支付尝试**
```gherkin
Given 已存在支付尝试 pa-001，状态为 "Succeeded"
And 已完成履约（发放积分或创建订阅）
When 前端查询支付尝试状态
Then 系统返回支付尝试信息：
  | ID     | pa-001        |
  | Status | Succeeded     |
  | Completed At | 2026-04-08 10:05:00 |
And 前端停止轮询
And 前端展示支付成功页面
```

**场景 3：查询 Failed 状态的支付尝试**
```gherkin
Given 已存在支付尝试 pa-001，状态为 "Failed"
When 前端查询支付尝试状态
Then 系统返回支付尝试信息：
  | ID     | pa-001        |
  | Status | Failed        |
  | Provider Status | TRADE_FAILED |
And 前端停止轮询
And 前端展示支付失败页面
And 提供"重新支付"按钮
```

**场景 4：查询 Expired 状态的支付尝试**
```gherkin
Given 已存在支付尝试 pa-001，状态为 "Expired"
And 支付尝试已超过 2 小时未支付
When 前端查询支付尝试状态
Then 系统返回支付尝试信息：
  | ID     | pa-001        |
  | Status | Expired       |
And 前端停止轮询
And 前端展示"二维码已过期"提示
And 提供"重新获取二维码"按钮
```

**场景 5：主动查询平台状态（Webhook 未到达时的补偿）**
```gherkin
Given 已存在支付尝试 pa-001，状态为 "Pending"
And 支付尝试创建已超过 5 分钟
And 未收到 Webhook 回调
When 系统执行补偿查询
Then 系统调用微信支付查询订单 API
And 如果微信侧已支付成功，更新本地状态为 "Succeeded" 并执行履约
And 如果微信侧仍未支付，保持 "Pending" 状态
```

---

## 故事 3：处理支付成功后的履约 [US-PA-003]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：根据支付尝试的购买对象（PurchasableTarget）执行不同的履约逻辑
**从而**：正确处理订阅购买和积分包购买的履约结果

**【验收标准】**

**场景 1：订阅购买的履约（创建 Subscription + 发放 subscription_credit）**
```gherkin
Given 支付尝试 pa-001 的目标是订阅套餐 "pro-monthly"
And 支付平台回调通知支付成功
Or 系统主动查询发现支付成功
When 系统将支付尝试状态更新为 "Succeeded"
And 系统识别 Target Type 为 "subscription_plan"
Then 系统执行以下履约逻辑：
  1. 创建或更新 Subscription 记录
  2. 发放 subscription_credit（会员积分）
  3. 记录 PaymentEvent 审计日志
And 履约完成后，用户获得订阅权益和会员积分
```

**场景 2：积分包购买的履约（仅发放 topup_credit）**
```gherkin
Given 支付尝试 pa-002 的目标是积分包 "credits-500"
And 支付平台回调通知支付成功
Or 系统主动查询发现支付成功
When 系统将支付尝试状态更新为 "Succeeded"
And 系统识别 Target Type 为 "points_package"
Then 系统执行以下履约逻辑：
  1. 发放 topup_credit（充值积分），不创建 Subscription
  2. 记录 PaymentEvent 审计日志
And 履约完成后，用户仅获得充值积分
And 系统不创建订阅记录
```

**场景 3：履约幂等性保证**
```gherkin
Given 支付尝试 pa-001 已经履约完成
And 用户已收到积分和订阅
When 系统收到重复的 Webhook 回调（支付成功通知）
Or 系统重复查询到支付成功状态
Then 系统识别为重复履约
And 系统不重复发放积分
And 系统不重复创建订阅
And 系统返回幂等成功响应
```

**场景 4：履约失败处理**
```gherkin
Given 支付尝试 pa-001 支付成功
When 系统执行履约逻辑时发生错误（如数据库故障）
Then 系统将支付尝试状态标记为 "Succeeded"（支付已成功）
And 系统记录履约失败日志
And 系统触发告警通知管理员
And 系统提供手动补发积分的接口
```

**场景 5：不同积分类型的发放**
```gherkin
Given 支付尝试 pa-001 的目标是订阅套餐
When 系统执行履约
Then 系统发放 subscription_credit（会员积分）
And 积分有过期时间（随订阅周期）
Given 支付尝试 pa-002 的目标是积分包
When 系统执行履约
Then 系统发放 topup_credit（充值积分）
And 积分无过期时间（长期有效）
```

---

## 故事 4：关闭过期的支付尝试 [US-PA-004]

**优先级**: P1

**【用户故事】**
**作为**：Herald 系统
**我希望**：自动关闭过期的支付尝试
**从而**：防止用户扫描过期二维码导致支付异常

**【验收标准】**

**场景 1：自动关闭过期订单**
```gherkin
Given 存在创建超过 2 小时仍未支付的支付尝试 pa-001
And 支付尝试状态为 "Pending"
When 系统执行定时清理任务
Then 系统调用支付平台关单 API（如微信关单 API）
And 系统将支付尝试状态更新为 "Expired"
And 系统记录过期时间
```

**场景 2：关闭订单前确认未支付**
```gherkin
Given 存在即将过期的支付尝试 pa-001
When 系统准备关闭该支付尝试
Then 系统先查询支付平台侧该订单的实际状态
And 如果平台侧已支付，不执行关单，按支付成功处理并履约
And 如果平台侧未支付，执行关单操作
```

**场景 3：用户重新创建支付尝试**
```gherkin
Given 支付尝试 pa-001 已过期（状态为 "Expired"）
When 用户重新选择订阅套餐并支付
Then 系统创建新的支付尝试 pa-002
And pa-002 与 pa-001 互不干扰
And pa-002 有新的过期时间
```

**场景 4：过期支付尝试的查询**
```gherkin
Given 支付尝试 pa-001 已过期
When 前端查询支付尝试状态
Then 系统返回 "Expired" 状态
And 前端提示用户"二维码已过期，请重新获取"
And 前端提供"重新支付"按钮
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 3 | 创建支付尝试、查询支付尝试状态、处理支付成功后的履约 |
| P1 | 1 | 关闭过期的支付尝试 |
| P2 | 0 | - |

---

## PaymentAttempt 状态机

```text
[Pending] → (支付成功) → [Succeeded]
    ↓                         ↓
(过期/取消)              (已履约)
    ↓                         ↓
[Expired/Cancelled] ←──────────┘

[Pending] → (支付失败) → [Failed]
```

## 适用的支付平台

| 平台 | 适用 PaymentAttempt | 说明 |
|------|-------------------|------|
| Wechat | 是 | 发起式支付，扫码 + 轮询 |
| Stripe | 是 | 发起式支付，跳转 Checkout |
| Creem | 是 | 发起式支付，跳转支付页 |
| Shopify | 否 | Webhook-driven 订阅同步，不进入 PaymentAttempt |

---

## 相关文档

- **PRD**: `docs/prd/billing/unified-purchase.md` - 统一购买架构产品需求文档（待创建）
- **技术方案**: `.ai/future/order.md` - 统一购买架构技术方案
- **用户故事**: `billing/wechat-pay.md` - 微信支付用户故事
- **用户故事**: `billing/points-package.md` - 积分包管理用户故事
- **用户故事**: `11-points-package-purchase-user-stories.md` - 积分包购买用户故事
