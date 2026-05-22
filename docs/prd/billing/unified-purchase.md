# 统一购买架构产品需求文档 (PRD)

**创建时间**: 2026-04-08
**状态**: Partially Implemented
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 相关故事

**积分包管理**
- `[US-PP-001]` 创建积分包，优先级 P0，来源 `docs/user-stories/billing/points-package.md`
- `[US-PP-002]` 编辑积分包，优先级 P0，来源 `docs/user-stories/billing/points-package.md`
- `[US-PP-003]` 配置积分包的支付平台映射，优先级 P0，来源 `docs/user-stories/billing/points-package.md`
- `[US-PP-004]` 查看积分包列表，优先级 P0，来源 `docs/user-stories/billing/points-package.md`
- `[US-PP-005]` 删除积分包，优先级 P1，来源 `docs/user-stories/billing/points-package.md`
- 角色：Realm Admin
- 摘要：管理员创建和管理积分包商品，用户购买积分包获得充值积分（topup_credit）

**积分包购买**
- `[US-PU-06]` 购买积分包，优先级 P0，来源 `docs/user-stories/billing/points-package-purchase.md`
- `[US-PU-07]` 查看积分包购买记录，优先级 P1，来源 `docs/user-stories/billing/points-package-purchase.md`
- `[US-PU-08]` 积分包与订阅购买的区别，优先级 P1，来源 `docs/user-stories/billing/points-package-purchase.md`
- 角色：Regular User
- 摘要：用户购买积分包获得充值积分，理解积分包与订阅的区别

**PaymentAttempt 支付尝试**
- `[US-PA-001]` 创建支付尝试（订阅或积分包），优先级 P0，来源 `docs/user-stories/billing/payment-attempt.md`
- `[US-PA-002]` 查询支付尝试状态，优先级 P0，来源 `docs/user-stories/billing/payment-attempt.md`
- `[US-PA-003]` 处理支付成功后的履约，优先级 P0，来源 `docs/user-stories/billing/payment-attempt.md`
- `[US-PA-004]` 关闭过期的支付尝试，优先级 P1，来源 `docs/user-stories/billing/payment-attempt.md`
- 角色：System
- 摘要：统一发起式支付平台的支付流程，处理支付成功后的履约逻辑

**现有相关用户故事**
- `[US-PR-001 ~ US-PR-006]` Product/Plan 管理
- `[US-BI-001 ~ US-BI-003]` 订阅套餐管理
- `[US-WP-001 ~ US-WP-008]` 微信支付
- `[US-PP-007 ~ US-PP-015]` Shopify 支付

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 12 | 创建/编辑/管理积分包、购买积分包、创建/查询 PaymentAttempt、履约逻辑 |
| P1 | 4 | 删除积分包、查看购买记录、理解积分包与订阅区别、关闭过期支付尝试 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- **积分包（PointsPackage）模型**：作为一等商品模型，支持创建、编辑、删除和启用/禁用
- **积分包支付平台映射**：配置积分包在不同支付平台的可售状态
- **PaymentAttempt 模型**：统一发起式支付平台（Wechat/Stripe/Creem）的支付尝试
- **购买对象（PurchasableTarget）区分**：支持订阅购买和积分包购买两种购买对象
- **履约分流逻辑**：根据购买对象执行不同的履约逻辑（订阅 vs 积分包）
- **多支付平台统一**：确保 Wechat/Stripe/Creem/Shopify 在统一架构下协调工作
- **积分包购买流程**：用户通过微信支付、Stripe 等平台购买积分包

### 2.2 不包含功能 (Out of Scope)

- 不改变 Shopify 的 webhook-driven 订阅同步模式
- 不统一所有支付平台的支付时序（Shopify 不进入 PaymentAttempt）
- 不引入第二套订阅聚合系统
- 不改变现有的积分发放逻辑（subscription_credit 和 topup_credit 分离）
- 不改变 Product/Plan 的现有编目结构

### 2.3 依赖项

- **Product/Plan 编目系统**：积分包和订阅套餐都属于商品编目系统
- **积分系统**：履约时发放 subscription_credit 或 topup_credit
- **订阅系统**：订阅购买时创建或更新 Subscription
- **支付平台配置**：已配置 Wechat/Stripe/Creem/Shopify 等支付平台
- **PaymentEvent 审计**：所有支付事件记录到 PaymentEvent

---

## 3. 需求概述

### 3.1 功能描述

Herald 当前已在 Billing 领域引入了多支付平台支持，但不同平台的支付形态并不相同：

- **Wechat、Stripe、Creem**：Herald 主动发起一次支付尝试，再等待支付完成
- **Shopify**：外部平台先形成订阅合同，Herald 通过 webhook 同步并处理归属

如果继续以"全平台统一 Order"作为中心，会产生结构性问题：

1. 把 Shopify 强行塞进前置订单生命周期，模型失真
2. 把"支付过程"和"支付成功后的履约结果"混在一起
3. 无法自然表达"购买积分包但不创建订阅"的场景
4. 容易把平台差异隐藏到 JSON 字段中，削弱查询、对账和补偿能力

因此，本方案引入统一购买架构，核心目标如下：

1. **统一"买什么、怎么付、支付后产出什么"的建模方式**
2. **同时支持订阅购买与积分包购买**
3. **同时支持发起式支付与 webhook 驱动式订阅同步**
4. **保留平台差异，不做假统一**
5. **为后续接入更多支付平台提供稳定边界**

### 3.2 关键特性

**三层模型**
- **Catalog Layer**：定义卖什么（Product/Plan、PointsPackage）
- **Purchase Layer**：表达一次支付尝试（PaymentAttempt）
- **Fulfillment Layer**：表达支付完成后的履约结果（Subscription、Points Grant）

**购买对象与履约结果解耦**
- 订阅购买 → 创建 Subscription + 发放 subscription_credit
- 积分包购买 → 仅发放 topup_credit（不创建 Subscription）

**平台分型**
- **发起式支付平台**（Wechat/Stripe/Creem）：进入 PaymentAttempt 模型
- **合同同步型平台**（Shopify）：保持 webhook-driven 模式

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| Product/Plan 管理 | ✅ 已实现 | 已支持 Product 编目和 Plan 管理 |
| 订阅购买 | 🚧 部分实现 | Wechat/Stripe/Shopify 已有相关实现 |
| 积分包（PointsPackage） | ❌ 未实现 | 需要新增积分包模型 |
| PaymentAttempt 模型 | ❌ 未实现 | 需要新增统一的支付尝试模型 |
| 积分包购买流程 | ❌ 未实现 | 需要实现积分包的购买和履约逻辑 |
| 履约分流逻辑 | ⚠️ 部分实现 | 订阅履约已实现，积分包履约待实现 |
| 多支付平台统一 | ⚠️ 部分实现 | 各平台独立实现，待统一 |

---

## 5. 功能需求

### 5.1 核心需求

**积分包（PointsPackage）模型**
- 支持创建、编辑、删除和启用/禁用积分包
- 积分包包含：名称、标题、描述、积分数、价格、货币、排序
- 支持配置积分包的支付平台映射（PointsPackagePaymentProvider）

**PaymentAttempt 模型**
- 统一发起式支付平台的支付尝试
- 包含：支付平台、购买对象类型、购买对象 ID、金额、货币、状态
- 支持状态：Pending、RequiresAction、Succeeded、Failed、Cancelled、Expired

**购买对象（PurchasableTarget）区分**
- 支持两种购买对象：subscription_plan、points_package
- 订阅套餐：购买后创建 Subscription + 发放 subscription_credit
- 积分包：购买后仅发放 topup_credit

**履约分流逻辑**
- 根据购买对象类型执行不同的履约逻辑
- 订阅购买：创建或更新 Subscription，发放 subscription_credit
- 积分包购买：仅发放 topup_credit，不创建 Subscription
- 确保履约幂等性（重复支付成功通知不重复履约）

**多支付平台协调**
- Wechat/Stripe/Creem：进入 PaymentAttempt 模型
- Shopify：保持 webhook-driven 订阅同步模式
- 不同平台保持其原生支付时序

### 5.2 验收目标

- **积分包管理**：管理员可以创建、编辑、删除和启用/禁用积分包
- **积分包购买**：用户可以通过微信支付、Stripe 等平台购买积分包
- **支付尝试统一**：Wechat/Stripe/Creem 使用统一的 PaymentAttempt 模型
- **履约分流**：订阅购买创建 Subscription，积分包购买仅发放积分
- **Shopify 独立**：Shopify 继续保持 webhook-driven 订阅同步模式
- **幂等性保证**：重复支付成功通知不重复履约
- **过期处理**：过期的支付尝试自动关闭，防止用户扫描过期二维码

### 5.3 异常场景处理

**支付失败场景**
- 用户主动取消支付
- 支付平台返回失败状态
- 支付超时未完成
- 二维码过期未扫描
- 前端需要展示明确的失败原因和重试选项

**履约失败场景**
- 支付成功但积分发放失败
- 支付成功但订阅创建失败
- 数据库连接异常
- 外部服务调用超时
- 系统需要记录失败日志并触发告警

**并发冲突场景**
- 同一用户同时创建多个支付尝试
- 重复的支付成功回调通知
- 积分包被删除但仍有购买记录
- 系统需要保证数据一致性和幂等性

**业务规则验证场景**
- 积分包名称重复
- 价格或积分数为非正数
- 未配置支付平台时尝试购买
- 删除有购买记录的积分包
- 系统需要提供清晰的验证错误提示

---

## 6. API 相关约束

**状态**: 必填

### 6.1 接口能力范围

**积分包管理接口**
- 创建/编辑/删除/查询积分包
- 配置积分包的支付平台映射
- 启用/禁用积分包

**购买接口**
- 创建支付尝试（订阅或积分包）
- 查询支付尝试状态
- 关闭支付尝试

**履约接口**
- 处理支付成功后的履约（内部接口）
- 发放 subscription_credit 或 topup_credit

### 6.2 访问控制原则

- **积分包管理**：仅 Realm Admin 可访问
- **购买接口**：已登录用户可访问
- **履约接口**：系统内部调用，不对外暴露

### 6.3 租户/Realm 数据边界

- 积分包按 Realm 隔离
- PaymentAttempt 按 Realm 隔离
- 不同 Realm 的积分包和支付尝试互不干扰

### 6.4 兼容性要求

- 不影响现有订阅购买流程
- 不影响现有 Shopify webhook 处理
- 不改变现有积分发放逻辑

### 6.5 相关接口说明位置

- 积分系统接口：`docs/prd/billing/points.md`
- 微信支付接口：`docs/prd/billing/wechat-pay.md`
- Shopify 接口：`docs/prd/billing/shopify-pay.md`

---

## 7. 前端/交互约束

**状态**: 必填

### 7.1 页面入口

- **积分包管理页面**：`/billing/points-packages`（Realm Admin）
- **积分包购买页面**：`/billing/purchase-points`（Regular User）
- **订阅购买页面**：`/billing/subscribe`（Regular User）

### 7.2 关键交互

**积分包购买流程**
1. 用户访问积分包购买页面
2. 用户选择积分包
3. 用户选择支付平台
4. 前端调用创建支付尝试接口
5. 前端展示支付上下文（如二维码）
6. 前端轮询支付状态
7. 支付成功后展示结果页面

**订阅购买流程**
1. 用户选择订阅套餐
2. 用户选择支付平台
3. 前端调用创建支付尝试接口（除 Shopify 外）
4. 前端展示支付上下文
5. 前端轮询支付状态
6. 支付成功后展示结果和订阅信息

### 7.3 状态反馈

**支付尝试状态反馈**
- Pending：展示支付上下文（二维码或跳转链接）
- Succeeded：展示支付成功页面，展示履约结果
- Failed：展示支付失败页面，提供重新支付按钮
- Expired：展示二维码已过期，提供重新获取按钮
- Cancelled：展示支付已取消，返回选择页面

**履约结果反馈**
- 订阅购买：展示订阅信息、会员积分余额、下次续费时间
- 积分包购买：展示充值积分余额、积分有效期

### 7.4 权限可见性

- 积分包管理页面：仅 Realm Admin 可见
- 个人中心的 Points 入口：当 Realm 下存在 enabled PointsPackage 时显示；仅用于余额和购买记录查看的入口不要求积分包已经配置支付平台映射
- 积分包购买页面：已登录用户可访问，但只在存在 enabled PointsPackage 时作为个人中心入口展示
- 积分包购买按钮和支付平台选择：只有 enabled PointsPackage 已配置 enabled 支付平台映射，且对应支付平台已在 Realm 中启用时，才允许发起购买；支付平台选择仅显示满足这些条件的平台
- 管理后台积分包入口：只按管理权限控制，不因为当前 Realm 尚未创建积分包或尚未配置支付平台映射而隐藏

### 7.5 状态管理要求

**支付流程状态持久化**
- 前端需要在支付流程中保存当前选择的积分包和支付平台
- 用户刷新页面后能够恢复到支付前状态，不丢失选择信息
- 支付尝试 ID 需要持久化存储，支持轮询查询

**跨页面状态同步**
- 购买成功后需要同步更新积分余额显示
- 订阅购买成功后需要同步更新订阅状态和会员积分
- 支付状态变化需要实时反馈到前端页面

**错误恢复机制**
- 支付失败后保留用户选择，方便重新发起支付
- 网络异常时支持重试机制，不影响用户已提交的请求
- 前端需要处理支付超时和二维码过期场景

**状态一致性保证**
- 前端显示的支付状态需要与后端保持一致
- 轮询机制需要处理并发请求和响应乱序问题
- 支付成功后需要禁用重复支付按钮

### 7.6 分析与追踪

**关键事件追踪**
- 积分包创建/编辑/删除操作
- 支付平台配置变更
- 用户购买积分包行为
- 支付成功/失败事件
- 履约成功/失败事件

**业务指标监控**
- 积分包购买转化率
- 各支付平台使用率
- 支付失败率分析
- 履约失败率监控
- 积分发放统计

**审计日志要求**
- 记录所有支付相关操作到 PaymentEvent
- 支持按用户、时间范围、支付平台筛选审计记录
- 保留完整的支付链路追踪信息

### 7.7 可访问性要求

**页面可访问性**
- 积分包购买页面支持键盘导航
- 支付平台选择按钮支持键盘操作
- 二维码展示区域提供替代文本描述
- 错误提示信息支持屏幕阅读器

**交互反馈可访问性**
- 支付状态变化提供视觉和听觉反馈
- 表单验证错误信息清晰标注在对应字段
- 支付成功/失败提供明确的文本提示
- 加载状态提供进度指示器

**颜色和对比度**
- 支付按钮使用足够的颜色对比度
- 积分包价格信息清晰可读
- 支付状态标识使用图标+文字组合
- 支持高对比度模式


## 8. 相关文件索引

### 8.1 后端文件

**待实现**
- `backend/core/src/domain/points_package/` - 积分包领域模型与服务
- `backend/core/src/domain/payment_attempt/` - 支付尝试领域模型与服务
- `backend/core/src/domain/purchase/` - 统一购买与履约服务
- `backend/core/src/entity/points_package.rs` - 积分包实体
- `backend/core/src/entity/points_package_payment_provider.rs` - 积分包支付平台映射实体
- `backend/core/src/entity/payment_attempt.rs` - 支付尝试实体
- `backend/core/src/entity/points_package_purchase.rs` - 积分包购买记录实体
- `backend/api/src/application/http/billing/points_package_handler.rs` - 积分包 API
- `backend/api/src/application/http/billing/purchase_handler.rs` - 购买 API

**已有相关文件**
- `backend/core/src/domain/points/` - 积分领域模型
- `backend/api/src/application/http/billing/handlers.rs` - 现有 Stripe / Creem checkout 入口
- `backend/api/src/application/http/billing/wechat_order_handlers.rs` - 现有微信订单入口
- `backend/api/src/application/http/billing/wechat_webhook_handlers.rs` - 微信 webhook 入口
- `backend/api/src/application/http/billing/stripe_webhook_handlers.rs` - Stripe webhook 入口
- `backend/api/src/application/http/billing/webhook_handlers.rs` - Creem webhook 入口
- `backend/api/src/application/http/billing/shopify_webhook_handlers.rs` - Shopify webhook 入口
- `backend/core/src/entity/payment_event.rs` - 支付事件审计表

### 8.2 前端文件

**待实现**
- `frontend/src/routes/$realmId/manage/points-packages.tsx` - 积分包管理页面路由
- `frontend/src/routes/$realmId/user/purchase-points.tsx` - 购买积分包页面路由
- `frontend/src/components/points-packages/` - 积分包管理组件
- `frontend/src/components/purchase/` - 统一购买组件
- `frontend/src/stores/purchase-flow-store.ts` - 支付流程状态持久化 store

**已有相关文件**
- `frontend/src/routes/$realmId/manage/billing.tsx` - 当前管理端 Billing 入口
- `frontend/src/routes/$realmId/user/points.tsx` - 当前用户积分页入口
- `frontend/src/routeTree.gen.ts` - 当前路由真相
- `frontend/src/stores/auth-store.ts` - Zustand persist 持久化基线

---

## 9. 参考资料

### 9.1 用户故事

- 积分包管理：`docs/user-stories/billing/points-package.md`
- 积分包购买：`docs/user-stories/billing/points-package-purchase.md`
- PaymentAttempt：`docs/user-stories/billing/payment-attempt.md`

### 9.2 相关 PRD

- Billing 订阅计费：`docs/prd/billing/subscription.md`
- Product 编目管理：`docs/prd/billing/product-catalog.md`
- Points 积分系统：`docs/prd/billing/points.md`
- 微信支付集成：`docs/prd/billing/wechat-pay.md`
- Shopify Pay 支付集成：`docs/prd/billing/shopify-pay.md`

### 9.3 规范文档

