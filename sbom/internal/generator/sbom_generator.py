#!/usr/bin/env python3
"""SBOM generator - creates SPDX and CycloneDX output from Bazel aspect data.

This is the main entry point for SBOM generation. It reads dependency
information collected by the Bazel aspect and metadata from the module
extension, then generates SBOM files in SPDX 2.3 and CycloneDX 1.6 formats.
"""

import argparse
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from sbom.internal.generator.spdx_formatter import generate_spdx
from sbom.internal.generator.cyclonedx_formatter import generate_cyclonedx


def parse_module_bazel_files(file_paths: list[str]) -> dict[str, dict[str, str]]:
    """Parse MODULE.bazel files to extract module name and version.

    Reads each MODULE.bazel file and extracts the module() call's name and
    version fields. This allows automatic version detection for bazel_dep
    modules that don't appear in the sbom_metadata extension's module list
    (because they don't use_extension for sbom_metadata).

    Args:
        file_paths: List of paths to MODULE.bazel files

    Returns:
        Dict mapping module name to {"version": ..., "purl": ...}
    """
    modules: dict[str, dict[str, str]] = {}
    for fpath in file_paths:
        try:
            with open(fpath, encoding="utf-8") as f:
                content = f.read()
        except OSError:
            continue

        # Extract module(name = "...", version = "...")
        module_match = re.search(
            r"module\s*\((.*?)\)",
            content,
            re.DOTALL,
        )
        if not module_match:
            continue

        module_block = module_match.group(1)
        name_match = re.search(r'name\s*=\s*["\']([^"\']+)["\']', module_block)
        version_match = re.search(r'version\s*=\s*["\']([^"\']+)["\']', module_block)

        if name_match and version_match:
            name = name_match.group(1)
            version = version_match.group(1)
            modules[name] = {
                "version": version,
                "purl": f"pkg:generic/{name}@{version}",
            }

    return modules


def parse_module_lockfiles(file_paths: list[str]) -> dict[str, dict[str, str]]:
    """Parse MODULE.bazel.lock files to infer module versions and checksums.

    Uses registry URL keys from lockfiles. Only modules with a single unique
    observed version are emitted to avoid ambiguous version selection.

    For modules coming from the Bazel Central Registry, this also extracts the
    SHA-256 checksum from the corresponding ``source.json`` entry so that
    CycloneDX hashes can be populated for C/C++ dependencies.
    """
    # Track all observed versions per module and (optional) sha256 per
    # (module, version) tuple.
    module_versions: dict[str, set[str]] = {}
    module_sha256: dict[tuple[str, str], str] = {}

    for fpath in file_paths:
        try:
            with open(fpath, encoding="utf-8") as f:
                lock_data = json.load(f)
        except (OSError, json.JSONDecodeError):
            continue

        registry_hashes = lock_data.get("registryFileHashes", {})
        if not isinstance(registry_hashes, dict):
            continue

        for url, sha in registry_hashes.items():
            if not isinstance(url, str) or not isinstance(sha, str):
                continue

            # MODULE.bazel entry – records which version was selected.
            module_match = re.search(
                r"/modules/([^/]+)/([^/]+)/MODULE\.bazel$",
                url,
            )
            if module_match:
                module_name, version = module_match.groups()
                module_versions.setdefault(module_name, set()).add(version)

            # source.json entry – carries the sha256 of the downloaded source
            # tarball for this module@version. Use it as the component hash.
            source_match = re.search(
                r"/modules/([^/]+)/([^/]+)/source\.json$",
                url,
            )
            if source_match:
                src_module, src_version = source_match.groups()
                module_sha256[(src_module, src_version)] = sha

    modules: dict[str, dict[str, str]] = {}
    for name, versions in module_versions.items():
        if len(versions) != 1:
            # Skip modules with ambiguous versions.
            continue
        version = next(iter(versions))
        entry: dict[str, str] = {
            "version": version,
            "purl": f"pkg:generic/{name}@{version}",
        }
        sha = module_sha256.get((name, version))
        if sha:
            # Expose as sha256 so downstream code can turn it into a CycloneDX
            # SHA-256 hash entry.
            entry["sha256"] = sha
        modules[name] = entry

    return modules


def load_crates_cache(cache_path: str | None = None) -> dict[str, Any]:
    """Load crates metadata cache generated at build time.

    Args:
        cache_path: Path to crates_metadata.json (from --crates-cache)

    Returns:
        Dict mapping crate name to metadata (license, checksum, etc.)
    """
    if not cache_path:
        return {}
    try:
        with open(cache_path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError):
        return {}


# Known licenses for Bazel Central Registry (BCR) C++ modules.
# Used as a fallback when cdxgen and lockfile parsing cannot provide license data.
# Keys are BCR module names (exact or prefix for sub-modules like boost.*).
BCR_KNOWN_LICENSES: dict[str, dict[str, str]] = {
    "boost": {"license": "BSL-1.0", "supplier": "Boost.org"},
    "abseil-cpp": {"license": "Apache-2.0", "supplier": "Google LLC"},
    "zlib": {"license": "Zlib", "supplier": "Jean-loup Gailly and Mark Adler"},
    "nlohmann_json": {"license": "MIT", "supplier": "Niels Lohmann"},
    "nlohmann-json": {"license": "MIT", "supplier": "Niels Lohmann"},
    "googletest": {"license": "BSD-3-Clause", "supplier": "Google LLC"},
    "google-benchmark": {"license": "Apache-2.0", "supplier": "Google LLC"},
    "flatbuffers": {"license": "Apache-2.0", "supplier": "Google LLC"},
    "protobuf": {"license": "BSD-3-Clause", "supplier": "Google LLC"},
    "re2": {"license": "BSD-3-Clause", "supplier": "Google LLC"},
    "openssl": {"license": "Apache-2.0", "supplier": "OpenSSL Software Foundation"},
    "curl": {"license": "curl", "supplier": "Daniel Stenberg"},
    "libpng": {"license": "libpng", "supplier": "Glenn Randers-Pehrson"},
    "libjpeg": {"license": "IJG", "supplier": "Independent JPEG Group"},
}


def apply_known_licenses(metadata: dict[str, Any]) -> None:
    """Apply BCR known licenses and user license overrides to modules.

    Priority (highest to lowest):
    1. Module already has a license (skip).
    2. Exact match in metadata["licenses"] (user-declared via sbom_ext.license).
    3. Parent match in metadata["licenses"] (e.g., "boost" covers "boost.config").
    4. BCR_KNOWN_LICENSES exact match.
    5. BCR_KNOWN_LICENSES parent match (e.g., "boost" entry covers "boost.config").

    Args:
        metadata: Metadata dict with "modules" and "licenses" keys. Modified in place.
    """
    modules = metadata.get("modules", {})
    licenses = metadata.get("licenses", {})

    for module_name, module_data in modules.items():
        if module_data.get("license"):
            continue  # Already has a license — do not overwrite

        license_source: dict[str, str] | None = None

        # 1. Exact match in user-declared licenses (highest priority)
        if module_name in licenses:
            license_source = licenses[module_name]
        # 2. Parent match in user-declared licenses (e.g. "boost" → "boost.config")
        elif "." in module_name:
            parent = module_name.split(".")[0]
            if parent in licenses:
                license_source = licenses[parent]

        # 3. BCR known licenses — exact match
        if license_source is None and module_name in BCR_KNOWN_LICENSES:
            license_source = BCR_KNOWN_LICENSES[module_name]
        # 4. BCR known licenses — parent prefix match (e.g. boost.config → boost)
        if license_source is None and "." in module_name:
            parent = module_name.split(".")[0]
            if parent in BCR_KNOWN_LICENSES:
                license_source = BCR_KNOWN_LICENSES[parent]

        if license_source:
            module_data["license"] = license_source["license"]
            if not module_data.get("supplier") and license_source.get("supplier"):
                module_data["supplier"] = license_source["supplier"]


def normalize_name(name: str) -> str:
    """Normalize a dependency name for fuzzy matching.

    Handles naming differences between Bazel repos and C++ metadata cache:
    e.g. nlohmann_json vs nlohmann-json, libfmt vs fmt.

    Args:
        name: Dependency name to normalize

    Returns:
        Normalized name string for comparison
    """
    n = name.lower().strip()
    for prefix in ("lib", "lib_"):
        if n.startswith(prefix) and len(n) > len(prefix):
            n = n[len(prefix) :]
    n = n.replace("-", "").replace("_", "").replace(".", "")
    return n


def enrich_components_from_cpp_cache(
    components: list[dict[str, Any]],
    cpp_components: list[dict[str, Any]],
    metadata: dict[str, Any],
) -> list[dict[str, Any]]:
    """Enrich Bazel-discovered components with C++ metadata cache.

    For each Bazel component, finds a matching C++ cache entry by normalized
    name and fills in missing fields (license, supplier, version, purl).
    Components not present in Bazel's discovered dependency graph are ignored.

    Args:
        components: Bazel-discovered components to enrich
        cpp_components: Components from C++ metadata cache
        metadata: Metadata dict

    Returns:
        Enriched list of components
    """
    # Build lookup: normalized_name -> cache component
    cpp_by_name: dict[str, dict[str, Any]] = {}
    for cc in cpp_components:
        norm = normalize_name(cc["name"])
        cpp_by_name[norm] = cc
        cpp_by_name[cc["name"].lower()] = cc

    for comp in components:
        comp_name = comp.get("name", "")
        norm_name = normalize_name(comp_name)

        cpp_match = cpp_by_name.get(norm_name) or cpp_by_name.get(comp_name.lower())
        # Try parent name match (e.g., boost.config+ -> boost)
        if not cpp_match:
            base_name = comp_name.rstrip("+")
            if "." in base_name:
                parent = base_name.split(".")[0]
                cpp_match = cpp_by_name.get(normalize_name(parent))
        if not cpp_match:
            continue

        # Enrich missing fields only
        if not comp.get("license") and cpp_match.get("license"):
            comp["license"] = cpp_match["license"]

        if not comp.get("description") and cpp_match.get("description"):
            comp["description"] = cpp_match["description"]

        if not comp.get("supplier") and cpp_match.get("supplier"):
            comp["supplier"] = cpp_match["supplier"]

        if comp.get("version") in ("unknown", "") and cpp_match.get("version") not in (
            "unknown",
            "",
        ):
            comp["version"] = cpp_match["version"]

        if comp.get("purl", "").endswith("@unknown") and cpp_match.get("purl"):
            comp["purl"] = cpp_match["purl"]

        if not comp.get("url") and cpp_match.get("url"):
            comp["url"] = cpp_match["url"]

        if not comp.get("checksum") and cpp_match.get("checksum"):
            comp["checksum"] = cpp_match["checksum"]

    return components


def load_cdxgen_sbom(cdxgen_path: str) -> list[dict[str, Any]]:
    """Load and convert cdxgen CycloneDX SBOM to component list.

    Args:
        cdxgen_path: Path to cdxgen-generated CycloneDX JSON file

    Returns:
        List of component dicts in internal format
    """
    try:
        with open(cdxgen_path, encoding="utf-8") as f:
            cdx_data = json.load(f)
    except (OSError, json.JSONDecodeError):
        return []

    components: list[dict[str, Any]] = []
    for comp in cdx_data.get("components", []):
        # Extract license information
        licenses = comp.get("licenses", [])
        license_str = ""
        if licenses:
            # Take first license
            lic = licenses[0]
            if isinstance(lic, dict):
                license_str = (
                    lic.get("expression", "")
                    or lic.get("license", {}).get("id", "")
                    or lic.get("license", {}).get("name", "")
                )

        # Extract purl
        purl = comp.get("purl", "")

        # Extract SHA-256 hash if present
        checksum = ""
        for h in comp.get("hashes", []):
            if not isinstance(h, dict):
                continue
            if h.get("alg") == "SHA-256" and h.get("content"):
                checksum = str(h["content"])
                break

        # Build component
        component = {
            "name": comp.get("name", ""),
            "version": comp.get("version", "unknown"),
            "purl": purl,
            "type": comp.get("type", "library"),
            "license": license_str,
            "description": comp.get("description", ""),
            "supplier": comp.get("supplier", {}).get("name", "")
            if isinstance(comp.get("supplier"), dict)
            else "",
            "cpe": comp.get("cpe", ""),
            "url": "",
            "checksum": checksum,
        }

        # Add component if it has a name
        if component["name"]:
            components.append(component)

    return components


def mark_missing_cpp_descriptions(
    components: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Mark missing descriptions for non-Rust libraries as 'Missing'."""
    for comp in components:
        if comp.get("description"):
            continue
        if comp.get("type") != "library":
            continue
        purl = comp.get("purl", "")
        if purl.startswith("pkg:cargo/"):
            continue
        comp["description"] = "Missing"
    return components


def main() -> int:
    """Main entry point for SBOM generation."""
    parser = argparse.ArgumentParser(description="Generate SBOM from Bazel deps")
    parser.add_argument("--input", required=True, help="Input JSON from Bazel rule")
    parser.add_argument(
        "--metadata", required=True, help="Metadata JSON from module extension"
    )
    parser.add_argument("--spdx-output", help="SPDX 2.3 JSON output file")
    parser.add_argument("--cyclonedx-output", help="CycloneDX 1.6 output file")
    parser.add_argument("--crates-cache", help="Path to crates_metadata.json override")
    parser.add_argument(
        "--cdxgen-sbom",
        help="Path to cdxgen-generated CycloneDX JSON for C++ enrichment",
    )
    args = parser.parse_args()

    # Load dependency data from Bazel
    with open(args.input, encoding="utf-8") as f:
        data = json.load(f)

    # Load metadata from module extension
    with open(args.metadata, encoding="utf-8") as f:
        metadata = json.load(f)

    # Parse MODULE.bazel files from dependency modules for version extraction
    # This fills in versions for bazel_dep modules that don't use the sbom_metadata extension
    dep_module_files = data.get("dep_module_files", [])
    if dep_module_files:
        dep_modules = parse_module_bazel_files(dep_module_files)
        if "modules" not in metadata:
            metadata["modules"] = {}
        for name, mod_data in dep_modules.items():
            # Don't override entries already in metadata (from the extension)
            if name not in metadata["modules"]:
                metadata["modules"][name] = mod_data

    # Parse MODULE.bazel.lock files to infer selected module versions.
    # This helps for modules that don't participate in the sbom_metadata
    # extension (for example, transitive Bazel modules like boost.*).
    module_lockfiles = data.get("module_lockfiles", [])
    if module_lockfiles:
        lock_modules = parse_module_lockfiles(module_lockfiles)
        if "modules" not in metadata:
            metadata["modules"] = {}
        for name, mod_data in lock_modules.items():
            if name not in metadata["modules"]:
                metadata["modules"][name] = mod_data

    # Load crates metadata cache (licenses + checksums + versions)
    crates_cache = load_crates_cache(args.crates_cache)

    # Add crates cache to metadata
    if crates_cache:
        if "crates" not in metadata:
            metadata["crates"] = {}
        for name, cache_data in crates_cache.items():
            metadata["crates"].setdefault(name, cache_data)

    # Apply BCR known licenses and user overrides to modules
    apply_known_licenses(metadata)

    # Load cdxgen SBOM if provided (C++ dependency enrichment)
    cpp_components = []
    if args.cdxgen_sbom:
        cpp_components = load_cdxgen_sbom(args.cdxgen_sbom)

    # Filter external repos (exclude build tools)
    external_repos = data.get("external_repos", [])
    exclude_patterns = data.get("exclude_patterns", [])
    filtered_repos = filter_repos(external_repos, exclude_patterns)

    # Build component list with metadata
    components = []

    for repo in filtered_repos:
        component = resolve_component(repo, metadata)
        if component:
            components.append(component)

    # Deduplicate components by name
    components = deduplicate_components(components)

    # Enrich components with C++ metadata cache
    if cpp_components:
        components = enrich_components_from_cpp_cache(
            components, cpp_components, metadata
        )
        components = deduplicate_components(components)

    # Mark missing C++ descriptions explicitly when cdxgen has no description.
    components = mark_missing_cpp_descriptions(components)

    # Generate timestamp in SPDX-compliant format (YYYY-MM-DDTHH:MM:SSZ)
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    # Get configuration
    config = data.get("config", {})

    # Auto-detect component_version from metadata if not explicitly set
    component_name = config.get("component_name", "")
    if not config.get("component_version") and component_name:
        modules = metadata.get("modules", {})
        if component_name in modules:
            config["component_version"] = modules[component_name].get("version", "")

    # Filter out the main component from the dependency list to avoid self-dependency
    # (e.g., sbom for score_kyron should not list score_kyron as its own dependency)
    if component_name:
        components = [c for c in components if c.get("name") != component_name]

    # Generate outputs
    if args.spdx_output:
        spdx = generate_spdx(components, config, timestamp)
        Path(args.spdx_output).write_text(json.dumps(spdx, indent=2), encoding="utf-8")

    if args.cyclonedx_output:
        cdx = generate_cyclonedx(
            components,
            config,
            timestamp,
            external_dep_edges=data.get("external_dep_edges", []),
        )
        Path(args.cyclonedx_output).write_text(
            json.dumps(cdx, indent=2), encoding="utf-8"
        )

    return 0


def filter_repos(repos: list[str], exclude_patterns: list[str]) -> list[str]:
    """Filter out build tool repositories based on exclude patterns.

    Crates from crate_universe are always kept even if they match exclude patterns,
    since they are legitimate dependencies, not build tools.

    Args:
        repos: List of repository names
        exclude_patterns: Patterns to exclude

    Returns:
        Filtered list of repository names
    """
    filtered = []
    for repo in repos:
        # Always keep crates from crate_universe - these are real dependencies
        if "crate_index__" in repo or "crates_io__" in repo or "_crates__" in repo:
            filtered.append(repo)
            continue

        should_exclude = False
        for pattern in exclude_patterns:
            if pattern in repo:
                should_exclude = True
                break
        if not should_exclude:
            filtered.append(repo)
    return filtered


def _build_crate_result(
    crate_name: str,
    version: str,
    crate_meta: dict[str, Any],
) -> dict[str, Any]:
    """Build a crate component dict from parsed name/version and cache metadata."""
    result: dict[str, Any] = {
        "name": crate_name,
        "version": version,
        "purl": f"pkg:cargo/{crate_name}@{version}",
        "type": "library",
        "source": "crates.io",
    }
    if crate_meta.get("license"):
        result["license"] = crate_meta["license"]
    if crate_meta.get("description"):
        result["description"] = crate_meta["description"]
    if crate_meta.get("supplier"):
        result["supplier"] = crate_meta["supplier"]
    if crate_meta.get("cpe"):
        result["cpe"] = crate_meta["cpe"]
    if crate_meta.get("aliases"):
        result["aliases"] = crate_meta["aliases"]
    if crate_meta.get("pedigree_ancestors"):
        result["pedigree_ancestors"] = crate_meta["pedigree_ancestors"]
    if crate_meta.get("pedigree_descendants"):
        result["pedigree_descendants"] = crate_meta["pedigree_descendants"]
    if crate_meta.get("pedigree_variants"):
        result["pedigree_variants"] = crate_meta["pedigree_variants"]
    if crate_meta.get("pedigree_notes"):
        result["pedigree_notes"] = crate_meta["pedigree_notes"]
    if crate_meta.get("repository"):
        result["url"] = crate_meta["repository"]
    if crate_meta.get("checksum"):
        result["checksum"] = crate_meta["checksum"]
    return result


def resolve_component(
    repo_name: str, metadata: dict[str, Any]
) -> dict[str, Any] | None:
    """Resolve repository to component with version and PURL.

    Args:
        repo_name: Name of the repository
        metadata: Metadata dictionary from module extension

    Returns:
        Component dictionary or None if not resolved
    """
    # Normalize repo name - bzlmod adds "+" suffix to module repos
    normalized_name = repo_name.rstrip("+")

    # Check if it's a bazel_dep module
    modules = metadata.get("modules", {})
    if normalized_name in modules:
        mod = modules[normalized_name]
        result: dict[str, Any] = {
            "name": normalized_name,
            "version": mod.get("version", "unknown"),
            "purl": mod.get("purl", f"pkg:generic/{normalized_name}@unknown"),
            "type": "library",
            "supplier": mod.get("supplier", ""),
            "license": mod.get("license", ""),
            "cpe": mod.get("cpe", ""),
            "aliases": mod.get("aliases", []),
            "pedigree_ancestors": mod.get("pedigree_ancestors", []),
            "pedigree_descendants": mod.get("pedigree_descendants", []),
            "pedigree_variants": mod.get("pedigree_variants", []),
            "pedigree_notes": mod.get("pedigree_notes", ""),
        }
        # MODULE.bazel.lock can provide a sha256 via source.json; expose it as
        # checksum so CycloneDX hashes are populated for C/C++ modules.
        if mod.get("sha256"):
            result["checksum"] = mod["sha256"]
        return result

    # Check if it's an http_archive dependency
    http_archives = metadata.get("http_archives", {})
    if normalized_name in http_archives:
        archive = http_archives[normalized_name]
        result = {
            "name": normalized_name,
            "version": archive.get("version", "unknown"),
            "purl": archive.get("purl", f"pkg:generic/{normalized_name}@unknown"),
            "type": "library",
            "url": archive.get("url", ""),
            "license": archive.get("license", ""),
            "supplier": archive.get("supplier", ""),
            "cpe": archive.get("cpe", ""),
            "aliases": archive.get("aliases", []),
            "pedigree_ancestors": archive.get("pedigree_ancestors", []),
            "pedigree_descendants": archive.get("pedigree_descendants", []),
            "pedigree_variants": archive.get("pedigree_variants", []),
            "pedigree_notes": archive.get("pedigree_notes", ""),
        }
        if archive.get("sha256"):
            result["checksum"] = archive["sha256"]
        return result

    # Check if it's a git_repository dependency
    git_repos = metadata.get("git_repositories", {})
    if normalized_name in git_repos:
        repo = git_repos[normalized_name]
        result = {
            "name": normalized_name,
            "version": repo.get("version", "unknown"),
            "purl": repo.get("purl", f"pkg:generic/{normalized_name}@unknown"),
            "type": "library",
            "url": repo.get("remote", ""),
            "license": repo.get("license", ""),
            "supplier": repo.get("supplier", ""),
            "cpe": repo.get("cpe", ""),
            "aliases": repo.get("aliases", []),
            "pedigree_ancestors": repo.get("pedigree_ancestors", []),
            "pedigree_descendants": repo.get("pedigree_descendants", []),
            "pedigree_variants": repo.get("pedigree_variants", []),
            "pedigree_notes": repo.get("pedigree_notes", ""),
        }
        commit_date = repo.get("commit_date", "")
        if result.get("version") in ("unknown", "") and commit_date:
            result["version"] = commit_date
        return result

    # Check if it's a crate from the metadata cache
    # Cargo.lock uses underscores, Bazel uses hyphens — try both
    crates = metadata.get("crates", {})
    crate_key = (
        normalized_name
        if normalized_name in crates
        else normalized_name.replace("-", "_")
    )
    if crate_key in crates:
        crate = crates[crate_key]
        result = {
            "name": normalized_name,
            "version": crate.get("version", "unknown"),
            "purl": crate.get("purl", f"pkg:cargo/{normalized_name}@unknown"),
            "type": "library",
            "source": "crates.io",
            "license": crate.get("license", ""),
            "description": crate.get("description", ""),
            "supplier": crate.get("supplier", ""),
            "cpe": crate.get("cpe", ""),
            "aliases": crate.get("aliases", []),
            "pedigree_ancestors": crate.get("pedigree_ancestors", []),
            "pedigree_descendants": crate.get("pedigree_descendants", []),
            "pedigree_variants": crate.get("pedigree_variants", []),
            "pedigree_notes": crate.get("pedigree_notes", ""),
        }
        if crate.get("checksum"):
            result["checksum"] = crate["checksum"]
        return result

    # Handle score_ prefixed repos that might be modules
    if normalized_name.startswith("score_"):
        return {
            "name": normalized_name,
            "version": "unknown",
            "purl": f"pkg:github/eclipse-score/{normalized_name}@unknown",
            "type": "library",
            "supplier": "Eclipse Foundation",
            "license": "",
            "cpe": "",
            "aliases": [],
            "pedigree_ancestors": [],
            "pedigree_descendants": [],
            "pedigree_variants": [],
            "pedigree_notes": "",
        }

    # Handle crate universe repos - bzlmod format
    # e.g., rules_rust++crate+crate_index__serde-1.0.228
    # e.g., rules_rust++crate+crate_index__iceoryx2-qnx8-0.7.0
    cached_crates = metadata.get("crates", {})

    if "crate_index__" in repo_name or "crate+" in repo_name:
        # Extract the crate info part after crate_index__
        if "crate_index__" in repo_name:
            crate_part = repo_name.split("crate_index__")[-1]
        else:
            crate_part = repo_name.split("+")[-1]

        # Parse name-version format (e.g., "serde-1.0.228")
        # Handle complex names like "iceoryx2-qnx8-0.7.0" where last part is version
        parts = crate_part.split("-")
        if len(parts) >= 2:
            # Find the version part (starts with a digit)
            version_idx = -1
            for i, part in enumerate(parts):
                if part and part[0].isdigit():
                    version_idx = i
                    break

            if version_idx > 0:
                crate_name = "-".join(parts[:version_idx]).replace("_", "-")
                version = "-".join(parts[version_idx:])

                # Look up crate metadata from cache
                # Cargo.lock uses underscores, Bazel uses hyphens — try both
                crate_meta = cached_crates.get(crate_name) or cached_crates.get(
                    crate_name.replace("-", "_"), {}
                )
                return _build_crate_result(crate_name, version, crate_meta)

    # Handle legacy crate universe format (e.g., crates_io__tokio-1.10.0)
    if repo_name.startswith("crates_io__") or "_crates__" in repo_name:
        parts = repo_name.split("__")
        if len(parts) >= 2:
            crate_info = parts[-1]
            # Try to split by last hyphen to get name-version
            last_hyphen = crate_info.rfind("-")
            if last_hyphen > 0:
                crate_name = crate_info[:last_hyphen].replace("_", "-")
                version = crate_info[last_hyphen + 1 :]

                # Look up crate metadata from cache
                # Cargo.lock uses underscores, Bazel uses hyphens — try both
                crate_meta = cached_crates.get(crate_name) or cached_crates.get(
                    crate_name.replace("-", "_"), {}
                )
                return _build_crate_result(crate_name, version, crate_meta)

    # Check if repo is a sub-library of a known parent (e.g., boost.config+ -> boost)
    # rules_boost splits Boost into individual repos like boost.config+, boost.assert+, etc.
    if "." in normalized_name:
        parent_name = normalized_name.split(".")[0].rstrip("+")
        # Look up parent in all metadata sources (modules, http_archives, git_repos, licenses)
        licenses = metadata.get("licenses", {})
        parent = None
        if parent_name in modules:
            parent = modules[parent_name]
        elif parent_name in http_archives:
            parent = http_archives[parent_name]
        elif parent_name in git_repos:
            parent = git_repos[parent_name]
        elif parent_name in licenses:
            parent = licenses[parent_name]
        if parent:
            parent_version = parent.get("version", "unknown")
            result: dict[str, Any] = {
                "name": normalized_name,
                "version": parent_version,
                "purl": f"pkg:generic/{normalized_name}@{parent_version}",
                "type": "library",
                "license": parent.get("license", ""),
                "supplier": parent.get("supplier", ""),
            }
            # Propagate checksum from parent if available (e.g., http_archive
            # sha256 or module sha256 from MODULE.bazel.lock).
            if parent.get("sha256"):
                result["checksum"] = parent["sha256"]
            elif parent.get("checksum"):
                result["checksum"] = parent["checksum"]
            return result

    # Unknown repository - return with unknown version
    return {
        "name": repo_name,
        "version": "unknown",
        "purl": f"pkg:generic/{repo_name}@unknown",
        "type": "library",
    }


def deduplicate_components(components: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Remove duplicate components, keeping the one with most metadata.

    Args:
        components: List of component dictionaries

    Returns:
        Deduplicated list of components
    """
    seen: dict[str, dict[str, Any]] = {}
    for comp in components:
        name = comp.get("name", "")
        if name not in seen:
            seen[name] = comp
        else:
            # Keep the one with more information (non-unknown version preferred)
            existing = seen[name]
            if (
                existing.get("version") == "unknown"
                and comp.get("version") != "unknown"
            ):
                seen[name] = comp
            elif comp.get("license") and not existing.get("license"):
                # Prefer component with license info
                seen[name] = comp

    return list(seen.values())


if __name__ == "__main__":
    sys.exit(main())
