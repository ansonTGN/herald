# Realm Admin 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：配置支付平台 [US-PV-001]

**优先级**: P0

> **待实现**：当前后端仅支持读取 Creem/Stripe 配置，创建/编辑/删除功能尚未实现。

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：配置支付平台（Creem/Stripe）
**从而**：为用户提供多种支付选项，提高支付成功率和用户体验

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：创建 Creem 配置（测试环境）**
```gherkin
Given 我是 realm-1 的管理员
When 我在支付平台管理页面点击 "Add Provider" 按钮
And 我选择平台类型为 "Creem"
And 我填写配置信息：
  | Environment  | sandbox  |
  | API Public  | pk_test_creem_123  |
  | API Secret   | sk_test_creem_456  |
And 我提交表单
Then 支付平台配置创建成功
And 系统显示成功消息："Payment provider 'Creem' configured successfully"
And 配置列表显示新创建的 Creem 配置
And API Secret 显示为脱敏格式
```

**场景 2：创建 Stripe 配置（测试环境）**
```gherkin
Given 我是 realm-1 的管理员
When 我在支付平台管理页面点击 "Add Provider" 按钮
And 我选择平台类型为 "Stripe"
And 我填写配置信息：
  | Environment       | test                  |
  | Account ID        | acct_1234567890       |
  | API Public Key   | pk_test_51M...         |
  | API Secret Key   | sk_test_51M...         |
  | Webhook Secret   | whsec_...             |
  | Webhook Endpoint | https://example.com/api/billing/realm-1/stripe/webhook |
And 我提交表单
Then 支付平台配置创建成功
And 系统显示成功消息："Payment provider 'Stripe' configured successfully"
And Webhook 端点已注册到 Stripe Dashboard
```

**场景 3：创建 Stripe 配置（生产环境）**
```gherkin
Given 我是 realm-1 的管理员
And 我已创建测试环境的 Stripe 配置
When 我再次选择平台类型为 "Stripe"
And 我选择 Environment 为 "production"
And 我填写生产环境的 API 密钥（以 pk_live_、sk_live_ 开头）
And 我提交表单
Then 系统显示确认对话框："You are configuring production mode. This will process real payments."
And 我确认后配置创建成功
```

**场景 4：API Key 格式验证**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试创建 Stripe 配置
And 我输入错误的 API Key 格式（不是以 pk_、sk_ 开头）
Then 系统显示验证错误："Invalid API Key format"
And 配置创建失败
```

**场景 5：Webhook URL 验证**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试创建 Stripe 配置
And 我输入无效的 Webhook URL（不是 http/https）
Then 系统显示验证错误："Webhook URL must be a valid HTTP/HTTPS URL"
And 配置创建失败
```

**场景 6：敏感信息安全存储**
```gherkin
Given 我是 realm-1 的管理员
When 我创建支付平台配置
And 我提交包含 API Secret 的表单
Then 查看配置时，Secret 显示为脱敏格式（如 "sk_test_*******************"）
```

**场景 7：同一平台只能有一个配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Stripe 配置
When 我尝试创建另一个 Stripe 配置
Then 系统显示错误："A Stripe configuration already exists. Please edit the existing configuration."
And 配置创建失败
```

---

### 故事 2：查看支付平台配置 [US-PV-002]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看支付平台配置和状态
**从而**：了解当前支付平台的配置情况和运行状态

**【验收标准】**

**场景 1：查看所有支付平台配置**
```gherkin
Given 我是 realm-1 的管理员
And 已配置多个支付平台：
  | Platform | Environment |
  | Creem    | sandbox     |
  | Stripe   | test        |
When 我访问支付平台管理页面
Then 我看到支付平台配置列表
And 列表包含以下列：
  | 列名               | 说明                   |
  | Platform           | 支付平台名称           |
  | Environment        | 环境（sandbox/production） |
  | API Public Key    | API 公钥               |
  | API Secret Key    | API 密钥（脱敏）        |
  | Webhook Endpoint  | Webhook 端点（如适用）  |
  | Last Updated      | 最后更新时间            |
  | Actions          | 操作（编辑、删除）        |
And API Secret Key 显示脱敏格式（如 "sk_test_*******************"）
```

**场景 2：按平台类型筛选**
```gherkin
Given 我在支付平台管理页面
When 我选择平台类型筛选为 "Stripe"
Then 列表只显示 Stripe 配置
When 我选择平台类型筛选为 "Creem"
Then 列表只显示 Creem 配置
```

**场景 3：查看单个配置详情**
```gherkin
Given 我在支付平台配置列表
When 我点击某个配置的 "View" 按钮
Then 我看到配置详情页面
And 页面显示：
  | 字段               | 内容                       |
  | Platform           | Stripe                    |
  | Environment        | test                      |
  | Account ID        | acct_1234567890           |
  | API Public Key    | pk_test_51M...             |
  | API Secret Key    | sk_test_******************* |
  | Webhook Secret    | whsec_*******************  |
  | Webhook Endpoint  | https://example.com/...    |
  | Created At        | 2026-03-20 10:00:00 UTC  |
  | Updated At        | 2026-03-20 10:00:00 UTC  |
```

**场景 4：测试 Webhook 连接（仅 Shopify 支持）**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 Shopify 平台
When 我在配置详情页面点击 "Test Webhook" 按钮
Then 系统向 Shopify 发送测试 Webhook 事件
And 系统显示测试结果：
  | 测试项           | 状态   |
  | Signature 验证   | 成功    |
  | 事件处理         | 成功    |
When Webhook 测试失败
Then 系统显示错误详情和修复建议
```

> **注意**：当前仅 Shopify 支持测试连接功能，Stripe 和 Creem 的测试连接功能待实现。

---

### 故事 3：编辑支付平台配置 [US-PV-003]

**优先级**: P1

> **待实现**：当前后端仅支持读取 Creem/Stripe 配置，编辑功能尚未实现。

**【用户故事】**
**作为**：Realm Admin
**我希望**：编辑支付平台配置
**从而**：应对密钥轮换和配置变更

**【验收标准】**

**场景 1：更新 API 密钥**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Stripe 配置
When 我点击 "Edit" 按钮
And 我更新 API Secret Key（密钥轮换）
And 我保存更改
Then 配置更新成功
And 新的 Secret Key 安全存储
And 旧 Secret Key 被替换
And 系统显示成功消息："Configuration updated successfully"
```

**场景 1a：保留已有密钥**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Stripe 配置
When 我点击 "Edit" 按钮
And 我不修改 API Secret Key（留空）
And 我修改其他非敏感字段（如 API Version）
And 我保存更改
Then 配置更新成功
And 原有的 Secret Key 保持不变
And 非敏感字段更新为新值
```

**场景 2：更新 Webhook Endpoint**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 Stripe Webhook
When 我更新 Webhook Endpoint URL
And 我保存更改
Then 系统提示更新 Stripe Dashboard 的 Webhook 配置
And 我看到确认对话框："Please update your Stripe Dashboard webhook endpoint to the new URL"
```

**场景 3：切换环境（test -> production）**
```gherkin
Given 我是 realm-1 的管理员
And 当前 Stripe 配置为 test 环境
When 我将 Environment 更改为 "production"
And 我更新 API 密钥为生产密钥（以 pk_live_、sk_live_ 开头）
And 我保存更改
Then 系统显示警告对话框："Switching to production mode will process real payments"
When 我确认
Then 配置更新为生产环境
And 所有支付将使用 Stripe 生产环境
```

**场景 4：不允许修改平台类型**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Stripe 配置
When 我点击 "Edit" 按钮
Then "Platform" 字段为只读或禁用
And 我无法将 Stripe 改为其他平台类型
```

**场景 5：更新时验证配置**
```gherkin
Given 我是 realm-1 的管理员
When 我编辑配置
And 我输入无效的 API Key 格式
And 我保存更改
Then 系统显示验证错误
And 配置不更新
And 原有配置保持不变
```

---

### 故事 4：删除支付平台配置 [US-PV-004]

**优先级**: P1

> **待实现**：当前后端仅支持读取 Creem/Stripe 配置，删除功能尚未实现。

**【用户故事】**
**作为**：Realm Admin
**我希望**：删除支付平台配置
**从而**：保持配置列表的整洁

**【验收标准】**

**场景 1：无法删除有活跃订阅的配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Stripe 配置且有 10 个活跃订阅
When 我尝试删除该配置
Then 系统显示错误消息："Cannot delete payment provider with active subscriptions"
And 显示活跃订阅数量："10 active subscriptions"
And 配置删除失败
```

**场景 2：删除无订阅的配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Creem 配置
And 该配置的所有订阅都已取消
When 我删除该配置
Then 配置删除成功
```

**场景 3：删除前二次确认**
```gherkin
Given 我是 realm-1 的管理员
When 我点击 "Delete" 按钮
Then 系统显示确认对话框：
  | 标题   | 确认删除支付平台配置？ |
  | 消息   | 删除后将无法恢复此配置 |
  | 按钮   | 取消 / 删除 |
When 我点击 "取消"
Then 配置保持不变
When 我点击 "删除"
Then 配置被删除
```

---

### 故事 5：查看支付平台使用统计 [US-PV-005]

**优先级**: P2

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看各支付平台的使用统计
**从而**：优化支付平台配置和策略

**【验收标准】**

**场景 1：查看平台统计概览**
```gherkin
Given 我是 realm-1 的管理员
When 我访问支付平台管理页面
Then 我看到每个平台的统计信息：
  | 列名           | 说明                   |
  | Platform       | 支付平台名称           |
  | Total Payments | 总支付次数             |
  | Success Rate   | 支付成功率             |
  | Total Revenue  | 总收入                 |
  | Active Subs    | 活跃订阅数             |
And Stripe 显示：
  | Total Payments | 1,234                |
  | Success Rate   | 92.5%                |
  | Total Revenue  | $45,678.90           |
  | Active Subs    | 150                  |
```

**场景 2：按时间范围筛选统计**
```gherkin
Given 我在支付平台管理页面
When 我选择时间范围为 "Last 7 days"
Then 统计数据更新为最近 7 天的数据
When 我选择时间范围为 "Last 30 days"
Then 统计数据更新为最近 30 天的数据
```

**场景 3：比较不同平台表现**
```gherkin
Given 我是 realm-1 的管理员
When 我查看多个平台的统计数据
Then 我可以看到各平台的对比：
  | Platform | Success Rate | Avg Payment Time |
  | Stripe   | 92.5%        | 2.3s            |
  | Creem    | 99.9%        | 0.1s            |
And 我可以根据数据决定优先使用的平台
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 2 | US-PV-001: 配置支付平台, US-PV-002: 查看支付平台配置 |
| P1 | 2 | US-PV-003: 编辑配置, US-PV-004: 删除配置 |
| P2 | 1 | US-PV-005: 查看平台使用统计 |

---

## 相关文档

- **PRD**: `docs/prd/billing/subscription.md` - Billing 订阅计费产品需求文档
- **PRD**: `docs/prd/billing/stripe-payment.md` - Stripe 支付集成产品需求文档
