#!/usr/bin/env python
"""
Demo 测试运行器 - 集成环境管理。

自动检查并在必要时启动 Demo 环境，简化测试执行流程。
"""

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import time
from pathlib import Path

from lib.cli import require_executable
from lib.logger import Logger, LogLevel
from lib.paths import REPO_ROOT
from lib import demo_env, ngrok


def escape_regex_pattern(pattern: str) -> str:
    """转义正则表达式中的特殊字符，使其字面匹配。"""
    # 需要转义的特殊字符：. ^ $ * + ? { } [ ] \ | ( )
    # 但保留 > (Playwright测试层级分隔符) 不转义
    special_chars = r'.^$*+?{}[]\|()'
    result = re.escape(pattern)
    # 恢复 > 符号（Playwright测试标题分隔符）
    result = result.replace('\\>', '>')
    return result


# list reporter 单用例行：`  ✓  1 [demo-fast] › file:line:col › Suite › Title (dur)`
TEST_RESULT_LINE_RE = re.compile(r'^\s*([✓✘×])\s+\d+\s+\[[^\]]+\]\s*›\s*(.+)$')
# 行尾耗时，如 ` (1.2s)` / ` (250 ms)` / ` (1.3m)`；标题自带的括号（如 `(US-IF-007)`）不会被误删
DURATION_TAIL_RE = re.compile(r'\s*\([\d.]+\s*(?:ms|sec|min|s|m|h)\)\s*$')
SUMMARY_COUNT_RES = {
    "passed": re.compile(r'(\d+)\s+passed', re.IGNORECASE),
    "failed": re.compile(r'(\d+)\s+failed', re.IGNORECASE),
    "skipped": re.compile(r'(\d+)\s+skipped', re.IGNORECASE),
}


def parse_per_test_results(log_path: Path) -> dict:
    """解析 Playwright list reporter 输出，提取用例级通过/失败明细与计数。

    用于整文件失败时快速定位是哪些用例失败，免去逐个 `--grep` 重跑。

    返回:
        {"passed": int|None, "failed": int|None, "skipped": int|None,
         "passedTests": [str], "failedTests": [str]}
    计数为 None 表示未在日志中找到汇总行；列表按日志出现顺序去重。
    """
    result = {
        "passed": None,
        "failed": None,
        "skipped": None,
        "passedTests": [],
        "failedTests": [],
    }
    try:
        content = log_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return result

    for line in content.splitlines():
        m = TEST_RESULT_LINE_RE.match(line)
        if m:
            status, rest = m.group(1), m.group(2)
            segments = rest.split("›")
            title = segments[-1].strip() if segments else rest.strip()
            title = DURATION_TAIL_RE.sub("", title).strip()
            if not title:
                continue
            if status == "✓":
                if title not in result["passedTests"]:
                    result["passedTests"].append(title)
            else:
                if title not in result["failedTests"]:
                    result["failedTests"].append(title)
            continue
        # 计数行取最后一次匹配（末尾的 `N passed / M failed` 为权威汇总）
        for key, pat in SUMMARY_COUNT_RES.items():
            mm = pat.search(line)
            if mm:
                result[key] = int(mm.group(1))
    return result


def normalize_legacy_args(argv: list[str]) -> list[str]:
    """转换旧版参数格式。"""
    mapping = {
        "-Mode": "--mode",
        "-LogLevel": "--log-level",
        "-RunId": "--run-id",
        "-Grep": "--grep",
        "-NoDedup": "--no-dedup",
        "-NoAggregate": "--no-aggregate",
        "-NoFilter": "--no-filter",
        "-VerboseLog": "--verbose-log",
        "-QuietMode": "--quiet-mode",
        "-ListTests": "--list-tests",
    }
    return [mapping.get(arg, arg) for arg in argv]


def build_parser() -> argparse.ArgumentParser:
    """构建命令行参数解析器。"""
    parser = argparse.ArgumentParser(
        description="Demo test runner with integrated environment management"
    )
    parser.add_argument("test_file", nargs="?", default="", help="Test file or directory")
    parser.add_argument(
        "--mode",
        default="fast",
        choices=["fast", "full"],
        help="Test mode (default: fast)",
    )
    parser.add_argument(
        "--log-level", default="", help="Log level: verbose, mini (default: mini)"
    )
    parser.add_argument("--run-id", default="", help="Run ID for logging")
    parser.add_argument("--grep", default="", help="Filter tests by pattern")
    parser.add_argument("--no-dedup", action="store_true", help="Disable log deduplication")
    parser.add_argument("--no-aggregate", action="store_true", help="Disable log aggregation")
    parser.add_argument("--no-filter", action="store_true", help="Disable log filtering")
    parser.add_argument("--verbose-log", action="store_true", help="Verbose log output")
    parser.add_argument("--quiet-mode", action="store_true", help="Quiet mode (minimal output)")
    parser.add_argument("--list-tests", action="store_true", help="List tests without running")
    parser.add_argument("--compact", action="store_true", help="Compact log format")
    parser.add_argument(
        "--no-auto-env",
        action="store_true",
        help="Do not auto-manage environment (assume it's already running)",
    )
    parser.add_argument(
        "--no-ngrok",
        action="store_true",
        help="Do not start the ngrok public tunnel (needed for live webhook tests)",
    )
    return parser


def ensure_environment(
    auto_manage: bool = True, require_frontend: bool = True
) -> bool:
    """确保 Demo 环境运行且健康。

    Args:
        auto_manage: 是否自动管理环境（启动/停止）
        require_frontend: 是否要求前端必须启动

    Returns:
        环境健康返回 True，否则返回 False
    """
    if not auto_manage:
        return True

    # 检查环境状态
    status = demo_env.check_environment_health(require_frontend=require_frontend)

    if status.healthy:
        if demo_env.check_default_admin_credentials() and demo_env.ensure_demo_seed_data():
            return True

        print('[demo-test-runner] Demo environment is healthy but credentials or demo seed data are polluted; rebuilding environment...')

    # 环境不健康，启动新环境
    logger = Logger(LogLevel.NORMAL)
    return demo_env.start_environment(logger=logger, timeout=120)


def run_tests(
    test_file: str,
    mode: str,
    log_level: str,
    run_id: str | None,
    grep: str,
    no_dedup: bool,
    no_aggregate: bool,
    no_filter: bool,
    verbose_log: bool,
    quiet_mode: bool,
    list_tests: bool,
    compact: bool,
) -> int:
    """运行 Playwright 测试。

    Args:
        test_file: 测试文件路径
        mode: 测试模式
        log_level: 日志级别
        run_id: 运行 ID
        grep: 测试过滤模式
        no_dedup: 禁用日志去重
        no_aggregate: 禁用日志聚合
        no_filter: 禁用日志过滤
        verbose_log: 详细日志
        quiet_mode: 静默模式
        list_tests: 仅列出测试
        compact: 紧凑格式

    Returns:
        退出码（0 表示成功）
    """
    demo_dir = REPO_ROOT / "demo"
    if not demo_dir.exists():
        print(f"Error: demo directory not found at: {demo_dir}")
        return 1

    if not test_file:
        print("Usage: uv run scripts/demo-test-runner.py [test-file] [options]")
        return 1

    # 规范化测试文件路径
    # Playwright testDir is './e2e', so we need path relative to that
    # Input can be: 'demo/e2e/regular-user/test.e2e.ts' or 'e2e/regular-user/test.e2e.ts'
    # Output should be: 'regular-user/test.e2e.ts'
    original_test_file = test_file
    test_file = test_file.replace("\\", "/")

    # Remove 'demo/' prefix if present
    if test_file.startswith("demo/"):
        test_file = test_file[5:]  # Remove 'demo/'

    # Remove 'e2e/' prefix if present (since testDir is './e2e')
    if test_file.startswith("e2e/"):
        test_file = test_file[4:]  # Remove 'e2e/'

    # Debug output for path transformation
    if verbose_log:
        print(f"[DEBUG] Original test file path: {original_test_file}")
        print(f"[DEBUG] Normalized test file path: {test_file}")
        print(f"[DEBUG] Current working directory: {os.getcwd()}")

    # Validate the transformed path exists relative to demo_dir
    test_file_full = demo_dir / "e2e" / test_file
    if not test_file_full.exists():
        print(f"Error: Test file not found at: {test_file_full}")
        print(f"Original input: {original_test_file}")
        print(f"Transformed to: {test_file}")
        print(f"Expected location: {test_file_full}")
        return 1

    # 确定日志级别
    if verbose_log:
        log_level = "verbose"
    elif quiet_mode:
        log_level = "mini"
    if not log_level:
        log_level = "mini"

    # 切换到 demo 目录
    os.chdir(demo_dir)

    # 清理旧的测试结果（保留各 run 日志目录以便失败回溯；仅清理可重建产物与当前 run 目录）
    for old in ("test-results/artifacts", "playwright-report"):
        path = Path(old)
        if path.exists():
            shutil.rmtree(path, ignore_errors=True)

    # 创建日志目录
    run_id = run_id or f"run-{time.strftime('%Y%m%d-%H%M%S')}"
    log_dir = Path("test-results/runs") / run_id
    if log_dir.exists():
        shutil.rmtree(log_dir, ignore_errors=True)
    log_dir.mkdir(parents=True, exist_ok=True)
    playwright_log = log_dir / "playwright-output.log"

    # 转换为绝对路径用于清晰输出
    abs_playwright_log = playwright_log.resolve()
    # 设置环境变量
    env = dict(os.environ)
    env["DEMO_LOG_LEVEL"] = log_level
    env["DEMO_LOG_DEDUP"] = "false" if no_dedup else "true"
    env["DEMO_LOG_AGGREGATE"] = "false" if no_aggregate else "true"
    env["DEMO_LOG_FILTER"] = "false" if no_filter else "true"
    env["DEMO_RUN_ID"] = run_id
    env["DEMO_LOG_COMPACT"] = "true" if compact else "false"
    env["DEBUG"] = env.get("DEBUG", "pw:api")
    env["INTERNAL_API_KEY"] = "demo-internal-api-key"

    # 构建 Playwright 命令
    npx = require_executable("npx", windows_fallback="npx.cmd")
    cmd = [npx, "playwright", "test", test_file, "--project=demo-fast"]
    if grep:
        # 转义正则表达式特殊字符，使其字面匹配
        escaped_grep = escape_regex_pattern(grep)
        cmd.append(f"--grep={escaped_grep}")

        # Debug output for grep pattern
        if verbose_log:
            print(f"[DEBUG] Original grep pattern: {grep}")
            print(f"[DEBUG] Escaped grep pattern: {escaped_grep}")

    if list_tests:
        cmd.append("--list")
        print(f"Listing tests in: {test_file}")
    else:
        # 运行全部用例（覆盖 playwright.config.ts 的 maxFailures:1，与 CI 行为一致），
        # 使整文件失败时能拿到完整的用例级通过/失败明细，而非停在首个失败。
        cmd.append("--max-failures=0")
        cmd.append("--quiet")

    # Debug output for command construction
    if verbose_log:
        print(f"[DEBUG] Working directory: {os.getcwd()}")
        print(f"[DEBUG] Playwright command: {' '.join(shlex.quote(arg) for arg in cmd)}")

    # 运行测试
    start = time.time()
    exit_code = -1
    with playwright_log.open("w", encoding="utf-8") as log_fp:
        proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=env,
        )
        assert proc.stdout is not None
        for line in proc.stdout:
            log_fp.write(line)
            if list_tests:
                # Windows 控制台编码问题的安全输出处理
                try:
                    print(line, end="")
                except UnicodeEncodeError:
                    # 回退：替换有问题的字符
                    print(
                        line.encode("ascii", errors="replace").decode("ascii"), end=""
                    )
        exit_code = proc.wait()
    duration = round(time.time() - start, 1)

    # 检测 "所有测试被跳过" 的情况：Playwright exit_code=0 但实际没有测试通过
    all_skipped = False
    if not list_tests and exit_code == 0:
        log_content = playwright_log.read_text(encoding="utf-8", errors="replace")
        has_passed = bool(re.search(r"\d+ passed", log_content))
        has_failed = bool(re.search(r"\d+ failed", log_content))
        has_skipped = bool(re.search(r"\d+ skipped", log_content))
        if has_skipped and not has_passed and not has_failed:
            all_skipped = True

    if all_skipped:
        exit_code = 2  # 使用特殊退出码区分 "全部跳过"

    # 解析用例级结果（仅在实际运行时；--list 不解析）
    test_breakdown = parse_per_test_results(playwright_log) if not list_tests else None

    # 生成摘要
    summary = {
        "success": "true" if exit_code == 0 else "false",
        "logs": str(log_dir).replace("\\", "/"),
        "exitCode": exit_code,
        "testFile": test_file,
        "mode": mode,
        "logLevel": log_level,
        "duration": duration,
        "runId": run_id,
        "grep": grep,
    }
    if test_breakdown is not None:
        summary["passed"] = test_breakdown["passed"]
        summary["failed"] = test_breakdown["failed"]
        summary["skipped"] = test_breakdown["skipped"]
        summary["failedTests"] = test_breakdown["failedTests"]
        summary["passedTests"] = test_breakdown["passedTests"]
    if all_skipped:
        summary["error"] = "All tests skipped"

    # 打印结果
    if not list_tests and exit_code != 0:
        if all_skipped:
            print("[!] All tests were skipped — no tests actually executed")
        try:
            print(f"✗ Failed ({exit_code})")
        except UnicodeEncodeError:
            print(f"[X] Failed ({exit_code})")
        unified_logs_dir = (demo_dir / "test-results" / "unified-logs").resolve()
        service_log_dir = (REPO_ROOT / "log").resolve()
        print(f"  Playwright: {abs_playwright_log}")
        print(f"  Unified: {unified_logs_dir}/{run_id}-*")
        print(f"  Backend: {service_log_dir}/backend-demo.log.*")
        print(f"  Frontend: {service_log_dir}/frontend-demo.log.*")
        if test_breakdown and test_breakdown["failedTests"]:
            try:
                print("  Failed cases:")
            except UnicodeEncodeError:
                print("  Failed cases:")
            for title in test_breakdown["failedTests"]:
                try:
                    print(f"    ✘ {title}")
                except UnicodeEncodeError:
                    print(f"    [X] {title}")
        elif test_breakdown and not test_breakdown["failedTests"]:
            print("  (No per-test failure parsed; inspect playwright-output.log)")
    print(f"Result: {json.dumps(summary, ensure_ascii=False, separators=(',', ':'))}")

    return exit_code


def main() -> int:
    # 解析参数
    args = build_parser().parse_args(normalize_legacy_args(sys.argv[1:]))

    # 检查环境
    # --list-tests 不需要检查环境（快速列出测试用例）
    auto_manage = not args.no_auto_env and not args.list_tests
    require_frontend = not args.list_tests

    if auto_manage:
        if not ensure_environment(
            auto_manage=auto_manage, require_frontend=require_frontend
        ):
            print("ERROR: Failed to start/verify environment")
            return 1

        # 启动 ngrok 公网隧道，供 live 支付测试接收第三方 webhook 回调
        # （与 demo-start 一致；隧道指向前端 :3000，由 vite /api 代理转发到后端 :8080）
        if not args.no_ngrok:
            ngrok_logger = Logger(LogLevel.NORMAL)
            ngrok.start(logger=ngrok_logger)

    # 运行测试
    exit_code = run_tests(
        test_file=args.test_file,
        mode=args.mode,
        log_level=args.log_level,
        run_id=args.run_id,
        grep=args.grep,
        no_dedup=args.no_dedup,
        no_aggregate=args.no_aggregate,
        no_filter=args.no_filter,
        verbose_log=args.verbose_log,
        quiet_mode=args.quiet_mode,
        list_tests=args.list_tests,
        compact=args.compact,
    )

    return exit_code


if __name__ == "__main__":
    sys.exit(main())


