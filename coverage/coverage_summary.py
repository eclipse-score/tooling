#!/usr/bin/env python3
# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************
"""Render a markdown coverage summary from the pipeline's LCOV output.

Invoked by generate_coverage_html.sh to produce a human-readable summary for
GitHub job summary pages (GITHUB_STEP_SUMMARY) or an arbitrary markdown file
(--summary-md). Standard library only.

Inputs:
    --lcov <path>                  LCOV trace produced by the coverage reporter
                                   (includes exact-0% baseline records).
    --justification-report <path>  Optional report.json from effective_coverage.py
                                   (raw vs effective metrics, justified counts).
    --output <path>                Markdown destination.
    --append                       Append to --output instead of overwriting
                                   (GITHUB_STEP_SUMMARY convention).
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Optional

BAR_WIDTH = 10
LEAST_COVERED_LIMIT = 15


class FileCoverage:
    """Line/branch counters for one SF record."""

    def __init__(self, path: str) -> None:
        self.path = path
        self.lines_found = 0
        self.lines_hit = 0
        # None means "no branch data in the record" (rendered as an em dash).
        self.branches_found: Optional[int] = None
        self.branches_hit: Optional[int] = None

    @property
    def line_pct(self) -> Optional[float]:
        return percent(self.lines_hit, self.lines_found)


def percent(hit: int, total: int) -> Optional[float]:
    """Percentage, or None when the denominator is zero."""
    if total <= 0:
        return None
    return 100.0 * hit / total


def fmt_pct(pct: Optional[float]) -> str:
    return "—" if pct is None else f"{pct:.2f}%"


def progress_bar(pct: Optional[float], width: int = BAR_WIDTH) -> str:
    """Inline-code text progress bar, e.g. `███████░░░`."""
    if pct is None:
        return "—"
    filled = int(round(pct / 100.0 * width))
    filled = max(0, min(width, filled))
    return "`" + "█" * filled + "░" * (width - filled) + "`"


def escape_cell(text: str) -> str:
    """Make a path safe inside a markdown table cell."""
    return text.replace("|", "\\|")


def parse_lcov(path: Path) -> Optional[List[FileCoverage]]:
    """Parse an LCOV trace into per-file counters.

    Returns None when the file does not exist; an empty list when it exists
    but contains no records. Branch data prefers BRF/BRH sums and falls back
    to counting BRDA entries (taken > 0 counts as hit).
    """
    if not path.is_file():
        return None

    files: List[FileCoverage] = []
    current: Optional[FileCoverage] = None
    brf = brh = 0
    brda_total = brda_hit = 0
    saw_brf = False

    def flush() -> None:
        nonlocal current, brf, brh, brda_total, brda_hit, saw_brf
        if current is not None:
            if saw_brf:
                current.branches_found = brf
                current.branches_hit = brh
            elif brda_total > 0:
                current.branches_found = brda_total
                current.branches_hit = brda_hit
            files.append(current)
        current = None
        brf = brh = 0
        brda_total = brda_hit = 0
        saw_brf = False

    # errors="replace" keeps non-UTF8 bytes in paths from crashing the parse.
    with open(path, encoding="utf-8", errors="replace") as f:
        for raw_line in f:
            line = raw_line.strip()
            if line.startswith("SF:"):
                flush()
                current = FileCoverage(line[3:])
            elif current is None:
                continue
            elif line.startswith("LF:"):
                current.lines_found += _int_suffix(line)
            elif line.startswith("LH:"):
                current.lines_hit += _int_suffix(line)
            elif line.startswith("BRF:"):
                saw_brf = True
                brf += _int_suffix(line)
            elif line.startswith("BRH:"):
                saw_brf = True
                brh += _int_suffix(line)
            elif line.startswith("BRDA:"):
                # BRDA:<line>,<block>,<branch>,<taken|-> — "-" means never
                # evaluated, any positive count means taken.
                brda_total += 1
                taken = line.rsplit(",", 1)[-1]
                if taken not in ("-", "0"):
                    brda_hit += 1
            elif line == "end_of_record":
                flush()
    flush()
    return files


def _int_suffix(line: str) -> int:
    try:
        return int(line.split(":", 1)[1])
    except (IndexError, ValueError):
        return 0


def load_justification_summary(path: Path) -> Optional[Dict]:
    """Load the summary block of effective_coverage.py's report.json."""
    try:
        with open(path, encoding="utf-8") as f:
            report = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(f"WARNING: could not read justification report {path}: {e}", file=sys.stderr)
        return None
    summary = report.get("summary")
    if not isinstance(summary, dict):
        return None
    summary = dict(summary)
    applied = report.get("applied_justifications")
    summary["applied_justification_count"] = len(applied) if isinstance(applied, list) else 0
    return summary


def directory_key(path: str) -> str:
    """Group by the first one or two path segments (generic, layout-agnostic)."""
    parts = path.split("/")
    if len(parts) <= 1:
        return "(root)"
    if len(parts) == 2:
        return parts[0]
    return "/".join(parts[:2])


def rollup_by_directory(files: List[FileCoverage]) -> List[Dict]:
    groups: Dict[str, Dict] = {}
    for fc in files:
        g = groups.setdefault(
            directory_key(fc.path),
            {"lines_found": 0, "lines_hit": 0, "files": 0},
        )
        g["lines_found"] += fc.lines_found
        g["lines_hit"] += fc.lines_hit
        g["files"] += 1
    rows = [
        {
            "directory": name,
            "pct": percent(g["lines_hit"], g["lines_found"]),
            **g,
        }
        for name, g in groups.items()
    ]
    # Worst first; groups without countable lines sink to the end.
    rows.sort(key=lambda r: (r["pct"] is None, r["pct"], r["directory"]))
    return rows


def render_markdown(files: List[FileCoverage], justification: Optional[Dict]) -> str:
    out: List[str] = ["## Coverage summary", ""]

    if not files:
        out.append("_No coverage records found in the LCOV report._")
        out.append("")
        return "\n".join(out)

    total_lf = sum(f.lines_found for f in files)
    total_lh = sum(f.lines_hit for f in files)
    branch_files = [f for f in files if f.branches_found is not None]
    total_brf = sum(f.branches_found for f in branch_files) if branch_files else 0
    total_brh = sum(f.branches_hit for f in branch_files) if branch_files else 0
    touched = [f for f in files if f.lines_hit > 0]
    zero = [f for f in files if f.lines_found > 0 and f.lines_hit == 0]

    line_pct = percent(total_lh, total_lf)
    branch_pct = percent(total_brh, total_brf) if branch_files else None
    touched_pct = percent(len(touched), len(files))

    out.append("| Metric | Covered | Total | % | |")
    out.append("|---|---:|---:|---:|---|")
    out.append(f"| Lines | {total_lh} | {total_lf} | {fmt_pct(line_pct)} | {progress_bar(line_pct)} |")
    out.append(
        f"| Branches | {total_brh if branch_files else '—'} | "
        f"{total_brf if branch_files else '—'} | {fmt_pct(branch_pct)} | {progress_bar(branch_pct)} |"
    )
    out.append(
        f"| Files with coverage | {len(touched)} | {len(files)} | "
        f"{fmt_pct(touched_pct)} | {progress_bar(touched_pct)} |"
    )
    out.append(f"| Files at exact 0% | {len(zero)} | {len(files)} | | |")
    out.append("")

    if justification is not None:
        out.append("### Raw vs effective (justifications applied)")
        out.append("")
        out.append("| Metric | Raw | Effective |")
        out.append("|---|---:|---:|")
        out.append(
            f"| Line coverage | {justification.get('raw_line_coverage_pct', 0)}% "
            f"| {justification.get('effective_line_coverage_pct', 0)}% |"
        )
        out.append(
            f"| Branch coverage | {justification.get('raw_branch_coverage_pct', 0)}% "
            f"| {justification.get('effective_branch_coverage_pct', 0)}% |"
        )
        out.append("")
        out.append(
            f"Justified: {justification.get('justified_lines', 0)} lines, "
            f"{justification.get('justified_branches', 0)} branches "
            f"({justification.get('applied_justification_count', 0)} justification entries applied, "
            f"{justification.get('stale_justifications', 0)} stale)."
        )
        out.append("")

    out.append("### Coverage by directory (worst first)")
    out.append("")
    out.append("| Directory | Files | Lines hit/total | % | |")
    out.append("|---|---:|---:|---:|---|")
    for row in rollup_by_directory(files):
        out.append(
            f"| {escape_cell(row['directory'])} | {row['files']} "
            f"| {row['lines_hit']}/{row['lines_found']} "
            f"| {fmt_pct(row['pct'])} | {progress_bar(row['pct'])} |"
        )
    out.append("")

    least = sorted(
        (f for f in touched if f.lines_hit < f.lines_found),
        key=lambda f: (f.line_pct is None, f.line_pct, f.path),
    )[:LEAST_COVERED_LIMIT]
    if least:
        out.append("<details>")
        out.append(f"<summary>Least-covered files with coverage (top {len(least)})</summary>")
        out.append("")
        out.append("| File | Lines hit/total | % | |")
        out.append("|---|---:|---:|---|")
        for fc in least:
            out.append(
                f"| {escape_cell(fc.path)} | {fc.lines_hit}/{fc.lines_found} "
                f"| {fmt_pct(fc.line_pct)} | {progress_bar(fc.line_pct)} |"
            )
        out.append("")
        out.append("</details>")
        out.append("")

    if zero:
        out.append("<details>")
        out.append(f"<summary>Files at exact 0% ({len(zero)})</summary>")
        out.append("")
        for fc in sorted(zero, key=lambda f: f.path):
            out.append(f"- `{fc.path}` ({fc.lines_found} lines)")
        out.append("")
        out.append("</details>")
        out.append("")

    out.append("_Full per-line HTML report: download the coverage artifact of this run._")
    out.append("")
    return "\n".join(out)


def main() -> None:
    parser = argparse.ArgumentParser(description="Markdown coverage summary from LCOV")
    parser.add_argument("--lcov", type=Path, required=True)
    parser.add_argument("--justification-report", type=Path, default=None)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--append", action="store_true")
    args = parser.parse_args()

    files = parse_lcov(args.lcov)
    if files is None:
        print(f"WARNING: LCOV file not found: {args.lcov}", file=sys.stderr)
        files = []

    justification = None
    if args.justification_report is not None:
        justification = load_justification_summary(args.justification_report)

    markdown = render_markdown(files, justification)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    mode = "a" if args.append else "w"
    with open(args.output, mode, encoding="utf-8") as f:
        f.write(markdown)
    print(
        f"INFO: coverage summary {'appended to' if args.append else 'written to'} {args.output}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
