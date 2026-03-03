#!/usr/bin/env python3
"""Generate crates metadata cache for SBOM generation.

This script parses Cargo.lock files and/or MODULE.bazel.lock files for
crate version/checksum data, then fetches license metadata via
dash-license-scan (Eclipse Foundation + ClearlyDefined) and creates a
cache file for SBOM generation.

Usage:
    python3 generate_crates_metadata_cache.py <output.json> --module-lock <MODULE.bazel.lock>
    python3 generate_crates_metadata_cache.py <output.json> --cargo-lock <Cargo.lock>
    python3 generate_crates_metadata_cache.py <output.json> --cargo-lock <Cargo.lock> --module-lock <MODULE.bazel.lock>

Example:
    python3 generate_crates_metadata_cache.py crates_metadata.json \\
        --module-lock ../../score-crates/MODULE.bazel.lock
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any


def parse_cargo_lock(lockfile_path: str) -> dict[str, dict[str, Any]]:
    """Parse Cargo.lock and extract crate information.

    Args:
        lockfile_path: Path to Cargo.lock file

    Returns:
        Dict mapping crate name to {version, checksum, source}
    """
    try:
        import tomllib as tomli  # Python 3.11+
    except ImportError:
        try:
            import tomli
        except ImportError:
            print(
                "ERROR: tomli/tomllib library not found. Use Python 3.11+ or install tomli",
                file=sys.stderr,
            )
            sys.exit(1)

    with open(lockfile_path, "rb") as f:
        lock_data = tomli.load(f)

    crates = {}
    for package in lock_data.get("package", []):
        name = package["name"]
        source = package.get("source", "")

        # Only include crates from crates.io
        if "registry+https://github.com/rust-lang/crates.io-index" in source:
            crates[name] = {
                "name": name,
                "version": package["version"],
                "checksum": package.get("checksum", ""),
                "source": source,
            }

    return crates


def parse_module_bazel_lock(lockfile_path: str) -> dict[str, dict[str, Any]]:
    """Parse MODULE.bazel.lock and extract crate information from cargo-bazel resolution.

    The MODULE.bazel.lock (from score_crates or similar) contains resolved crate
    specs under moduleExtensions -> crate_universe -> generatedRepoSpecs.
    Each crate entry has name, version, sha256, and download URL.

    Args:
        lockfile_path: Path to MODULE.bazel.lock file

    Returns:
        Dict mapping crate name to {version, checksum, source}
    """
    with open(lockfile_path, encoding="utf-8") as f:
        lock_data = json.load(f)

    crates = {}
    extensions = lock_data.get("moduleExtensions", {})

    # Find the crate_universe extension (key contains "crate_universe" or "crate")
    crate_ext = None
    for ext_key, ext_val in extensions.items():
        if "crate" in ext_key.lower():
            crate_ext = ext_val
            break

    if not crate_ext:
        print(
            "  WARNING: No crate extension found in MODULE.bazel.lock", file=sys.stderr
        )
        return crates

    # Get generatedRepoSpecs from 'general' (or the first available key)
    general = crate_ext.get("general", {})
    specs = general.get("generatedRepoSpecs", {})

    for repo_name, spec in specs.items():
        # Skip the crate_index meta-repo itself
        if repo_name == "crate_index" or not repo_name.startswith("crate_index__"):
            continue

        crate_part = repo_name.replace("crate_index__", "")

        # Parse name-version (e.g., "serde-1.0.228", "iceoryx2-qnx8-0.7.0")
        m = re.match(r"^(.+?)-(\d+\.\d+\.\d+.*)$", crate_part)
        if not m:
            continue

        name = m.group(1)
        version = m.group(2)
        attrs = spec.get("attributes", {})
        sha256 = attrs.get("sha256", "")

        crates[name] = {
            "name": name,
            "version": version,
            "checksum": sha256,
            "source": "module-bazel-lock",
        }

    return crates


def generate_synthetic_cargo_lock(
    crates: dict[str, dict[str, Any]], output_path: str
) -> None:
    """Generate a minimal synthetic Cargo.lock from parsed crate data.

    The dash-license-scan parser splits on '[[package]]' blocks and extracts
    name, version, and source fields. Source must contain 'crates' if present.

    Args:
        crates: Dict mapping crate name to {name, version, checksum, source}
        output_path: Path to write the synthetic Cargo.lock
    """
    lines = ["version = 4", ""]
    for _name, info in sorted(crates.items()):
        lines.append("[[package]]")
        lines.append(f'name = "{info["name"]}"')
        lines.append(f'version = "{info["version"]}"')
        lines.append('source = "registry+https://github.com/rust-lang/crates.io-index"')
        lines.append("")

    with open(output_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))


def _find_uvx() -> str:
    """Locate the uvx binary, checking PATH and common install locations."""
    found = shutil.which("uvx")
    if found:
        return found

    # Standard uv install location (works inside Bazel sandbox where PATH is minimal)
    home = os.environ.get("HOME", os.path.expanduser("~"))
    candidate = os.path.join(home, ".local", "bin", "uvx")
    if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
        return candidate

    return "uvx"  # fall back, will raise FileNotFoundError in subprocess


def run_dash_license_scan(cargo_lock_path: str, summary_output_path: str) -> None:
    """Invoke dash-license-scan via uvx and write summary to file.

    Args:
        cargo_lock_path: Path to (real or synthetic) Cargo.lock
        summary_output_path: Path to write the dash-licenses summary CSV

    Raises:
        SystemExit: If uvx/dash-license-scan is not found or fatally crashes
    """
    uvx = _find_uvx()
    cmd = [
        uvx,
        "--from",
        "dash-license-scan@git+https://github.com/eclipse-score/dash-license-scan",
        "dash-license-scan",
        "--summary",
        summary_output_path,
        cargo_lock_path,
    ]
    print(f"Running: {' '.join(cmd)}")

    # Redirect uv's cache and tool directories to writable temp locations.
    # Inside Bazel sandbox, ~/.cache and ~/.local/share are read-only.
    env = os.environ.copy()
    uv_tmp = os.path.join(tempfile.gettempdir(), "uv_sbom")
    if "UV_CACHE_DIR" not in env:
        env["UV_CACHE_DIR"] = os.path.join(uv_tmp, "cache")
    if "UV_TOOL_DIR" not in env:
        env["UV_TOOL_DIR"] = os.path.join(uv_tmp, "tools")

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=600,
            env=env,
        )
    except FileNotFoundError:
        print(
            "ERROR: 'uvx' not found on PATH or ~/.local/bin/. Install uv: https://docs.astral.sh/uv/",
            file=sys.stderr,
        )
        sys.exit(1)
    except subprocess.TimeoutExpired:
        print("ERROR: dash-license-scan timed out after 600 seconds", file=sys.stderr)
        sys.exit(1)

    # dash-license-scan exits with returncode = number of restricted items.
    # This is normal behavior, not an error. Only signal kills are fatal.
    if result.returncode < 0:
        print(
            f"ERROR: dash-license-scan killed by signal {-result.returncode}",
            file=sys.stderr,
        )
        if result.stderr:
            print(result.stderr, file=sys.stderr)
        sys.exit(1)

    if result.stderr:
        # Print dash-license-scan's own output (INFO lines from the JAR)
        for line in result.stderr.splitlines():
            print(f"  {line}")

    if not os.path.exists(summary_output_path):
        print(
            f"ERROR: dash-license-scan did not produce summary file: {summary_output_path}",
            file=sys.stderr,
        )
        sys.exit(1)

    if result.returncode > 0:
        print(f"  NOTE: {result.returncode} crate(s) have 'restricted' license status")


def parse_dash_summary(summary_path: str) -> dict[str, str]:
    """Parse the dash-licenses summary CSV file into a license lookup dict.

    Each line has format:
        crate/cratesio/-/<name>/<version>, <license_expr>, <status>, <source>

    Args:
        summary_path: Path to the dash-licenses summary file

    Returns:
        Dict mapping crate name to SPDX license expression string
    """
    licenses: dict[str, str] = {}
    with open(summary_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = [p.strip() for p in line.split(",")]
            if len(parts) < 4:
                continue

            content_id = parts[0]
            license_expr = parts[1].strip()

            # Extract crate name from content_id: "crate/cratesio/-/<name>/<version>"
            id_parts = content_id.split("/")
            if len(id_parts) >= 5 and id_parts[0] == "crate":
                crate_name = id_parts[3]
                if license_expr:
                    licenses[crate_name] = license_expr

    return licenses


def _extract_supplier(repository_url: str) -> str:
    """Extract supplier (GitHub org/user) from a repository URL.

    Examples:
        https://github.com/serde-rs/serde -> serde-rs
        https://github.com/eclipse-iceoryx/iceoryx2 -> eclipse-iceoryx
    """
    if not repository_url:
        return ""
    m = re.match(r"https?://github\.com/([^/]+)/", repository_url)
    return m.group(1) if m else ""


def _fetch_one_crate_meta(name: str) -> tuple[str, dict[str, str]]:
    """Fetch metadata for a single crate from crates.io API.

    Returns (name, {description, supplier}) dict.
    If the crate isn't found, retries with platform suffixes stripped
    (e.g. "-qnx8") to find the upstream crate.
    """
    candidates = [name]
    # Platform-specific forks (e.g. iceoryx2-bb-lock-free-qnx8 -> iceoryx2-bb-lock-free)
    for suffix in ("-qnx8",):
        if name.endswith(suffix):
            candidates.append(name[: -len(suffix)])

    for candidate in candidates:
        url = f"https://crates.io/api/v1/crates/{candidate}"
        req = urllib.request.Request(
            url,
            headers={"User-Agent": "score-sbom-tool (https://eclipse.dev/score)"},
        )
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read())
            crate = data.get("crate", {})
            desc = (crate.get("description") or "").strip()
            supplier = _extract_supplier(crate.get("repository", ""))
            if desc or supplier:
                return name, {"description": desc, "supplier": supplier}
        except Exception:
            continue
    return name, {}


def fetch_crate_metadata_from_cratesio(
    crate_names: list[str],
) -> dict[str, dict[str, str]]:
    """Fetch metadata (description, supplier) from crates.io API (parallel).

    Args:
        crate_names: List of crate names to look up

    Returns:
        Dict mapping crate name to {description, supplier}
    """
    total = len(crate_names)
    print(f"Fetching metadata from crates.io for {total} crates...")

    metadata: dict[str, dict[str, str]] = {}
    done = 0
    with ThreadPoolExecutor(max_workers=10) as pool:
        futures = {pool.submit(_fetch_one_crate_meta, n): n for n in crate_names}
        for future in as_completed(futures):
            name, meta = future.result()
            if meta:
                metadata[name] = meta
            done += 1
            if done % 50 == 0:
                print(f"  ... {done}/{total} crates queried")

    with_desc = sum(1 for m in metadata.values() if m.get("description"))
    with_supplier = sum(1 for m in metadata.values() if m.get("supplier"))
    print(
        f"Retrieved from crates.io: {with_desc} descriptions, {with_supplier} suppliers"
    )
    return metadata


def generate_cache(
    cargo_lock_path: str | None = None,
    module_lock_paths: list[str] | None = None,
) -> dict[str, dict[str, Any]]:
    """Generate metadata cache from lockfiles + dash-license-scan.

    1. Parse Cargo.lock and/or MODULE.bazel.lock files for crate names, versions, checksums
    2. Generate a synthetic Cargo.lock combining all crates
    3. Run dash-license-scan for license data
    4. Fetch descriptions from crates.io (parallel)
    5. Combine version/checksum from lockfile with license and description

    Args:
        cargo_lock_path: Optional path to Cargo.lock file
        module_lock_paths: Optional list of paths to MODULE.bazel.lock files

    Returns:
        Dict mapping crate name to metadata
    """
    crates: dict[str, dict[str, Any]] = {}

    if cargo_lock_path:
        print(f"Parsing {cargo_lock_path}...")
        crates = parse_cargo_lock(cargo_lock_path)
        print(f"Found {len(crates)} crates from Cargo.lock")

    # Merge crates from MODULE.bazel.lock files
    for module_lock_path in module_lock_paths or []:
        print(f"Parsing {module_lock_path}...")
        module_crates = parse_module_bazel_lock(module_lock_path)
        added = 0
        for name, info in module_crates.items():
            if name not in crates:
                crates[name] = info
                added += 1
        print(f"Found {len(module_crates)} crates in {module_lock_path} ({added} new)")

    if not crates:
        print("No crates found in lockfiles.")
        return {}

    # Generate synthetic Cargo.lock containing only crates.io crates.
    # This avoids dash-license-scan's ValueError on non-crates.io sources
    # (git dependencies, path dependencies) that may be in a real Cargo.lock.
    temp_dir = tempfile.mkdtemp(prefix="sbom_dash_")
    synthetic_path = os.path.join(temp_dir, "Cargo.lock")
    generate_synthetic_cargo_lock(crates, synthetic_path)
    print(f"Generated synthetic Cargo.lock with {len(crates)} crates")

    summary_path = os.path.join(temp_dir, "dash_summary.txt")

    try:
        print("Fetching license data via dash-license-scan...")
        run_dash_license_scan(synthetic_path, summary_path)
        license_map = parse_dash_summary(summary_path)
        print(f"Retrieved licenses for {len(license_map)} crates")
    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)

    # Fetch descriptions + suppliers from crates.io (parallel, ~10 concurrent requests)
    cratesio_meta = fetch_crate_metadata_from_cratesio(list(crates.keys()))

    # Build final cache
    cache: dict[str, dict[str, Any]] = {}
    for name, info in crates.items():
        meta = cratesio_meta.get(name, {})
        cache[name] = {
            "version": info["version"],
            "checksum": info["checksum"],
            "purl": f"pkg:cargo/{name}@{info['version']}",
            "license": license_map.get(name, ""),
            "description": meta.get("description", ""),
            "supplier": meta.get("supplier", ""),
        }

    return cache


def main():
    parser = argparse.ArgumentParser(
        description="Generate crates metadata cache for SBOM generation (via dash-license-scan)"
    )
    parser.add_argument(
        "output",
        nargs="?",
        default="crates_metadata.json",
        help="Output JSON file (default: crates_metadata.json)",
    )
    parser.add_argument("--cargo-lock", help="Path to Cargo.lock file")
    parser.add_argument(
        "--module-lock",
        action="append",
        default=[],
        help="Path to MODULE.bazel.lock for additional crates (can be repeated)",
    )
    parser.add_argument(
        "--merge", help="Merge with existing cache file instead of overwriting"
    )

    args = parser.parse_args()

    if not args.cargo_lock and not args.module_lock:
        parser.error("At least one of --cargo-lock or --module-lock is required")

    # Generate new cache
    cache = generate_cache(
        cargo_lock_path=args.cargo_lock,
        module_lock_paths=args.module_lock,
    )

    # Merge with existing cache if requested
    if args.merge and Path(args.merge).exists():
        print(f"\nMerging with existing cache: {args.merge}")
        with open(args.merge) as f:
            existing = json.load(f)

        # Prefer new data, but keep entries not in current lockfiles
        merged = existing.copy()
        merged.update(cache)
        cache = merged
        print(f"Merged cache now contains {len(cache)} entries")

    if not cache:
        print("\nNo crates to write.")
        with open(args.output, "w") as f:
            json.dump({}, f)
        return 0

    # Write cache
    print(f"\nWriting cache to {args.output}...")
    with open(args.output, "w") as f:
        json.dump(cache, f, indent=2, sort_keys=True)

    # Print statistics
    total = len(cache)
    with_license = sum(1 for c in cache.values() if c.get("license"))
    with_checksum = sum(1 for c in cache.values() if c.get("checksum"))
    with_desc = sum(1 for c in cache.values() if c.get("description"))
    with_supplier = sum(1 for c in cache.values() if c.get("supplier"))

    print(f"\n✓ Cache generated successfully!")
    print(f"  Total crates: {total}")
    print(f"  With licenses: {with_license} ({with_license / total * 100:.1f}%)")
    print(f"  With checksums: {with_checksum} ({with_checksum / total * 100:.1f}%)")
    print(f"  With descriptions: {with_desc} ({with_desc / total * 100:.1f}%)")
    print(f"  With suppliers: {with_supplier} ({with_supplier / total * 100:.1f}%)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
