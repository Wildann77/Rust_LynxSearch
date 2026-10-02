#!/usr/bin/env python3
"""LynxSearch Desktop Bundle Budget Checker.

Validates that client-side JS and CSS bundle sizes (after gzip compression)
do not exceed specified budget thresholds (JS < 450 KB, CSS < 50 KB).
"""

import argparse
import gzip
import os
import sys
from pathlib import Path


def get_gzip_size(file_path: Path) -> int:
    """Read file content and return size of gzip-compressed bytes."""
    with open(file_path, "rb") as f:
        data = f.read()
    return len(gzip.compress(data, compresslevel=9))


def format_size(bytes_count: int) -> str:
    """Format bytes into readable KB."""
    return f"{bytes_count / 1024:.2f} KB"


def check_budget(dist_dir: Path, max_js_kb: float, max_css_kb: float) -> bool:
    if not dist_dir.exists():
        print(f"Error: Distribution directory not found: {dist_dir}", file=sys.stderr)
        print("Please build frontend first (e.g. npm run build).", file=sys.stderr)
        return False

    js_files = []
    css_files = []

    for root, _, files in os.walk(dist_dir):
        for file in sorted(files):
            file_path = Path(root) / file
            if file.endswith(".js"):
                js_files.append(file_path)
            elif file.endswith(".css"):
                css_files.append(file_path)

    total_js_raw = sum(f.stat().st_size for f in js_files)
    total_js_gzip = sum(get_gzip_size(f) for f in js_files)
    total_css_raw = sum(f.stat().st_size for f in css_files)
    total_css_gzip = sum(get_gzip_size(f) for f in css_files)

    total_js_gzip_kb = total_js_gzip / 1024
    total_css_gzip_kb = total_css_gzip / 1024

    js_pass = total_js_gzip_kb <= max_js_kb
    css_pass = total_css_gzip_kb <= max_css_kb

    print("=" * 68)
    print("           LYNXSEARCH BUNDLE BUDGET AUDIT REPORT")
    print("=" * 68)
    print(f"Target Directory: {dist_dir}")
    print("-" * 68)
    print(f"{'Asset':<42} {'Raw':>12} {'Gzipped':>12}")
    print("-" * 68)

    for f in js_files:
        rel = f.relative_to(dist_dir)
        raw = format_size(f.stat().st_size)
        gz = format_size(get_gzip_size(f))
        print(f"{str(rel):<42} {raw:>12} {gz:>12}")

    for f in css_files:
        rel = f.relative_to(dist_dir)
        raw = format_size(f.stat().st_size)
        gz = format_size(get_gzip_size(f))
        print(f"{str(rel):<42} {raw:>12} {gz:>12}")

    print("-" * 68)
    print("BUDGET VERIFICATION:")
    js_status = "PASS" if js_pass else "FAIL"
    css_status = "PASS" if css_pass else "FAIL"

    print(
        f"  [JS]  Total Gzip: {total_js_gzip_kb:>7.2f} KB | Limit: {max_js_kb:>7.2f} KB -> [{js_status}]"
    )
    print(
        f"  [CSS] Total Gzip: {total_css_gzip_kb:>7.2f} KB | Limit: {max_css_kb:>7.2f} KB -> [{css_status}]"
    )
    print("=" * 68)

    if not (js_pass and css_pass):
        print("BUDGET EXCEEDED! Build rejected.", file=sys.stderr)
        return False

    print("ALL BUNDLE BUDGET CHECKS PASSED.")
    return True


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check LynxSearch frontend bundle size against gzip budgets."
    )
    parser.add_argument(
        "--dist",
        type=Path,
        default=Path("apps/desktop/dist"),
        help="Path to frontend dist directory (default: apps/desktop/dist)",
    )
    parser.add_argument(
        "--max-js",
        type=float,
        default=450.0,
        help="Max allowable total JS gzip size in KB (default: 450)",
    )
    parser.add_argument(
        "--max-css",
        type=float,
        default=50.0,
        help="Max allowable total CSS gzip size in KB (default: 50)",
    )

    args = parser.parse_args()
    success = check_budget(args.dist, args.max_js, args.max_css)
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
