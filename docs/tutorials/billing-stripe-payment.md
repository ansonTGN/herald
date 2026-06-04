# Stripe 支付对接：从开通到收到付款

面向 Realm Admin 的 Stripe 对接操作指南。跟着做完，你的用户就能通过 Stripe 完成订阅支付和一次性购买（积分包）。

## 给谁看

负责管理 Herald 计费配置的 Realm Admin。全程在管理后台操作，不需要写代码。

## 前置条件

- Herald 管理后台可以正常访问，你有管理员账号
- 已创建至少一个 Realm 和一个 Client App
- 已有 Stripe 账号。没有的话去 [Stripe 官网](https://stripe.com) 注册，参考 [Stripe 快速上手](https://docs.stripe.com/get-started)

## 核心概念

先搞清楚 Herald 和 Stripe 两边的概念怎么对应。

Stripe 侧的 **Product** 是你卖的东西的大类（比如"AI 写作助手"）。Herald 侧的 **Plan（套餐）** 是具体的计费方案，挂在 Product 下面（比如"月付 $9.99"）。一个 Product 可以有多个 Plan。

Plan 本身不绑定支付平台。你需要配置 **Payment Provider Mapping（支付映射）**，告诉系统这个套餐在 Stripe 里对应哪个商品。Herald 同时支持 Creem 和 Stripe，一个套餐可以映射到多个支付平台，但本教程只涉及 Stripe。

对于一次性购买（积分包），Herald 使用 Stripe 的 **Payment Intent** API。积分包也需要独立的支付映射。

数据流向：Product → Plan → Mapping（映射到 Stripe） → 分配给 App → 用户购买 → Stripe Webhook 回调 → 订阅生效 / 积分到账

---

## Step 1: 配置 Stripe 连接

在 Herald 管理后台填入 Stripe 凭证。

1. 在左侧菜单 **Products & Payments** 分组下点击 **Payment Providers**
2. 在支付渠道列表中找到 **Stripe**，点击 **Configure**
3. 填写配置表单：
   - **Enable Stripe**：打开开关
   - **Publishable Key**（必填）：Stripe Publishable Key，以 `pk_` 开头。在 Stripe Dashboard → Developers → API Keys 中获取。测试环境用 `pk_test_` 开头的 key
   - **Secret Key**（必填）：Stripe Secret Key，以 `sk_` 开头。测试环境用 `sk_test_` 开头的 key
   - **Webhook Secret**（必填）：Webhook 签名验证密钥，以 `whsec_` 开头。创建 Webhook 端点后获取，下一步会讲
4. 点击 **Save**

Herald 通过 API Key 前缀自动判断环境：`sk_test_` 走测试环境，`sk_live_` 走生产环境。不需要单独配置。

## Step 2: 配置 Stripe Webhook

Stripe 通过 Webhook 把支付结果推送给 Herald。这是整个对接中最关键的一步。

### 创建 Webhook 端点

1. 打开 [Stripe Dashboard → Developers → Webhooks](https://dashboard.stripe.com/webhooks)
2. 确认处于测试模式（右上角 toggle），开发阶段先配测试环境
3. 点击 **Add endpoint**
4. 填写：
   - **Endpoint URL**：`https://你的Herald域名/api/third/pay/{realmId}/stripe/webhooks`
     - 把 `{realmId}` 替换成你的 realm ID，比如 `admin`
     - 本地开发用 ngrok 等工具暴露 Herald 服务，URL 格式为 `https://{ngrok-domain}/api/third/pay/{realmId}/stripe/webhooks`
   - **Listen to**：选择 **Events**（不是 Account events）

### 选择 Webhook 事件

点击 **Select events** 后，需要勾选以下事件。这些是 Herald 代码实际处理的事件，缺任何一个都会导致对应的支付流程断裂。

| 事件 | Stripe 中的路径 | Herald 处理逻辑 |
|------|----------------|----------------|
| `checkout.session.completed` | Checkout → checkout.session.completed | 完成结账，创建订阅或完成一次性支付 |
| `customer.subscription.created` | Customers → customer.subscription.created | 新订阅创建，发放积分 |
| `customer.subscription.updated` | Customers → customer.subscription.updated | 订阅升降级处理 |
| `customer.subscription.deleted` | Customers → customer.subscription.deleted | 订阅取消（即时或期末取消） |
| `charge.refunded` | Payments → charge.refunded | 退款，按比例回收积分 |
| `invoice.payment_succeeded` | Billing → invoice.payment_succeeded | 续费成功，发放续费积分 |
| `payment_intent.succeeded` | Payments → payment_intent.succeeded | 一次性支付（积分包购买）成功 |
| `payment_intent.payment_failed` | Payments → payment_intent.payment_failed | 支付失败标记 |

以上 8 个事件必须全部勾选。Herald 收到不在列表中的事件会安全忽略（记录日志并返回 200），但缺少任何一个已列事件会导致支付状态不同步。

### 获取 Webhook Secret

1. 端点创建完成后，在端点详情页找到 **Signing secret**
2. 点击 **Reveal**，复制以 `whsec_` 开头的值
3. 回到 Herald 管理后台 → Payment Providers → Stripe 配置，把这个值填入 **Webhook Secret**

如果你在前面 Step 1 还没拿到 Webhook Secret，现在填回去就行。

### 生产环境

上线前，切换 Stripe Dashboard 到生产模式，重复以上步骤创建生产环境的 Webhook 端点。生产环境的 API Key 和 Webhook Secret 与测试环境完全独立，需要单独配置。配置方式一样，只是 Key 前缀不同（`sk_live_`、`pk_live_`、`whsec_` 生产值）。

## Step 3: 产品映射

这一步把 Herald 的套餐和积分包关联到 Stripe 的商品。

### Stripe 侧创建商品

在 [Stripe Dashboard → Products](https://dashboard.stripe.com/products) 中创建你的产品：

1. 点击 **Add product**
2. 填写产品名称（比如"AI Writing Assistant Pro"）
3. 价格设为 **Recurring**（周期性），选择月付或年付
4. 创建完成后记下 **Product ID**（形如 `prod_xxxxxxxxxxxx`）

Herald 使用 Stripe 的 inline pricing 创建 Checkout Session，不需要在 Stripe 中预创建 Price。但 Product ID 是映射的必要字段。

### 订阅套餐映射

在 Herald 管理后台操作：

1. 在 **Subscription Plans** 页面，找到你的套餐
2. 点击右侧操作菜单里的 **Manage Providers**
3. 在弹出的对话框中，点击 **Add Provider**
4. 填写映射信息：
   - **Payment Provider**：选择 **Stripe**
   - **External Product ID**：填入 Stripe 的 Product ID（比如 `prod_abc123def456`）
   - **Enabled**：打开
5. 点击 **Add Provider**

映射成功后，套餐列表的 **Payment Providers** 列会显示 "Stripe" 标签。

### 积分包映射

积分包使用相同的映射机制：

1. 在 **Points Packages** 页面，找到你的积分包
2. 点击 **Manage Providers**
3. 添加 Stripe 映射，填入对应的 Stripe Product ID

积分包走 Stripe Payment Intent 流程（一次性支付），不走 Checkout Session。用户购买时会获得一个 Stripe 支付表单，完成支付后积分自动到账。

## Step 4: 分配套餐到 Client App

用户只能看到分配给当前 Client App 的套餐。

1. 在 **Subscription Plans** 页面，找到你的套餐
2. 点击右侧操作菜单里的 **Assign to App**
3. 勾选要分配的 Client App，确认

一个套餐可以分配给多个 Client App。

## Step 5: 用户支付流程

配置完成。以下是用户侧的流程，你不需要操作 Herald 后台。

### 订阅支付（Checkout Session）

1. 用户在你的应用中选择套餐
2. 你的应用调用 Herald 的 Checkout API（传入 `plan_id`、`payment_provider=stripe`、`billing_period`）
3. Herald 调用 Stripe API 创建 Checkout Session，返回一个 Stripe 支付页面 URL
4. 用户跳转到 Stripe 托管的支付页面，填写卡号完成付款
5. Stripe 发送 `checkout.session.completed` 和 `customer.subscription.created` Webhook 给 Herald
6. Herald 创建订阅记录、发放积分
7. 后续每个计费周期，Stripe 自动扣款并发送 `invoice.payment_succeeded`，Herald 发放续费积分

### 一次性支付（Payment Intent / 积分包购买）

1. 用户选择积分包
2. 你的应用调用 Herald 的 Payment Attempt API
3. Herald 调用 Stripe Payment Intent API，返回 `client_secret`
4. 前端使用 Stripe.js 展示支付表单，用户完成支付
5. Stripe 发送 `payment_intent.succeeded` Webhook 给 Herald
6. Herald 完成支付、发放积分

### 退款

如果用户在 Stripe Dashboard 中发起退款：
- Stripe 发送 `charge.refunded` Webhook
- Herald 根据退款类型（`topup` 或 `subscription`）回收积分
  - 充值退款：按退款比例回收充值积分
  - 订阅退款：回收未使用的订阅积分

## Step 6: 确认结果

### 查看订阅状态

1. 在左侧菜单 **Transactions** 分组下点击 **Subscription History**
2. 查看订阅记录，确认状态为 Active

### 查看支付记录

1. 在 **Transactions** 下查看支付历史
2. 确认积分发放和扣款记录匹配

如果 Webhook 正常回调，状态会在几秒内更新。如果迟迟没变化，看下面的常见问题。

---

## 常见问题

### Webhook 没收到

Stripe 通过 Webhook 通知支付结果。排查步骤：

1. 确认 Herald 部署的公网地址可以从 Stripe 服务器访问到（本地开发用 ngrok）
2. 确认 Webhook 端点 URL 中的 realm ID 正确
3. 在 Stripe Dashboard → Webhooks → 你的端点，查看 **Attempts** 日志，看 Stripe 发送了什么、返回了什么
4. 确认 Step 1 中的 Webhook Secret 和端点的 Signing secret 一致

### Webhook 签名验证失败

Herald 使用 HMAC-SHA256 验证签名。常见原因：

- Webhook Secret 填错了（必须以 `whsec_` 开头）
- Herald 前面有反向代理修改了请求体（Nginx 的 `proxy_request_buffering` 关闭时可能出现）
- 签名时间戳超过 15 分钟（时钟不同步）

### 支付成功但订阅未激活

大概率是 Webhook 没到达 Herald。检查 Herald 日志里有没有收到 `POST /api/third/pay/{realmId}/stripe/webhooks` 请求。常见原因：

- Stripe Dashboard 里的 Webhook URL 配错了
- 服务器防火墙拦截了 Stripe 的回调请求
- SSL 证书有问题

### "Webhook secret not configured for realm" 报错

说明 Step 1 的 Webhook Secret 没配好。检查 realm_config 表里是否有 `config_type = 'stripe'`、`config_key = 'webhook_secret'`、`enabled = true` 的记录。

### 套餐在应用里看不到

检查三点：

1. 套餐的 **Active** 状态是否开启
2. 套餐是否已 **分配** 给对应的 Client App
3. 套餐的 Stripe 映射是否为 **Enabled** 状态

三项都满足，用户才能看到并购买。

### 测试卡号

Stripe 测试环境使用以下卡号：

- 成功支付：`4242 4242 4242 4242`
- 需要验证：`4000 0025 0000 3155`
- 被拒绝：`4000 0000 0000 0002`

过期日期填任何未来日期，CVC 填任何 3 位数字。

---

## 操作清单

配完后对照检查：

- [ ] Payment Providers 页面配置了 Stripe（Publishable Key、Secret Key、Webhook Secret 已填入并启用）
- [ ] Stripe Dashboard 创建了 Webhook 端点，8 个事件全部勾选
- [ ] Webhook Secret 和 Stripe 端点的 Signing secret 一致
- [ ] 创建了至少一个 Stripe Product，记下了 Product ID
- [ ] 为 Plan 和/或 Points Package 添加了 Stripe 支付映射（External Product ID 填了 Stripe 的 Product ID）
- [ ] 映射状态为 Enabled
- [ ] 把 Plan 分配给了 Client App
- [ ] 用测试 Key 和测试卡号跑通了一次支付流程
