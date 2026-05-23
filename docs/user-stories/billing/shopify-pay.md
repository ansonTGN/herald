# Realm Admin / Herald 系统 / Regular User 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：配置 Shopify 支付平台 [US-PP-007]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：配置 Shopify 作为支付平台
**从而**：为使用 Shopify 店铺的商户提供 Herald 订阅管理能力，支持周期性计费和订阅生命周期管理

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：创建 Shopify 配置（开发环境）**
```gherkin
Given 我是 realm-1 的管理员
When 我在支付平台管理页面点击 "Add Provider" 按钮
And 我选择平台类型为 "Shopify"
And 我填写配置信息：
  | Shop Domain            | demo-store.myshopify.com        |
  | Admin Access Token     | shpat_xxxxxxxxxxxxxxxxxxxxxxxxxxxx |
  | Storefront Access Token| shp_xxxxxxxxxxxxxxxxxxxxxxxxxxxx |
  | App Client Secret      | xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx |
  | API Version            | 2024-01                          |
And 我提交表单
Then 支付平台配置创建成功
And 系统显示成功消息："Payment provider 'Shopify' configured successfully"
And 配置列表显示新创建的 Shopify 配置
And 所有敏感凭据显示为脱敏格式
```

**场景 2：Shop Domain 格式验证**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试创建 Shopify 配置
And 我输入错误的 Shop Domain 格式（不是 .myshopify.com 结尾）
Then 系统显示验证错误："Shop Domain must end with .myshopify.com"
And 配置创建失败
```

**场景 3：API Token 格式验证**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试创建 Shopify 配置
And 我输入无效的 Admin Access Token（不是以 shpat_ 开头）
Then 系统显示验证错误："Invalid Admin Access Token format"
And 配置创建失败
```

**场景 4：同一 Realm 只能有一个 Shopify 配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Shopify 配置
When 我尝试创建另一个 Shopify 配置
Then 系统显示错误："A Shopify configuration already exists. Please edit the existing configuration."
And 配置创建失败
```

**场景 5：敏感信息安全存储**
```gherkin
Given 我是 realm-1 的管理员
When 我创建 Shopify 配置
And 我提交包含 API Tokens 的表单
Then 查看配置时，Tokens 显示为脱敏格式（如 "shpat_*******************"）
```

**场景 6：自动生成 Webhook Endpoint**
```gherkin
Given 我是 realm-1 的管理员
When 我创建 Shopify 配置成功
Then 系统自动生成 Webhook Endpoint URL
And URL 格式符合 Shopify webhook 订阅要求
And 配置详情页面显示该 Webhook Endpoint
```

---

### 故事 2：查看 Shopify 支付平台配置 [US-PP-008]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看 Shopify 支付平台配置和状态
**从而**：了解当前 Shopify 配置情况和运行状态

**【验收标准】**

**场景 1：查看 Shopify 配置列表**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 Shopify 支付平台
When 我访问支付平台管理页面
Then 我看到支付平台配置列表包含 Shopify
And Shopify 配置显示以下信息：
  | 列名                  | 说明                           |
  | Platform              | Shopify                        |
  | Shop Domain          | demo-store.myshopify.com       |
  | API Version          | 2024-01                        |
  | Webhook Endpoint     | 系统生成的 webhook 接收地址     |
  | Last Updated         | 最后更新时间                    |
  | Actions              | 操作（编辑、删除、测试连接）      |
And 敏感凭据显示脱敏格式
```

**场景 2：查看 Shopify 配置详情**
```gherkin
Given 我在支付平台配置列表
When 我点击 Shopify 配置的 "View" 按钮
Then 我看到配置详情页面
And 页面显示：
  | 字段                    | 内容                           |
  | Platform                | Shopify                        |
  | Shop Domain             | demo-store.myshopify.com       |
  | Admin Access Token      | shpat_*******************      |
  | Storefront Access Token | shp_*******************        |
  | App Client Secret       | ****************************   |
  | API Version             | 2024-01                        |
  | Webhook Endpoint        | 系统生成的 webhook 接收地址    |
  | Created At              | 2026-04-01 10:00:00 UTC       |
  | Updated At              | 2026-04-01 10:00:00 UTC       |
```

**场景 3：测试 Shopify 连接**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 Shopify 平台
When 我在配置详情页面点击 "Test Connection" 按钮
Then 系统向 Shopify 发送测试请求
And 系统显示测试结果：
  | 测试项           | 状态   |
  | Admin API 连接   | 成功    |
  | Storefront API   | 成功    |
  | Shop 访问权限    | 成功    |
When 连接测试失败
Then 系统显示错误详情和修复建议
```

---

### 故事 3：编辑 Shopify 支付平台配置 [US-PP-009]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：编辑 Shopify 支付平台配置
**从而**：应对凭据轮换和配置变更

**【验收标准】**

**场景 1：更新 API 凭据**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Shopify 配置
When 我点击 "Edit" 按钮
And 我更新 Admin Access Token（凭据轮换）
And 我保存更改
Then 配置更新成功
And 新的 Access Token 安全存储
And 旧 Access Token 被替换
And 系统显示成功消息："Shopify configuration updated successfully"
```

**场景 2：更新 Shop Domain**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 Shopify 平台
When 我更新 Shop Domain
And 我保存更改
Then 系统提示确认对话框："Changing Shop Domain will affect webhook subscriptions. Please ensure webhooks are re-registered in Shopify Admin."
When 我确认
Then 配置更新成功
```

**场景 3：更新 API Version**
```gherkin
Given 我是 realm-1 的管理员
And 当前 Shopify API 版本为 2024-01
When 我将 API Version 更改为 "2024-04"
And 我保存更改
Then 配置更新成功
And 所有后续 Shopify 操作使用新版本
```

**场景 4：不允许修改平台类型**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Shopify 配置
When 我点击 "Edit" 按钮
Then "Platform" 字段为只读或禁用
And 我无法将 Shopify 改为其他平台类型
```

**场景 5：更新时验证配置**
```gherkin
Given 我是 realm-1 的管理员
When 我编辑 Shopify 配置
And 我输入无效的 Shop Domain
And 我保存更改
Then 系统显示验证错误
And 配置不更新
And 原有配置保持不变
```

---

### 故事 4：删除 Shopify 支付平台配置 [US-PP-010]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：删除 Shopify 支付平台配置
**从而**：保持配置列表的整洁

**【验收标准】**

**场景 1：删除无订阅的 Shopify 配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Shopify 配置且无活跃订阅
When 我在支付平台列表中点击 "Delete" 按钮
And 我确认删除
Then 支付平台配置删除成功
And 系统显示成功消息："Payment provider 'Shopify' deleted"
And 支付平台列表不再显示该配置
```

**场景 2：无法删除有活跃订阅的配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Shopify 配置且有 5 个活跃订阅
When 我尝试删除该配置
Then 系统显示错误消息："Cannot delete payment provider with active subscriptions"
And 显示活跃订阅数量："5 active subscriptions"
And 配置删除失败
```

**场景 3：删除前二次确认**
```gherkin
Given 我是 realm-1 的管理员
When 我点击 Shopify 配置的 "Delete" 按钮
Then 系统显示确认对话框：
  | 标题   | 确认删除 Shopify 支付平台配置？ |
  | 消息   | 删除后将无法恢复此配置，且 Shopify webhook 将不再工作 |
  | 按钮   | 取消 / 删除 |
When 我点击 "取消"
Then 配置保持不变
When 我点击 "删除"
Then 配置被删除
```

**场景 4：删除配置后处理未完成的 webhook**
```gherkin
Given 我是 realm-1 的管理员
And 已删除 Shopify 配置
When Shopify webhook 事件到达
Then 系统忽略该事件并记录警告日志
```

---

### 故事 5：Shopify 订阅合同创建与同步 [US-PP-011]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：接收并处理 Shopify 的订阅合同创建事件
**从而**：实现 Shopify 订阅与 Herald 订阅的双向同步

**【验收标准】**

**场景 1：接收订阅合同创建事件**
```gherkin
Given realm-1 已配置 Shopify 支付平台
And 用户在 Shopify 完成订阅购买
When Herald 接收到订阅合同创建事件
And 事件签名验证通过
Then Herald 从事件中提取合同信息
And Herald 验证事件包含 Herald 关联标识
And Herald 创建本地订阅记录
And Herald 保存事件记录
```

**场景 2：首次订阅成功发放积分**
```gherkin
Given Herald 接收到订阅合同创建事件
And 事件包含完整的 Herald 关联标识
When Herald 成功创建订阅记录
And 订阅状态为 Active
Then 系统发放订阅套餐对应积分
And 用户获得订阅套餐对应积分
And 系统记录订阅变更历史
```

**场景 3：处理缺少 Herald 标识的事件（创建未归属订阅）**
```gherkin
Given Herald 接收到订阅合同创建事件
And 事件不包含 Herald 用户标识
And 不存在对应的用户绑定记录
When Herald 处理该事件
Then Herald 创建订阅记录（订阅未归属到任何用户）
And Herald 不发放初始订阅积分
And Herald 记录日志："Subscription created but unclaimed, awaiting user claim"
```

**场景 4：事件幂等处理**
```gherkin
Given Herald 已处理过某订阅合同创建事件
When Herald 接收到相同事件的重复通知
Then Herald 检测到重复事件
And Herald 跳过处理
And 不重复创建订阅记录
And 不重复发放积分
```

**场景 5：事件签名验证失败**
```gherkin
Given Herald 接收到订阅合同创建事件
And 事件的签名验证失败
When Herald 尝试验证签名
Then Herald 不处理该事件
And Herald 记录安全警告日志
```

**场景 6：通过用户绑定自动归属**
```gherkin
Given Herald 接收到订阅合同创建事件
And 事件不包含 Herald 用户标识
And 事件包含 Shopify 客户 ID
And 已存在该客户 ID 与 Herald 用户的绑定记录
When Herald 处理该事件
Then Herald 通过绑定记录找到对应用户
And Herald 创建订阅记录（归属到该用户）
Then 系统发放初始订阅积分
```

---

### 故事 6：Shopify 订阅续费与状态同步 [US-PP-012]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：接收并处理 Shopify 的订阅续费和状态变更事件
**从而**：确保 Herald 与 Shopify 的订阅状态保持一致

**【验收标准】**

**场景 1：处理续费成功事件**
```gherkin
Given realm-1 已有 Shopify 订阅
When Herald 接收到续费成功事件
And 签名验证通过
Then Herald 找到对应的本地订阅记录
When 订阅已归属到用户
  And 系统发放续费积分
  And 用户获得续费积分
When 订阅未归属到用户
  And Herald 更新订阅周期边界
  And Herald 不发放积分
  And Herald 记录："Renewal success but subscription unclaimed"
And 系统记录订阅变更历史
```

**场景 2：处理续费失败事件**
```gherkin
Given realm-1 已有 Shopify 订阅
And Shopify 订阅续费失败
When Herald 接收到续费失败事件
And 签名验证通过
Then Herald 找到对应的本地订阅记录
And Herald 将订阅状态更新为 "PastDue"
And Herald 保存失败原因
And 系统记录订阅变更历史
And 不发放积分
```

**场景 3：处理订阅合同更新事件**
```gherkin
Given realm-1 已有 Shopify 订阅
And Shopify 订阅合同发生变更（升级/降级/取消）
When Herald 接收到订阅合同更新事件
And 签名验证通过
And 事件版本高于本地存储的版本
Then Herald 更新订阅状态和计划信息
When 变更为计划升级
Then 系统处理升级逻辑
And 用户获得升级差额积分
When 变更为计划降级
Then Herald 更新订阅记录
And Herald 不回收已发放的积分
```

**场景 4：处理订阅取消事件**
```gherkin
Given realm-1 已有 Shopify 订阅
And Shopify 订阅标记为周期末取消
When Herald 接收到订阅合同更新事件
And 事件内容显示合同已取消
Then Herald 将订阅状态更新为 "ScheduledCancel" 或 "Canceled"
And 系统处理订阅取消逻辑
And 系统记录订阅变更历史
```

**场景 5：处理退款事件**
```gherkin
Given realm-1 已有 Shopify 订阅
When Herald 接收到退款事件
And 签名验证通过
Then Herald 记录退款事件
And Herald 关联到对应的订阅
When 订阅已归属到用户
  And Herald 执行退款积分回收
  And Herald 扣除对应积分
When 订阅未归属到用户
  And Herald 仅记录事件
  And Herald 不执行积分回收
And 系统记录订阅变更历史
```

**场景 6：处理 App 卸载事件**
```gherkin
Given realm-1 已配置 Shopify 支付平台
And 商户卸载 Shopify App
When Herald 接收到 App 卸载事件
Then Herald 将 Shopify 配置标记为禁用
And Herald 记录运维级告警
And Herald 阻止后续 Shopify 结账创建
And Herald 不删除历史订阅和事件数据
```

**场景 7：低版本事件不覆盖高版本**
```gherkin
Given Herald 本地存储的合同版本为 5
When Herald 接收到版本为 3 的订阅合同更新事件
Then Herald 检测到版本低于本地版本
And Herald 忽略该事件
And Herald 记录日志："Ignoring outdated contract update event"
And 不更新本地订阅状态
```

**场景 8：乱序事件补偿处理**
```gherkin
Given Herald 接收到订单支付成功事件
And 事件包含足够 Herald 标识
When 尚未收到订阅合同创建事件
Then Herald 创建最小化订阅记录
And Herald 记录事件
When 随后订阅合同创建事件到达
Then Herald 合并补充完整订阅信息
```

---

### 故事 7：用户认领 Shopify 订阅 [US-PP-013]

**优先级**: P0

**【用户故事】**
**作为**：Regular User
**我希望**：认领我在 Shopify 购买的订阅
**从而**：实现"先在 Shopify 购买、后登录 Herald 认领"的灵活流程，在 Herald 中看到我的订阅并获得积分

**【验收标准】**

**场景 1：用户登录后自动认领**
```gherkin
Given 我在 Shopify 直接购买了订阅（购买时未登录 Herald）
And Shopify 事件已创建未归属订阅
When 我首次登录 Herald
Then 系统检测到我有未认领的 Shopify 订阅
And 系统显示提示："发现 1 个 Shopify 订阅待认领"
When 我点击"认领订阅"按钮
Then 系统将订阅归属到我的用户账户
And 系统补发当前有效周期的订阅积分
And 系统显示成功消息："订阅认领成功"
```

**场景 2：手动认领订阅**
```gherkin
Given 我已登录 Herald
And 我有未认领的 Shopify 订阅
When 我访问"我的订阅"页面
And 我点击"同步 Shopify 订阅"按钮
Then 系统将未归属订阅归属到我的账户
And 系统补发当前有效周期积分
```

**场景 3：认领时提供 Contract ID**
```gherkin
Given 我已登录 Herald
And 系统无法自动识别我的 Shopify 客户身份
When 我在"认领订阅"表单中输入 Contract ID
And 我提交认领请求
Then 系统通过 Contract ID 找到订阅
And 系统完成认领并补偿积分
```

**场景 4：防止重复认领**
```gherkin
Given 我已认领了某个 Shopify 订阅
When 我再次尝试认领该订阅
Then 系统检测到订阅已归属到我的用户账户
And 系统返回幂等成功
And 系统不重复发放积分
```

**场景 5：客户已被他人认领**
```gherkin
Given Shopify 客户 "customer_123" 已被用户 A 认领
When 用户 B 尝试认领同一客户的订阅
Then 系统检测到绑定冲突
And 系统返回错误："该 Shopify 账户已被其他用户认领"
And 认领失败
```

**场景 6：认领后补偿积分**
```gherkin
Given 我认领了一个 Shopify 订阅
And 订阅当前状态为 Active
And 当前周期结束时间为 2026-05-01
And 系统尚未发放本周期积分
When 认领成功
Then 系统补发当前有效周期的一次订阅积分
And 系统确保不会重复发放积分
```

---

### 故事 8：Webhook 处理未归属订阅 [US-PP-014]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：正确处理未归属订阅的事件
**从而**：确保"先购买、后认领"流程的准确性，避免积分发放错误

**【验收标准】**

**场景 1：创建未归属订阅**
```gherkin
Given Herald 接收订阅合同创建事件
And 事件不包含 Herald 用户标识
And 不存在对应的用户绑定记录
When Herald 处理该事件
Then Herald 创建订阅记录（订阅未归属到任何用户）
And Herald 不发放初始订阅积分
And Herald 记录日志："Subscription created but unclaimed"
```

**场景 2：未归属订阅收到续费成功事件**
```gherkin
Given 存在未归属订阅（订阅未归属到任何用户）
When Herald 接收续费成功事件
Then Herald 通过绑定记录找到订阅
And Herald 检测到订阅未归属到用户
And Herald 更新订阅周期边界
And Herald 不发放续费积分
And Herald 记录："Billing success but subscription unclaimed, points deferred"
```

**场景 3：未归属订阅收到退款事件**
```gherkin
Given 存在未归属订阅
When Herald 接收退款事件
Then Herald 记录退款事件
And Herald 不执行积分回收（因为未归属时未发放积分）
And Herald 记录日志："Refund recorded for unclaimed subscription, no points to revoke"
```

**场景 4：未归属订阅收到升级事件**
```gherkin
Given 存在未归属订阅（订阅未归属到任何用户）
When Herald 接收订阅合同更新事件（计划变更）
Then Herald 更新订阅计划
And Herald 不处理升级积分发放
And Herald 不发放差额积分
And Herald 记录日志："Plan updated for unclaimed subscription"
```

---

### 故事 9：通过用户绑定自动归属 [US-PP-015]

**优先级**: P1

**【用户故事】**
**作为**：Herald 系统
**我希望**：当事件不带 Herald 用户标识但存在客户绑定时，自动归属订阅
**从而**：减少用户手动认领的操作次数

**【验收标准】**

**场景 1：事件无用户标识但有绑定记录**
```gherkin
Given 已存在客户绑定记录（Shopify 客户 → Herald 用户 A）
When Herald 接收订阅合同创建事件
And 事件包含该 Shopify 客户 ID
And 事件不包含 Herald 用户标识
Then Herald 通过绑定记录找到用户 A
And Herald 创建订阅记录（归属到用户 A）
And Herald 发放初始订阅积分
```

**场景 2：事件带用户标识时更新绑定**
```gherkin
Given 客户绑定记录不存在
When Herald 接收订阅合同创建事件
And 事件包含 Herald 用户标识和 Shopify 客户 ID
Then Herald 创建订阅记录（归属到该用户）
And Herald 创建或更新客户绑定记录
And Herald 发放初始订阅积分
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 6 | 配置 Shopify 平台、查看 Shopify 配置、订阅合同创建与同步、订阅续费与状态同步、用户认领订阅、Webhook 处理未归属订阅 |
| P1 | 3 | 编辑 Shopify 配置、删除 Shopify 配置、通过用户绑定自动归属 |
| P2 | 0 | - |

---

## 相关文档

- **PRD**: `docs/prd/billing/subscription.md` - Billing 订阅计费产品需求文档
- **用户故事**: `docs/user-stories/billing/payment-provider.md` - 其他支付平台用户故事
