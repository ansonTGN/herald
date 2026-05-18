# Herald

多租户认证与授权系统。Rust 后端 + React 前端，单体部署，Docker 上线。

[English version](README.md)

本项目用于实践 AI 编程，使用 Claude Code + GLM 模型以及 Codex 混合开发。

## 技术栈

- **后端**：Rust 2024 edition / Axum 0.8 / SeaORM 1.1 / PostgreSQL 16+ / Redis
- **前端**：React 19 / TypeScript / TanStack Router & Query / Tailwind CSS v4 / Vite
- **部署**：Docker 多阶段构建 + Caddy TLS 反代

## 快速开始

需要 Python 3.12+（[uv](https://github.com/astral-sh/uv)）、Docker、Cargo、npm。

```bash
uv run scripts/dev-start.py
```

启动完成后前端在 http://localhost:3000，后端 API 在 http://localhost:8080。

## 演示

- **地址**：https://auth.fornetcode.com
- **管理员**：admin@fornetcode.com / Herald@2026Admin

## 文档

完整教程见 [docs/tutorials/](docs/tutorials/)，涵盖本地开发、架构、配置和部署。

- [快速上手](docs/tutorials/getting-started.md)
- [架构](docs/tutorials/architecture.md)
- [配置](docs/tutorials/configuration.md)
- [部署](docs/tutorials/deployment.md)

## 许可证

[Apache-2.0](LICENSE)
