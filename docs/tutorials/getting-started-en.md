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

## Starting Up

1. Start PostgreSQL and Redis containers, then the backend and frontend:

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

Once everything is running, open http://localhost:3000 to access the frontend. The admin user specified during realm creation has super admin permissions, and can invite users and configure permissions. See [Architecture](architecture-en.md) for what each module does.
