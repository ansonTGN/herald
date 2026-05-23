# 统一购买架构产品需求文档 (PRD)

**创建时间**: 2026-04-08
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

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

### 1.2 优先级汇总表

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

### 2.2 不包含功能

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

- **三层模型**：Catalog Layer（定义卖什么）、Purchase Layer（表达一次支付尝试）、Fulfillment Layer（表达支付完成后的履约结果）
- **购买对象与履约结果解耦**：订阅购买创建 Subscription + 发放 subscription_credit；积分包购买仅发放 topup_credit
- **平台分型**：发起式支付平台（Wechat/Stripe/Creem）进入 PaymentAttempt 模型；合同同步型平台（Shopify）保持 webhook-driven 模式

---

## 4. 业务规则与状态

### 4.1 业务规则

- **积分包管理规则**：积分包包含名称、标题、描述、积分数、价格、货币、排序；支持启用/禁用状态切换
- **积分包名称唯一性**：同一 Realm 内积分包名称不可重复
- **价格与积分校验**：价格或积分数必须为正数
- **删除保护**：有购买记录的积分包不可删除
- **支付平台前置条件**：未配置支付平台时用户不可发起购买
- **购买对象区分**：支持 subscription_plan 和 points_package 两种购买对象类型
- **履约分流**：订阅购买创建/更新 Subscription + 发放 subscription_credit；积分包购买仅发放 topup_credit
- **履约幂等性**：重复支付成功通知不重复履约
- **平台分型规则**：Wechat/Stripe/Creem 进入 PaymentAttempt 模型；Shopify 保持 webhook-driven 订阅同步，不进入 PaymentAttempt

### 4.2 关键状态与异常

- **PaymentAttempt 状态**：Pending、RequiresAction、Succeeded、Failed、Cancelled、Expired
- **过期处理**：过期的支付尝试自动关闭，防止用户扫描过期二维码
- **支付失败场景**：用户主动取消、支付平台返回失败、支付超时未完成、二维码过期未扫描；前端需展示明确的失败原因和重试选项
- **履约失败场景**：支付成功但积分发放失败、订阅创建失败；系统记录失败日志并触发告警
- **并发冲突场景**：同一用户同时创建多个支付尝试、重复的支付成功回调通知；系统保证数据一致性和幂等性
- **业务规则验证场景**：积分包名称重复、价格或积分数为非正数、未配置支付平台时尝试购买、删除有购买记录的积分包；系统提供清晰的验证错误提示

---

## 5. 功能需求

### 5.1 核心需求

- **积分包（PointsPackage）模型**：支持创建、编辑、删除和启用/禁用积分包；支持配置积分包的支付平台映射（PointsPackagePaymentProvider）
- **PaymentAttempt 模型**：统一发起式支付平台的支付尝试，包含支付平台、购买对象类型、购买对象 ID、金额、货币、状态
- **购买对象（PurchasableTarget）区分**：支持 subscription_plan 和 points_package 两种购买对象
- **履约分流逻辑**：根据购买对象类型执行不同的履约逻辑，确保幂等性
- **多支付平台协调**：不同平台保持其原生支付时序

### 5.2 验收目标

- 管理员可以创建、编辑、删除和启用/禁用积分包
- 用户可以通过微信支付、Stripe 等平台购买积分包
- Wechat/Stripe/Creem 使用统一的 PaymentAttempt 模型
- 订阅购买创建 Subscription，积分包购买仅发放积分
- Shopify 继续保持 webhook-driven 订阅同步模式
- 重复支付成功通知不重复履约
- 过期的支付尝试自动关闭

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力范围**：积分包管理（创建/编辑/删除/查询、支付平台映射配置、启用/禁用）、购买（创建支付尝试、查询状态、关闭）、履约（支付成功后内部履约、发放 subscription_credit 或 topup_credit）
- **访问控制原则**：积分包管理仅 Realm Admin 可访问；购买接口已登录用户可访问；履约接口为系统内部调用不对外暴露
- **租户/Realm 数据边界**：积分包和 PaymentAttempt 按 Realm 隔离，不同 Realm 互不干扰
- **兼容性要求**：不影响现有订阅购买流程、Shopify webhook 处理和积分发放逻辑
- **相关接口说明位置**：积分系统见 `docs/prd/billing/points.md`，微信支付见 `docs/prd/billing/wechat-pay.md`，Shopify 见 `docs/prd/billing/shopify-pay.md`

---

## 7. 前端/交互约束

**适用性**: 适用

- **页面入口**：积分包管理页面（Realm Admin）；积分包购买页面（Regular User）；订阅购买页面（Regular User）

- **积分包购买关键交互**：用户选择积分包 → 选择支付平台 → 创建支付尝试 → 展示支付上下文（如二维码）→ 轮询支付状态 → 展示结果

- **订阅购买关键交互**：用户选择套餐 → 选择支付平台 → 创建支付尝试（除 Shopify 外）→ 展示支付上下文 → 轮询状态 → 展示结果和订阅信息

- **状态反馈**：
  - Pending：展示支付上下文（二维码或跳转链接）
  - Succeeded：展示支付成功页面和履约结果
  - Failed：展示支付失败页面，提供重新支付按钮
  - Expired：展示二维码已过期，提供重新获取按钮
  - Cancelled：展示支付已取消，返回选择页面

- **履约结果反馈**：订阅购买展示订阅信息、会员积分余额、下次续费时间；积分包购买展示充值积分余额和积分有效期

- **权限可见性**：
  - 积分包管理页面：仅 Realm Admin 可见
  - 个人中心 Points 入口：当 Realm 下存在 enabled PointsPackage 时显示
  - 积分包购买按钮和支付平台选择：只有 enabled PointsPackage 已配置 enabled 支付平台映射，且对应支付平台已在 Realm 中启用时，才允许发起购买
  - 管理后台积分包入口：只按管理权限控制，不因尚未创建积分包或配置映射而隐藏

- **支付流程状态持久化**：前端需保存当前选择的积分包和支付平台，刷新后可恢复；支付尝试 ID 需持久化存储支持轮询

- **审计追踪**：记录积分包创建/编辑/删除、支付平台配置变更、购买行为、支付和履约的成败事件到 PaymentEvent

---

## 8. 已确认决策

### 8.1 已确认决策

- 采用三层模型（Catalog / Purchase / Fulfillment）统一购买架构
- PaymentAttempt 仅用于发起式支付平台（Wechat/Stripe/Creem），Shopify 保持 webhook-driven 模式
- 订阅购买与积分包购买通过 PurchasableTarget 类型区分，履约逻辑分流
- 不改变现有 subscription_credit 和 topup_credit 的分离逻辑

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/points-package.md`、`docs/user-stories/billing/points-package-purchase.md`、`docs/user-stories/billing/payment-attempt.md`
- 相关 PRD：`docs/prd/billing/subscription.md`、`docs/prd/billing/product-catalog.md`、`docs/prd/billing/points.md`、`docs/prd/billing/wechat-pay.md`、`docs/prd/billing/shopify-pay.md`
