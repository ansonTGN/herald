# 快速上手

## 你需要先装好

| 依赖 | 版本要求 | 用途 |
|------|---------|------|
| Python | 3.12+ | 运行开发脚本 |
| [uv](https://github.com/astral-sh/uv) | 最新 | Python 版本管理和脚本执行 |
| Docker | 能跑容器就行 | PostgreSQL、Redis |
| Cargo | Rust 稳定版 | 编译后端 |
| npm | 最新 | 前端依赖 |

uv 的安装：

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
uv python install 3.12
uv python pin 3.12
```

Windows 用户可以直接 `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"`，或者去 [GitHub Releases](https://github.com/astral-sh/uv/releases) 下安装包。

## 一键启动

```bash
uv run scripts/dev-start.py
```

这个脚本做了这些事：

1. 启动 PostgreSQL 容器（`postgres:18-alpine`，用户 `postgres`，密码 `password`，数据库 `cas`，映射 5432 端口）
2. 启动 Redis 容器（`redis:8.4-alpine`，映射 6379 端口）
3. 后台运行 `cargo run --bin herald-app`（工作目录 `backend/`，脚本里写的是 `cas-app`，实际二进制名是 `herald-app`）
4. 后台运行 `npm run dev`（工作目录 `frontend/`，Vite 开发服务器）

启动完成后：

- 前端：http://localhost:3000
- 后端 API：http://localhost:8080
- 日志位置：`log/backend.log` 和 `log/frontend.log`

脚本每次运行会先停掉旧的 `cas-dev-postgres` 和 `cas-dev-redis` 容器再重建，所以数据不会保留。

## 配置

后端配置文件在 `backend/config.toml`，通过环境变量 `HERALD_CONFIG` 指定路径，不设就默认读当前目录下的 `config.toml`。

开发环境的默认配置：

```toml
[database]
url = "postgres://postgres:postgres@localhost/herald?sslmode=disable"
max_connections = 100

[redis]
url = "redis://127.0.0.1:6379/1"

[server]
bind_address = "0.0.0.0:8080"
log_level = "info"

[frontend]
url = "http://localhost:3000"
```

数据库连接字符串里的 `herald` 是数据库名。`dev-start.py` 脚本创建的容器实际库名是 `cas`，所以如果你用脚本启动的 PostgreSQL，需要改成 `cas` 或者自己在容器里建库。

Redis 用的是 db 1（`/1`），和生产环境隔离。

## 手动分步启动

如果不想用一键脚本，自己来：

```bash
# 1. 启动 PostgreSQL
docker run -d --name herald-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=herald \
  -p 5432:5432 \
  postgres:18-alpine

# 2. 启动 Redis
docker run -d --name herald-redis \
  -p 6379:6379 \
  redis:8.4-alpine

# 3. 启动后端（会自动运行迁移）
cd backend
cargo run --bin herald-app

# 4. 启动前端（另一个终端）
cd frontend
npm install
npm run dev
```

## 验证

后端健康检查：

```bash
curl http://localhost:8080/health
```

前端直接浏览器打开 http://localhost:3000，Vite 会热更新。

如果后端启动失败，先检查 PostgreSQL 和 Redis 容器是不是在跑（`docker ps`），再检查 `config.toml` 里的连接字符串。
