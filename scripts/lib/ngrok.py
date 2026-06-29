"""ngrok tunnel lifecycle for third-party (Stripe) webhook callbacks.

Wraps the official ngrok Docker image (container ``herald-ngrok``). Centralized
here so ``demo-start``/``demo-stop`` manage the tunnel alongside the demo
environment — avoiding the easy-to-forget manual step of starting it before
live webhook tests.

The tunnel targets the frontend port (3000); its ``/api`` proxy
(``frontend/vite.config.js``) forwards webhook calls to the backend (8080).
"""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

from lib import docker
from lib.paths import REPO_ROOT

if TYPE_CHECKING:
    from lib.logger import Logger

CONTAINER_NAME = "herald-ngrok"
# Pinned to the v3 line (only public version tag; Docker Hub exposes latest/3/debian/alpine).
# Matches the version-pinned convention used for postgres/redis in demo_env.py.
IMAGE_NAME = "ngrok/ngrok"
IMAGE_TAG = "3"
IMAGE = f"{IMAGE_NAME}:{IMAGE_TAG}"
INSPECTOR_PORT = 4040
DEFAULT_PORT = 3000
ENV_PATH = REPO_ROOT / "demo" / ".env.demo"


def _load_env(env_path: Path) -> dict[str, str]:
    """Parse KEY=VALUE lines from an env file (no interpolation)."""
    env: dict[str, str] = {}
    if not env_path.exists():
        return env
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        env[key.strip()] = value.strip()
    return env


def config() -> tuple[str, str]:
    """Return ``(NGROK_AUTHTOKEN, NGROK_DOMAIN)`` from ``demo/.env.demo``."""
    env = _load_env(ENV_PATH)
    return env.get("NGROK_AUTHTOKEN", ""), env.get("NGROK_DOMAIN", "")


def is_configured() -> bool:
    """True if ``NGROK_AUTHTOKEN`` is set in ``demo/.env.demo``."""
    authtoken, _ = config()
    return bool(authtoken)


def is_running() -> bool:
    """True if the ngrok tunnel container is currently running."""
    return docker.container_running(CONTAINER_NAME)


def start(port: int = DEFAULT_PORT, logger: "Logger | None" = None) -> bool:
    """Start the ngrok tunnel if configured and not already running.

    Returns True if a tunnel is running after the call (already-running counts),
    False if skipped (not configured) or failed to start.
    """
    if docker.container_running(CONTAINER_NAME):
        if logger:
            logger.verbose_info("ngrok tunnel already running")
        return True

    authtoken, domain = config()
    if not authtoken:
        if logger:
            logger.verbose_info(
                "ngrok skipped (NGROK_AUTHTOKEN not set in demo/.env.demo)"
            )
        return False

    docker.rm_force_container(CONTAINER_NAME)

    cmd = ["http", "--log=stdout"]
    if domain:
        cmd += ["--url", domain]
    cmd.append(f"http://host.docker.internal:{port}")

    args = [
        "--name", CONTAINER_NAME,
        "-p", f"{INSPECTOR_PORT}:{INSPECTOR_PORT}",
        "-e", f"NGROK_AUTHTOKEN={authtoken}",
        "--add-host=host.docker.internal:host-gateway",
        IMAGE,
        *cmd,
    ]
    ok = docker.run_detached(args)
    if ok:
        if logger:
            logger.info(f"Started ngrok tunnel → host.docker.internal:{port}")
        return True
    if logger:
        logger.error("Failed to start ngrok tunnel container")
    return False


def stop(logger: "Logger | None" = None) -> None:
    """Stop and remove the ngrok tunnel container (idempotent)."""
    if docker.container_exists(CONTAINER_NAME):
        docker.rm_force_container(CONTAINER_NAME)
        if logger:
            logger.verbose_info("Stopped ngrok tunnel")


def remove_image(logger: "Logger | None" = None) -> bool:
    """Remove the ngrok image (best-effort, idempotent).

    The container must already be gone (``docker rmi`` refuses images in use).
    Returns True if the image was removed, False if absent or removal failed.
    """
    if not docker.image_exists(IMAGE):
        return False
    if docker.rmi_image(IMAGE):
        if logger:
            logger.verbose_info(f"Removed ngrok image {IMAGE}")
        return True
    if logger:
        logger.warning(f"Failed to remove ngrok image {IMAGE}")
    return False
