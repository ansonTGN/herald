# Configuration

Herald manages all runtime configuration through a single TOML file. The file is read once at startup; there is no hot-reload -- changes require a process restart.

## Loading the Config File

At startup, Herald determines the config file path in this order:

1. Read the `HERALD_CONFIG` environment variable. If set, use its value as the file path.
2. If not set, fall back to `config.toml` in the current working directory.

The entry point is in `backend/app/src/main.rs`:

```rust
let config_path = env::var("HERALD_CONFIG").unwrap_or("config.toml".to_owned());
let config = ApiConfig::load(&config_path)?;
```

The path can be relative or absolute. Relative paths are resolved against the process working directory, not the directory containing the binary.

## Environment Variables

| Variable | Default | Required | Description |
|---|---|---|---|
| `HERALD_CONFIG` | `config.toml` | No | Path to the config file |
| `RUST_LOG` | Set by `server.log_level` | No | tracing log level; overrides `log_level` in the config file |

`RUST_LOG` is read at startup via `tracing_subscriber::EnvFilter::try_from_default_env()`. When set, it takes precedence over the `log_level` field in the config file. When unset, the config file value is used.

## Config File Sections

A complete example lives at `backend/api/config/config.toml`.

### [database]

PostgreSQL connection pool settings. `url` is the only required field; everything else has a built-in default.

| Parameter | Type | Default | Required | Description |
|---|---|---|---|---|
| `url` | string | — | Yes | PostgreSQL connection string |
| `max_connections` | u32 | 100 | No | Maximum number of connections in the pool |
| `acquire_timeout_secs` | u64 | 30 | No | Timeout (seconds) waiting to acquire a connection from the pool |
| `idle_timeout_secs` | u64 | 600 | No | How long an idle connection stays alive (seconds) |
| `max_lifetime_secs` | u64 | 1800 | No | Maximum lifetime of a single connection (seconds) |
| `connect_timeout_secs` | u64 | 10 | No | TCP connection establishment timeout (seconds) |

Connection string format: `postgresql://user:password@host:port/database`

For development, usually only `url` is needed:

```toml
[database]
url = "postgresql://herald:herald@localhost:5432/herald"
```

In production, consider lowering `max_lifetime_secs` to avoid connections being dropped by the database side, and tune `max_connections` to match actual concurrency. The pool is implemented through SeaORM's `ConnectOptions`.

### [redis]

| Parameter | Type | Default | Required | Description |
|---|---|---|---|---|
| `url` | string | `redis://127.0.0.1:6379` | No | Redis connection URL |

Redis is used for permission-check caching and session storage. You can omit this section entirely in local development -- Herald will connect to the default local Redis.

```toml
[redis]
url = "redis://localhost:6379"
```

### [server]

HTTP server and logging settings.

| Parameter | Type | Default | Required | Description |
|---|---|---|---|---|
| `bind_address` | string | `0.0.0.0:3000` | No | Listen address, format `ip:port` |
| `log_level` | string | `info` | No | Log level (trace/debug/info/warn/error) |
| `app_env` | string | `development` | No | Runtime environment identifier |

`app_env` is currently a label only -- the codebase does not branch on it. Using `0.0.0.0` for `bind_address` means listening on all network interfaces.

```toml
[server]
bind_address = "0.0.0.0:3000"
log_level = "info"
app_env = "production"
```

### [frontend]

Settings for the frontend application, primarily affecting CORS and optional static file serving.

| Parameter | Type | Default | Required | Description |
|---|---|---|---|---|
| `url` | string | `http://localhost:5173` | No | Frontend application URL, used for CORS allowlisting |
| `static_dir` | string | — | No | Path to a static files directory; when set, the backend serves the SPA |

The default `url` value of `http://localhost:5173` matches the Vite dev server. Change it to the actual frontend address in production.

When `static_dir` is unset, the backend does not serve static files and the frontend must be deployed separately. Setting it enables the backend to serve files from that directory, which suits single-process deployments.

```toml
[frontend]
url = "http://localhost:3000"
static_dir = "/app/frontend/dist"
```

### [jwt]

JWT secret key configuration, used to generate access tokens in the device code authorization flow (RFC 8628) and third-party OAuth login flows.

| Parameter | Type | Default | Required | Description |
|---|---|---|---|---|
| `secret` | string | — | Yes* | JWT signing secret key |

*When unconfigured, device code token polling and OAuth login will return 500 errors. Use a sufficiently long random string in production (32+ bytes, Base64-encoded recommended).

```toml
[jwt]
secret = "your-random-base64-secret-key-here"
```

Example key generation (Linux/macOS):

```bash
openssl rand -base64 48
```

## RBAC Configuration

RBAC policies are not stored in the main config file. They live in the `role_policies` database table. During initialization, `RealmInitializationService` creates default roles and permissions for each realm.

The policy model is defined in `backend/api/config/rbac_model.conf`:

```ini
[request_definition]
r = dom, sub, obj, act

[policy_definition]
p = dom, sub, obj, act

[role_definition]
g = _, _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = (g(r.dom, r.sub, p.sub) || r.sub == p.sub) && (r.dom == p.dom || p.dom == "*") && r.obj == p.obj && r.act == p.act
```

This file defines a Casbin-style RBAC model. The four fields:

- **dom** -- domain (realm ID), enabling multi-tenant isolation
- **sub** -- subject (user ID or role ID)
- **obj** -- object (resource identifier)
- **act** -- action (operation type)

The matcher `p.dom == "*"` allows cross-domain wildcard policies. The role definition `g = _, _, _` uses a three-part format: `domain, user, role`, where roles themselves are identified by UUID rather than name.
