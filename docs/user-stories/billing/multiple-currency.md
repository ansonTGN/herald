# 多货币（按货币选择/本地化）用户故事

> 角色定义以 `docs/user-stories/_roles.md` 为准。
> 复用的已发布用户故事（不重复创建）：
> - 多价格同步/购买/解析：`docs/user-stories/billing/entitlement-mapping.md`（US-EM-007～009、US-BL-SYNC-001～004）
> - 统一支付尝试与履约：`docs/user-stories/billing/payment-attempt.md`（US-PA-001～004）
> 本文件仅承载「按货币选择/本地化体验」特有的场景。

## 用户故事

### 故事 1：配置 Realm 默认偏好货币 [US-MC-001]

**优先级**: P0

> 与既有 Realm 设置旅程并列；本故事聚焦「为整个 Realm 设置购买解析时的默认货币」。用户级覆盖见 [US-MC-002]。

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：为本 Realm 设置一个默认货币
**从而**：未单独设置偏好货币的用户在购买时，系统按 Realm 默认货币解析并高亮合适的价格行

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：设置 Realm 默认货币**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 尚未设置默认货币
When 我在 Realm 设置中填写默认货币（ISO 货币码，如 USD、EUR、CNY）并保存
Then 设置保存成功
And 该默认货币对本 Realm 内未设置个人偏好货币的所有用户生效
```

**场景 2：更新 Realm 默认货币立即影响后续解析**
```gherkin
Given realm-1 的默认货币为 USD
When 我将默认货币改为 EUR 并保存
Then 此后未设个人偏好货币的用户在购买时按 EUR 解析与高亮
And 已设置个人偏好货币的用户不受影响
```

**场景 3：非法货币码被拒绝**
```gherkin
Given 我在设置默认货币
When 我填写了非 ISO 4217 标准的货币码（如 "美元" 或 "XXX"）
Then 系统拒绝保存并提示货币码无效
And 原设置保持不变
```

---

### 故事 2：设置个人偏好货币覆盖 [US-MC-002]

**优先级**: P1

> 用户级偏好优先于 Realm 默认。

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：设置只属于我个人的偏好货币，覆盖 Realm 默认货币
**从而**：购买页与默认解析按我最熟悉的货币展示与选价，而不受 Realm 默认限制

**【验收标准】**

**场景 1：设置个人偏好货币覆盖 Realm 默认**
```gherkin
Given 我是 realm-1 的用户
And realm-1 的默认货币为 USD
When 我在个人设置中将偏好货币设为 CNY 并保存
Then 此后我在购买页看到的价格行优先按 CNY 高亮与解析
And 其他用户的解析不受我的设置影响
```

**场景 2：清除个人偏好回退到 Realm 默认**
```gherkin
Given 我已将个人偏好货币设为 CNY
When 我清除个人偏好货币设置
Then 我的解析回退到 Realm 默认货币（USD）
```

**场景 3：非法货币码被拒绝**
```gherkin
Given 我在设置个人偏好货币
When 我填写了非 ISO 4217 标准的货币码
Then 系统拒绝保存并提示货币码无效
```

---

### 故事 3：购买页按货币分组并高亮偏好货币 [US-MC-003]

**优先级**: P0

> 仅对具备多个货币价格行的 Stripe 多 Price 产品生效；Creem / IAP / WeChat Pay 等渠道的降级展示见 [US-MC-006]。同货币下多计费周期并存时，用户仍须在货币组内选择计费周期/类型。

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：在购买页按货币分组看到某个产品的所有价格，并能切换货币与计费周期
**从而**：用我最方便的货币完成购买，同时仍可在该货币下选择月付/年付等周期

**【验收标准】**

**场景 1：按货币分组并高亮偏好货币**
```gherkin
Given 一个 Stripe 产品配置了 USD/月、USD/年、EUR/月、CNY/月 四个价格行
And 我的偏好货币为 CNY
When 我打开该产品的购买页
Then 购买页按货币（CNY / USD / EUR）分组展示价格行
And CNY 组被高亮并默认展开
And USD 组内同时列出月付与年付两个可选周期
```

**场景 2：手动切换货币**
```gherkin
Given 购买页已按我的偏好货币 CNY 分组展示
When 我手动切换到 EUR
Then 购买页高亮 EUR 组
And 我选择 EUR 组内的价格行后按 EUR 发起购买
```

**场景 3：偏好货币缺失时仍展示全部可用货币**
```gherkin
Given 一个 Stripe 产品只有 USD/月、EUR/月 两个价格行
And 我的偏好货币为 CNY
When 我打开该产品的购买页
Then 购买页仍展示 USD 与 EUR 两组价格行
And 提示我的偏好货币（CNY）在该产品不可用
And 允许我手动选择 USD 或 EUR 发起购买
```

---

### 故事 4：按偏好货币解析价格行发起购买 [US-MC-004]

**优先级**: P0

> 货币解析以 (产品/权益 + 计费维度 + 货币) 共同定位唯一价格行。
> 回退行为：展示侧始终展示全部可用货币并高亮 偏好→Realm 默认→首个可用；程序化默认解析仅按生效偏好货币（用户覆盖，无覆盖时即 Realm 默认）解析一次，无二级回退，无匹配即 fail-loud（不静默换币、不零金额下单）。

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：当我按偏好货币选择某个计费周期发起购买时，系统解析到正确货币的价格行下单
**从而**：我被扣款的货币与我在购买页看到的一致，不会发生串货币或零金额下单

**【验收标准】**

**场景 1：偏好货币命中，按所选计费周期解析下单**
```gherkin
Given 一个 Stripe 产品配置了 USD/月、USD/年、EUR/月 价格行
And 我的偏好货币为 USD
When 我在购买页 USD 组内选择「年付」并发起购买
Then 系统解析到 USD/年那一行价格
And 我进入支付时被扣款的货币为 USD、金额为该价格行金额
```

**场景 2：同货币多计费周期不可仅凭货币定位（fail-loud）**
```gherkin
Given 一个 Stripe 产品在 USD 下同时有月付与年付两个价格行
When 系统在未指定计费周期的情况下仅凭「USD」尝试默认解析
Then 系统不静默选择其中任一行
And 解析失败并给出明确错误（fail-loud）
And 不发起任何支付
```

**场景 3：偏好货币无匹配时的程序化默认解析（fail-loud）**
```gherkin
Given 一个 Stripe 产品只有 EUR/月 一个价格行
And 我的偏好货币为 CNY
When 第三方应用仅凭我的偏好货币（不指定具体价格行）请求默认 checkout
Then 系统仅按 CNY 解析且未命中价格行
And 系统拒绝默认解析并返回明确错误（fail-loud）
And 不回退到 Realm 默认或其他货币，不以 EUR 静默替代，不发起零金额或跨币下单
```

---

### 故事 5：查询可购权益支持的货币集合 [US-MC-005]

**优先级**: P0

> 服务于第三方应用/SDK 的货币切换与分组集成（api-ext 暴露「可购权益 → 支持货币集合」）。

**【用户故事】**
**作为**：Third-party App（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查询某个可购权益（entitlement）支持哪些货币，并能为用户按货币解析默认价格行
**从而**：在我的应用内为用户提供货币切换，并按所选货币发起正确的购买

**【验收标准】**

**场景 1：获取权益支持的全部货币**
```gherkin
Given 一个 entitlement 对应的启用映射行覆盖了 USD、EUR、CNY 三种货币
When 第三方应用查询该权益的可购信息
Then 返回结果中包含该权益支持的全部货币集合（USD、EUR、CNY）
And 订阅类与一次性购买类映射都暴露其货币
```

**场景 2：按货币解析默认价格行（命中）**
```gherkin
Given 第三方应用已取得某权益支持的货币集合
And 用户的偏好货币为 EUR
When 第三方应用请求按 EUR 解析该权益的默认价格行
And 该权益在 EUR 下存在唯一价格行（或同时指定了计费维度）
Then 系统返回 EUR 对应的价格行
And 第三方应用据此引导用户完成 EUR 购买
```

**场景 3：按货币解析无匹配（fail-loud）**
```gherkin
Given 某权益仅支持 USD 与 EUR
And 用户的偏好货币为 CNY
When 第三方应用仅凭 CNY 请求默认解析
Then 系统仅按 CNY 解析且未命中价格行
And 系统返回明确错误（fail-loud）
And 不回退到 Realm 默认（USD）或其他货币，不静默选其他货币
```

---

### 故事 6：Creem / IAP / WeChat Pay 单一价格降级展示 [US-MC-006]

**优先级**: P2

> Creem 为产品级单一价格、无每货币价格对象；IAP 价格由商店按地区管理，Herald 只验凭证；WeChat Pay 由商户在渠道侧定价。provider/store 侧定价渠道均不纳入 Herald 货币解析。

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：购买由 Creem、IAP 或 WeChat Pay 提供的产品时，看到的是与该渠道一致的单一价格
**从而**：不被无意义的货币切换器误导，也不会因偏好货币解析失败而无法购买

**【验收标准】**

**场景 1：Creem 产品不显示货币切换器**
```gherkin
Given 一个由 Creem 提供的产品（产品级单一价格、无多货币价格行）
When 我打开该产品的购买页
Then 购买页只展示 Creem 提供的单一价格
And 不渲染货币切换器或货币分组
And 我的偏好货币不影响该产品的展示价格
```

**场景 2：IAP 产品按商店地区价格展示**
```gherkin
Given 一个由 IAP（Apple/Google）提供的产品
When 我打开该产品的购买页
Then 购买页按应用商店返回的当前地区价格展示
And Herald 不对该产品做货币解析或货币切换
```

**场景 3：WeChat Pay 产品按渠道侧定价单一展示**
```gherkin
Given 一个由 WeChat Pay 提供的产品（价格由商户在渠道侧配置）
When 我打开该产品的购买页
Then 购买页只展示该渠道配置的单一价格
And 不渲染货币切换器或货币分组
And 我的偏好货币不影响该产品的展示价格
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 4 | US-MC-001 配置 Realm 默认货币、US-MC-003 购买页按货币分组并高亮、US-MC-004 按偏好货币解析下单、US-MC-005 查询可购权益货币集合 |
| P1 | 1 | US-MC-002 设置个人偏好货币覆盖 |
| P2 | 1 | US-MC-006 Creem/IAP/WeChat Pay 单一价格降级展示 |

---

## 相关文档

- **PRD**: `docs/prd/billing/multiple-currency.md`
- **多价格同步/购买/解析基线**: `docs/user-stories/billing/entitlement-mapping.md`（US-EM-007～009、US-BL-SYNC-001～004）
- **统一支付尝试与履约**: `docs/user-stories/billing/payment-attempt.md`（US-PA-001～004）
- **订阅计费 PRD**: `docs/prd/billing/subscription.md`
- **Stripe 支付 PRD**: `docs/prd/billing/stripe-payment.md`
- **WeChat Pay PRD**: `docs/prd/billing/wechat-support.md`（渠道侧定价渠道参考）
