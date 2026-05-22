# Creem 支付流程：从创建产品到收到付款

面向 Realm Admin 的端到端操作指南。跟着做完，你的用户就能通过 Creem 完成订阅支付。

## 给谁看

负责管理 Herald 计费配置的 Realm Admin。不需要写代码，全程在管理后台操作（Creem API Key 配置除外）。

## 前置条件

- Herald 管理后台可以正常访问，你有管理员账号
- 已创建至少一个 Realm 和一个 Client App
- 已在 [Creem](https://creem.io) 注册账号，拿到 API Key（测试环境用 `ck_test_` 开头的 key）

## 核心概念

先花两分钟搞清楚几个东西的关系，后面操作就顺畅了。

**Product（产品）**是你卖的东西的大类，比如"AI 写作助手"。**Plan（套餐）**是具体的计费方案，挂在产品下面，比如"月付 $9.99"和"年付 $99"。一个产品可以有多个套餐。

Plan 本身不绑定支付平台。你需要单独配置 **Plan Payment Provider Mapping（支付映射）**，告诉系统这个套餐在 Creem 里对应哪个商品。一个套餐可以同时映射到多个支付平台，但本教程只用 Creem。

最后，把套餐**分配**给 Client App，用户才能在你的应用里看到并购买它。

数据流向：Product → Plan → Mapping（映射到 Creem） → 分配给 App → 用户购买 → Creem 回调 → 订阅生效

---

## Step 1: 创建产品

1. 在左侧菜单找到 **Products & Payments** 分组，点击 **Products**
2. 点击 **Add Product** 按钮
3. 填写表单：
   - **Code**（必填）：产品的唯一标识，只能用小写字母、数字、横线。比如 `ai-writing`
   - **Title**（必填）：显示名称，比如 "AI Writing Assistant"
   - **Description**（可选）：产品描述
   - **Enabled**：默认开启，保持不动
4. 点击 **Create Product**
5. 看到 "Product 'AI Writing Assistant' created successfully" 提示，说明创建成功

创建成功后，Products 页面的表格里会出现你的产品。

## Step 2: 创建订阅套餐

1. 在左侧菜单 **Products & Payments** 分组下点击 **Subscription Plans**
2. 点击 **Add Subscription Plan** 按钮，进入创建页面
3. 填写表单：
   - **Product**（必填）：下拉选择刚创建的产品，比如 "AI Writing Assistant"
   - **Plan Name**（必填）：套餐标识符，创建后不可修改。比如 `basic-monthly`
   - **Title**（必填）：给用户看的名字，比如 "Basic Monthly"
   - **Description**（可选）：套餐描述，比如 "适合个人用户"
   - **Billing Period**（必填）：选 Monthly 或 Yearly
   - **Currency**（必填）：选 USD、EUR 或 GBP
   - **Price**（必填）：单位是分。填 `999` 表示 $9.99
   - **Checkout URL**（可选）：自定义支付页面地址，一般留空即可
   - **Trial Days**（可选）：免费试用天数，0 表示无试用期
   - **Sort Order**（可选）：排列顺序，数字越小越靠前
   - **Active**：默认开启，保持不动
4. 点击 **Create Subscription Plan**
5. 看到 "Subscription Plan 'Basic Monthly' created successfully" 提示

回到套餐列表，你会看到刚创建的套餐。注意 **Payment Providers** 列显示 "Not configured"，这是正常的，下一步来配。

## Step 3: 配置 Creem API Key

在 Herald 管理后台配置 Creem 的连接信息。

1. 在左侧菜单 **Products & Payments** 分组下点击 **Payment Providers**
2. 在未配置的支付渠道列表中找到 **Creem**，点击 **Configure**
3. 填写配置表单：
   - **Enable Creem**：打开开关
   - **API Key**（必填）：填入在 Creem 后台拿到的 API Key，测试环境用 `ck_test_` 开头的 key
   - **Timeout**（可选）：请求超时时间（秒）
   - **Webhook Secret**（可选）：Webhook 签名验证密钥
4. 点击 **Save**

配置完成后，系统创建 Checkout Session 时会自动读取这个 key 来调用 Creem API。

## Step 4: 为套餐添加 Creem 支付映射

这一步把套餐和 Creem 平台上的商品关联起来。

### 前提：在 Creem 后台创建商品

在 Herald 之外，你需要先到 [Creem Dashboard](https://creem.io) 创建一个 Product，拿到 **Product ID**（形如 `prod_xxxxxxxx`）。Creem 的商品价格等信息在 Creem 后台管理。

### 在 Herald 中配置映射

1. 回到 **Subscription Plans** 页面
2. 找到刚创建的套餐，点击右侧操作菜单里的 **Manage Providers**
3. 在弹出的对话框中，点击 **Add Provider**
4. 填写映射信息：
   - **Payment Provider**（必填）：下拉选择 **Creem**
   - **External Product ID**（必填）：填入在 Creem 后台拿到的 Product ID，比如 `prod_abc123`
   - **External Price ID**（可选）：Creem 的价格 ID，如果有的话填上
   - **Enabled**：默认开启
5. 点击 **Add Provider**

配置成功后，套餐列表的 **Payment Providers** 列会从 "Not configured" 变成显示 "Creem" 标签。

## Step 5: 分配套餐到 Client App

用户只能看到分配给当前 Client App 的套餐。如果没分配，用户看不到任何购买选项。

1. 在 **Subscription Plans** 页面，找到你的套餐
2. 点击右侧操作菜单里的 **Assign to App**
3. 在弹出的对话框中，勾选你要分配的 Client App
4. 点击确认

一个套餐可以分配给多个 Client App。反过来，一个 Client App 也可以有多个套餐。

## Step 6: 用户发起支付

配置到这里就完成了。接下来是用户侧的流程，你不需要操作 Herald 后台。

1. 用户在你的应用中选择套餐
2. 你的应用调用 Herald 的 Checkout API（传入 plan_id、payment_provider=creem、billing_period）
3. Herald 调用 Creem API 创建支付会话，返回一个支付 URL
4. 用户跳转到 Creem 的支付页面，完成付款
5. Creem 通过 Webhook 通知 Herald 支付结果
6. Herald 更新订阅状态、发放积分

用户支付的完整流程在你的应用端完成，Herald 负责后端的套餐查询、支付会话创建和 Webhook 处理。

## Step 7: 确认结果

用户完成支付后，你可以在 Herald 后台查看结果。

### 查看订阅状态

1. 在左侧菜单 **Transactions** 分组下点击 **Subscription History**
2. 查看订阅记录，确认状态为 Active

### 查看支付映射状态

1. 回到 **Subscription Plans** 页面
2. 点击套餐的 **Manage Providers**，确认映射状态为 Enabled

如果 Webhook 正常回调，订阅状态会在几秒内更新。如果迟迟没变化，看下面的常见问题。

---

## 常见问题

### Webhook 没收到

Creem 通过 Webhook 通知支付结果。确保 Herald 部署的公网地址可以从 Creem 服务器访问到。

Webhook 端点地址格式：`https://你的域名/api/third/pay/{realmId}/creem/webhooks`

你需要在 Creem Dashboard 的 Webhook 配置里填写这个地址。

### 套餐在应用里看不到

检查以下三点：

1. 套餐的 **Active** 状态是否开启
2. 套餐是否已**分配**给对应的 Client App
3. 套餐的 Creem 映射是否为 **Enabled** 状态

三项都满足，用户才能看到并购买。

### 支付成功但订阅未激活

大概率是 Webhook 没到达 Herald。检查 Herald 日志里有没有收到 `POST /api/third/pay/{realmId}/creem/webhooks` 请求。常见原因：

- Creem Dashboard 里的 Webhook URL 配错了
- Herald 服务器防火墙拦截了 Creem 的回调请求
- SSL 证书有问题

### "Creem not configured for realm" 报错

说明 Step 3 的 API Key 没配好。检查 realm_config 表里是否有 `config_type = 'creem'`、`config_key = 'api_key'`、`enabled = true` 的记录。

### 测试环境和生产环境的区别

Creem 的 API Key 前缀决定了请求发到哪里：

- `ck_test_` 开头：自动使用 Creem 测试环境 `test-api.creem.io`，不会产生真实扣款
- 其他前缀：使用 Creem 生产环境 `api.creem.io`，会真实扣款

开发阶段用测试 key，上线前换成生产 key。

---

## 操作清单

配完后对照检查：

- [ ] 创建了至少一个 Product
- [ ] 在 Product 下创建了 Plan（月付或年付）
- [ ] Payment Providers 页面配置了 Creem（API Key 已填入并启用）
- [ ] 为 Plan 添加了 Creem 支付映射（External Product ID 填了 Creem 的 Product ID）
- [ ] 映射状态为 Enabled
- [ ] 把 Plan 分配给了 Client App
- [ ] Creem Dashboard 配置了 Webhook URL
- [ ] 用测试 Key 跑通了一次支付流程
