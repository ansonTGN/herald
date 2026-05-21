# Herald Docker Build

Multi-stage Docker build for Herald. Produces a single image containing the Rust backend, React frontend, and default production config.

## Build

```bash
# Recommended: use the build script
uv run scripts/docker-build.py

# Or build directly
docker build -f docker/Dockerfile -t herald-app:latest .
```

Build options:

```bash
uv run scripts/docker-build.py --tag v1.0.0
uv run scripts/docker-build.py --push --registry registry.example.com/team
uv run scripts/docker-build.py --verbose
```

## Run

```bash
docker run -d \
  -p 3000:3000 \
  -v /path/to/config.toml:/app/config.toml:ro \
  herald-app:latest
```

The image bundles a default production config at `/app/config.toml`. Override it by mounting your own config file or setting `HERALD_CONFIG` to a different path.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `HERALD_CONFIG` | `/app/config.toml` | Path to config file inside container |
| `RUST_LOG` | Set by config `log_level` | Tracing log level, overrides config |

## Health check

Built-in health check on `/health`:
- Interval: 30s, timeout: 3s, startup grace: 10s, retries: 3

```bash
docker ps                    # check "healthy" status
docker inspect <id> | grep -A 10 Health
```

## Verify image

```bash
docker images | grep herald-app
docker run --rm herald-app:latest ls -la /app
docker run --rm herald-app:latest cat /app/config.toml
```

## Troubleshoot

```bash
docker build --no-cache -f docker/Dockerfile -t herald-app:latest .
docker build --progress=plain -f docker/Dockerfile -t herald-app:latest .
docker logs <container_id>
docker exec -it <container_id> sh
```
