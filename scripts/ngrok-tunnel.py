#!/usr/bin/env python
"""Start an ngrok tunnel via Docker for third-party callbacks.

Reads NGROK_AUTHTOKEN and NGROK_DOMAIN from demo/.env.demo,
then runs the official ngrok Docker image tunneling to localhost:3000 (frontend).

Usage:
  uv run scripts/ngrok-tunnel.py
  uv run scripts/ngrok-tunnel.py --port 8080
  uv run scripts/ngrok-tunnel.py --stop
"""

import argparse
import subprocess
import sys
from pathlib import Path

CONTAINER_NAME = "herald-ngrok"
INSPECTOR_PORT = 4040


def load_env(env_path: Path) -> dict[str, str]:
    """Parse KEY=VALUE lines from an env file (no interpolation)."""
    env = {}
    if not env_path.exists():
        return env
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" in line:
            key, _, value = line.partition("=")
            env[key.strip()] = value.strip()
    return env


def load_ngrok_config() -> tuple[str, str]:
    """Load only NGROK_AUTHTOKEN and NGROK_DOMAIN from demo/.env.demo."""
    env_path = Path(__file__).resolve().parent.parent / "demo" / ".env.demo"
    env = load_env(env_path)
    return env.get("NGROK_AUTHTOKEN", ""), env.get("NGROK_DOMAIN", "")


def stop():
    subprocess.run(
        ["docker", "rm", "-f", CONTAINER_NAME],
        capture_output=True,
    )
    print(f"Stopped {CONTAINER_NAME}")


def start(port: int):
    authtoken, domain = load_ngrok_config()

    if not authtoken:
        print(
            "Error: NGROK_AUTHTOKEN not set. "
            "Add it to demo/.env.demo (see demo/.env.demo.example).",
            file=sys.stderr,
        )
        sys.exit(1)

    cmd = ["http", "--log=stdout"]
    if domain:
        cmd += ["--url", domain]
    cmd.append(f"http://host.docker.internal:{port}")

    print(f"Starting ngrok tunnel → host.docker.internal:{port}")
    if domain:
        print(f"  Fixed domain: {domain}")
    print(f"  Inspector UI: http://localhost:{INSPECTOR_PORT}")

    try:
        subprocess.run(
            [
                "docker", "run",
                "--name", CONTAINER_NAME,
                "--rm",
                "-p", f"{INSPECTOR_PORT}:{INSPECTOR_PORT}",
                "-e", f"NGROK_AUTHTOKEN={authtoken}",
                "--add-host=host.docker.internal:host-gateway",
                "ngrok/ngrok:latest",
                *cmd,
            ],
        )
    except KeyboardInterrupt:
        print("\nStopping ngrok tunnel...")
        stop()


def main():
    parser = argparse.ArgumentParser(description="Start ngrok tunnel via Docker")
    parser.add_argument("--port", type=int, default=3000, help="Local port to tunnel (default: 3000)")
    parser.add_argument("--stop", action="store_true", help="Stop the running ngrok container")
    args = parser.parse_args()

    if args.stop:
        stop()
    else:
        start(args.port)


if __name__ == "__main__":
    main()
