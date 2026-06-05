# Realm Admin / Regular User / Herald 系统 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：配置发票策略 [US-IF-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：配置 Realm 的发票策略和各支付平台的发票能力开关
**从而**：控制发票来源，选择使用外部平台发票或 Herald 自研发票

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：设置发票策略为 provider_first**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 已配置 Stripe 支付平台
When 我在 Billing 设置中将发票策略设置为 "Provider First"
And 我启用 Stripe 的外部发票能力
Then 配置保存成功
And 后续通过 Stripe 支付的交易，发票由 Stripe 提供（只读展示）
And 其他支付平台的交易仍可使用 Herald 自研发票
```

**场景 2：设置发票策略为 manual_only**
```gherkin
Given 我是 realm-1 的管理员
When 我在 Billing 设置中将发票策略设置为 "Manual Only"
Then 所有非 MoR 交易使用 Herald 自研发票系统
And 管理员可以手动创建、编辑、开具发票
```

**场景 3：设置发票策略为 none**
```gherkin
Given 我是 realm-1 的管理员
When 我在 Billing 设置中将发票策略设置为 "None"
Then Herald 不提供自研发票入口
And 外部平台自带的发票仍可只读展示
```

**场景 4：Creem MoR 不可被 manual 覆盖**
```gherkin
Given 我是 realm-1 的管理员
And 发票策略为 "Manual Only"
And 存在一笔通过 Creem 支付的交易
When 我尝试为该 Creem 交易创建 Herald 手动发票
Then 系统拒绝操作
And 提示 "Creem transactions are managed by Creem as Merchant of Record"
```

**场景 5：未启用的 provider 不展示发票配置**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 未配置微信电子发票能力
When 我查看发票策略配置页面
Then 微信支付平台不展示外部发票能力开关
```

---

### 故事 2：系统同步 Stripe 发票 [US-IF-002]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：通过 Stripe webhook 自动同步 Stripe 发票数据到 Herald
**从而**：在 Herald 中只读展示 Stripe 管理的发票，无需人工干预

**【验收标准】**

**场景 1：Stripe invoice.created 事件同步**
```gherkin
Given Realm 配置了 Stripe 支付平台且启用了外部发票能力
And Stripe Invoicing 为某客户创建了发票
When Herald 收到 Stripe 的 "invoice.created" webhook 事件
Then Herald 在本地创建一条来源为 Stripe 的发票记录
And 发票状态为草稿
And 记录 Stripe 发票 ID 和原始数据快照
```

**场景 2：Stripe invoice.finalized 事件同步**
```gherkin
Given Herald 已同步一条 Stripe 发票（状态为草稿）
When Herald 收到 Stripe 的 "invoice.finalized" webhook 事件
Then 发票状态更新为已开具
And 更新外部托管页面 URL 和 PDF 下载 URL
```

**场景 3：Stripe invoice.voided 事件同步**
```gherkin
Given Herald 已同步一条 Stripe 发票
When Herald 收到 Stripe 的 "invoice.voided" webhook 事件
Then 发票状态更新为已作废
```

**场景 4：Stripe invoice.paid 事件同步**
```gherkin
Given Herald 已同步一条 Stripe 发票
When Herald 收到 Stripe 的 "invoice.paid" webhook 事件
Then 发票状态更新为已支付
```

**场景 5：重复 webhook 不产生重复发票**
```gherkin
Given Herald 已同步 Stripe 发票 in_abc123
When Herald 再次收到同一 Stripe 发票的事件
Then Herald 更新已有记录而非创建重复发票
```

---

### 故事 3：系统同步 Creem 交易税务数据 [US-IF-003]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：同步 Creem MoR 交易的税务数据到 Herald
**从而**：在 Herald 中只读展示 Creem 管理的发票和税务信息

**【验收标准】**

**场景 1：Creem 支付成功后同步交易税务数据**
```gherkin
Given Realm 配置了 Creem 支付平台
And 一笔 Creem 支付成功
When 系统处理 Creem 交易回调
Then Herald 创建一条来源为 Creem 的发票记录
And 记录交易金额、税额、税区等税务信息
And 记录 Creem 交易 ID 作为外部发票标识
```

**场景 2：Creem 发票不可被 Herald 修改**
```gherkin
Given Herald 存在一条来源为 Creem 的发票
When 任何用户尝试通过 Herald 编辑、开具、作废或标记该发票已付
Then 系统拒绝操作
And 提示 "This invoice is managed by the payment provider"
```

**场景 3：Creem 交易不允许创建 Herald 手动发票**
```gherkin
Given 存在一笔通过 Creem 完成的支付
When 用户尝试为该交易申请 Herald 手动发票
Then 系统拒绝操作
And 提示该交易的发票由 Creem 管理
```

---

### 故事 4：查看外部 Provider 发票（管理员） [US-IF-004]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：在发票列表中查看外部 provider 同步的发票
**从而**：了解所有发票全貌，包括自研和外部 provider 来源

**【验收标准】**

**场景 1：发票列表显示 provider 来源**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 既有 Herald 自研发票，也有 Stripe 同步的发票
When 我查看发票列表
Then 列表显示每条发票的来源标识（Manual / Stripe / Creem）
And 列表同时展示自研和外部 provider 的发票
```

**场景 2：外部发票详情为只读**
```gherkin
Given 我在查看一条来自 Stripe 的发票详情
Then 我可以看到发票的完整信息（编号、金额、行项目、税务明细）
And 我无法看到 "Edit"、"Issue"、"Void"、"Mark as Paid" 按钮
And 详情页显示 "This invoice is managed by Stripe"
And 如有外部托管 URL，显示 "View in Stripe" 链接
```

**场景 3：自研发票功能不受影响**
```gherkin
Given 我在查看一条 Herald 自研发票详情
Then 所有操作按钮按现有逻辑启用或禁用
And 自研发票的全部功能保持不变
```

**场景 4：按 provider 筛选发票**
```gherkin
Given 我在发票列表页面
And 列表中包含来自不同 provider 的发票
When 我选择 provider 筛选为 "Stripe"
Then 列表只显示 Stripe 同步的发票
```

---

### 故事 5：查看外部 Provider 发票（普通用户） [US-IF-005]

**优先级**: P1

**【用户故事】**
**作为**：Regular User
**我希望**：在"我的发票"中查看外部 provider 同步的发票
**从而**：了解我的所有发票，无论来自哪个平台

**【验收标准】**

**场景 1：我的发票列表显示外部 provider 发票**
```gherkin
Given 我是普通用户 user@example.com
And 我有通过 Stripe 支付的交易
And Stripe 已生成发票并同步到 Herald
When 我查看"我的发票"页面
Then 列表中包含 Stripe 同步的发票
And 发票带有 Stripe 来源标识
```

**场景 2：外部发票详情为只读**
```gherkin
Given 我在查看一条 Stripe 同步的发票
Then 我可以看到发票详情（编号、金额、状态）
And 我无法申请开具或编辑该发票
And 如有外部托管 URL，显示 "View in Stripe" 链接
```

**场景 3：用户为 Creem 交易申请发票被拒绝**
```gherkin
Given 我有一笔通过 Creem 支付的交易
When 我尝试为该交易申请 Herald 发票
Then 系统提示该交易的发票由 Creem 管理
And 申请入口不可用
```

---

### 故事 6：下载外部发票 PDF 或查看 Provider 页面 [US-IF-006]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin / Regular User
**我希望**：下载或查看外部 provider 管理的发票 PDF
**从而**：获得正式的发票文档

**【验收标准】**

**场景 1：Stripe 发票有 PDF URL 时直接下载**
```gherkin
Given 存在一条来自 Stripe 的发票
And 该发票有外部 PDF 下载 URL
When 我点击下载 PDF
Then 系统将我重定向到 Stripe 的 PDF 下载链接
```

**场景 2：外部发票有托管页面 URL**
```gherkin
Given 存在一条来自 Stripe 的发票
And 该发票有外部托管页面 URL
When 我点击 "View in Stripe" 链接
Then 系统在新标签页打开 Stripe 托管的发票页面
```

**场景 3：Creem 发票无 PDF URL 时跳转 Creem 平台**
```gherkin
Given 存在一条来自 Creem 的发票
And 该发票无外部 PDF 下载 URL
When 我查看发票详情
Then 详情页提示 "Invoice managed by Creem"
And 如有 Creem 平台链接则显示 "View in Creem"
```

**场景 4：自研发票 PDF 功能不受影响**
```gherkin
Given 存在一条 Herald 自研发票
When 我下载该发票的 PDF
Then 系统使用 Herald 内置 PDF 生成器生成并下载
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|-------------|---------|
| P0 | 4 | 配置发票策略、同步 Stripe 发票、同步 Creem 税务、管理员查看外部发票 |
| P1 | 2 | 用户查看外部发票、下载外部 PDF |
| P2 | 0 | - |

---

## 相关文档

- **PRD**: `docs/prd/billing/invoice-fallback.md` - Invoice Fallback PRD
- **PRD**: `docs/prd/billing/invoice.md` - Invoice 自研发票 PRD
- **技术预研**: `.ai/tech-research/invoice_fallback.md`
- **需求文档**: `.ai/future/invoice_fallback.md`
