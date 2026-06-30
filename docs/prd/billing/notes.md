# Credit Note（发票贷记凭证）产品需求文档 (PRD)

**创建时间**: 2026-06-29
**优先级**: P0
**所属域**: billing

---

## 1. 相关用户故事

> 已发布故事详见 `docs/user-stories/billing/invoice-fallback.md`。

### 1.1 相关故事

- `[US-IF-007]` 系统同步 Stripe Credit Note，优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Herald 系统
  - 摘要：通过 Stripe `credit_note.created` webhook 同步 Credit Note 数据，更新关联发票的退款金额与剩余应付

- `[US-IF-008]` 管理员查看发票退款信息与 Credit Note 列表，优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Realm Admin
  - 摘要：在发票详情中查看累计退款金额、剩余应付与只读 Credit Note 列表（覆盖 Stripe 与 Manual 两种来源）

- `[US-IF-009]` 普通用户查看退款标注，优先级 P1，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Regular User
  - 摘要：在"我的发票"中看到自己已退款发票的退款标注与剩余应付

- `[US-IF-010]` 管理员记录自研发票的线下退款，优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Realm Admin
  - 摘要：为已付款的 Herald 自研发票创建 Manual Credit Note，记录线下退款，保留税务合规凭证

- `[US-IF-011]` 系统处理 Stripe Credit Note 作废，优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Herald 系统
  - 摘要：通过 Stripe `credit_note.voided` webhook 同步作废状态，恢复关联发票的剩余应付

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 4 | 同步 Stripe Credit Note、查看退款信息、记录自研发票退款、处理 Stripe Credit Note 作废 |
| P1 | 1 | 普通用户查看退款标注 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- **Stripe Credit Note 同步**：处理 Stripe `credit_note.created` 与 `credit_note.voided` webhook 事件，在 Herald 本地记录 Credit Note 并更新关联发票（`provider=stripe`）的累计退款金额与剩余应付
- **Manual Credit Note 创建**：Realm Admin 可为已付款的 Herald 自研发票（`provider=manual`）创建 Manual Credit Note，记录线下退款
- **Credit Note 生命周期**：支持 `active`（有效）与 `voided`（已作废）两种状态；作废后关联发票的退款金额回滚、剩余应付恢复
- 支持部分退款：同一发票可关联多张 Credit Note，累计退款金额不超过发票总额（Stripe 与 Manual 两侧均适用）
- 在发票详情页展示总额、已退款金额、剩余应付与只读 Credit Note 列表（包含 Stripe 来源与 Manual 来源）
- 在管理员与用户的发票列表行上展示退款标注，主状态显示保持不变
- 保留现有 `charge.refunded` 事件积分回收逻辑（与 Credit Note 处理相互独立、不重复执行）
- 保留现有 `invoice.voided` 事件同步逻辑
- 保留幂等保证：重复 `credit_note.created` / `credit_note.voided` 事件不产生重复记录、不重复累加或回滚退款金额

### 2.2 不包含功能 (Out of Scope)

**Creem 侧（基于 MoR 边界排除）**：

- **Creem 退款不触发 Credit Note 同步**：Creem 作为 **Merchant of Record (MoR)**，税务合规由 Creem 负责；Herald 无需为 Creem 退款维护发票层面的退款凭证
- **Creem `refund.created` 事件无发票数据**：payload 仅含 transaction 级数据，不携带 invoice 或 credit note 信息，无法做 invoice 关联
- **Creem 发票不展示退款维度**：`provider=creem` 的发票详情页不显示"已退款 / 剩余应付"区域；积分回收沿用现有 `charge.refunded` 处理逻辑

**Herald 不主动操作 Stripe 侧**：

- Herald 不暴露发起 Stripe 退款、开具 Stripe Credit Note、作废 Stripe 发票的能力；这些操作在 Stripe Dashboard 完成，Herald 仅通过 webhook 同步结果

**Manual Credit Note 边界**：

- **Manual Credit Note 不可删除/撤销**：一旦创建即作为永久凭证保留；如记录错误，需通过线下补偿或客服渠道处理，本 PRD 不提供撤销机制
- **Manual Credit Note 仅作用于 `provider=manual` 发票**：对 Stripe / Creem 发票创建 Manual Credit Note 一律拒绝
- **仅 `paid` 状态可记录退款**：草稿、已开具（未付款）、已作废的发票不可创建 Manual Credit Note
- **Manual Credit Note 不影响发票主状态**：发票主状态保持 `paid`，不引入新的状态值；自研发票的 `void` 状态机保持不变

**其他**：

- 退款金额的币种转换：退款币种与发票币种一致；Herald 不做跨币种换算
- Credit Note PDF 渲染：Stripe Credit Note 的 PDF 由 Stripe 生成；Manual Credit Note 在本 PRD 不生成独立 PDF，其数据通过发票详情页与发票 PDF 的"已退款 / 剩余应付"摘要展示
- 税务申报自动生成：本 PRD 只保证数据完整展示，不直接产出税务申报文件
- 争议（Dispute）与退款的合并处理：争议走现有 `charge.dispute.*` 链路，与本 PRD 的退款链路独立

### 2.3 依赖项

- **Stripe 支付集成**（`docs/prd/billing/stripe-payment.md`）— 复用 Stripe webhook 处理框架、签名验证、幂等机制
- **现有发票系统**（`docs/prd/billing/invoice.md`）— 复用外部发票同步表、自研发票状态机、只读展示约束、provider 标识
- **积分回收规则**（`docs/prd/billing/points.md`）— 现有 `charge.refunded` 的 topup / subscription 积分回收策略保持不变
- **Webhook 补偿机制**（`docs/prd/billing/webhook-compensation.md`）— `credit_note.created` 与 `credit_note.voided` 事件需纳入补偿 Job 的事件清单，确保 webhook 缺失时通过 Events API 补处理
- **现有权限模型**（`docs/prd/billing/invoice.md` §6）— 复用 `billing.view` / `billing.manage` 权限，不新增发票细粒度权限

---

## 3. 需求概述

### 3.1 功能描述

Herald 的发票有三种来源（`provider`）：**Stripe 外部同步**、**Creem MoR 同步**、**Herald 自研（manual）**。三种来源在退款凭证处理上的边界由"谁负责税务合规"决定：

**Stripe 外部发票**：Stripe 中 Refund 与 Invoice 是独立对象；对 Charge 执行退款不自动更新关联 Invoice。完整合规流程需要三层操作：
1. **Refund**（Charge 层）→ `charge.refunded` — Herald 用于积分回收
2. **Credit Note**（Invoice 层）→ `credit_note.created` / `credit_note.voided` — Herald **记录发票层面的退款凭证**，更新发票的累计退款金额与剩余应付
3. **Void Invoice**（可选）→ `invoice.voided` — Herald 现有状态同步能力

Credit Note 对税务合规至关重要：缺少 Credit Note 时，原发票仍以完整金额存在，税务机关会将其视为完整有效收入；Credit Note 作废后，发票剩余应付应同步恢复，避免 Herald 与 Stripe Dashboard 金额不一致。

**Creem MoR 发票**：Creem 作为 Merchant of Record，**税务合规由 Creem 负责**。Creem 退款仅触发 `refund.created`（transaction 级），无 Credit Note 概念。Herald 仅做积分回收，不在 invoice 层面跟踪退款。

**Herald 自研发票（manual）**：Herald（Realm Admin）是发票的开具方，也是线下退款的发起方。当前 `paid` 状态为真终态，无法记录线下退款，存在税务合规空白。本 PRD 引入 **Manual Credit Note**：由 Realm Admin 主动创建，与 Stripe Credit Note 数据语义对齐（amount / currency / reason / operator / timestamp），支持部分退款，原发票主状态保持 `paid`。Manual Credit Note 同样不可撤销，创建后作为永久凭证。

### 3.2 关键特性

- **三层职责分离**（Stripe 侧）：`charge.refunded`（积分回收）/ `credit_note.created`（发票金额扣减）/ `credit_note.voided`（发票金额回滚）/ `invoice.voided`（发票状态作废）各自独立处理
- **MoR 边界驱动 Creem 排除**：Creem 的税务合规由 Creem 负责，Herald 不在 invoice 层面跟踪 Creem 退款
- **双轨 Credit Note 模型**：Stripe Credit Note（被动同步）与 Manual Credit Note（主动创建）共享相同的展示语义（累计已退款 / 剩余应付），但创建路径不同
- **Credit Note 生命周期**：`active` 表示退款已生效；`voided` 表示 Stripe 端作废，Herald 同步回滚对应金额
- **保留原发票**：Credit Note 不作废发票，而是叠加"已退款 / 剩余应付"维度，发票主状态保持不变
- **部分退款支持**：一张发票可有多张 Credit Note，累计退款金额上限为发票总额
- **Manual Credit Note 不可撤销**：创建即永久凭证，避免财务数据被覆盖

---

## 4. 业务规则与状态

### 4.1 业务规则

**核心事实（必须作为后续设计与实现的判断基础）**：

- Stripe 中 Refund 与 Invoice 是独立对象；对 Charge 执行退款不自动更新关联 Invoice
- Stripe 完整合规退款流程需要在 Charge 层执行 Refund、在 Invoice 层开具 Credit Note
- Credit Note 减少发票的剩余应付，不改变发票主状态、不作废发票
- Credit Note 被作废后，其金额应从发票累计退款中扣除，剩余应付相应恢复
- Creem 作为 MoR，其退款不涉及 invoice 层面的合规凭证；Herald 不为 Creem 维护 Credit Note
- 自研发票由 Herald 开具，线下退款也由 Herald 记录；Manual Credit Note 是其唯一的退款凭证机制

**事件职责矩阵（Stripe 侧）**：

| Stripe 事件 | Herald 处理职责 | 影响范围 | 备注 |
|------|------|------|------|
| `charge.refunded` | 积分回收（topup 按比例、subscription 回收未使用） | 用户积分余额 | 现有，保留不变 |
| `credit_note.created` | 记录 Stripe Credit Note（active）并更新关联发票退款金额维度 | 发票的退款数据 | 新增同步 |
| `credit_note.voided` | 将已存在的 Stripe Credit Note 置为 voided，并回滚关联发票退款金额 | 发票的退款数据 | 新增同步 |
| `invoice.voided` | 发票主状态映射为 `void` | 发票主状态 | 现有，保留不变 |

**Provider 边界矩阵**：

| Provider | 退款处理路径 | Credit Note 来源 | 发票退款维度展示 |
|------|------|------|------|
| `stripe` | `charge.refunded`（积分）+ `credit_note.created/voided`（金额） | Stripe webhook 同步（被动） | 展示 |
| `creem` | `refund.created`（仅积分） | 不适用（MoR 边界） | **不展示** |
| `manual` | Realm Admin 手动创建 Manual Credit Note | Admin UI 主动创建 | 展示 |

**Credit Note 生命周期规则**：

- 新创建的 Stripe Credit Note 默认状态为 `active`
- 新创建的 Manual Credit Note 默认状态为 `active`
- 收到 Stripe `credit_note.voided` 事件时，将对应 Credit Note 状态更新为 `voided`，并反向调整关联发票的累计退款金额与剩余应付
- 已被 voided 的 Credit Note 再次收到 `credit_note.voided` 事件时幂等，不重复回滚金额
- Manual Credit Note 一旦创建即永久 `active`，不提供任何撤销或作废入口

**Stripe Credit Note 同步规则**：

- 收到 `credit_note.created` 事件时，按事件 payload 中的 Invoice 标识定位本地发票记录
- 若本地存在该发票且 `provider=stripe`：在本地创建状态为 `active` 的 Credit Note 记录，包含 Credit Note 编号、金额、币种、开具时间、关联发票标识
- 同步更新关联发票的累计退款金额与剩余应付
- 发票主状态保持不变（如 `paid` 仍为 `paid`），不引入新的状态值
- 累计退款金额不得超过发票总额；超过时记录 Error 日志（与 webhook-compensation 一致策略），不自动修复
- 若本地不存在关联发票（例如发票同步尚未完成）：记录 Warn 日志并跳过，等待 `invoice.*` 与 `credit_note.created` 通过补偿 Job 在下一周期对齐

- 收到 `credit_note.voided` 事件时，按 Credit Note 标识定位本地 Credit Note 记录
- 若本地存在该 Credit Note 且状态为 `active`：将其置为 `voided`，并回滚关联发票的累计退款金额与剩余应付
- 若本地不存在该 Credit Note：返回错误以触发 Stripe 重投递，不创建孤儿记录
- 已被 voided 的 Credit Note 再次收到 voided 事件时幂等，不重复回滚

**Manual Credit Note 创建规则**：

- 仅 `provider=manual` 且状态为 `paid` 的发票可创建 Manual Credit Note
- 创建时必须填写：金额（正整数，最小货币单位）、原因（memo，自由文本）
- 创建后系统记录：金额、币种、原因、操作者（admin user_id）、操作时间
- 单笔金额不得超过该发票的剩余应付
- 累计退款金额不得超过发票总额
- 创建后立即生效，不可删除、不可修改、不可撤销
- 发票主状态保持 `paid` 不变

**幂等规则**：

- Stripe Credit Note 以 Stripe Credit Note ID 为唯一键；重复 `credit_note.created` 事件更新已有记录而非创建
- 重复 `credit_note.voided` 事件对已 voided 记录幂等，不重复回滚金额
- Manual Credit Note 由 admin 显式创建，无幂等问题；同一 admin 可对同一发票创建多张不同金额的 Manual Credit Note（部分退款场景）
- 复用现有 Stripe webhook 幂等机制（以 Stripe Credit Note ID 为唯一键，仅 Stripe 侧）

**与积分回收的解耦**：

- `charge.refunded` 与 `credit_note.*` 是两个独立事件，分别由独立的 handler 处理
- 积分回收仅在 `charge.refunded` 中执行；`credit_note.created` / `credit_note.voided` 不重复执行积分回收
- Manual Credit Note 创建不触发积分回收——自研发票的"线下退款"是否回收积分是另一独立决策，本 PRD 不引入积分回收联动

**展示规则**：

- 发票详情页（管理员与用户）在已退款金额大于 0 时展示"已退款 / 剩余应付"区域，适用于 `provider=stripe` 与 `provider=manual`
- `provider=creem` 发票**不展示**该区域
- 发票列表行在已退款时展示退款标注（如 "Refunded 30/100"），主状态显示保持原值
- Credit Note 列表仅管理员可见，且为只读；普通用户只看到退款摘要，不暴露 Credit Note 内部编号
- Credit Note 列表显示来源标识（Stripe / Manual）、状态（active / voided），便于管理员识别凭证类型

### 4.2 关键状态与异常

- **发票主状态**：不新增状态值；现有 `draft / issued / paid / void / overdue` 保持不变
- **退款维度**：在 `paid` 状态上叠加"已退款 / 剩余应付"两个派生展示维度；这些维度不影响主状态
- **Credit Note 与 Void Invoice 的关系**：Void Invoice 会将发票置为 `void`；若同一发票同时存在 Credit Note，Void 之后 Credit Note 记录仍保留为审计凭证
- **Manual Credit Note 与 Void 的关系**：Manual Credit Note 创建后，发票不可被作废——`paid` 仍是真终态，本 PRD 不放宽 void 状态机。若发票已有 active Credit Note，admin 不能再 void 该发票（避免与已记录的退款凭证冲突）
- **关联发票缺失（Stripe 侧）**：补偿 Job 或正常 webhook 收到 `credit_note.created` 但本地无对应发票时，记录 Warn 日志并跳过；不创建孤儿 Credit Note 记录
- **Credit Note 不存在（Stripe voided 侧）**：收到 `credit_note.voided` 但本地无对应记录时，返回错误以触发 Stripe 重投递
- **金额越界**：累计退款金额超过发票总额时，记录 Error 日志（Stripe 侧）或拒绝创建（Manual 侧）
- **provider 不匹配**：
  - Stripe 侧：`credit_note.created` 关联的本地发票若为 `manual` 或 `creem`，记录 Warn 日志并跳过
  - Manual 侧：对 `stripe` 或 `creem` 发票创建 Manual Credit Note 一律拒绝并提示 "Refunds for this provider are managed externally"
- **权限边界**：管理员端通过现有 `billing.view` / `billing.manage` 控制（创建 Manual Credit Note 需 `billing.manage`）；用户端复用登录用户身份判断；Credit Note 列表仅管理员可见

---

## 5. 功能需求

### 5.1 核心需求

- **Stripe Credit Note Webhook 同步**：Herald 接收并处理 Stripe `credit_note.created` 与 `credit_note.voided` 事件，在本地创建或更新只读 Credit Note 记录并更新关联发票（`provider=stripe`）的累计退款金额与剩余应付
- **Manual Credit Note 创建 API**：Herald 提供管理员能力，为已付款的 `provider=manual` 发票创建 Manual Credit Note；写入金额、币种、原因、操作者
- **Credit Note 作废回滚**：Stripe `credit_note.voided` 事件触发时，将对应 Credit Note 置为 voided 并恢复发票剩余应付
- **部分退款支持**：同一发票允许多张 Credit Note 累加（Stripe 与 Manual 两侧均适用），退款金额维度持续更新；不超过发票总额
- **发票详情退款展示（管理员）**：管理员发票详情页展示总额、已退款、剩余应付与只读 Credit Note 列表（覆盖 Stripe 与 Manual 来源，含状态）；`provider=creem` 不展示
- **发票详情退款展示（用户）**：用户发票详情页展示总额、已退款、剩余应付；不暴露 Credit Note 内部编号；`provider=creem` 不展示
- **列表退款标注**：发票列表行在已退款时展示退款标注，主状态显示保持不变；`provider=creem` 不展示
- **幂等保证**：复用现有 Stripe webhook 幂等机制，重复 webhook 不产生副作用
- **补偿 Job 集成**：将 `credit_note.created` 与 `credit_note.voided` 纳入 webhook 补偿 Job 的事件清单，确保 webhook 缺失时通过 Stripe Events API 补处理
- **日志可观测**：关联缺失、Credit Note 不存在、provider 不匹配、金额越界等异常以 Warn / Error 日志记录，便于后续诊断

### 5.2 验收目标

- Stripe 退款 + 开具 Credit Note 的完整流程后，Herald 中关联发票（`provider=stripe`）的退款金额与剩余应付与 Stripe Dashboard 一致
- Stripe 端作废 Credit Note 后，Herald 同步将对应 Credit Note 置为 voided，并正确回滚发票累计退款金额与剩余应付
- 部分退款场景下，多张 Credit Note 正确累加；剩余应付 = 总额 - 累计退款金额
- 发票主状态在 Credit Note 同步或作废后保持 `paid` 不变
- 重复 webhook 不导致退款金额重复累加或重复回滚
- 管理员能为已付款的自研发票（`provider=manual`）创建 Manual Credit Note，记录线下退款
- Manual Credit Note 创建时金额超过剩余应付被拒绝；累计退款金额不超过发票总额
- 对 `provider=stripe` 或 `provider=creem` 的发票创建 Manual Credit Note 被拒绝
- Manual Credit Note 创建后不可删除、不可修改
- 管理员能在发票详情看到 Credit Note 列表（含来源标识 Stripe / Manual 与状态 active / voided），普通用户只看到退款摘要
- `provider=creem` 的发票详情页与列表行不展示退款维度
- Webhook 缺失时，补偿 Job 能通过 Stripe Events API 补处理 `credit_note.created` / `credit_note.voided`
- 异常场景（关联缺失、Credit Note 不存在、provider 不匹配、金额越界）有对应日志记录，不阻塞其他事件处理

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力范围**：
  - 新增管理员侧 Manual Credit Note 创建能力（`provider=manual` 发票专用）
  - 现有发票详情/列表响应扩展退款金额维度与 Credit Note 列表字段（只读）
  - Stripe Credit Note 通过 webhook 被动同步，不暴露对外 CRUD 接口
- **访问控制原则**：
  - 创建 Manual Credit Note 需 `billing.manage` 权限
  - 读取 Credit Note 列表与退款维度需 `billing.view` 权限（管理员侧）；用户侧复用登录用户身份判断，仅能查看自己的发票
  - 用户侧响应不暴露 Credit Note 内部编号
- **租户/Realm 数据边界**：Credit Note 与关联发票共享同一 Realm 隔离边界；查询与创建必须带 `realm_id` 条件
- **状态操作约束**：
  - 现有发票 CRUD API 对 `provider != manual` 的发票禁止写操作（创建/编辑/开具/作废/标记已付），本约束在 Credit Note 存在时同样生效
  - Manual Credit Note 创建能力对发票状态为非 `paid` 一律拒绝
  - Manual Credit Note 创建能力对发票 `provider != manual` 一律拒绝
  - 已存在 active Credit Note 的 `provider=manual` 发票不得被作废（避免与已记录的退款凭证冲突）
- **兼容性要求**：现有发票 API 响应向后兼容，新增的退款金额维度与 Credit Note 列表为可选字段；不修改现有接口契约
- **Webhook 处理边界**：在 billing 域内新增 `credit_note.created` / `credit_note.voided` 处理逻辑，签名验证、幂等检查、日志规范复用现有 webhook 框架

---

## 7. 前端/交互约束

**适用性**: 适用

- **管理后台**（Realm Admin）：
  - 入口：复用现有 `/$realmId/manage/billing/invoices` 页面，不新增独立菜单
  - 发票列表：行内金额列旁展示退款标注（如 "Refunded 30/100"），主状态显示保持原值；`provider=creem` 行不展示标注
  - 发票详情页：在费用汇总区域新增"已退款 / 剩余应付"两行展示，无退款或 `provider=creem` 时不显示
  - Credit Note 列表：作为详情页的子区域展示，已退款且 `provider in (stripe, manual)` 时显示；每条 Credit Note 显示：编号、来源标识（Stripe / Manual）、状态（active / voided）、开具时间、金额、币种、原因（Manual 才有）、操作者（Manual 才有），只读
  - **Record Refund 按钮**：仅在 `provider=manual` 且 `status=paid` 的发票详情页显示；点击后弹窗填写金额、原因（memo，必填），确认后创建 Manual Credit Note
  - 退款金额异常（如累计超过总额）时显示告警提示，引导管理员通过 Stripe Dashboard 核对或检查 Manual Credit Note 记录

- **个人页面**（Regular User）：
  - 入口：复用现有 `/$realmId/user/invoices` 页面
  - 我的发票列表：行内金额列旁展示退款标注（金额摘要形式）；`provider=creem` 行不展示
  - 我的发票详情：展示"已退款 / 剩余应付"；**不展示 Credit Note 列表与内部编号**；`provider=creem` 不展示
  - 用户侧无任何创建/修改 Credit Note 的入口

- **状态反馈**：
  - Manual Credit Note 创建成功后立即刷新发票详情，展示新的退款维度与 Credit Note 列表
  - 创建失败（金额越界、provider 不匹配、状态不合法）时显示具体错误信息
  - 列表与详情页同步刷新展示最新退款金额
- **权限可见性**：
  - Record Refund 按钮仅 `billing.manage` 权限且满足 provider/manual + paid 条件时可见
  - Credit Note 列表对 Regular User 不可见
  - `provider=creem` 发票的退款维度对所有角色不可见

---

## 8. 已确认决策

- **核心事实**：Stripe 中 Refund 与 Invoice 是独立对象；Credit Note 是发票层面的退款凭证，不作废原发票，对税务合规必需；Credit Note 作废后应同步回滚发票剩余应付
- **三层职责分离**（Stripe 侧）：`charge.refunded`（积分回收）/ `credit_note.created`（发票金额扣减）/ `credit_note.voided`（发票金额回滚）/ `invoice.voided`（发票状态作废）各自独立处理，不相互替代
- **Creem MoR 边界**：Creem 作为 Merchant of Record 负责税务合规；Herald 不为 Creem 维护 Credit Note，不展示退款维度；现有 `refund.created` 仅做积分回收
- **状态机不扩展**：发票主状态保持现有 `draft / issued / paid / void / overdue`；退款以"已退款 / 剩余应付"派生维度叠加在 `paid` 状态之上；`paid` 仍为真终态（不可 void）
- **双轨 Credit Note**：
  - Stripe Credit Note：webhook 被动同步，Herald 不创建、不修改（仅 voided 状态同步）
  - Manual Credit Note：Realm Admin 通过 UI/API 主动创建，仅作用于 `provider=manual` 发票
  - 两者共享相同的展示语义与数据维度（累计已退款 / 剩余应付）
- **Credit Note 生命周期**：存在 `active` 与 `voided` 两种状态；Stripe `credit_note.voided` 事件驱动状态迁移与金额回滚；Manual Credit Note 一旦创建即为永久 `active`
- **Manual Credit Note 不可撤销**：创建即永久凭证；本 PRD 不提供删除/修改机制；如记录错误通过线下补偿处理
- **Manual Credit Note 不触发积分回收**：自研发票的线下退款是否回收积分是独立决策，本 PRD 不引入联动
- **provider 范围**：
  - Stripe Credit Note 同步仅作用于 `provider=stripe`
  - Manual Credit Note 创建仅作用于 `provider=manual`
  - `provider=creem` 完全不涉及 Credit Note
- **Herald 不主动操作 Stripe 侧**：Herald 不暴露发起 Stripe 退款、开具 Stripe Credit Note、作废 Stripe 发票的 API（与现有 Disputes 边界一致）
- **补偿 Job 集成**：`credit_note.created` 与 `credit_note.voided` 纳入 webhook 补偿 Job 的事件清单（详见 `docs/prd/billing/webhook-compensation.md`）
- **权限模型复用**：管理端继续使用 `billing.view` / `billing.manage`，不新增发票细粒度权限
- **异常策略与现有 PRD 一致**：关联缺失、Credit Note 不存在、provider 不匹配、金额越界记录日志而非自动修复（沿用 `webhook-compensation.md` §4.2 的"Error 日志优先"原则）

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/invoice-fallback.md`（US-IF-007 ~ US-IF-011）
- 相关 PRD：`docs/prd/billing/invoice.md`（Invoice 主 PRD，状态机、外部发票同步边界）
- 相关 PRD：`docs/prd/billing/stripe-payment.md`（Stripe webhook 事件清单）
- 相关 PRD：`docs/prd/billing/points.md`（退款积分回收规则：topup / subscription）
- 相关 PRD：`docs/prd/billing/webhook-compensation.md`（补偿 Job 与幂等策略）
