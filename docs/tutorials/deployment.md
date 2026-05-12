# 部署

Herald 用 Docker 部署，生产环境四个容器跑在同一台机器上。

## 架构

```
Internet ──→ Caddy (80/443) ──→ App (3000)
                                    |
                           PostgreSQL + Redis
```

外部流量走 Caddy 进来。Caddy 负责 TLS 终止，把请求转发到 App 容器的 3000 端口。App 同时处理 API 请求（`/api/*`）和前端静态文件（`/app/frontend/dist`）。

所有容器挂在同一个 Docker 网络上，用容器名互相访问（比如 App 连 PostgreSQL 用 `herald-postgres:5432`）。

## 前置条件

- Linux 服务器（Ubuntu 22.04+ 或 Debian 12+），至少 2GB 内存
- Docker Engine 24+ 和 Docker CLI
- 一个域名，DNS A 记录指向服务器 IP
- 防火墙开放两个端口：80（HTTP）、443（HTTPS）

## 准备工作

### 创建 Docker 网络

```bash
docker network create herald-net
```

后面所有容器都会加入这个网络。

### 创建 Volumes

```bash
docker volume create pgdata
docker volume create redisdata
docker volume create caddy-data
docker volume create caddy-config
```

四个 volume 分别存数据库数据、Redis 持久化、Caddy 证书和配置。Docker volume 的数据在容器删除后不会丢。

### 创建配置目录

```bash
mkdir -p /opt/herald/conf
```

### 准备 App 配置

创建生产配置文件 `/opt/herald/config.production.toml`，内容如下：

```toml
[database]
url = "postgres://herald:你的密码@herald-postgres:5432/herald"

[redis]
url = "redis://herald-redis:6379"

[server]
bind_address = "0.0.0.0:3000"
log_level = "info"
app_env = "production"

[frontend]
url = "https://your-domain.com"
static_dir = "/app/frontend/dist"
```

把 `你的密码` 和 `your-domain.com` 替换成实际值。Redis 没有密码，因为 Docker 网络不对外暴露端口。如果你对外暴露了 Redis 端口，需要加密码。

### 准备 Caddy 配置

创建 `/opt/herald/Caddyfile`：

```
your-domain.com {
    reverse_proxy herald-app:3000
}
```

把 `your-domain.com` 改成你的域名。

Caddy 会自动向 Let's Encrypt 申请 TLS 证书，也会自动续期。不需要额外配置证书。

## 启动服务

按 PostgreSQL → Redis → App → Caddy 的顺序启动。App 启动时要连数据库和 Redis，所以先把基础服务拉起来。

### PostgreSQL

```bash
docker run -d \
    --name herald-postgres \
    --network herald-net \
    --restart unless-stopped \
    -e POSTGRES_USER=herald \
    -e POSTGRES_PASSWORD=你的密码 \
    -e POSTGRES_DB=herald \
    -v pgdata:/var/lib/postgresql/data \
    postgres:16-alpine
```

验证：

```bash
docker exec herald-postgres pg_isready -U herald
```

输出 `/var/run/postgresql:5432 - accepting connections` 就表示数据库就绪。

### Redis

```bash
docker run -d \
    --name herald-redis \
    --network herald-net \
    --restart unless-stopped \
    -v redisdata:/data \
    redis:7-alpine \
    redis-server --appendonly yes
```

`--appendonly yes` 开启 AOF 持久化，Redis 重启后数据不会丢。

验证：

```bash
docker exec herald-redis redis-cli ping
```

输出 `PONG` 就行。

### App

```bash
docker run -d \
    --name herald-app \
    --network herald-net \
    --restart unless-stopped \
    -e HERALD_CONFIG=/app/config.toml \
    -v /opt/herald/config.production.toml:/app/config.toml:ro \
    ghcr.io/timzaak/herald:latest
```

如果要用指定版本，把 `latest` 换成 tag（比如 `v0.1.0`）。

App 启动时会自动运行数据库迁移（`sqlx::migrate!`）。你不需要手动建表。但每次部署新版本前建议备份数据库，因为迁移不可逆。

验证：

```bash
docker exec herald-app wget -qO- http://localhost:3000/health
```

返回 `{"status":"healthy",...}` 表示服务正常。

### Caddy

```bash
docker run -d \
    --name herald-caddy \
    --network herald-net \
    --restart unless-stopped \
    -p 80:80 \
    -p 443:443 \
    -v /opt/herald/Caddyfile:/etc/caddy/Caddyfile:ro \
    -v caddy-data:/data \
    -v caddy-config:/config \
    caddy:2-alpine
```

Caddy 首次启动时会向 Let's Encrypt 发起证书申请。如果域名 DNS 还没生效，或者 80 端口被防火墙挡了，证书申请会失败，Caddy 会不断重试。

验证：

```bash
curl -I https://your-domain.com
```

返回 `HTTP/2 200` 就表示部署完成。浏览器打开 `https://your-domain.com` 应该能看到前端页面。

## 验证整体部署

部署完成后，按这个清单检查：

1. 浏览器访问 `https://your-domain.com`，能看到前端界面
2. `curl https://your-domain.com/health` 返回 `{"status":"healthy"}`
3. `docker exec herald-redis redis-cli ping` 返回 PONG
4. `docker exec herald-postgres pg_isready -U herald` 返回 accepting connections

## CI/CD

项目用 GitHub Actions 做自动构建和推送。流程在 `.github/workflows/cd.yml` 里定义。

触发条件：推送以 `v` 开头的 tag（比如 `git tag v0.1.0 && git push origin v0.1.0`）。

流程做的事：

1. 分别在 amd64 和 arm64 上构建镜像
2. 合并多架构 manifest
3. 推送到 `ghcr.io/timzaak/herald:<tag>` 和 `ghcr.io/timzaak/herald:latest`
4. 创建 GitHub Release

Dockerfile 用多阶段构建，一共五个阶段：

| 阶段 | 基础镜像 | 做什么 |
|------|---------|--------|
| backend-chef | rust:1.90-alpine | 安装 cargo-chef 和编译依赖 |
| backend-planner | backend-chef | 分析依赖图，生成 recipe.json |
| backend-builder | backend-chef | 先编译依赖（缓存层），再编译项目二进制 |
| frontend-builder | node:20-alpine | 从后端导出 OpenAPI spec，生成前端 API 客户端，构建前端 |
| 最终镜像 | alpine:3.20 | 只拷贝二进制和前端产物，非 root 用户运行 |

依赖缓存的设计：只要 `Cargo.toml` 和 `Cargo.lock` 没变，依赖层就会命中缓存，只重新编译业务代码。前端也一样，`package.json` 和 `package-lock.json` 不变就复用 `node_modules`。

运行镜像包含二进制文件、前端静态资源、数据库迁移脚本和配置文件。进程以 `herald` 用户（UID 1000）运行，不是 root。内置健康检查，每 30 秒请求 `/health`。

### 发布新版本

1. 打 tag 并推送，触发 GitHub Actions 构建镜像：

```bash
git tag v0.2.0
git push origin v0.2.0
```

2. 等 GitHub Actions 构建完成后，SSH 到生产服务器升级：

```bash
VERSION=v0.2.0

# 拉取新镜像
docker pull ghcr.io/timzaak/herald:${VERSION}

# 停止并删除旧容器
docker stop herald-app
docker rm herald-app

# 用新镜像启动
docker run -d \
    --name herald-app \
    --network herald-net \
    --restart unless-stopped \
    -e HERALD_CONFIG=/app/config.toml \
    -v /opt/herald/config.production.toml:/app/config.toml:ro \
    ghcr.io/timzaak/herald:${VERSION}
```

3. 验证：

```bash
# 检查日志
docker logs herald-app --tail 10

# 健康检查
docker exec herald-app wget -qO- http://localhost:3000/health
```

### 回滚

如果新版本有问题，用旧版本 tag 重新启动：

```bash
docker stop herald-app
docker rm herald-app
docker run -d \
    --name herald-app \
    --network herald-net \
    --restart unless-stopped \
    -e HERALD_CONFIG=/app/config.toml \
    -v /opt/herald/config.production.toml:/app/config.toml:ro \
    ghcr.io/timzaak/herald:v0.1.0
```

### 升级前备份数据库

App 启动时会自动运行数据库迁移，迁移不可逆。升级前备份：

```bash
docker exec herald-postgres pg_dump -U herald herald > backup_$(date +%Y%m%d).sql
```

> 升级过程只有 `herald-app` 容器需要替换，其他容器不需要变动。stop 到 start 之间会有几秒服务中断。

## 数据持久化

App 本身无状态。所有持久化数据在这几个地方：

| 数据 | 存储位置 | Volume 或挂载 |
|------|---------|--------------|
| 业务数据 | PostgreSQL | `pgdata` volume |
| 缓存和会话 | Redis | `redisdata` volume（AOF 持久化） |
| TLS 证书 | Caddy | `caddy-data` volume |
| App 配置 | 宿主机 | `/opt/herald/config.production.toml` 文件挂载 |
| Caddyfile | 宿主机 | `/opt/herald/Caddyfile` 文件挂载 |

备份数据库：

```bash
docker exec herald-postgres pg_dump -U herald herald > backup.sql
```

恢复：

```bash
cat backup.sql | docker exec -i herald-postgres psql -U herald herald
```

Redis 的数据丢了不严重，App 会自动重建缓存。如果确实想备份：

```bash
docker exec herald-redis redis-cli BGSAVE
docker cp herald-redis:/data/dump.rdb ./redis-backup.rdb
```

## 常见问题

### Caddy 证书申请失败

日志里看到 `acme: error` 之类的信息。检查：

- 域名 DNS 是否指向服务器 IP（`dig your-domain.com` 确认）
- 防火墙是否开放 80 和 443 端口
- 服务器 80 端口是否被其他进程占用（`ss -tlnp | grep :80`）

### App 连不上数据库

App 日志里看到 `connection refused` 或 `no route to host`。

检查容器是否在同一个网络：

```bash
docker network inspect herald-net
```

应该能看到 postgres、app 等容器都挂在这个网络上。确认配置文件里的数据库地址是 `herald-postgres:5432`，不是 `localhost`。

### App 启动后马上退出

通常是数据库迁移失败。看日志：

```bash
docker logs herald-app --tail 100
```

常见原因：数据库密码配置错误，或者 PostgreSQL 还没完全启动。等 `pg_isready` 返回正常后再启动 App。

### 修改 Caddyfile 后生效

```bash
docker exec herald-caddy caddy reload --config /etc/caddy/Caddyfile
```

不需要重启 Caddy 容器。
