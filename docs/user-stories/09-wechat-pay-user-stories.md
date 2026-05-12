# 微信支付用户故事

**角色代码**: WP
**角色定义**：Realm Admin 负责管理微信支付配置；Regular User 通过微信支付完成订阅购买；System 负责处理 Webhook 回调和状态同步。

**故事范围**: US-WP-001 ~ US-WP-008
**创建时间**: 2026-04-04
**状态**: Active

---

## 故事 1：配置微信支付平台 [US-WP-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：配置微信支付作为支付平台
**从而**：用户可以使用微信支付完成订阅购买

**【验收标准】**

**场景 1：创建微信支付配置**
```gherkin
Given 我是 realm-1 的管理员
When 我在支付平台管理页面点击 "Add Provider" 按钮
And 我选择平台类型为 "WeChat Pay"
And 我填写配置信息：
  | App ID       | wx1234567890abcdef    |
  | Merchant ID  | 1234567890            |
  | Serial No    | 1A2B3C4D5E6F          |
  | API v3 Key   | my_v3_key_32chars_long_xxxxxx  |
  | Notify URL   | https://example.com/api/third/pay/realm-1/wechat/webhooks |
And 我上传商户 RSA 私钥文件
And 我提交表单
Then 支付平台配置创建成功
And 系统显示成功消息："Payment provider 'WeChat Pay' configured successfully"
And 私钥和 API v3 Key 已加密存储
And 查看配置时敏感信息显示为脱敏格式
```

**场景 2：同一平台只能有一个配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在微信支付配置
When 我尝试创建另一个微信支付配置
Then 系统显示错误："A WeChat Pay configuration already exists. Please edit the existing configuration."
And 配置创建失败
```

**场景 3：Notify URL 必须为 HTTPS**
```gherkin
Given 我是 realm-1 的管理员
When 我填写 Notify URL 为非 HTTPS 地址 "http://example.com/..."
Then 系统显示验证错误："Notify URL must be a valid HTTPS URL"
And 配置创建失败
```

**场景 4：私钥文件格式验证**
```gherkin
Given 我是 realm-1 的管理员
When 我上传的私钥文件格式不正确（非 PEM 格式）
Then 系统显示验证错误："Invalid private key format. Please upload a valid PEM file."
And 配置创建失败
```

---

## 故事 2：查看微信支付平台配置 [US-WP-002]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看微信支付平台配置和状态
**从而**：管理支付集成的运行状态

**【验收标准】**

**场景 1：查看微信支付配置**
```gherkin
Given 我是 realm-1 的管理员
And 已配置微信支付平台
When 我访问支付平台管理页面
Then 我看到微信支付配置信息：
  | 字段           | 显示内容                     |
  | Platform       | WeChat Pay                  |
  | App ID         | wx1234567890abcdef          |
  | Merchant ID    | 1234567890                  |
  | Serial No      | 1A2B3C4D5E6F                |
  | API v3 Key     | my_v3_*******************    |
  | Private Key    | *********** (已配置)         |
  | Notify URL     | https://example.com/...     |
  | Last Updated   | 2026-04-04 10:00:00 UTC    |
And 私钥和 API v3 Key 显示为脱敏格式
```

**场景 2：查看未配置状态**
```gherkin
Given 我是 realm-1 的管理员
And 未配置微信支付平台
When 我访问支付平台管理页面
Then 微信支付平台显示为 "Not configured" 状态
And 提供 "Configure" 操作入口
```

---

## 故事 3：编辑微信支付平台配置 [US-WP-003]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：编辑微信支付平台配置
**从而**：应对密钥轮换和配置变更

**【验收标准】**

**场景 1：更新 API v3 Key**
```gherkin
Given 我是 realm-1 的管理员
And 已存在微信支付配置
When 我点击 "Edit" 按钮
And 我更新 API v3 Key（密钥轮换）
And 我保存更改
Then 配置更新成功
And 新的 API v3 Key 加密存储
And 系统显示成功消息："Configuration updated successfully"
```

**场景 2：更新商户私钥**
```gherkin
Given 我是 realm-1 的管理员
And 已存在微信支付配置
When 我上传新的商户 RSA 私钥文件
And 我更新证书序列号
And 我保存更改
Then 新的私钥加密存储
And 旧私钥被替换
And 系统显示成功消息："Configuration updated successfully"
```

**场景 3：不允许修改平台类型**
```gherkin
Given 我是 realm-1 的管理员
And 已存在微信支付配置
When 我点击 "Edit" 按钮
Then "Platform" 字段为只读或禁用
And 我无法将 WeChat Pay 改为其他平台类型
```

---

## 故事 4：删除微信支付平台配置 [US-WP-004]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：删除微信支付平台配置
**从而**：移除不再使用的支付平台

**【验收标准】**

**场景 1：删除无订阅的配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在微信支付配置且无活跃订阅
When 我在支付平台列表中点击 "Delete" 按钮
And 我确认删除
Then 支付平台配置删除成功
And 系统显示成功消息："Payment provider 'WeChat Pay' deleted"
And 支付平台列表不再显示该配置
```

**场景 2：无法删除有活跃订阅的配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在微信支付配置且有活跃订阅
When 我尝试删除该配置
Then 系统显示错误："Cannot delete payment provider with active subscriptions"
And 显示活跃订阅数量
And 配置删除失败
```

**场景 3：删除前二次确认**
```gherkin
Given 我是 realm-1 的管理员
When 我点击 "Delete" 按钮
Then 系统显示确认对话框："删除后将无法恢复此配置。确认删除微信支付配置？"
When 我点击 "取消"
Then 配置保持不变
When 我点击 "删除"
Then 配置被删除
```

---

## 故事 5：用户通过微信扫码支付 [US-WP-005]

**优先级**: P0

**【用户故事】**
**作为**：Regular User
**我希望**：在 Herald 订阅页面选择微信支付后扫码完成付款
**从而**：使用微信便捷地购买订阅套餐

**【验收标准】**

**场景 1：正常扫码支付流程**
```gherkin
Given 我是已登录用户
And 当前 Realm 已配置微信支付
And 我选择了一个订阅套餐
When 我点击 "微信支付" 按钮
Then 页面显示一个二维码
And 二维码下方显示 "请使用微信扫描二维码完成支付" 提示
And 页面自动轮询支付状态（间隔 2-3 秒）
When 我使用微信扫描二维码并完成支付
Then 页面显示 "支付成功"
And 页面显示我的订阅信息
And 我获得对应的积分
```

**场景 2：二维码过期**
```gherkin
Given 我已经获取了支付二维码
When 二维码超过 2 小时未支付
Then 页面显示 "二维码已过期" 提示
And 提供 "重新获取二维码" 按钮
When 我点击 "重新获取二维码"
Then 系统关闭旧订单并生成新的二维码
```

**场景 3：支付失败**
```gherkin
Given 我已经获取了支付二维码
When 微信支付回调通知支付失败
Then 页面显示 "支付失败" 提示
And 提供 "重新支付" 按钮
```

**场景 4：用户取消支付**
```gherkin
Given 我已经获取了支付二维码
When 我点击 "取消支付" 按钮
Then 系统关闭微信支付订单
And 页面返回套餐选择页面
```

**场景 5：未配置微信支付时禁用按钮**
```gherkin
Given 我是已登录用户
And 当前 Realm 未配置微信支付
When 我查看套餐详情
Then "微信支付" 按钮为禁用状态
And 显示提示 "该支付方式暂未开通"
```

---

## 故事 6：微信支付 Webhook 回调处理 [US-WP-006]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：接收并处理微信支付 Webhook 回调通知
**从而**：及时同步支付状态并触发积分发放等业务逻辑

**【验收标准】**

**场景 1：支付成功回调处理**
```gherkin
Given 微信支付平台已配置
And 用户已完成微信支付
When 微信发送支付成功回调通知到 Herald
Then Herald 验证回调签名（SHA256-RSA）
And Herald 解密回调数据（AEAD_AES_256_GCM）
And Herald 根据商户订单号查找本地订单
And Herald 校验回调金额与订单金额一致
And Herald 创建本地订阅记录并发放积分
And Herald 在 5 秒内返回 200 响应
```

**场景 2：签名验证失败**
```gherkin
Given 微信支付平台已配置
When 收到签名不合法的回调请求
Then Herald 返回 401 错误
And Herald 记录安全审计日志
And 不执行任何业务逻辑
```

**场景 3：重复回调幂等处理**
```gherkin
Given 微信支付平台已配置
And 订单 order-123 的支付成功回调已处理
When 微信再次发送 order-123 的支付成功回调（重试）
Then Herald 识别为重复通知
And Herald 返回 200 响应
And 不重复发放积分或创建订阅
```

**场景 4：金额不一致**
```gherkin
Given 微信支付平台已配置
And 本地订单金额为 1000 分
When 收到回调通知金额为 500 分
Then Herald 记录金额不一致告警日志
And Herald 不发放积分
And Herald 不更新订单状态为成功
```

---

## 故事 7：主动查询支付状态 [US-WP-007]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统
**我希望**：主动查询微信支付订单状态
**从而**：作为 Webhook 的补充手段，确保支付状态一致性

**【验收标准】**

**场景 1：前端轮询触发状态查询**
```gherkin
Given 用户已创建微信支付订单
When 前端发起支付状态查询请求
Then 后端调用微信支付查询订单 API
And 如果支付成功，返回支付成功状态
And 如果支付未完成，返回待支付状态
And 如果订单已关闭，返回已关闭状态
```

**场景 2：Webhook 未到达时的补偿查询**
```gherkin
Given 用户已创建微信支付订单
And 订单创建超过 5 分钟且未收到 Webhook 回调
When 系统执行定时补偿查询
Then 后端调用微信支付查询订单 API
And 如果实际已支付，更新本地订单状态并补发积分
And 如果未支付，保持待支付状态
```

**场景 3：微信 API 调用失败**
```gherkin
Given 用户已创建微信支付订单
When 前端发起支付状态查询
And 后端调用微信支付查询订单 API 失败（网络超时或服务不可用）
Then 后端返回当前本地订单状态
And 系统记录查询失败的告警日志
And 前端继续轮询并提示用户稍后再试
```

**场景 4：微信 API 返回异常状态**
```gherkin
Given 用户已创建微信支付订单
When 后端调用微信支付查询订单 API
And 微信返回异常状态（如 TRADE_ERROR 或 SYSTEMERROR）
Then 后端保持本地订单为待支付状态
And 系统记录异常状态日志
And 下次补偿查询时重试
```

---

## 故事 8：关闭过期支付订单 [US-WP-008]

**优先级**: P1

**【用户故事】**
**作为**：Herald 系统
**我希望**：自动关闭过期的微信支付订单
**从而**：防止用户扫描过期二维码导致支付异常

**【验收标准】**

**场景 1：自动关闭过期订单**
```gherkin
Given 存在创建超过 2 小时仍未支付的微信支付订单
When 系统执行定时清理任务
Then 系统调用微信关单 API 关闭该订单
And 本地订单状态更新为 "已关闭"
```

**场景 2：关闭订单前确认未支付**
```gherkin
Given 存在即将关闭的微信支付订单
When 系统准备关闭该订单
Then 系统先查询微信侧该订单的实际状态
And 如果已支付，不执行关单操作，按支付成功处理
And 如果未支付，执行关单操作
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 5 | 配置微信支付、查看配置、用户扫码支付、Webhook 处理、主动查询状态 |
| P1 | 3 | 编辑配置、删除配置、关闭过期订单 |
| P2 | 0 | - |

---

## 与其他支付平台对比

| 特性 | Creem | Stripe | Shopify | WeChat Pay |
|------|-------|--------|---------|------------|
| 类型 | 模拟平台 | 真实支付 | 真实支付 | 真实支付 |
| 支付方式 | 重定向 | 重定向 | 订阅合同 | 扫码（Native） |
| Webhook 签名 | HMAC-SHA256 | HMAC-SHA256 | HMAC-SHA256 | SHA256-RSA |
| 回调数据 | 明文 JSON | 明文 JSON | 明文 JSON | AEAD 加密 |
| 前端交互 | 跳转 | 跳转 | 跳转 | 二维码 + 轮询 |
| 状态确认 | Webhook | Webhook | Webhook | Webhook + 主动查询 |
| 金额单位 | 各异 | 各异 | 各异 | 分（整数） |
| 适用场景 | 开发测试 | 国际市场 | 电商订阅 | 中国市场 |

---

## 相关文档

- **PRD**: `docs/prd/billing/wechat-pay.md` - 微信支付集成产品需求文档
- **技术背景**: `.ai/future/wechat.md` - 微信支付技术背景调研
- **通用支付平台配置**: `docs/user-stories/07-payment-provider-user-stories.md`
- **PRD**: `docs/prd/billing/billing.md` - Billing 订阅计费产品需求文档
- **PRD**: `docs/prd/billing/stripe-payment.md` - Stripe 支付集成产品需求文档
- **PRD**: `docs/prd/billing/shopify-pay.md` - Shopify Pay 支付集成产品需求文档
