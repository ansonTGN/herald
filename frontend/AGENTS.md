# Herald Frontend

React 19 + TypeScript + TanStack Router/Query + Tailwind CSS v4 + Vitest + MSW。

所有命令在 `frontend/` 目录下执行。

## 验证

```bash
npm run type-check                                    # 类型检查 — 每次改动后必跑
npm run build                                         # 生产构建 — 提交前必跑
npm run test:run                                      # 运行全部测试
npm run test:run -- src/path/to/file.test.tsx         # 运行指定测试
npm run lint                                          # ESLint
npm run format:check                                  # Prettier 检查
```

## 开发

```bash
npm run dev                  # 启动 dev server (port 3000)
npm run generate-api         # 从后端 OpenAPI spec 生成客户端（不手动编辑 src/lib/api-generated/）
```

## 约束

- 不硬编码 API 路径，使用生成的客户端
- 不用 `any` 绕过类型系统
