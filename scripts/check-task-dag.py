#!/usr/bin/env python3
"""Check generated task Markdown files for cyclic item dependencies.

The task files use a few stable dependency formats:
- Markdown tables with an `id` column and a `depends_on` column.
- Front-matter-like lines such as `depends_on: [BE-D01]`.
- Bullet lines such as `- **depends_on**: [BE-T08]`.
- Explicit DAG edge lists such as `- BE-D01 -> BE-D02` or `BE-D01 → BE-D02`.

Internally, every dependency is normalized to an execution edge:
`dependency -> dependent`.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict, deque
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

PROJECT_ROOT = Path(__file__).parent.parent
DEFAULT_ROOT = PROJECT_ROOT / ".ai" / "task"

ITEM_ID_PATTERN = re.compile(r"\b[A-Z]{2}-[DTAFRM][A-Za-z0-9]*\b")
ARROW_PATTERN = re.compile(
    r"\b(?P<left>[A-Z]{2}-[DTAFRM][A-Za-z0-9]*)\b\s*(?:->|→|-->|─+>|=+>)\s*"
    r"\b(?P<right>[A-Z]{2}-[DTAFRM][A-Za-z0-9]*)\b"
)
DEPENDS_KEY_PATTERN = re.compile(
    r"^\s*(?:[-*]\s*)?(?:\*\*)?depends_on(?:\*\*)?\s*:\s*(?P<value>.*)$",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class Edge:
    prerequisite: str
    dependent: str
    source: Path
    line_number: int
    origin: str


def repo_relative(path: Path) -> str:
    try:
        return path.resolve().relative_to(PROJECT_ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def strip_inline_code(text: str) -> str:
    result: list[str] = []
    in_backtick = False
    for index, char in enumerate(text):
        if char == "`" and (index == 0 or text[index - 1] != "\\"):
            in_backtick = not in_backtick
            continue
        if not in_backtick:
            result.append(char)
    return "".join(result)


def split_markdown_table_row(line: str) -> list[str] | None:
    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        return None
    return [cell.strip() for cell in stripped.strip("|").split("|")]


def is_markdown_table_separator(cells: list[str]) -> bool:
    return bool(cells) and all(re.fullmatch(r":?-{3,}:?", cell.strip()) for cell in cells)


def extract_ids(text: str) -> list[str]:
    seen: set[str] = set()
    ids: list[str] = []
    for match in ITEM_ID_PATTERN.finditer(text):
        item_id = match.group(0)
        if item_id not in seen:
            seen.add(item_id)
            ids.append(item_id)
    return ids


def extract_dependency_ids(value: str) -> list[str]:
    """Extract IDs from an explicit dependency value.

    If a bracketed list is present, only the list content is authoritative.
    This keeps notes such as `[] (independent of FE-D02)` from creating edges.
    """
    bracket = re.search(r"\[(?P<items>[^\]]*)\]", value)
    if bracket:
        return extract_ids(bracket.group("items"))
    if value.strip() in {"", "—", "-", "none", "None", "NONE"}:
        return []
    return extract_ids(value)


def parse_table_edges(lines: list[str], source: Path) -> list[Edge]:
    edges: list[Edge] = []
    index = 0
    while index < len(lines):
        header = split_markdown_table_row(lines[index])
        if not header:
            index += 1
            continue

        normalized_header = [cell.lower().replace(" ", "_") for cell in header]
        if "id" not in normalized_header or "depends_on" not in normalized_header:
            index += 1
            continue

        separator_index = index + 1
        separator = split_markdown_table_row(lines[separator_index]) if separator_index < len(lines) else None
        if not separator or not is_markdown_table_separator(separator):
            index += 1
            continue

        id_column = normalized_header.index("id")
        depends_column = normalized_header.index("depends_on")
        row_index = index + 2
        while row_index < len(lines):
            cells = split_markdown_table_row(lines[row_index])
            if not cells or len(cells) <= max(id_column, depends_column):
                break

            item_ids = extract_ids(cells[id_column])
            if item_ids:
                dependent = item_ids[0]
                for prerequisite in extract_ids(cells[depends_column]):
                    if prerequisite != dependent:
                        edges.append(
                            Edge(
                                prerequisite=prerequisite,
                                dependent=dependent,
                                source=source,
                                line_number=row_index + 1,
                                origin="table depends_on",
                            )
                        )
            row_index += 1

        index = row_index
    return edges


def parse_depends_line_edges(lines: list[str], source: Path) -> list[Edge]:
    edges: list[Edge] = []
    current_item = infer_item_id_from_filename(source)

    for line_number, line in enumerate(lines, 1):
        searchable = strip_inline_code(line)
        ids = extract_ids(searchable)
        if ids and re.match(r"^\s*#\s+", searchable):
            current_item = ids[0]

        depends_match = DEPENDS_KEY_PATTERN.match(searchable)
        if not depends_match:
            continue

        dependent_candidates = extract_ids(source.stem)
        dependent = dependent_candidates[0] if dependent_candidates else current_item
        if not dependent:
            continue

        for prerequisite in extract_dependency_ids(depends_match.group("value")):
            if prerequisite != dependent:
                edges.append(
                    Edge(
                        prerequisite=prerequisite,
                        dependent=dependent,
                        source=source,
                        line_number=line_number,
                        origin="depends_on line",
                    )
                )
    return edges


def parse_depends_section_edges(lines: list[str], source: Path) -> list[Edge]:
    dependent = infer_item_id_from_filename(source)
    if not dependent:
        return []

    edges: list[Edge] = []
    in_section = False
    for line_number, line in enumerate(lines, 1):
        stripped = line.strip()
        if re.match(r"^#{2,}\s+depends_on\s*$", stripped, re.IGNORECASE):
            in_section = True
            continue
        if in_section and stripped.startswith("#"):
            in_section = False
        if not in_section:
            continue

        section_line = strip_inline_code(line).strip()
        if not section_line or section_line.startswith("("):
            continue
        for prerequisite in extract_dependency_ids(section_line):
            if prerequisite != dependent:
                edges.append(
                    Edge(
                        prerequisite=prerequisite,
                        dependent=dependent,
                        source=source,
                        line_number=line_number,
                        origin="depends_on section",
                    )
                )
    return edges


def parse_arrow_edges(lines: list[str], source: Path) -> list[Edge]:
    edges: list[Edge] = []
    in_fenced_block = False
    for line_number, line in enumerate(lines, 1):
        stripped = line.strip()
        if stripped.startswith("```"):
            in_fenced_block = not in_fenced_block
            continue

        searchable = line if in_fenced_block else strip_inline_code(line)
        for match in ARROW_PATTERN.finditer(searchable):
            left = match.group("left")
            right = match.group("right")
            if left != right:
                edges.append(
                    Edge(
                        prerequisite=left,
                        dependent=right,
                        source=source,
                        line_number=line_number,
                        origin="arrow edge",
                    )
                )
    return edges


def infer_item_id_from_filename(path: Path) -> str | None:
    ids = extract_ids(path.stem)
    return ids[0] if ids else None


def parse_file(path: Path) -> list[Edge]:
    lines = path.read_text(encoding="utf-8").splitlines()
    edges: list[Edge] = []
    edges.extend(parse_table_edges(lines, path))
    edges.extend(parse_depends_line_edges(lines, path))
    edges.extend(parse_depends_section_edges(lines, path))
    edges.extend(parse_arrow_edges(lines, path))
    return dedupe_edges(edges)


def infer_scope(path: Path) -> str:
    """Group task files by project/phase/slot so reused item IDs do not collide."""
    try:
        rel = path.resolve().relative_to(DEFAULT_ROOT)
    except ValueError:
        return repo_relative(path.parent)

    parts = rel.parts
    if len(parts) >= 3:
        project = parts[0]
        phase = parts[1]
        third = parts[2]
        if third in {"dev.md", "test.md", "accept.md", "finalize.md", "index.md"}:
            return f"{project}/{phase}/{Path(third).stem}"
        if third in {"dev", "test", "accept", "demo"}:
            return f"{project}/{phase}/{third}"
        return f"{project}/{phase}"
    if len(parts) >= 2:
        return "/".join(parts[:2])
    return repo_relative(path.parent)


def dedupe_edges(edges: Iterable[Edge]) -> list[Edge]:
    seen: set[tuple[str, str, Path, int, str]] = set()
    unique: list[Edge] = []
    for edge in edges:
        key = (edge.prerequisite, edge.dependent, edge.source, edge.line_number, edge.origin)
        if key in seen:
            continue
        seen.add(key)
        unique.append(edge)
    return unique


def find_markdown_files(paths: list[Path]) -> list[Path]:
    files: list[Path] = []
    for path in paths:
        resolved = path if path.is_absolute() else PROJECT_ROOT / path
        if resolved.is_file() and resolved.suffix.lower() == ".md":
            files.append(resolved)
        elif resolved.is_dir():
            files.extend(sorted(resolved.rglob("*.md")))
    return sorted({file.resolve() for file in files})


def build_graph(edges: list[Edge]) -> tuple[dict[str, set[str]], dict[str, set[str]]]:
    outgoing: dict[str, set[str]] = defaultdict(set)
    incoming: dict[str, set[str]] = defaultdict(set)
    for edge in edges:
        outgoing[edge.prerequisite].add(edge.dependent)
        incoming[edge.dependent].add(edge.prerequisite)
        outgoing.setdefault(edge.dependent, set())
        incoming.setdefault(edge.prerequisite, set())
    return outgoing, incoming


def find_cycle(edges: list[Edge]) -> list[str] | None:
    outgoing, _incoming = build_graph(edges)
    visiting: set[str] = set()
    visited: set[str] = set()
    stack: list[str] = []

    def visit(node: str) -> list[str] | None:
        if node in visiting:
            start = stack.index(node)
            return stack[start:] + [node]
        if node in visited:
            return None

        visiting.add(node)
        stack.append(node)
        for next_node in sorted(outgoing[node]):
            cycle = visit(next_node)
            if cycle:
                return cycle
        stack.pop()
        visiting.remove(node)
        visited.add(node)
        return None

    for node in sorted(outgoing):
        cycle = visit(node)
        if cycle:
            return cycle
    return None


def topological_order(edges: list[Edge]) -> list[str]:
    outgoing, incoming = build_graph(edges)
    ready = deque(sorted(node for node in outgoing if not incoming[node]))
    order: list[str] = []

    while ready:
        node = ready.popleft()
        order.append(node)
        for dependent in sorted(outgoing[node]):
            incoming[dependent].remove(node)
            if not incoming[dependent]:
                ready.append(dependent)

    return order


def format_edge(edge: Edge) -> str:
    return (
        f"{edge.prerequisite} -> {edge.dependent} "
        f"({repo_relative(edge.source)}:{edge.line_number}, {edge.origin})"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Check .ai task Markdown dependency DAGs for cycles.")
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        default=[DEFAULT_ROOT],
        help="Markdown file or directory to scan. Defaults to .ai/task.",
    )
    parser.add_argument("--show-edges", action="store_true", help="Print parsed dependency edges.")
    parser.add_argument(
        "--global",
        dest="global_scope",
        action="store_true",
        help="Check all parsed edges as one graph. By default, edges are checked per task/phase/slot.",
    )
    args = parser.parse_args()

    files = find_markdown_files(args.paths)
    if not files:
        print("No Markdown files found.", file=sys.stderr)
        return 2

    edges_by_scope: dict[str, list[Edge]] = defaultdict(list)
    for file in files:
        scope = "__global__" if args.global_scope else infer_scope(file)
        edges_by_scope[scope].extend(parse_file(file))
    edges_by_scope = {scope: dedupe_edges(edges) for scope, edges in edges_by_scope.items()}

    if args.show_edges:
        for scope in sorted(edges_by_scope):
            scope_edges = edges_by_scope[scope]
            if not scope_edges:
                continue
            print(f"[{scope}]")
            for edge in sorted(
                scope_edges,
                key=lambda item: (repo_relative(item.source), item.line_number, item.prerequisite, item.dependent),
            ):
                print(format_edge(edge))

    edges = [edge for scope_edges in edges_by_scope.values() for edge in scope_edges]
    if not edges:
        print(f"Checked {len(files)} Markdown files; no dependency edges found.")
        return 0

    failed = False
    for scope, scope_edges in sorted(edges_by_scope.items()):
        if not scope_edges:
            continue
        cycle = find_cycle(scope_edges)
        if cycle:
            failed = True
            print(f"Task DAG cycle detected in scope {scope}:", file=sys.stderr)
            print(" -> ".join(cycle), file=sys.stderr)
            print("\nRelevant parsed edges:", file=sys.stderr)
            cycle_pairs = set(zip(cycle, cycle[1:]))
            for edge in scope_edges:
                if (edge.prerequisite, edge.dependent) in cycle_pairs:
                    print(f"- {format_edge(edge)}", file=sys.stderr)
            print("", file=sys.stderr)
    if failed:
        return 1

    print(
        f"Task DAG check passed: {len(files)} Markdown files, "
        f"{len(edges)} dependency edges, {len(edges_by_scope)} scopes, no cycles."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
