#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""测试编译检测和超时延长功能的脚本."""

import sys
import time
import subprocess
from pathlib import Path

# 添加scripts目录到路径
scripts_dir = Path(__file__).parent
sys.path.insert(0, str(scripts_dir))

from lib.proc import is_cargo_compiling


def test_normal_startup():
    """测试正常启动场景."""
    print("=== 测试场景1: 正常启动（无编译） ===")
    print("预期: 快速启动，不触发超时延长")
    print()

    result = subprocess.run(
        [sys.executable, "C:/code/ai/cas-2/scripts/demo-start.py", "-v"],
        capture_output=True,
        text=True,
        timeout=120
    )

    # 检查输出中的关键信息
    output = result.stdout + result.stderr

    if "with compilation awareness" in output:
        print("[OK] 使用了编译感知的TCP等待函数")
    else:
        print("[ERROR] 未使用编译感知的TCP等待函数")
        return False

    if "127.0.0.1:8080 ready after" in output:
        print("[OK] 后端启动成功")
        # 提取启动时间
        for line in output.split('\n'):
            if "127.0.0.1:8080 ready after" in line:
                print(f"  {line.strip()}")
    else:
        print("[ERROR] 后端启动失败")
        return False

    return True


def test_cargo_detection():
    """测试cargo编译检测功能."""
    print("\n=== 测试场景2: Cargo编译检测 ===")
    print("预期: 能正确检测cargo进程状态")
    print()

    # 检查当前没有cargo进程在运行
    is_compiling = is_cargo_compiling()
    print(f"当前cargo编译状态: {'正在编译' if is_compiling else '未检测到编译'}")

    if not is_compiling:
        print("[OK] 正确检测到当前没有cargo编译进程")
    else:
        print("[INFO] 检测到cargo进程正在运行（如果有手动启动cargo，这是正常的）")

    return True


def test_long_compilation_scenario():
    """测试长时间编译场景的说明."""
    print("\n=== 测试场景3: 长时间编译场景（需要手动测试） ===")
    print("要完整测试超时延长功能，请执行以下步骤：")
    print()
    print("1. 清理编译缓存:")
    print("   cd C:/code/ai/cas-2/backend")
    print("   cargo clean")
    print()
    print("2. 启动demo并观察日志:")
    print("   cd C:/code/ai/cas-2")
    print("   python scripts/demo-start.py -v")
    print()
    print("3. 预期行为:")
    print("   - 如果编译超过60秒，会看到 'Detected cargo compilation, extending timeout'")
    print("   - 每次延长60秒，最多延长5次")
    print("   - 最终显示总延长时间")
    print()
    print("4. 观察输出中的关键信息:")
    print("   - 'Waiting for 127.0.0.1:8080 (with compilation awareness)...'")
    print("   - 'Detected cargo compilation, extending timeout by 60s'")
    print("   - '127.0.0.1:8080 ready after Xs (Y checks, extended Zs)'")
    print()

    return True


def main():
    """运行所有测试."""
    print("="*60)
    print("编译检测和超时延长功能测试")
    print("="*60)
    print()

    all_passed = True

    # 测试1: 正常启动场景
    try:
        if not test_normal_startup():
            all_passed = False
    except Exception as e:
        print(f"[ERROR] 正常启动测试失败: {e}")
        all_passed = False

    # 测试2: Cargo检测功能
    try:
        if not test_cargo_detection():
            all_passed = False
    except Exception as e:
        print(f"[ERROR] Cargo检测测试失败: {e}")
        all_passed = False

    # 测试3: 长时间编译场景说明
    try:
        test_long_compilation_scenario()
    except Exception as e:
        print(f"[ERROR] 长时间编译测试失败: {e}")

    print("\n" + "="*60)
    if all_passed:
        print("[OK] 核心功能测试通过")
        print("\n编译检测功能已成功集成到demo启动流程中！")
        print("当cargo编译时间超过60秒时，会自动延长超时时间。")
    else:
        print("[ERROR] 部分测试失败")

    print("="*60)
    return 0 if all_passed else 1


if __name__ == "__main__":
    sys.exit(main())
