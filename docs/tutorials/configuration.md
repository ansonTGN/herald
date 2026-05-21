# 配置

Herald 通过一个 TOML 文件管理所有运行时配置。应用启动时读取该文件，不支持热加载——修改配置需要重启进程。

## 配置文件加载

启动时按以下顺序决定配置文件路径：

1. 读取环境变量 `HERALD_CONFIG`，如果有值则作为文件路径
2. 未设置则默认读取 `config/config.toml`

代码入口在 `backend/app/src/main.rs`：

```rust
let config_path = env::var("HERALD_CONFIG").unwrap_or("config/config.toml".to_owned());
let config = ApiConfig::load(&config_path)?;
```

路径可以是相对路径或绝对路径。相对路径基于进程工作目录解析，不是二进制文件所在目录。

## 环境变量

| 变量名 | 默认值 | 必填 | 说明 |
|---|---|---|---|
| `HERALD_CONFIG` | `config/config.toml` | 否 | 配置文件路径 |
| `RUST_LOG` | 由 `server.log_level` 决定 | 否 | tracing 日志级别，优先级高于配置文件中的 `log_level` |

`RUST_LOG` 在应用启动时由 `tracing_subscriber::EnvFilter::try_from_default_env()` 读取。如果设置了该环境变量，它会覆盖配置文件中的 `log_level`。没设置则回退到配置文件的值。

## 配置文件段

完整示例见 `backend/config/config.toml`。

### [database]

PostgreSQL 连接池配置。`url` 是唯一必填项，其余都有内置默认值。

| 参数 | 类型 | 默认值 | 必填 | 说明 |
|---|---|---|---|---|
| `url` | string | — | 是 | PostgreSQL 连接字符串 |
| `max_connections` | u32 | 100 | 否 | 连接池最大连接数 |
| `acquire_timeout_secs` | u64 | 30 | 否 | 从连接池获取连接的超时（秒） |
| `idle_timeout_secs` | u64 | 600 | 否 | 空闲连接存活时间（秒） |
| `max_lifetime_secs` | u64 | 1800 | 否 | 单个连接最大生命周期（秒） |
| `connect_timeout_secs` | u64 | 10 | 否 | 建立 TCP 连接的超时（秒） |

连接字符串格式：`postgresql://用户名:密码@主机:端口/数据库名`

开发环境通常只需要填 `url`：

```toml
[database]
url = "postgresql://herald:herald@localhost:5432/herald"
```

生产环境建议降低 `max_lifetime_secs`（避免连接被数据库侧主动断断），并根据实际并发量调整 `max_connections`。连接池实现基于 SeaORM 的 `ConnectOptions`。

### [redis]

| 参数 | 类型 | 默认值 | 必填 | 说明 |
|---|---|---|---|---|
| `url` | string | `redis://127.0.0.1:6379` | 否 | Redis 连接地址 |

Redis 用于权限检查缓存和 session 存储。本地开发不写这个段也可以，会连默认的本地 Redis。

```toml
[redis]
url = "redis://localhost:6379"
```

### [server]

HTTP 服务器和日志配置。

| 参数 | 类型 | 默认值 | 必填 | 说明 |
|---|---|---|---|---|
| `bind_address` | string | `0.0.0.0:3000` | 否 | 监听地址，格式为 `ip:port` |
| `log_level` | string | `info` | 否 | 日志级别（trace/debug/info/warn/error） |
| `app_env` | string | `development` | 否 | 运行环境标识 |

`app_env` 目前是一个标识字段，代码中不直接用它做分支逻辑。`bind_address` 用 `0.0.0.0` 表示监听所有网卡。

```toml
[server]
bind_address = "0.0.0.0:3000"
log_level = "info"
app_env = "production"
```

### [frontend]

前端应用相关的配置，主要影响 CORS 和可选的静态文件托管。

| 参数 | 类型 | 默认值 | 必填 | 说明 |
|---|---|---|---|---|
| `url` | string | `http://localhost:5173` | 否 | 前端应用地址，用于 CORS 白名单 |
| `static_dir` | string | — | 否 | 静态文件目录路径，设置后由后端托管 SPA |

`url` 的默认值 `http://localhost:5173` 是 Vite 开发服务器的地址。生产部署时改成实际前端地址。

`static_dir` 不设置时后端不提供静态文件服务，前端需要单独部署。设置后，后端会从指定目录提供静态文件，适合单体部署场景。

```toml
[frontend]
url = "http://localhost:3000"
static_dir = "/app/frontend/dist"
```

### [jwt]

JWT 密钥配置，用于设备码授权（RFC 8628）和第三方 OAuth 登录流程中生成访问令牌。

| 参数 | 类型 | 默认值 | 必填 | 说明 |
|---|---|---|---|---|
| `secret` | string | — | 是* | JWT 签名密钥 |

*未配置时设备码令牌轮询和 OAuth 登录将返回 500 错误。生产环境应使用足够长的随机字符串（建议 32 字节以上的 Base64 编码）。

```toml
[jwt]
secret = "your-random-base64-secret-key-here"
```

生成密钥示例（Linux/macOS）：

```bash
openssl rand -base64 48
```

## RBAC 配置

RBAC 策略不在主配置文件中，而是通过数据库中的 `role_policies` 表存储。系统初始化时，`RealmInitializationService` 会为每个 realm 创建默认角色和权限。权限检查由 `RedisPermissionChecker` 完成，先查 Redis 缓存，缓存 miss 时回源 PostgreSQL。
