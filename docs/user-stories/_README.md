# Herald 用户故事

本目录包含 Herald 系统的用户故事，遵循 [`spec/product/user-story.md`](/spec/product/user-story.md) 编写规范。

---

## 文件结构

| 文件 | 说明 |
|------|------|
| [`_roles.md`](/docs/user-stories/_roles.md) | **统一角色定义参考**（新增角色/修改权限时更新本文档） |
| `_README.md` | 本文档 |
| `01-admin-realm-user-stories.md` | 主管理员用户故事 |
| `02-realm-admin-user-stories.md` | 次管理员用户故事 |
| `03-regular-user-user-stories.md` | 普通用户用户故事 |
| `04-third-party-app-user-stories.md` | 第三方应用用户故事 |

---

## 编写新用户故事

1. 参考 [`_roles.md`](/docs/user-stories/_roles.md) 确认角色定义
2. 遵循 [`spec/product/user-story.md`](/spec/product/user-story.md) 的 INVEST 原则
3. 使用 GWT 格式编写验收标准
4. 在用户故事文件开头引用 `_roles.md` 中的对应角色

**引用格式示例**：

```markdown
### 故事 1：用户管理

**【用户故事】**
**作为**：主管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：能够查看所有Realm的用户
**从而**：统一管控系统用户访问权限
```

---

## 维护规范

### 新增/修改角色

更新角色定义时的步骤：

1. **更新** [`_roles.md`](/docs/user-stories/_roles.md) 中的角色定义
2. **同步代码实现**：
   - 前端：`frontend/src/auth.tsx` 的 `UserRole` 枚举
   - 后端：`api/src/application/http/admin/middleware.rs` 的权限校验逻辑
3. **在用户故事中引用**：使用新的角色定义

### 新增用户故事

- 在对应角色文件中追加新故事
- 或创建新文件（按编号 05、06...）
- 确保引用 [`_roles.md`](/docs/user-stories/_roles.md) 中的角色定义

### 禁止事项

- ❌ 在单个用户故事文件中重复定义角色
- ❌ 与 [`_roles.md`](/docs/user-stories/_roles.md) 中的定义不一致
- ❌ 直接在用户故事中硬编码角色权限详情

---

## 相关文档

- [`spec/product/user-story.md`](/spec/product/user-story.md) - 用户故事编写规范
- [`_roles.md`](/docs/user-stories/_roles.md) - 统一角色定义参考
