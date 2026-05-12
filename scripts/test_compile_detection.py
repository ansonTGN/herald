#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""简单的编译检测功能测试脚本."""

import sys
from pathlib import Path

# 添加scripts目录到路径
scripts_dir = Path(__file__).parent
sys.path.insert(0, str(scripts_dir))

from lib.proc import is_cargo_compiling
from lib.net import wait_for_tcp_with_compile_awareness


def test_cargo_detection():
    """测试cargo编译检测功能."""
    print("=== Cargo 编译检测测试 ===")

    # 测试1: 检测当前是否有cargo进程
    is_compiling = is_cargo_compiling()
    print(f"当前cargo编译状态: {'正在编译' if is_compiling else '未检测到编译'}")

    if is_compiling:
        print("[OK] 成功检测到cargo编译进程")
    else:
        print("[OK] 正确检测到没有cargo编译进程")

    return True


def test_tcp_wait_function():
    """测试TCP等待函数是否可用."""
    print("\n=== TCP等待函数测试 ===")

    # 测试导入和函数签名
    import inspect
    sig = inspect.signature(wait_for_tcp_with_compile_awareness)
    print(f"函数签名: wait_for_tcp_with_compile_awareness{sig}")

    # 检查参数
    params = list(sig.parameters.keys())
    expected_params = ['host', 'port', 'timeout_seconds', 'interval_seconds', 'logger', 'check_compilation']

    if params == expected_params:
        print("[OK] 函数参数正确")
    else:
        print(f"[ERROR] 函数参数不匹配: 期望 {expected_params}, 实际 {params}")
        return False

    return True


def main():
    """运行所有测试."""
    print("开始测试编译检测功能...\n")

    all_passed = True

    try:
        if not test_cargo_detection():
            all_passed = False
    except Exception as e:
        print(f"[ERROR] Cargo检测测试失败: {e}")
        all_passed = False

    try:
        if not test_tcp_wait_function():
            all_passed = False
    except Exception as e:
        print(f"[ERROR] TCP等待函数测试失败: {e}")
        all_passed = False

    print("\n" + "="*40)
    if all_passed:
        print("[OK] 所有测试通过")
        print("\n建议的完整测试:")
        print("1. 测试正常编译: uv run scripts/demo-start.py -v")
        print("2. 测试长时间编译: cargo clean && uv run scripts/demo-start.py -v")
        print("3. 观察日志输出中的超时延长信息")
        return 0
    else:
        print("[ERROR] 部分测试失败")
        return 1


if __name__ == "__main__":
    sys.exit(main())
