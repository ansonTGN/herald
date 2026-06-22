#!/usr/bin/env python
"""Start an ngrok tunnel via Docker for third-party callbacks.

Thin CLI over ``lib.ngrok``. Reads ``NGROK_AUTHTOKEN`` and ``NGROK_DOMAIN``
from ``demo/.env.demo``, then runs the official ngrok Docker image (detached)
tunneling to the frontend (default 3000); its ``/api`` proxy forwards webhook
calls to the backend (8080).

Usage:
  uv run scripts/ngrok-tunnel.py
  uv run scripts/ngrok-tunnel.py --port 8080
  uv run scripts/ngrok-tunnel.py --stop
"""

import argparse
import sys

from lib import ngrok

# Force UTF-8 on stdout/stderr so non-ASCII chars (e.g. "→") render correctly
# on Windows terminals whose default code page is GBK.
for _stream in (sys.stdout, sys.stderr):
    if hasattr(_stream, "reconfigure"):
        _stream.reconfigure(encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description="Start ngrok tunnel via Docker")
    parser.add_argument(
        "--port",
        type=int,
        default=ngrok.DEFAULT_PORT,
        help=f"Local port to tunnel (default: {ngrok.DEFAULT_PORT})",
    )
    parser.add_argument(
        "--stop", action="store_true", help="Stop the running ngrok container"
    )
    args = parser.parse_args()

    if args.stop:
        ngrok.stop()
        print(f"Stopped {ngrok.CONTAINER_NAME}")
        return

    authtoken, domain = ngrok.config()
    if not authtoken:
        print(
            "Error: NGROK_AUTHTOKEN not set. "
            "Add it to demo/.env.demo (see demo/.env.demo.example).",
            file=sys.stderr,
        )
        sys.exit(1)

    if ngrok.is_running():
        print(f"ngrok tunnel already running ({ngrok.CONTAINER_NAME})")
    elif ngrok.start(port=args.port):
        print(f"Started ngrok tunnel → host.docker.internal:{args.port} (detached)")
        if domain:
            print(f"  Fixed domain:  {domain}")
        print(f"  Inspector UI:  http://localhost:{ngrok.INSPECTOR_PORT}")
        print(f"  View logs:     docker logs -f {ngrok.CONTAINER_NAME}")
        print(f"  Stop:          uv run scripts/ngrok-tunnel.py --stop")
    else:
        print("Error: failed to start ngrok container.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
