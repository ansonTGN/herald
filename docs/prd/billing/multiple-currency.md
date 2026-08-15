# 多货币（按货币选择/本地化）产品需求文档 (PRD)

**创建时间**: 2026-08-15
**优先级**: P1

> 场景背景：多货币（按货币选择/本地化）建立在 Herald 已支持的「一产品多货币多价格」目录能力之上，为 Stripe 多 Price 产品叠加按 Realm/用户偏好货币的解析、购买页货币分组展示与第三方货币查询能力。本文档不承载接口端点、请求/响应 schema、HTTP 状态码、数据库建表/迁移或代码类型定义；技术方案细节请参见对应技术设计。

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 相关故事

**多货币特有**，来源 `docs/user-stories/billing/multiple-currency.md`：
- `[US-MC-001]` 配置 Realm 默认偏好货币，优先级 P0，角色 Realm Admin
- `[US-MC-002]` 设置个人偏好货币覆盖，优先级 P1，角色 Regular User
- `[US-MC-003]` 购买页按货币分组并高亮偏好货币，优先级 P0，角色 Regular User
- `[US-MC-004]` 按偏好货币解析价格行发起购买，优先级 P0，角色 Regular User
- `[US-MC-005]` 查询可购权益支持的货币集合，优先级 P0，角色 Third-party App
- `[US-MC-006]` Creem / IAP / WeChat Pay 单一价格降级展示，优先级 P2，角色 Regular User

**多价格同步/购买/解析基线（本特性复用）**，来源 `docs/user-stories/billing/entitlement-mapping.md`：
- `[US-EM-007～009]` 多价格同步配置、Webhook 解析与指定价格购买 —— P0
- `[US-BL-SYNC-001～004]` 产品名、价格单位与计费周期同步展示 —— P0

**统一支付尝试与履约（本特性复用）**，来源 `docs/user-stories/billing/payment-attempt.md`：
- `[US-PA-001～004]` 创建支付尝试、查询状态、成功后履约、关闭过期 —— P0/P1

### 1.2 优先级汇总

| 优先级 | 数量（多货币特有） | 关键故事 |
|--------|------|----------|
| P0 | 4 | 配置 Realm 默认货币、购买页按货币分组并高亮、按偏好货币解析下单、查询可购权益货币集合 |
| P1 | 1 | 设置个人偏好货币覆盖 |
| P2 | 1 | Creem / IAP / WeChat Pay 单一价格降级展示 |

> 多价格同步/购买/解析与统一支付尝试/履约的优先级见各自来源文件，此处不重复汇总。

---

## 2. 范围界定

### 2.1 包含功能

- **偏好货币承载**：Realm 默认货币（域内兜底）与用户覆盖货币（优先于 Realm 默认）两级；不设 Client App 级偏好（DEC-multiple_currency-004）
- **按货币解析价格行**：在购买/默认解析时按 (产品/权益 + 计费维度 + 货币) 反查启用的 Stripe 多 Price 映射行；货币为过滤维度，非唯一键（DEC-multiple_currency-005）
- **缺失回退**：偏好货币无对应价格行时，展示侧始终展示全部可用货币并高亮 偏好→Realm 默认→首个可用；程序化默认解析仅按生效偏好货币解析一次，无匹配即 fail-loud（见 §4.1，DEC-multiple_currency-006）
- **购买页本地化**：对 Stripe 多 Price 产品按货币分组、高亮偏好货币、提供货币切换；同货币下多计费周期/类型并存时由用户在货币组内选择
- **api-ext 货币暴露**：对每个可购权益暴露其支持的货币集合；订阅类与一次性购买类映射均暴露货币，供 SDK/第三方应用做货币切换与默认解析
- **渠道降级展示**：对 provider/store 侧定价渠道（Creem、IAP、WeChat Pay），购买页降级为单一价格展示，不渲染货币切换器（DEC-multiple_currency-003）

### 2.2 不包含功能 (Out of Scope)

- **改用 Stripe 单 Price `currency_options` 范式**：货币选择层建立在现有「一 Price 一映射行」目录之上，不读取 `currency_options`、不改同步逻辑（DEC-multiple_currency-002）
- **provider 侧定价渠道的货币解析**：Creem 产品级单一价格、IAP 由商店按地区定价、WeChat Pay 由商户在渠道侧定价，均不纳入 Herald 货币解析（DEC-multiple_currency-003）
- **Client App 级偏好货币**：偏好仅在 Realm 默认 + 用户覆盖两级承载（DEC-multiple_currency-004）
- **汇率换算 / 自动本地化定价**：Herald 不做货币换算；Stripe Adaptive Pricing 若启用属商店侧行为，Herald 只引用真实 Price（见 §4.2 已知展示局限）
- **provider 侧目录同步模型改造**：多货币多价格的目录能力已就绪，不新增 Product/Price schema、不改同步（DEC-multiple_currency-002）
- **促销 / 折扣 / 价格实验**：沿用既有「促销委托支付平台」边界，不在本期扩展
- **退款 / 订阅状态的货币相关改造**：退款与积分回收沿用既有模型，不因多货币改变

### 2.3 依赖项

- **既有 Billing 目录与同步**：`provider_entitlement_mappings`（一 Price 一行）与 provider 产品同步是货币解析的数据基线（`docs/prd/billing/subscription.md`）
- **既有统一支付尝试与履约**：货币解析只决定「选哪一行映射」，Checkout 构造与履约链路不变（`docs/user-stories/billing/payment-attempt.md`）
- **既有 Stripe / Creem / IAP / WeChat Pay 集成**：货币解析仅对 Stripe 多 Price 生效；其余渠道保持 provider 侧定价（DEC-multiple_currency-003）
- **Realm 设置与用户个人资料**：偏好货币承载点（DEC-multiple_currency-007）
- **权限系统**：偏好货币配置写入需 `settings.manage`、查看需 `settings.view`（Realm 默认货币）；用户覆盖为认证用户自助；api-ext 货币查询遵循既有 SDK 访问控制（DEC-multiple_currency-011）

---

## 3. 需求概述

### 3.1 功能描述

多货币（按货币选择/本地化）在 Herald 已支持的「一产品多货币多价格」目录能力之上，叠加一层「按货币选择与本地化购买体验」：系统按 Realm/用户偏好货币自动匹配价格行、在偏好缺失时按既定规则回退、在购买页按货币分组并高亮偏好货币、并向第三方应用暴露每个可购权益支持的货币集合。其本质是购买解析与展示的「货币维度本地化」，不改变既有目录、同步、Checkout 构造与履约链路。

### 3.2 关键特性

- **偏好驱动解析**：购买与默认解析读取 Realm 默认 + 用户覆盖两级偏好货币
- **货币即过滤维度**：货币缩小候选范围，与计费维度共同定位唯一价格行，避免串货币或零金额下单
- **展示本地化**：购买页按货币分组、高亮偏好、支持手动切换
- **SDK 可集成**：api-ext 暴露「可购权益 → 支持货币集合」，第三方应用可做货币切换与默认解析
- **渠道降级**：对 provider 侧定价渠道（Creem/IAP/WeChat Pay）自动降级为单一价格展示
- **零新增外部依赖**：复用现有目录模型与支付渠道集成，不改同步（DEC-multiple_currency-002）

---

## 4. 业务规则与状态

### 4.1 业务规则

**偏好货币规则**：
- 偏好货币分两级：Realm 默认货币（域内兜底）与用户覆盖货币；用户覆盖优先于 Realm 默认（DEC-multiple_currency-004）
- 不设 Client App 级偏好货币
- 货币码以 ISO 4217 标准表达；非法货币码（含保留码）在设置时被拒绝，不静默接受（DEC-multiple_currency-010）
- 货币码比较不区分 ASCII 大小写；偏好货币与对外暴露的货币集合、解析结果统一以大写 ISO 4217 码表达（DEC-multiple_currency-012）
- 偏好货币仅参与「展示高亮/排序」与「默认解析」，不改变用户最终可选的所有价格行

**货币解析规则**：
- 解析键 = (产品/权益 + 计费维度[类型/周期] + 货币)；货币是过滤维度而非唯一键（DEC-multiple_currency-005）
- 同一产品在同一货币下可并存多个计费周期/类型（如 USD 月付 + USD 年付）；此时货币选择只缩小候选范围，用户仍须在所选货币内选择计费周期/类型
- 仅指定货币（未指定计费维度）且该货币下存在多行时，不得静默选择任一行；解析失败须 fail-loud，不使用默认价格（沿用 subscription PRD 既有约束）
- 货币解析仅对 Stripe 多 Price 产品生效；Creem、IAP 与 WeChat Pay 等 provider 侧定价渠道不参与（DEC-multiple_currency-003）

**缺失回退规则（DEC-multiple_currency-006）**：
- **展示侧**：始终展示该产品全部可用货币的价格行，按偏好货币排序/高亮；偏好货币缺失时高亮 Realm 默认货币，Realm 默认也缺失时取首个可用货币；任何情况下都允许用户手动切换货币
- **程序化默认解析侧**（第三方应用/默认 checkout 仅传货币不传具体价格行时）：仅按「生效偏好货币」（用户覆盖货币，无覆盖时即 Realm 默认货币）解析一次，不做二级货币回退；无匹配即 fail-loud（不静默换币、不零金额下单）

**Checkout 构造规则**：
- 货币解析只决定「选哪一行映射」；选中价格行后，Checkout 仍引用真实 Stripe Price / Creem `product_id`，构造方式不变
- Stripe 映射行缺失价格信息时拒绝下单（fail-loud），不产生零金额或串货币支付；显式 `target_id` 购买的既有路径行为不变（DEC-multiple_currency-009）
- provider/store 侧定价渠道（Creem、IAP、WeChat Pay）的价格由渠道侧决定，Herald 不做服务端价格解析；其映射行无 Herald 侧价格信息属合法状态，不触发 fail-loud（DEC-multiple_currency-013）

**api-ext 暴露规则**：
- 对每个可购权益聚合其启用映射行覆盖的货币集合并对外暴露
- 订阅类映射与一次性购买类映射均暴露货币
- 货币集合仅反映已启用映射行，不包含禁用映射行的货币

**数据隔离规则**：
- Realm 默认货币为 Realm 级配置，仅本 Realm 生效
- 用户覆盖货币为用户级，仅影响该用户，不外泄给其他用户
- 不同 Realm 的偏好货币与解析结果完全隔离

### 4.2 关键状态与异常

**异常场景**：
- 偏好货币在该产品无对应价格行：按 §4.1「缺失回退规则」处理（展示侧展示全部可用货币；程序化默认解析仅按生效偏好货币解析一次，无匹配即 fail-loud，DEC-multiple_currency-006）
- 同货币多计费周期且未指定计费维度：解析 fail-loud，不静默选价，不发起支付
- 非法货币码：设置被拒绝，原设置不变（DEC-multiple_currency-010）
- Stripe 映射行缺失价格信息：下单被拒绝（fail-loud），不产生零金额或串货币支付（DEC-multiple_currency-009）
- 渠道无可选货币价格（Creem/IAP/WeChat Pay）：降级为单一价格展示，不渲染货币切换器，偏好货币不影响其展示价格（DEC-multiple_currency-003/013）
- Stripe Adaptive Pricing 启用导致的展示与实付货币不一致：**已知展示局限**。Herald 缓存的产品基础货币为展示货币；若运营方在 Stripe 启用 Adaptive Pricing，Checkout 会按用户地区自动换算展示与扣款，可能出现购买页展示货币（基础货币）与用户实付货币不一致。以 Checkout 实际呈现为准，购买页对基础货币做标注，不在 Herald 侧做换算（非阻塞，沿用 DEC-multiple_currency-002 不改同步）

---

## 5. 功能需求

### 5.1 核心需求

**偏好货币管理**：
- Realm Admin 可为本 Realm 设置与更新默认货币（ISO 货币码）；非法码被拒绝
- Regular User 可设置与清除个人偏好货币，优先级高于 Realm 默认
- 未设个人偏好货币的用户按 Realm 默认货币解析

**按货币解析价格行**：
- 给定产品/权益 + 计费维度 + 偏好货币，从该产品的启用 Stripe 多 Price 映射行中解析匹配行
- 货币为过滤维度；同货币多计费周期时由计费维度共同定位，缺失计费维度且多行时 fail-loud
- 仅对 Stripe 多 Price 产品生效；provider 侧定价渠道不触发货币解析
- Stripe 映射行缺失价格信息时拒绝下单（fail-loud）；显式 `target_id` 路径行为不变（DEC-multiple_currency-009）

**缺失回退**：
- 展示侧始终展示全部可用货币并高亮 偏好→Realm 默认→首个可用；允许手动切换
- 程序化默认解析仅按生效偏好货币解析一次，无匹配即 fail-loud（DEC-multiple_currency-006）

**购买页本地化**：
- 对 Stripe 多 Price 产品按货币分组展示价格行，高亮偏好货币并默认展开
- 同货币组内列出可选计费周期/类型
- 提供货币切换；偏好货币不可用时提示并允许选择其他可用货币

**api-ext 货币暴露**：
- 对每个可购权益聚合并暴露其支持货币集合
- 订阅类与一次性购买类映射均暴露货币
- 支持第三方应用按用户偏好货币解析默认价格行（命中返回；无匹配即 fail-loud，不回退其他货币）

**渠道降级展示**：
- 对 Creem 产品只展示其单一价格，不渲染货币切换器
- 对 IAP 产品按商店地区价格展示，不做货币解析或切换
- 对 WeChat Pay 产品按渠道侧配置的单一价格展示，不做货币解析或切换（DEC-multiple_currency-013）

### 5.2 验收目标

- Realm Admin 可设置/更新 Realm 默认货币，且对本 Realm 未设个人偏好的用户生效
- Regular User 可设置/清除个人偏好货币，且优先级高于 Realm 默认
- 购买页对 Stripe 多 Price 产品按货币分组、高亮偏好货币、支持切换；同货币多计费周期时用户可在货币组内选择周期
- 按偏好货币 + 计费维度解析能命中正确价格行；同货币多行且未指定计费维度时 fail-loud，不发起支付
- 偏好货币无匹配时：展示侧展示全部可用货币并高亮回退；程序化默认解析仅按生效偏好货币解析一次，无匹配即 fail-loud（DEC-multiple_currency-006）
- api-ext 对每个可购权益暴露支持货币集合，订阅类与一次性类均含货币；第三方应用可据此做货币切换与默认解析
- Creem / IAP / WeChat Pay 产品降级为单一价格展示，不渲染货币切换器，偏好货币不影响其展示
- 货币解析路径不出现串货币或零金额下单；Stripe 映射行缺失价格信息时下单被拒绝；显式 `target_id` 购买的既有行为不变
- 非法货币码在设置时被拒绝

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力范围**：偏好货币的读写（Realm 默认货币属 Realm 设置能力，用户覆盖货币属用户自助）；货币维度解析属购买/默认解析能力的扩展；可购权益的「支持货币集合」聚合与按货币解析默认价格行属 api-ext 查询能力的扩展。Checkout 发起与履约沿用既有统一能力，不在 PRD 列出端点、schema 或状态码。
- **访问控制原则**：遵守 realm 隔离；Realm 默认货币写入需 `settings.manage`、查看需 `settings.view`（与 Realm 设置承载一致，DEC-multiple_currency-011）；用户覆盖货币为认证用户自助；api-ext 货币查询遵循既有 SDK/第三方应用访问控制；金额与积分变更必须可追溯；货币解析须 fail-loud 而非静默替代。
- **租户/realm 边界**：偏好货币与解析结果按 Realm 隔离；货币集合仅反映该 Realm 内启用映射行。
- **兼容性要求**：项目未上线，偏好货币承载为破坏性新增，不做回填/灰度；货币暴露为纯增量字段，不破坏既有 api-ext 消费者；与 Stripe/Creem/IAP/WeChat Pay、积分账本、订阅系统的详细契约下沉到技术设计。

---

## 7. 前端/交互约束

**适用性**: 适用

- **管理入口**：Realm 设置中提供 Realm 默认货币的设置与更新（Realm Admin）；非法货币码在表单校验阶段被拒绝。
- **用户自助入口**：个人设置中提供偏好货币的设置与清除（Regular User）；保存后立即影响本人后续解析。
- **购买页（Stripe 多 Price 产品）**：按货币分组展示价格行，偏好货币组高亮并默认展开；同货币组内列出可选计费周期/类型；提供货币切换；偏好货币不可用时提示「当前产品不支持您的偏好货币」并允许手动选择其他可用货币。
- **购买页（Creem / IAP / WeChat Pay 等 provider 侧定价渠道）**：降级为单一价格展示，不渲染货币切换器与货币分组；偏好货币不影响其展示价格。
- **状态反馈**：偏好货币保存成功/失败、货币码非法、偏好货币不可用、解析失败（fail-loud）的明确反馈；Stripe Adaptive Pricing 场景下对基础货币做标注，说明实际扣款以支付页为准。
- **权限可见性**：Realm 默认货币入口仅 Realm Admin 可见；用户覆盖货币入口仅本人可见；终端用户在购买页看到的货币集合取决于该 Realm 已启用的映射行。
- **金额/积分变化**：货币切换与购买场景必须突出所选货币的金额变化与不可逆风险提示。

---

## 8. 已确认决策

> 以下决策来自决策账本 `.ai/decision-log/multiple-currency.md`。仅记录带稳定 DEC ID 的已确认结论。

| Decision ID | 状态 | 决策项 | 结论 | PRD 落点 | 来源 |
|---|---|---|---|---|---|
| `DEC-multiple_currency-001` | Applied | 范围路线 | 新增「按货币选择/本地化体验」功能，构建于现有多 Price 目录；非「仅确认现状」、非「改走 currency_options」 | §2.1、§3 | `.ai/decision-log/multiple-currency.md` |
| `DEC-multiple_currency-002` | Applied | 目录模型 | 货币选择层基于现有「一映射行对应一个 provider Price」模型，不改同步、不读 `currency_options` | §2.2、§4.1、§6 | 同上 |
| `DEC-multiple_currency-003` | Applied | 渠道覆盖 | 仅 Stripe 多 Price 纳入货币解析；Creem/IAP 等 provider 侧定价渠道保持渠道侧定价，降级为单一价格展示 | §2.1、§2.2、§4.1、§7 | 同上 |
| `DEC-multiple_currency-004` | Applied | 偏好层级 | 偏好货币两级：Realm 默认 + 用户覆盖（用户优先）；不设 Client App 级 | §2.1、§4.1、§7 | 同上 |
| `DEC-multiple_currency-005` | Applied | 解析键 | 解析键 = (产品/权益 + 计费维度 + 货币)；货币为过滤维度，非唯一键；同货币多计费周期并存时由用户在货币内选周期 | §4.1、§5.1、§7 | 同上 |
| `DEC-multiple_currency-006` | Applied | 缺失回退 | 展示侧：始终展示全部可用货币并按 偏好→Realm 默认→首个可用 依次高亮，允许手动切换；程序化默认解析侧：仅按生效偏好货币（用户覆盖，无覆盖时即 Realm 默认）解析一次，无二级回退，无匹配即 fail-loud | §2.1、§4.1、§4.2、§5.1、§5.2 | conversation（/t-prd-check AskUserQuestion 2026-08-13） |
| `DEC-multiple_currency-007` | Applied | 偏好承载点 | Realm 默认货币承载于 Realm 设置（realm_config），用户覆盖货币承载于用户个人资料（profile） | §2.3、§7 | `.ai/decision-log/multiple-currency.md` |
| `DEC-multiple_currency-008` | Applied | 程序化解析暴露 | 程序化默认解析经 api-ext 暴露（货币集合聚合 + 按货币解析默认价格行，fail-loud）；终端用户购买始终显式选定价格行，货币分组由前端完成 | §4.1、§5.1 | 同上 |
| `DEC-multiple_currency-009` | Applied | 缺价 fail-loud | Stripe 映射行缺失价格信息时拒绝下单（fail-loud），不产生零金额/串货币支付；显式 `target_id` 路径行为不变 | §4.1、§4.2、§5.1 | 同上 |
| `DEC-multiple_currency-010` | Applied | 货币码校验 | `^[A-Z]{3}$` 格式 + 拒绝 ISO 4217 保留码（`XXX`/`XTS`）；非法码设置被拒，前后端共用规则 | §4.1、§5.1、§7 | 同上 |
| `DEC-multiple_currency-011` | Applied | 权限门控 | Realm 默认货币写 `settings.manage`/读 `settings.view`；用户覆盖为认证用户 profile 自助；api-ext 货币/解析端点 `billing.view` | §2.3、§6 | 同上 |
| `DEC-multiple_currency-012` | Applied | 大小写归一 | 货币码匹配不区分 ASCII 大小写（目录存储 provider 原生码）；偏好写入与对外暴露统一大写 ISO 码 | §4.1 | 同上 |
| `DEC-multiple_currency-013` | Applied | fail-loud 范围 | 缺价 fail-loud 仅对 Stripe 映射行生效；provider/store 侧定价渠道（apple/google/wechat/creem）缺价信息属合法状态，不视为异常 | §4.1、§4.2、§5.1、§7 | 同上 |

---

## 9. 参考资料

- 用户故事（多货币特有）：`docs/user-stories/billing/multiple-currency.md`（US-MC-001～006）
- 用户故事（多价格基线）：`docs/user-stories/billing/entitlement-mapping.md`
- 用户故事（统一支付尝试/履约）：`docs/user-stories/billing/payment-attempt.md`
- 相关 PRD：`docs/prd/billing/subscription.md`（订阅计费/多价格目录基线）
- 相关 PRD：`docs/prd/billing/stripe-payment.md`（Stripe 集成）
- 相关 PRD：`docs/prd/billing/credit-bucket.md`（积分账户与履约路由）
- 决策账本：`.ai/decision-log/multiple-currency.md`
- 角色定义：`docs/user-stories/_roles.md`
