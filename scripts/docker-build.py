#!/usr/bin/env python
"""Build Docker image for Herald.

This script builds a complete Docker image containing both backend and frontend.
It performs the following steps:
  1. Prerequisite checks (Docker, disk space, project structure)
  2. Build backend binary (Rust)
  3. Generate frontend API types (if backend is available)
  4. Build frontend (React)
  5. Build Docker image with proper tagging
  6. Optional: Push to registry
  7. Verify the build was successful

Usage:
  # Basic build with default tag
  uv run scripts/docker-build.py

  # Build with custom tag
  uv run scripts/docker-build.py --tag herald-app:1.0.0

  # Build with custom name and tag
  uv run scripts/docker-build.py --name my-herald --tag v2.0.0

  # Push to registry after build
  uv run scripts/docker-build.py --push --registry registry.example.com/team

  # Verbose output
  uv run scripts/docker-build.py --verbose

Requirements:
  - Docker installed and running
  - Sufficient disk space (at least 3GB recommended)
  - Python 3.x
  - Rust toolchain (cargo) for backend compilation
  - Node.js and npm for frontend build

Output:
  - Docker image: <name>:<tag> (default: herald-app:latest)
  - Frontend built in frontend/dist/
  - Backend binary in backend/target/release/herald-app
"""

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

from lib.cli import run_cmd, require_executable
from lib.logger import Logger
from lib.paths import REPO_ROOT


def check_prerequisites(logger: Logger) -> tuple[bool, str]:
    """Check necessary tools and resources are available.

    Verifies the following prerequisites:
    - Docker is installed and daemon is running
    - Python is installed
    - At least 3GB free disk space
    - Project structure is correct

    Args:
        logger: Logger instance for output

    Returns:
        Tuple of (all_passed, error_message). If all checks pass,
        returns (True, ""). Otherwise returns (False, error_message).
    """
    checks = []

    # Check Docker
    logger.verbose_info("Checking Docker availability...")
    try:
        docker_path = require_executable("docker")
        # Check if Docker daemon is running
        result = run_cmd([docker_path, "version"], capture=True)
        if result.returncode != 0:
            return False, "Docker daemon is not running. Please start Docker Desktop."
        checks.append(("Docker", True))
    except RuntimeError as e:
        return False, f"Docker is not installed or not in PATH. Error: {e}"

    # Check Python
    logger.verbose_info("Checking Python availability...")
    if shutil.which("python") or shutil.which("python3"):
        checks.append(("Python", True))
    else:
        return False, "Python is not installed or not in PATH."

    # Check cargo
    logger.verbose_info("Checking cargo availability...")
    if shutil.which("cargo"):
        checks.append(("cargo", True))
    else:
        return False, "cargo is not installed or not in PATH. Please install Rust toolchain."

    # Check npm
    logger.verbose_info("Checking npm availability...")
    try:
        require_executable("npm", windows_fallback="npm.cmd")
        checks.append(("npm", True))
    except RuntimeError as e:
        return False, f"npm is not installed or not in PATH. Error: {e}"

    # Check disk space (at least 3GB free)
    logger.verbose_info("Checking disk space...")
    try:
        if os.name == "nt":
            # Windows
            import ctypes
            free_bytes = ctypes.c_ulonglong(0)
            ctypes.windll.kernel32.GetDiskFreeSpaceExW(
                ctypes.c_wchar_p(str(REPO_ROOT)), None, None, ctypes.pointer(free_bytes)
            )
            free_gb = free_bytes.value / (1024 ** 3)
        else:
            # Unix-like
            stat = shutil.disk_usage(REPO_ROOT)
            free_gb = stat.free / (1024 ** 3)

        if free_gb < 3:
            return False, f"Insufficient disk space: {free_gb:.1f}GB free. At least 3GB required."
        checks.append(("Disk space", True))
    except Exception as e:
        logger.warning(f"Could not check disk space: {e}")

    # Check project structure
    logger.verbose_info("Checking project structure...")
    required_dirs = [
        REPO_ROOT / "backend",
        REPO_ROOT / "frontend",
        REPO_ROOT / "backend" / "Cargo.toml",
        REPO_ROOT / "docker" / "Dockerfile",
    ]

    for item in required_dirs:
        if not item.exists():
            return False, f"Required project structure not found: {item}"
    checks.append(("Project structure", True))

    # Log all passed checks
    logger.verbose_info("Prerequisite checks passed:")
    for name, _ in checks:
        logger.verbose_info(f"  [OK] {name}")

    return True, ""


def export_api(logger: Logger) -> bool:
    """Export OpenAPI spec from backend and generate frontend types.

    This function runs the generate-api script to create TypeScript types
    from the backend OpenAPI specification.

    Args:
        logger: Logger instance for output

    Returns:
        bool: True if export succeeded, False otherwise
    """
    logger.info("Exporting API types...")

    frontend_dir = REPO_ROOT / "frontend"
    npm = require_executable("npm", windows_fallback="npm.cmd")

    # Generate frontend types
    logger.verbose_info("Running frontend API generation...")
    gen_result = run_cmd([npm, "run", "generate-api"], cwd=frontend_dir, capture=True)
    if gen_result.returncode != 0:
        logger.warning("API generation failed (will use existing types if available)")
        logger.verbose_info(f"stdout: {gen_result.stdout[-500:] if gen_result.stdout else 'N/A'}")
        logger.verbose_info(f"stderr: {gen_result.stderr[-500:] if gen_result.stderr else 'N/A'}")
        logger.verbose_info("Continuing with existing API types...")
        return True  # Don't fail the build, just warn

    logger.verbose_info("API export complete")
    return True


def build_frontend(logger: Logger) -> bool:
    """Build frontend locally.

    This function runs npm run build in the frontend directory to
    compile and bundle the React application for production.

    Args:
        logger: Logger instance for output

    Returns:
        bool: True if build succeeded, False otherwise
    """
    logger.info("Building frontend...")
    npm = require_executable("npm", windows_fallback="npm.cmd")

    # Check if frontend directory exists
    frontend_dir = REPO_ROOT / "frontend"
    if not frontend_dir.exists():
        logger.error(f"Frontend directory not found: {frontend_dir}")
        return False

    result = run_cmd(
        [npm, "run", "build"],
        cwd=frontend_dir,
    )
    if result.returncode != 0:
        logger.error("Frontend build failed")
        logger.error(f"stdout: {result.stdout[-500:] if result.stdout else 'N/A'}")
        logger.error(f"stderr: {result.stderr[-500:] if result.stderr else 'N/A'}")
        logger.error("\nTroubleshooting suggestions:")
        logger.error("  1. Ensure frontend dependencies are installed: cd frontend && npm install")
        logger.error("  2. Check for TypeScript errors: cd frontend && npx tsc --noEmit")
        logger.error("  3. Verify build scripts in package.json")
        return False

    # Verify dist directory was created
    dist_dir = frontend_dir / "dist"
    if not dist_dir.exists():
        logger.error(f"Frontend dist directory not created: {dist_dir}")
        return False

    logger.verbose_info(f"Frontend build output: {dist_dir}")
    return True


def build_docker_image(
    logger: Logger,
    image_name: str,
    image_tag: str,
    registry: str | None = None
) -> bool:
    """Build Docker image.

    This function builds a Docker image containing both the backend application
    and the pre-built frontend assets.

    Args:
        logger: Logger instance for output
        image_name: Docker image name, e.g. "herald-app"
        image_tag: Docker image tag, e.g. "latest" or "1.0.0"
        registry: Optional registry prefix, e.g. "registry.example.com/team"

    Returns:
        bool: True if build succeeded, False otherwise
    """
    # Construct full image tag
    if registry:
        full_tag = f"{registry}/{image_name}:{image_tag}"
    else:
        full_tag = f"{image_name}:{image_tag}"

    logger.info(f"Building Docker image: {full_tag}")
    docker_path = require_executable("docker")

    # Check Dockerfile exists
    dockerfile_path = REPO_ROOT / "docker" / "Dockerfile"
    if not dockerfile_path.exists():
        logger.error(f"Dockerfile not found: {dockerfile_path}")
        logger.error("\nTroubleshooting suggestions:")
        logger.error("  1. Verify the Dockerfile exists at docker/Dockerfile")
        logger.error("  2. Check the project structure")
        return False

    # Build with proper context and Dockerfile
    result = run_cmd(
        [docker_path, "build", "-t", full_tag, "-f", "docker/Dockerfile", "."],
        cwd=REPO_ROOT,
    )
    if result.returncode != 0:
        logger.error("Docker build failed")
        logger.error(f"stdout: {result.stdout[-500:] if result.stdout else 'N/A'}")
        logger.error(f"stderr: {result.stderr[-500:] if result.stderr else 'N/A'}")
        logger.error("\nTroubleshooting suggestions:")
        logger.error("  1. Check Docker daemon is running: docker ps")
        logger.error("  2. Verify backend/Cargo.toml and dependencies")
        logger.error("  3. Check .dockerignore for exclusion issues")
        logger.error("  4. Try building with verbose output: docker build --progress=plain -f docker/Dockerfile .")
        return False

    logger.verbose_info(f"Docker image built successfully: {full_tag}")
    return True


def push_docker_image(logger: Logger, image_name: str, image_tag: str, registry: str) -> bool:
    """Push the Docker image to the configured registry.

    Args:
        logger: Logger instance for output
        image_name: Docker image name
        image_tag: Docker image tag
        registry: Registry prefix

    Returns:
        bool: True if push succeeded, False otherwise
    """
    full_tag = f"{registry}/{image_name}:{image_tag}"
    logger.info(f"Pushing Docker image: {full_tag}")
    docker_path = require_executable("docker")

    result = run_cmd(
        [docker_path, "push", full_tag],
        cwd=REPO_ROOT,
    )
    if result.returncode != 0:
        logger.error("Docker push failed")
        logger.error(f"stdout: {result.stdout[-500:] if result.stdout else 'N/A'}")
        logger.error(f"stderr: {result.stderr[-500:] if result.stderr else 'N/A'}")
        logger.error("\nTroubleshooting suggestions:")
        logger.error("  1. Verify you are logged in to the target registry")
        logger.error(f"  2. Check registry permissions: {registry}")
        logger.error(f"  3. Confirm the repository exists and you have push permission")
        return False

    logger.verbose_info(f"Docker image pushed successfully: {full_tag}")
    return True


def verify_build(logger: Logger, image_name: str, image_tag: str, registry: str | None = None) -> bool:
    """Verify the Docker image was created successfully.

    Args:
        logger: Logger instance
        image_name: Docker image name
        image_tag: Docker image tag
        registry: Optional registry prefix

    Returns:
        bool: True if verification succeeded
    """
    logger.info("Verifying build...")
    docker_path = require_executable("docker")

    # Construct full image tag
    if registry:
        full_tag = f"{registry}/{image_name}:{image_tag}"
    else:
        full_tag = f"{image_name}:{image_tag}"

    result = run_cmd(
        [docker_path, "images", full_tag, "--format", "{{.Size}}"],
        capture=True,
    )
    if result.returncode != 0:
        logger.warning("Could not verify image size")
        return True

    size = result.stdout.strip()
    logger.verbose_info(f"Docker image size: {size}")

    # Warn if image is too large
    try:
        size_parts = size.split()
        if len(size_parts) == 2:
            value, unit = size_parts
            value_num = float(value)
            if unit == "GB" and value_num > 1:
                logger.warning(f"Image size {size} exceeds recommended size (<1GB)")
            elif unit == "MB" and value_num > 500:
                logger.warning(f"Image size {size} exceeds recommended size (<500MB)")
    except Exception:
        pass

    return True


def main() -> int:
    """Main entry point for the Docker build script.

    This function orchestrates the entire build process:
    1. Check prerequisites (Docker, disk space, project structure)
    2. Export API types from backend
    3. Build frontend locally
    4. Build Docker image
    5. Optional: Push Docker image
    6. Verify the build was successful

    Returns:
        int: Exit code (0 for success, 1 for failure)
    """
    parser = argparse.ArgumentParser(
        description="Build Docker image for Herald.\n"
                    "This script performs API type export, frontend build, "
                    "and Docker image creation.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""Examples:
  # Basic build with default settings
  %(prog)s

  # Build with custom tag
  %(prog)s --tag v1.0.0

  # Build with custom name and tag
  %(prog)s --name my-herald --tag v2.0.0

  # Build and push to registry
  %(prog)s --push --registry registry.example.com/team

  # Build with verbose output
  %(prog)s --verbose

View available images:
  docker images herald-app
        """
    )
    parser.add_argument(
        "--name",
        default="herald-app",
        help="Docker image name (default: herald-app)",
    )
    parser.add_argument(
        "--tag",
        default="latest",
        help="Docker image tag (default: latest)",
    )
    parser.add_argument(
        "--registry",
        help="Docker registry prefix for pushing (e.g., registry.example.com/team)",
    )
    parser.add_argument(
        "--push",
        action="store_true",
        help="Push the image to registry after building",
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Enable verbose output",
    )
    args = parser.parse_args()

    # Setup logger
    if args.verbose:
        logger = Logger().verbose()
    else:
        logger = Logger()

    logger.info("=" * 50)
    logger.info("Herald Docker Build Script")
    logger.info("=" * 50)
    logger.verbose_info(f"Image: {args.name}:{args.tag}")
    if args.registry:
        logger.verbose_info(f"Registry: {args.registry}")
    logger.info("")

    # Step 1: Check prerequisites
    logger.info("Step 1: Checking prerequisites...")
    prereq_passed, error_msg = check_prerequisites(logger)
    if not prereq_passed:
        logger.error(f"Prerequisite check failed: {error_msg}")
        logger.error("\nPlease resolve the above issues before continuing.")
        return 1

    # Step 2: Export API types
    logger.info("Step 2: Exporting API types...")
    if not export_api(logger):
        logger.warning("API export failed, continuing with existing types")

    # Step 3: Build frontend
    logger.info("Step 3: Building frontend...")
    if not build_frontend(logger):
        logger.error("Frontend build failed")
        return 1

    # Step 4: Build Docker image
    logger.info("Step 4: Building Docker image...")
    if not build_docker_image(logger, args.name, args.tag, args.registry):
        logger.error("Docker build failed")
        return 1

    # Step 5: Push to registry (if requested)
    if args.push:
        if not args.registry:
            logger.error("--push requires --registry to be specified")
            return 1
        logger.info("Step 5: Pushing Docker image...")
        if not push_docker_image(logger, args.name, args.tag, args.registry):
            logger.error("Docker push failed")
            return 1

    # Step 6: Verify build
    logger.info("Step 6: Verifying build...")
    verify_build(logger, args.name, args.tag, args.registry)

    # Success!
    logger.info("")
    logger.info("=" * 50)
    full_tag = f"{args.name}:{args.tag}"
    if args.registry:
        full_tag = f"{args.registry}/{full_tag}"
    logger.info(f"✓ Docker image built successfully: {full_tag}")
    logger.info("=" * 50)

    if args.push:
        logger.info(f"✓ Image pushed to registry: {args.registry}")
    else:
        logger.info("\nTo run the container:")
        logger.info(f"  docker run -p 3000:3000 {full_tag}")
        logger.info("\nTo push to registry:")
        logger.info(f"  docker push {full_tag}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
