#!/usr/bin/env python
"""
Parse Playwright JSON report and output a concise failure summary.

Writes to $GITHUB_STEP_SUMMARY when available (CI), otherwise prints to stdout.
Usage: python scripts/demo-failure-summary.py <results.json> [--ci]
"""

import json
import os
import sys
from pathlib import Path


def extract_failures(data: dict) -> list[dict]:
    """Extract failed test suites/specs from Playwright JSON report."""
    failures = []

    for suite in data.get("suites", []):
        _walk_suite(suite, failures)

    return failures


def _walk_suite(suite: dict, failures: list[dict], parent_path: str = "") -> None:
    """Recursively walk suites to find failed tests."""
    suite_title = suite.get("title", "")
    current_path = f"{parent_path} > {suite_title}" if parent_path else suite_title

    for spec in suite.get("specs", []):
        for test in spec.get("tests", []):
            for result in test.get("results", []):
                if result.get("status") == "failed":
                    test_title = spec.get("title", "")
                    file = spec.get("file", "")
                    failures.append({
                        "file": file,
                        "suite_path": current_path,
                        "test_title": test_title,
                        "error": result.get("error", {}).get("message", "")[:200],
                        "duration_ms": result.get("duration", 0),
                    })

    for child in suite.get("suites", []):
        _walk_suite(child, failures, current_path)


def format_summary(failures: list[dict], stats: dict) -> str:
    """Format failure summary as markdown."""
    lines = []

    # Stats header
    total = stats.get("total", 0)
    passed = stats.get("passed", 0)
    failed = stats.get("failed", 0)
    skipped = stats.get("skipped", 0)
    lines.append(f"## Demo Test Results: {passed}/{total} passed")
    if failed:
        lines.append(f"**{failed} failed**, {skipped} skipped\n")
    else:
        lines.append("All tests passed!\n")

    if not failures:
        return "\n".join(lines)

    # Group by file
    by_file: dict[str, list[dict]] = {}
    for f in failures:
        by_file.setdefault(f["file"], []).append(f)

    lines.append("### Failed Tests\n")

    for file, tests in sorted(by_file.items()):
        lines.append(f"**{file}**")
        for t in tests:
            lines.append(f"- `{t['test_title']}`")
            if t["error"]:
                # Trim to first line of error
                first_line = t["error"].split("\n")[0].strip()
                if first_line:
                    lines.append(f"  > {first_line}")
        lines.append(f"  Re-run: `uv run scripts/demo-test-runner.py {file}`\n")

    # Quick re-run all command
    files = sorted(set(f["file"] for f in failures))
    lines.append("### Re-run All Failed\n")
    lines.append("```bash")
    for f in files:
        lines.append(f"uv run scripts/demo-test-runner.py {f}")
    lines.append("```\n")

    return "\n".join(lines)


def main() -> int:
    check_only = "--check-only" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("--")]

    if not args:
        print("Usage: python demo-failure-summary.py <results.json> [--check-only]")
        return 1

    report_path = Path(args[0])
    if not report_path.exists():
        print(f"No results file found at {report_path}")
        # Still write a summary saying tests didn't produce results
        summary = "## Demo Test Results\n\nNo Playwright JSON report found. Tests may have crashed before producing results.\n"
        _output_summary(summary)
        return 1

    try:
        data = json.loads(report_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, Exception) as e:
        print(f"Failed to parse report: {e}")
        return 1

    stats = {
        "total": len(data.get("suites", [])),
        "passed": 0,
        "failed": 0,
        "skipped": 0,
    }

    # Count from top-level stats if available
    if "stats" in data:
        stats = {
            "total": data["stats"].get("total", 0),
            "passed": data["stats"].get("expected", 0),
            "failed": data["stats"].get("unexpected", 0) + data["stats"].get("flaky", 0),
            "skipped": data["stats"].get("skipped", 0),
        }

    failures = extract_failures(data)
    stats["failed"] = max(stats["failed"], len(failures))

    if check_only:
        if failures:
            files = sorted(set(f["file"] for f in failures))
            print(f"FAILED: {len(failures)} test(s) in {len(files)} file(s)")
            for f in files:
                print(f"  - {f}")
            return 1
        return 0

    summary = format_summary(failures, stats)
    _output_summary(summary)

    return 0 if not failures else 1


def _output_summary(summary: str) -> None:
    """Write to $GITHUB_STEP_SUMMARY if in CI, else print to stdout."""
    step_summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if step_summary:
        Path(step_summary).write_text(summary, encoding="utf-8")
        print(summary)
    else:
        print(summary)


if __name__ == "__main__":
    sys.exit(main())
