# Getting Started

## Prerequisites

| Dependency | Version | Purpose |
|------------|---------|---------|
| Python | 3.12+ | Running dev scripts |
| [uv](https://github.com/astral-sh/uv) | Latest | Python version management and script execution |
| Docker | Any version that runs containers | PostgreSQL, Redis |
| Cargo | Rust stable | Compiling the backend |
| npm | Latest | Frontend dependencies |

Install uv:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
uv python install 3.12
uv python pin 3.12
```

On Windows, run `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"` instead, or grab the installer from [GitHub Releases](https://github.com/astral-sh/uv/releases).

## One-command Start

```bash
uv run scripts/dev-start.py
```

The script does the following:

1. Starts a PostgreSQL container (`postgres:18-alpine`, user `postgres`, password `password`, database `herald`, mapped to port 5432)
2. Starts a Redis container (`redis:8.4-alpine`, mapped to port 6379)
3. Runs `cargo run --bin herald-app` in the background (from `backend/`)
4. Runs `npm run dev` in the background (from `frontend/`, Vite dev server)

Once everything is up:

- Frontend: http://localhost:3000
- Backend API: http://localhost:8080
- Logs: `log/backend.log` and `log/frontend.log`

Frontend `/api` requests are automatically proxied to the backend on port 8080 by Vite (configured in `frontend/vite.config.js`). During development, just hit the frontend on port 3000 and API calls will be forwarded.

Each run stops and removes the old `cas-dev-postgres` and `cas-dev-redis` containers before recreating them, so data does not persist between runs.

## Configuration

Backend config lives in `backend/config/config.toml`. The file path is set via the `HERALD_CONFIG` environment variable; if unset, it defaults to `config/config.toml`.

Default development config:

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

The database name in the connection string is `herald`. The `dev-start.py` script also creates a database named `herald`, so the default config matches the script setup.

Redis uses db 1 (`/1`) to keep dev data separate from production.

## Starting Manually

If you prefer to start services yourself:

```bash
# 1. Start PostgreSQL
docker run -d --name herald-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=herald \
  -p 5432:5432 \
  postgres:18-alpine

# 2. Start Redis
docker run -d --name herald-redis \
  -p 6379:6379 \
  redis:8.4-alpine

# 3. Start the backend (migrations run automatically)
cd backend
cargo run --bin herald-app

# 4. Start the frontend (in another terminal)
cd frontend
npm install
npm run dev
```

## Verifying It Works

Backend health check:

```bash
curl http://localhost:8080/health
```

Open http://localhost:3000 in a browser for the frontend. Vite serves it with hot module replacement.

If the backend fails to start, check that PostgreSQL and Redis containers are running (`docker ps`), then verify the connection strings in `config.toml`.

## Next Steps

Once everything is running, open http://localhost:3000 to access the frontend. The first user to register becomes the super admin, who can create realms, invite users, and configure permissions. See [Architecture](architecture-en.md) for what each module does.
