#!/usr/bin/env python
import sys

from lib import docker


def main() -> int:
    if docker.container_running("cas-dev-postgres"):
        docker.stop_container("cas-dev-postgres")
    if docker.container_exists("cas-dev-postgres"):
        docker.rm_container("cas-dev-postgres")

    if docker.container_running("cas-dev-redis"):
        docker.stop_container("cas-dev-redis")
    if docker.container_exists("cas-dev-redis"):
        docker.rm_container("cas-dev-redis")

    print("Development environment stopped")
    return 0


if __name__ == "__main__":
    sys.exit(main())
