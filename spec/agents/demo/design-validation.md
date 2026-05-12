# 设计文档验证详细流程

## 步骤 0: 设计文档验证 (MANDATORY)

⚠️ **CRITICAL**: 验证设计文档存在后再编写测试。

### 0.1 识别任务名称

从用户故事路径提取：`docs/user-stories/01-admin-realm-user-stories.md` → 角色: `admin-realm`, 任务: `user-permission-management`

### 0.2 检查设计文档

```bash
Read: .ai/design/[任务名].md
```

- ✅ 存在 → 继续
- ❌ 不存在 → 检查是否可以降级处理
  - 如果任务名包含 `bugfix-`, `refactor-`, `setup-` → 豁免，继续执行步骤 0.4
  - 否则 → 执行降级处理流程（步骤 0.3）

### 0.3 设计文档缺失时的降级处理 (FALLBACK)

⚠️ **条件**: 仅在以下情况触发降级流程：
- 设计文档不存在（`.ai/design/[任务名].md`）
- 任务名称**不包含** `bugfix-`, `refactor-`, `setup-` 前缀

#### 0.3.1 信息充足性检查

**执行降级处理前，验证信息源是否充足**：

```bash
# 检查用户故事
Glob: docs/user-stories/*[角色]*user-stories.md

# 检查前端代码
Glob: frontend/src/features/[相关feature]/*-form.tsx
Glob: frontend/src/features/[相关feature]/*-table.tsx
```

**拒绝条件**（任一满足则拒绝执行）：
- ❌ 用户故事文件不存在
- ❌ 前端相关代码不存在（表单组件或表格组件）
- ❌ 用户故事缺少验收标准（Given-When-Then 场景）

**拒绝提示模板**：
```markdown
## 无法生成测试 - 信息不足

**任务**: [任务名称]

**原因**: 无法获取足够的参考信息来推断设计。

**缺失信息**:
- [ ] 用户故事（docs/user-stories/*[角色]*user-stories.md）
- [ ] 前端代码（frontend/src/features/[相关feature]/*-form.tsx）
- [ ] 用户故事验收标准（Given-When-Then 场景）

**建议操作**:

1. **生成设计文档**:
   ```bash
   /t-design [任务名称]
   ```

2. **或补充用户故事**:
   - 添加详细的验收标准（Given-When-Then）
   - 补充失败场景和边界条件

3. **或实现前端代码**:
   - 确保表单组件和表格组件已实现
```

**信息充足 → 继续步骤 0.3.2**

#### 0.3.2 优先级 1: 读取用户故事

```bash
# 确定用户故事文件路径
Read: docs/user-stories/[角色]-user-stories.md
```

**提取信息**：

1. **识别任务和角色**:
   - 从用户故事标题提取功能名称
   - 从文件名提取角色标识

2. **提取验收标准**（Given-When-Then 场景）:
   - **Given** → 前置条件、初始状态
   - **When** → 用户操作、导航路径、按钮点击、表单输入
   - **Then** → 预期结果、验证点、成功/失败提示

3. **提取设计元素**:

   | 设计元素 | 提取方法 | 示例 |
   |---------|---------|------|
   | **导航路径** | "访问 /admin/manage/users 页面" | `/{realmId}/manage/users` |
   | **表单字段** | "输入邮箱为...和密码为..." | `email`, `password` |
   | **按钮操作** | "点击'创建用户'按钮" | 按钮文本: "创建用户" |
   | **验证规则** | "系统提示'邮箱格式错误'" | email 需要格式验证 |
   | **数据展示** | "看到所有用户的列表" | 用户列表表格 |
   | **权限控制** | "系统提示'权限不足'" | 权限校验点 |

4. **存储上下文**：
   - 将提取的信息存储为 `userStoryContext` 对象
   - 包含：角色、功能描述、验收标准、设计元素

#### 0.3.3 优先级 2: 读取前端代码

```bash
# 查找相关前端组件
Glob: frontend/src/features/**/*[相关功能]*.tsx

# 读取表单组件
Read: frontend/src/features/[feature-name]/[feature-name]-form.tsx
```

**提取信息**：

1. **数据模型**（从 TypeScript interfaces）
2. **表单字段和验证规则**（从 Zod schemas）
3. **API 端点**（从 mutation/query functions）
4. **UI 组件模式**（Dialog, Form, Button）

5. **存储上下文**：
   - 将提取的信息存储为 `frontendContext` 对象
   - 包含：数据模型、表单字段、验证规则、API 端点、UI 模式

#### 0.3.4 优先级 3: 读取现有测试代码

```bash
# 读取选择器配置
Read: demo/e2e/selectors.ts

# 查找相似功能的测试
Glob: demo/e2e/**/*[相关功能]*.e2e.ts
```

**提取信息**：

1. **选择器**（从 `selectors.ts`）
2. **测试流程**（从现有测试）
3. **验证点**（从 `expect()` 语句）

4. **存储上下文**：
   - 将提取的信息存储为 `testContext` 对象
   - 包含：选择器、测试流程、验证点

#### 0.3.5 信息整合与推断

**整合策略**：交叉验证和冲突解决

| 信息类型 | 优先级 | 规则 |
|---------|--------|------|
| 表单字段 | 前端代码 > 用户故事 | 以前端代码 Zod schema 为准 |
| API 端点 | 前端代码 > 用户故事 | 以前端 API function 为准 |
| 验证规则 | 前端代码 > 用户故事 | 以 Zod schema 为准 |
| 业务规则 | 用户故事 > 前端代码 | 用户故事的业务规则优先 |
| 选择器 | selectors.ts > 测试代码 | 以 selectors.ts 为准 |

**保守推断策略**：

1. **缺失信息使用保守默认值**，标记为「待确认」
2. **不明确的信息标记为「待确认」**
3. **不主动询问用户**，使用保守推断

#### 0.3.6 继续测试生成

**生成临时文档后**，继续执行原定流程：

- **步骤 1**：选择器策略检查
- **步骤 2-4**：生成测试代码（使用推断的信息）

**重要**：在生成的测试代码中添加注释，说明数据来源：

```typescript
// 测试基于以下推断信息生成：
// - 用户故事: docs/user-stories/[角色]-user-stories.md
// - 前端代码: frontend/src/features/[feature]/*-form.tsx
// - 选择器: demo/e2e/selectors.ts
//
// ⚠️ 注意：此测试基于推断信息生成，建议验证后使用
```

### 0.4 确定输出文件

| 场景归属 | 输出位置 |
|------|---------|
| `super-admin/` 目录内功能 | `demo/e2e/super-admin/[feature]-demo.e2e.ts` 或对应综合文件 |
| `realm-admin/` 目录内功能 | `demo/e2e/realm-admin/[feature]-demo.e2e.ts` 或对应综合文件 |
| `regular-user/` 目录内功能 | `demo/e2e/regular-user/[feature]-demo.e2e.ts` 或对应综合文件 |
| `billing-admin/` 目录内功能 | `demo/e2e/billing-admin/[feature]-demo.e2e.ts` |
| 跨角色或重定向类场景 | `demo/e2e/[feature]-demo.e2e.ts` |

说明：输出位置以当前 `demo/e2e/` 真实目录结构为准，不使用参考仓库中的 `admin-comprehensive-demo.e2e.ts` 或默认 `third-party-app/` 落位。
