# Demo Test Fixtures

本目录包含两种类型的fixtures，满足不同的测试需求。

## 文件说明

### 1. `demo-page.fixtures.ts` ⭐ **推荐**

**返回Page Objects的高级fixtures**

适用于：日常开发、新测试编写

**优势**：
- ✅ 返回Page Objects，不是原始Page
- ✅ 自动完成login + navigation
- ✅ 代码量减少30-50%
- ✅ 类型安全
- ✅ 符合Playwright官方最佳实践

**使用示例**：
```typescript
import { test } from '../fixtures/demo-page.fixtures'

test('should create user', async ({ usersPage }) => {
  // usersPage已经初始化并导航到/admin/users
  await usersPage.createUser({ email: 'test@example.com' })
})
```

**可用的fixtures**：
- `loginPage` - LoginPage（未登录）
- `usersPage` - UsersPage（已登录+已导航）
- `rolesPage` - RolesPage（已登录+已导航）
- `permissionsPage` - PermissionsPage（已登录+已导航）
- `realmsPage` - RealmsPage（已登录+已导航）

### 2. `demo-auth.fixtures.ts`

**返回Page的基础fixtures**

适用于：特殊场景、需要更多控制的测试

**特点**：
- 返回原始`Page`对象
- 提供authenticatedPage（已登录但未导航）
- 需要手动创建Page Objects

**使用示例**：
```typescript
import { test } from '../fixtures/demo-auth.fixtures'

test('should create user', async ({ authenticatedPage, demoLogger }) => {
  const usersPage = new UsersPage(authenticatedPage, demoLogger)
  await usersPage.goto()
  await usersPage.createUser({ email: 'test@example.com' })
})
```

**可用的fixtures**：
- `demoLogger` - UnifiedLogger
- `authenticatedPage` - 已登录的Page
- `testStartTime` - 测试开始时间

## 选择建议

### 使用 demo-page.fixtures.ts（推荐）

当你的测试：
- ✅ 需要操作特定页面（users, roles, permissions, realms）
- ✅ 想要最少的代码量
- ✅ 新编写的测试

### 使用 demo-auth.fixtures.ts

当你的测试：
- ⚠️ 需要在多个页面间跳转
- ⚠️ 需要自定义login流程
- ⚠️ 测试login功能本身
- ⚠️ 不在预定义的Page Objects中

## 迁移指南

### 从 demo-auth.fixtures 到 demo-page.fixtures

**之前** (demo-auth.fixtures):
```typescript
import { test } from '../fixtures/demo-auth.fixtures'

test('should create user', async ({ authenticatedPage, demoLogger }) => {
  const usersPage = new UsersPage(authenticatedPage, demoLogger)
  await usersPage.goto()
  await usersPage.createUser({ email: 'test@example.com' })
})
```

**之后** (demo-page.fixtures):
```typescript
import { test } from '../fixtures/demo-page.fixtures'

test('should create user', async ({ usersPage }) => {
  // 无需创建Page Object，无需导航
  await usersPage.createUser({ email: 'test@example.com' })
})
```

**代码减少**: ~60%

## 相关文档

- [Playwright Test Fixtures](https://playwright.dev/docs/test-fixtures)
- [Playwright Page Object Model](https://playwright.dev/docs/pom)
- `../../../spec/demo/e2e-testing.md` - Demo测试规范
