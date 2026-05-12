import socket
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .logger import Logger


def is_port_open(host: str, port: int, timeout: float = 0.5) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(timeout)
        return sock.connect_ex((host, port)) == 0


def wait_for_tcp(host: str, port: int, timeout_seconds: int, interval_seconds: float = 1.0, logger: "Logger | None" = None) -> bool:
    """Wait for a TCP port to become available.

    Args:
        host: Host address
        port: Port number
        timeout_seconds: Maximum time to wait
        interval_seconds: Time between checks
        logger: Optional logger for progress reporting

    Returns:
        True if port is available, False if timeout
    """
    if logger and logger.level >= 2:
        logger.verbose_info(f"{host}:{port} to be available...")

    deadline = time.time() + timeout_seconds
    start_time = time.time()
    check_count = 0

    while time.time() < deadline:
        if is_port_open(host, port):
            elapsed = time.time() - start_time
            if logger and logger.level >= 2:
                logger.verbose_info(f"{host}:{port} ready after {elapsed:.1f}s ({check_count} checks)")
            return True
        time.sleep(interval_seconds)
        check_count += 1

        if logger and logger.level >= 2:
            elapsed = time.time() - start_time
            if check_count % 10 == 0 or check_count == 1:
                logger.progress(f"{host}:{port}", check_count, int(timeout_seconds / interval_seconds))

    return False


def wait_for_http_ok(url: str, timeout_seconds: int, interval_seconds: float = 1.0, logger: "Logger | None" = None) -> bool:
    """Wait for an HTTP endpoint to return a 2xx or 3xx status.

    Args:
        url: URL to check
        timeout_seconds: Maximum time to wait
        interval_seconds: Time between checks
        logger: Optional logger for progress reporting

    Returns:
        True if endpoint is healthy, False if timeout
    """
    if logger and logger.level >= 2:
        logger.verbose_info(f"{url} to be healthy...")

    parsed = urllib.parse.urlparse(url)
    is_loopback = parsed.hostname in {"localhost", "127.0.0.1", "::1"}
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({})) if is_loopback else None

    deadline = time.time() + timeout_seconds
    start_time = time.time()
    check_count = 0

    while time.time() < deadline:
        try:
            request = urllib.request.Request(url)
            response = opener.open(request, timeout=1) if opener else urllib.request.urlopen(request, timeout=1)
            with response as resp:
                if 200 <= resp.status < 400:
                    elapsed = time.time() - start_time
                    if logger and logger.level >= 2:
                        logger.verbose_info(f"{url} healthy after {elapsed:.1f}s ({check_count} checks)")
                    return True
        except (urllib.error.URLError, TimeoutError):
            pass
        time.sleep(interval_seconds)
        check_count += 1

        if logger and logger.level >= 2:
            elapsed = time.time() - start_time
            if check_count % 5 == 0 or check_count == 1:
                logger.progress(f"{url}", check_count, int(timeout_seconds / interval_seconds))

    return False


def wait_for_tcp_with_compile_awareness(
    host: str,
    port: int,
    timeout_seconds: int,
    interval_seconds: float = 1.0,
    logger: "Logger | None" = None,
    check_compilation: bool = True
) -> bool:
    """等待TCP端口可用，支持检测编译活动并自动延长超时.

    Args:
        host: 主机地址
        port: 端口号
        timeout_seconds: 初始超时时间（秒）
        interval_seconds: 检查间隔
        logger: 日志记录器
        check_compilation: 是否检测编译活动

    Returns:
        True 如果端口可用，False 如果超时
    """
    from .proc import is_cargo_compiling

    deadline = time.time() + timeout_seconds
    initial_deadline = deadline  # 保存初始deadline用于比较
    start_time = time.time()
    check_count = 0
    extension_count = 0
    max_extensions = 5  # 最多延长5次
    extension_time = 60  # 每次延长60秒

    if logger and logger.level >= 2:
        logger.verbose_info(f"Waiting for {host}:{port} (with compilation awareness)...")

    while time.time() < deadline:
        # 检查端口是否可用
        if is_port_open(host, port):
            elapsed = time.time() - start_time
            if logger and logger.level >= 2:
                total_extended = extension_count * extension_time
                logger.verbose_info(
                    f"{host}:{port} ready after {elapsed:.1f}s "
                    f"({check_count} checks, extended {total_extended}s)"
                )
            return True

        # 检查是否有编译活动
        if check_compilation and extension_count < max_extensions:
            if is_cargo_compiling():
                if logger and logger.level >= 2:
                    logger.verbose_info(
                        f"Detected cargo compilation, extending timeout by {extension_time}s"
                    )
                deadline = time.time() + extension_time
                extension_count += 1
                # 给编译一些时间，避免频繁检测
                time.sleep(5)
                continue

        time.sleep(interval_seconds)
        check_count += 1

        if logger and logger.level >= 2:
            elapsed = time.time() - start_time
            if check_count % 10 == 0 or check_count == 1:
                total_extended = extension_count * extension_time
                logger.progress(
                    f"{host}:{port}",
                    check_count,
                    int((deadline - start_time) / interval_seconds)
                )

    return False
