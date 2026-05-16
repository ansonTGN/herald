# Product 管理 用户故事

**角色代码**: PR
**角色定义**：Realm Admin 负责管理 Realm 的产品目录和产品线组织。

**故事范围**: US-PR-001 ~ US-PR-006
**创建时间**: 2026-03-26
**状态**: Active

---

## 故事 1：创建 Product [US-PR-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：创建 Product（产品），以便将相关的套餐组织在一起
**从而**：为用户提供清晰的产品线分类和更好的产品展示

**【验收标准】**

**场景 1：创建新 Product**
```gherkin
Given 我是 realm-1 的管理员
When 我在 Billing 管理页面点击 "Create Product" 按钮
And 我填写 Product 信息：
  | Code        | ai-services          |
  | Title       | AI 服务              |
  | Description | AI 驱动的智能服务集合 |
And 我提交表单
Then Product 创建成功
And 系统显示成功消息："Product 'ai-services' created successfully"
And Product 列表显示新创建的 Product
```

**场景 2：Product 代码唯一性验证**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services"
When 我尝试创建同代码 Product "ai-services"
Then 系统显示验证错误："Product code 'ai-services' already exists in this realm"
And Product 创建失败
```

**场景 3：Product 代码格式验证**
```gherkin
Given 我是 realm-1 的管理员
When 我创建 Product
And 我设置 Code 为无效格式（包含空格或特殊字符）
Then 系统显示验证错误："Product code must contain only letters, numbers, hyphens, and underscores"
```

---

## 故事 2：编辑 Product [US-PR-002]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：编辑 Product 信息，以便更新产品描述
**从而**：保持产品目录的准确性和良好的用户体验

**【验收标准】**

**场景 1：编辑 Product 基本信息**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services"
When 我在 Product 列表中点击 "Edit" 按钮
And 我修改以下信息：
  | Title       | AI 服务（已更新）   |
  | Description | 企业级 AI 解决方案 |
And 我保存更改
Then Product 更新成功
And 系统显示成功消息："Product updated successfully"
And Product 列表显示更新后的信息
```

**场景 2：Product 代码不可修改**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services"
When 我编辑 Product
Then "Code" 字段为只读或禁用
And 我无法修改 Product 代码
```

**场景 3：启用/禁用 Product**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services" 且状态为 "enabled"
When 我切换 Product 的 "Status" 开关为 "disabled"
And 我保存更改
Then Product 状态更新为 "disabled"
And 该 Product 下的所有 Plan 对新用户不可见
And 已订阅用户不受影响
```

---

## 故事 3：查看 Product 列表 [US-PR-003]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看所有 Product 列表，以便了解产品目录结构
**从而**：有效管理产品线和套餐组织

**【验收标准】**

**场景 1：查看所有 Product**
```gherkin
Given 我是 realm-1 的管理员
When 我访问 Billing 管理页面
Then 我看到 Product 列表按创建时间排序
And 每个 Product 显示：
  | Code        | ai-services        |
  | Title       | AI 服务            |
  | Description | AI 驱动的智能服务  |
  | Plans Count | 3                  |
  | Status      | Enabled            |
```

**场景 2：按状态筛选 Product**
```gherkin
Given 我是 realm-1 的管理员
And 存在多个 Product，状态包括 enabled 和 disabled
When 我选择状态筛选 "Enabled"
Then 列表仅显示 enabled 状态的 Product
When 我选择状态筛选 "All"
Then 列表显示所有 Product
```

**场景 3：查看 Product 下的 Plans**
```gherkin
Given 我是 realm-1 的管理员
And Product "ai-services" 下有 3 个 Plan
When 我点击 Product "ai-services" 的展开按钮
Then 我看到该 Product 下的所有 Plan 列表
And 每个 Plan 显示其名称、价格、状态信息
```

---

## 故事 4：启用/禁用 Product [US-PR-004]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：启用或禁用 Product，以便控制产品的可见性
**从而**：在不删除数据的情况下临时下架产品

**【验收标准】**

**场景 1：禁用 Product**
```gherkin
Given 我是 realm-1 的管理员
And Product "ai-services" 状态为 "enabled"
When 我禁用该 Product
Then Product 状态更新为 "disabled"
And 新用户无法看到该 Product 及其下的所有 Plan
And 已订阅该 Product 下 Plan 的用户继续正常使用
```

**场景 2：启用 Product**
```gherkin
Given 我是 realm-1 的管理员
And Product "ai-services" 状态为 "disabled"
When 我启用该 Product
Then Product 状态更新为 "enabled"
And Product 下的所有 Plan 重新对用户可见（前提是 Plan 本身也是 enabled）
```

**场景 3：批量启用/禁用**
```gherkin
Given 我是 realm-1 的管理员
And 我选择了多个 Product
When 我点击 "Batch Disable" 按钮
Then 所有选中的 Product 状态更新为 "disabled"
And 系统显示成功消息："X products disabled successfully"
```

---

## 故事 5：删除 Product [US-PR-005]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：删除不再需要的 Product，以便保持产品目录整洁
**从而**：移除过时或测试用的产品线

**【验收标准】**

**场景 1：删除无 Plan 的 Product**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "legacy-product"
And 该 Product 下没有 Plan
When 我在 Product 列表中点击 "Delete" 按钮
And 我确认删除
Then Product 删除成功
And 系统显示成功消息："Product deleted successfully"
And Product 列表不再显示该 Product
```

**场景 2：无法删除有 Plan 的 Product**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services"
And 该 Product 下有 3 个 Plan
When 我尝试删除该 Product
Then 系统显示错误消息："Cannot delete product with existing plans"
And 显示 Plan 数量："This product has 3 plans"
And Product 删除失败
```

**场景 3：删除有活跃订阅的 Product 下的 Plan**
```gherkin
Given 我是 realm-1 的管理员
And Product "ai-services" 下有 Plan "pro-ai"
And 该 Plan 有活跃订阅
When 我先删除 Product 下的其他 Plan
Then 我可以删除无订阅的 Plan
When 我尝试删除最后一个有订阅的 Plan
Then 系统阻止删除并提示活跃订阅数量
```

**场景 4：删除前确认**
```gherkin
Given 我是 realm-1 的管理员
And 我点击删除 Product 按钮
Then 系统显示确认对话框
And 对话框显示警告信息："This will permanently delete the product. Are you sure?"
And 我必须点击 "Confirm" 才能执行删除
```

---

## 故事 6：在 Product 下管理 Plan [US-PR-006]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：在 Product 视图下管理 Plan，以便按产品线组织套餐
**从而**：保持产品目录的层次结构和清晰度

**【验收标准】**

**场景 1：在 Product 下创建 Plan**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services"
When 我在 Product 详情页面点击 "Create Plan" 按钮
And 我填写 Plan 信息：
  | Product     | ai-services (预选，不可修改) |
  | Name        | pro-ai                     |
  | Title       | AI 专业版                  |
  | Type        | monthly                    |
  | Price       | 5000                       |
And 我提交表单
Then Plan 创建成功
And Plan 自动归属到 Product "ai-services"
And Product 详情页显示新创建的 Plan
```

**场景 2：查看 Product 下的 Plans**
```gherkin
Given 我是 realm-1 的管理员
And Product "ai-services" 下有 3 个 Plan：
  | basic-ai  |
  | pro-ai    |
  | enterprise-ai |
When 我访问 Product "ai-services" 详情页面
Then 我看到 Plans 列表按名称或价格排序
And 每个 Plan 显示：
  | Name   | basic-ai    |
  | Title  | AI 基础版   |
  | Price  | $10.00      |
  | Type   | monthly     |
  | Status | Enabled     |
```

**场景 3：在 Product 之间移动 Plan**
```gherkin
Given 我是 realm-1 的管理员
And Plan "basic-ai" 当前属于 Product "ai-services"
And 已存在另一个 Product "cloud-storage"
When 我编辑 Plan "basic-ai"
And 我将 Product 从 "ai-services" 改为 "cloud-storage"
Then Plan 更新成功
And Plan 现在属于 Product "cloud-storage"
And Product "ai-services" 的 Plan 数量减少 1
And Product "cloud-storage" 的 Plan 数量增加 1
```

**场景 4：Product 删除限制**
```gherkin
Given 我是 realm-1 的管理员
And Product "ai-services" 下有 Plan
When 我尝试删除 Product "ai-services"
Then 系统显示错误："Cannot delete product with existing plans"
And 系统提示先移动或删除所有 Plan
```

**场景 5：Plan 创建时必须选择 Product**
```gherkin
Given 我是 realm-1 的管理员
When 我创建新 Plan
Then Product 字段为必填
And 我必须从现有 Product 列表中选择一个
And 至少存在一个 Product 才能创建 Plan
```

---

## 业务规则总结

### Product 规则
1. **唯一性**：Product 代码在同一 Realm 内必须唯一
2. **命名规范**：只能包含字母、数字、横线和下划线
3. **排序**：按创建时间排序
4. **状态管理**：支持启用/禁用，禁用不影响已订阅用户
5. **删除限制**：有 Plan 的 Product 不能删除

### Plan 与 Product 关系规则
1. **从属关系**：每个 Plan 必须属于一个 Product
2. **迁移**：Plan 可以在不同 Product 之间移动
3. **创建约束**：创建 Plan 时必须选择已存在的 Product
4. **级联显示**：Product 禁用时，其下的 Plan 对新用户不可见
5. **删除顺序**：必须先删除 Product 下的所有 Plan 才能删除 Product

### Points 边界规则
1. **当前主配置对象**：当前阶段积分配置仍以 Plan 为主
2. **Product 上下文**：当 Plan 归属于 Product 后，积分配置展示和查询应能反映所属 Product 上下文
3. **范围边界**：Product 级默认规则、按 Client App 差异化规则和多层优先级匹配不属于当前用户故事范围
4. **后续演进**：如需升级 Points 分层规则，应由独立的 Points 用户故事和 PRD 另行定义

---

## 相关文档

- **PRD**: `docs/prd/billing/product-catalog.md` - Product 编目管理产品需求文档
- **PRD**: `docs/prd/billing/billing.md` - Billing 订阅计费产品需求文档
- **PRD**: `docs/prd/billing/points.md` - Points 积分系统产品需求文档
- **用户故事**: `docs/user-stories/06-billing-user-stories.md` - Billing 用户故事
- **用户故事**: `docs/user-stories/points-admin-manage.md` - 积分管理用户故事
