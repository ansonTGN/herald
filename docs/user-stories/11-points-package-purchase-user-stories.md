# 积分包购买用户故事

**角色代码**: PU (Purchase User)
**角色定义**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）购买积分包获得充值积分（topup_credit）。

**故事范围**: US-PU-06 ~ US-PU-08
**创建时间**: 2026-04-08
**状态**: Active

**依赖关系**:
- US-PU-06 依赖 US-PP-003（需要积分包配置支付平台映射）
- US-PU-06 依赖 US-PA-001（需要创建支付尝试功能）
- US-PU-07 依赖 US-PU-06（需要先有购买记录）
- US-PU-08 无前置依赖，可并行实现
- 依赖积分系统（docs/prd/billing/points.md）
- 依赖支付平台配置（docs/user-stories/07-payment-provider-user-stories.md）

---

## 故事 1：购买积分包 [US-PU-06]

**优先级**: P0

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：购买积分包获得充值积分
**从而**：快速增加积分余额，无需订阅

**【验收标准】**

**场景 1：通过微信支付购买积分包**
```gherkin
Given 我是已登录用户
And 当前 Realm 已配置微信支付
And 已存在 enabled 积分包 "credits-500"（500 积分，¥50）
And 该积分包已配置 enabled 微信支付映射
When 我访问积分购买页面
And 我选择积分包 "500 积分包"
And 我点击 "微信支付" 按钮
Then 页面显示一个二维码
And 二维码下方显示 "请使用微信扫描二维码完成支付" 提示
And 页面显示积分包信息：
  | 积分包 | 500 积分包 |
  | 积分数 | 500 积分   |
  | 价格   | ¥50.00     |
When 我使用微信扫描二维码并完成支付
Then 页面显示 "支付成功"
And 系统发放 500 充值积分（topup_credit）到我的账户
And 页面显示我的新积分余额
And 系统不创建订阅记录
```

**场景 2：通过 Stripe 支付购买积分包**
```gherkin
Given 我是已登录用户
And 当前 Realm 已配置 Stripe 支付
And 已存在 enabled 积分包 "credits-1000"（1000 积分，$10）
And 该积分包已配置 enabled Stripe 支付映射
When 我访问积分购买页面
And 我选择积分包 "1000 积分包"
And 我点击 "Stripe 支付" 按钮
Then 页面跳转到 Stripe Checkout 页面
And Stripe 页面显示积分包信息和价格
When 我在 Stripe 页面完成支付
Then Stripe 页面跳转回 Herald
And Herald 页面显示 "支付成功"
And 系统发放 1000 充值积分（topup_credit）到我的账户
And 系统不创建订阅记录
```

**场景 3：积分包购买失败**
```gherkin
Given 我是已登录用户
And 我已经创建了积分包购买订单
When 支付平台回调通知支付失败
Then 页面显示 "支付失败" 提示
And 提供 "重新支付" 按钮
And 系统不发放积分
And 系统不创建订阅记录
```

**场景 4：积分包支付二维码过期**
```gherkin
Given 我已经获取了积分包购买的支付二维码
When 二维码超过 2 小时未支付
Then 页面显示 "二维码已过期" 提示
And 提供 "重新获取二维码" 按钮
When 我点击 "重新获取二维码"
Then 系统关闭旧的 PaymentAttempt 并生成新的二维码
```

**场景 5：未配置支付平台时禁用按钮**
```gherkin
Given 我是已登录用户
And 已存在 enabled 积分包 "credits-500"
And 当前 Realm 未配置微信支付
When 我查看积分包 "credits-500"
Then "微信支付" 按钮为禁用状态
And 显示提示 "该支付方式暂未开通"
```

**场景 6：个人中心 Points 入口按 enabled 积分包显示**
```gherkin
Given 我是已登录用户
And 当前 Realm 已存在 enabled 积分包
When 我打开个人中心
Then 侧边栏显示 "Points" 菜单
When 当前 Realm 不存在 enabled 积分包
Then 侧边栏不显示 "Points" 菜单
```

**场景 7：查看可用积分包列表**
```gherkin
Given 我是已登录用户
And 当前 Realm 已存在 enabled 积分包
When 我访问积分购买页面
Then 我看到所有可用的积分包：
  | 积分包名称  | 积分数 | 价格    | 可用支付平台       |
  | 500 积分包  | 500    | ¥50.00  | 微信支付, Stripe  |
  | 1000 积分包 | 1000   | ¥90.00  | 微信支付          |
  | 2000 积分包 | 2000   | ¥170.00 | Stripe            |
And 我可以看到每个积分包的性价比（积分/价格比）
And 我可以选择购买任意积分包
```

**场景 8：存在积分包但无可用支付平台映射**
```gherkin
Given 我是已登录用户
And 当前 Realm 已存在 enabled 积分包 "credits-500"
And 该积分包没有 enabled 支付平台映射
When 我访问积分购买页面
Then 我可以看到 "credits-500"
And 购买按钮为禁用状态
And 系统提示 "该积分包暂无可用支付方式"
```

---

## 故事 2：查看积分包购买记录 [US-PU-07]

**优先级**: P1

**【用户故事】**
**作为**：Regular User
**我希望**：查看我的积分包购买记录
**从而**：追踪我的充值积分（topup_credit）来源

**【验收标准】**

**场景 1：查看积分包购买历史**
```gherkin
Given 我是已登录用户
And 我有 3 次积分包购买记录
When 我访问"我的积分"页面的"购买记录"标签
Then 我看到积分包购买历史列表：
  | 购买时间        | 积分包   | 积分数 | 支付平台   | 支付金额 | 交易 ID |
  | 2026-04-08 10:00| 500 积分包 | 500 | WeChat Pay | ¥50.00 | pay_001 |
  | 2026-04-05 15:30| 1000 积分包| 1000| Stripe     | $10.00 | pay_002 |
  | 2026-04-01 09:15| 500 积分包 | 500 | WeChat Pay | ¥50.00 | pay_003 |
And 列表按时间倒序排列
```

**场景 2：查看单次购买详情**
```gherkin
Given 我是已登录用户
And 我有积分包购买记录
When 我在购买列表中点击某条记录
Then 我看到购买详情：
  | 字段         | 内容                       |
  | 购买时间     | 2026-04-08 10:00:00 UTC   |
  | 积分包名称   | 500 积分包                 |
  | 积分数       | 500                        |
  | 积分类型     | topup_credit               |
  | 支付平台     | WeChat Pay                 |
  | 支付金额     | ¥50.00                     |
  | 支付状态     | 成功                       |
  | 交易 ID     | pay_001                    |
  | 积分发放时间 | 2026-04-08 10:01:00 UTC   |
```

**场景 3：按支付平台筛选购买记录**
```gherkin
Given 我是已登录用户
And 我有多个支付平台的购买记录
When 我选择筛选"支付平台：WeChat Pay"
Then 我只看到通过微信支付购买的记录
And 其他支付平台的记录不显示
```

**场景 4：按时间范围筛选购买记录**
```gherkin
Given 我是已登录用户
And 我有跨越多月的购买记录
When 我选择时间范围"2026-04-01 到 2026-04-30"
Then 我只看到该时间范围内的购买记录
And 范围外的记录不显示
```

---

## 故事 3：积分包与订阅购买的区别 [US-PU-08]

**优先级**: P1

**【用户故事】**
**作为**：Regular User
**我希望**：理解积分包购买和订阅购买的区别
**从而**：选择最适合我的购买方式

**【验收标准】**

**场景 1：查看购买方式对比说明**
```gherkin
Given 我是已登录用户
When 我访问购买页面
Then 我看到两种购买方式的对比说明：
  | 购买方式 | 积分包购买           | 订阅购买               |
  | 积分类型 | 充值积分（topup_credit）| 会员积分（subscription_credit）|
  | 有效期   | 长期有效             | 随订阅周期过期         |
  | 是否续费 | 否（一次性）         | 是（自动续费）         |
  | 履约结果 | 仅发放积分           | 创建订阅 + 发放积分    |
  | 适用场景 | 临时使用、按需充值   | 长期使用、持续获得积分  |
And 我可以点击"了解更多"查看详细说明
```

**场景 2：在购买页面的提示信息**
```gherkin
Given 我是已登录用户
When 我选择购买积分包
Then 我看到提示信息：
  "购买积分包后，积分将立即发放到您的账户，积分长期有效，不会过期。"
When 我选择购买订阅
Then 我看到提示信息：
  "购买订阅后，您将在每个计费周期获得会员积分，积分随订阅周期过期。"
```

**场景 3：购买成功后的反馈差异**
```gherkin
Given 我是已登录用户
When 我购买积分包成功
Then 我看到成功消息：
  "购买成功！500 充值积分已发放到您的账户，积分长期有效。"
And 我的充值积分余额增加
And 系统不显示订阅信息
When 我购买订阅成功
Then 我看到成功消息：
  "订阅成功！您已订阅「专业版」，1000 会员积分已发放，积分将在下次续费前有效。"
And 我的会员积分余额增加
And 系统显示订阅信息和下次续费时间
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 1 | US-PU-06: 购买积分包 |
| P1 | 2 | US-PU-07: 查看积分包购买记录, US-PU-08: 理解积分包与订阅区别 |
| P2 | 0 | - |

---

## 与订阅购买的区别

| 特性 | 积分包购买 | 订阅购买 |
|------|-----------|---------|
| 购买对象 | PointsPackage | Plan |
| 履约结果 | 发放 topup_credit | 创建 Subscription + 发放 subscription_credit |
| 积分类型 | topup_credit | subscription_credit |
| 积分有效期 | 长期有效（无过期） | 随订阅周期过期 |
| 是否续费 | 否（一次性购买） | 是（自动续费） |
| 适用 PaymentAttempt | 是 | 是（除 Shopify 外） |
| 适用平台 | Wechat, Stripe, Creem | Wechat, Stripe, Creem, Shopify |

---

## 相关文档

- **PRD**: `docs/prd/billing/unified-purchase.md` - 统一购买架构产品需求文档（待创建）
- **PRD**: `docs/prd/billing/points.md` - 积分系统产品需求文档
- **用户故事**: `10-points-package-user-stories.md` - 积分包管理用户故事
- **用户故事**: `points-user-view.md` - 积分查询用户故事
- **技术方案**: `.ai/future/order.md` - 统一购买架构技术方案
