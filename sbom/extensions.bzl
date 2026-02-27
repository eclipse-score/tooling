"""Module extension to collect dependency metadata from bzlmod.

This extension collects version and metadata information for all modules
and other dependencies in the workspace, making it available for
SBOM generation. License metadata is collected automatically from
bundled caches (crates_metadata.json, cpp_metadata.json).

Usage in MODULE.bazel:
    sbom_ext = use_extension("@score_tooling//sbom:extensions.bzl", "sbom_metadata")
    use_repo(sbom_ext, "sbom_metadata")
"""

def _generate_purl_from_url(url, name, version):
    """Generate Package URL from download URL."""
    if not url:
        return "pkg:generic/{}@{}".format(name, version or "unknown")

    version_str = version or "unknown"

    # GitHub
    if "github.com" in url:
        parts = url.split("github.com/")
        if len(parts) > 1:
            path_parts = parts[1].split("/")
            if len(path_parts) >= 2:
                owner = path_parts[0]
                repo = path_parts[1].split(".")[0].split("/")[0]
                return "pkg:github/{}/{}@{}".format(owner, repo, version_str)

    # GitLab
    if "gitlab.com" in url or "gitlab" in url:
        if "gitlab.com/" in url:
            parts = url.split("gitlab.com/")
            if len(parts) > 1:
                path_parts = parts[1].split("/")
                if len(path_parts) >= 2:
                    owner = path_parts[0]
                    repo = path_parts[1].split(".")[0]
                    return "pkg:gitlab/{}/{}@{}".format(owner, repo, version_str)

    return "pkg:generic/{}@{}".format(name, version_str)

def _generate_purl_from_git(remote, name, version):
    """Generate Package URL from git remote."""
    if not remote:
        return "pkg:generic/{}@{}".format(name, version or "unknown")

    version_str = version or "unknown"

    # GitHub (https or ssh)
    if "github.com" in remote:
        if "github.com:" in remote:
            path = remote.split("github.com:")[-1]
        else:
            path = remote.split("github.com/")[-1]
        parts = path.replace(".git", "").split("/")
        if len(parts) >= 2:
            return "pkg:github/{}/{}@{}".format(parts[0], parts[1], version_str)

    # GitLab
    if "gitlab" in remote:
        if "gitlab.com:" in remote:
            path = remote.split("gitlab.com:")[-1]
        elif "gitlab.com/" in remote:
            path = remote.split("gitlab.com/")[-1]
        else:
            return "pkg:generic/{}@{}".format(name, version_str)
        parts = path.replace(".git", "").split("/")
        if len(parts) >= 2:
            return "pkg:gitlab/{}/{}@{}".format(parts[0], parts[1], version_str)

    return "pkg:generic/{}@{}".format(name, version_str)

def _extract_version_from_url(url):
    """Extract version from URL patterns."""
    if not url:
        return None

    # Try common patterns
    for sep in ["/v", "/archive/v", "/archive/", "/releases/download/v", "/releases/download/"]:
        if sep in url:
            rest = url.split(sep)[-1]
            version = rest.split("/")[0].split(".tar")[0].split(".zip")[0]
            if version and len(version) > 0 and (version[0].isdigit() or version[0] == "v"):
                return version.lstrip("v")

    # Try filename pattern: name-version.tar.gz
    filename = url.split("/")[-1]
    if "-" in filename:
        parts = filename.rsplit("-", 1)
        if len(parts) == 2:
            version = parts[1].split(".tar")[0].split(".zip")[0]
            if version and version[0].isdigit():
                return version

    return None

def _parse_version_from_module_bazel(content):
    """Parse module name and version from MODULE.bazel content using string ops.

    Starlark doesn't have regex, so we parse with string find/split operations.

    Args:
        content: String content of a MODULE.bazel file

    Returns:
        Tuple of (name, version) or (None, None) if not found
    """
    idx = content.find("module(")
    if idx < 0:
        return None, None

    # Find the closing paren for the module() call
    block_end = content.find(")", idx)
    if block_end < 0:
        return None, None

    block = content[idx:block_end]

    # Extract name
    name = None
    for quote in ['"', "'"]:
        marker = "name = " + quote
        name_idx = block.find(marker)
        if name_idx >= 0:
            name_start = name_idx + len(marker)
            name_end = block.find(quote, name_start)
            if name_end > name_start:
                name = block[name_start:name_end]
            break

    # Extract version
    version = None
    for quote in ['"', "'"]:
        marker = "version = " + quote
        ver_idx = block.find(marker)
        if ver_idx >= 0:
            ver_start = ver_idx + len(marker)
            ver_end = block.find(quote, ver_start)
            if ver_end > ver_start:
                version = block[ver_start:ver_end]
            break

    return name, version

def _sbom_metadata_repo_impl(repository_ctx):
    """Implementation of the sbom_metadata repository rule."""

    # Start with metadata from the extension
    metadata = json.decode(repository_ctx.attr.metadata_content)
    modules = metadata.get("modules", {})

    # Read MODULE.bazel from tracked dependency modules to extract versions
    # Use canonical labels (@@module+) to bypass repo visibility restrictions
    for module_name in repository_ctx.attr.tracked_modules:
        if module_name in modules:
            continue  # Already have this module's info

        # Try to read the module's MODULE.bazel file using canonical label
        label = Label("@@{}+//:MODULE.bazel".format(module_name))
        path = repository_ctx.path(label)
        if path.exists:
            content = repository_ctx.read(path)
            parsed_name, parsed_version = _parse_version_from_module_bazel(content)
            if parsed_name and parsed_version:
                modules[parsed_name] = {
                    "version": parsed_version,
                    "purl": "pkg:generic/{}@{}".format(parsed_name, parsed_version),
                }

    metadata["modules"] = modules
    repository_ctx.file("metadata.json", json.encode(metadata))
    repository_ctx.file("BUILD.bazel", """\
# Generated SBOM metadata repository
exports_files(["metadata.json"])
""")

_sbom_metadata_repo = repository_rule(
    implementation = _sbom_metadata_repo_impl,
    attrs = {
        "metadata_content": attr.string(mandatory = True),
        "tracked_modules": attr.string_list(default = []),
    },
)

def _sbom_metadata_impl(module_ctx):
    """Collects SBOM metadata from all modules in dependency graph."""
    all_http_archives = {}
    all_git_repos = {}
    all_modules = {}
    all_crates = {}
    all_licenses = {}
    tracked_modules = []

    for mod in module_ctx.modules:
        # Collect tracked module names for version extraction
        for tag in mod.tags.track_module:
            if tag.name not in tracked_modules:
                tracked_modules.append(tag.name)
        module_name = mod.name
        module_version = mod.version

        # Collect module info from bazel_dep automatically
        if module_name and module_version:
            all_modules[module_name] = {
                "version": module_version,
                "purl": "pkg:generic/{}@{}".format(module_name, module_version),
            }

        # Collect http_archive metadata
        for tag in mod.tags.http_archive:
            url = tag.urls[0] if tag.urls else (tag.url if hasattr(tag, "url") and tag.url else "")
            version = tag.version if tag.version else _extract_version_from_url(url)
            purl = tag.purl if tag.purl else _generate_purl_from_url(url, tag.name, version)

            all_http_archives[tag.name] = {
                "version": version or "unknown",
                "url": url,
                "purl": purl,
                "license": tag.license if tag.license else "",
                "supplier": tag.supplier if tag.supplier else "",
                "sha256": tag.sha256 if tag.sha256 else "",
                "cpe": tag.cpe if hasattr(tag, "cpe") and tag.cpe else "",
                "aliases": tag.aliases if hasattr(tag, "aliases") and tag.aliases else [],
                "pedigree_ancestors": tag.pedigree_ancestors if hasattr(tag, "pedigree_ancestors") and tag.pedigree_ancestors else [],
                "pedigree_descendants": tag.pedigree_descendants if hasattr(tag, "pedigree_descendants") and tag.pedigree_descendants else [],
                "pedigree_variants": tag.pedigree_variants if hasattr(tag, "pedigree_variants") and tag.pedigree_variants else [],
                "pedigree_notes": tag.pedigree_notes if hasattr(tag, "pedigree_notes") and tag.pedigree_notes else "",
                "declared_by": module_name,
            }

        # Collect git_repository metadata
        for tag in mod.tags.git_repository:
            version = tag.tag if tag.tag else (tag.commit[:12] if tag.commit else "unknown")
            purl = tag.purl if tag.purl else _generate_purl_from_git(tag.remote, tag.name, version)

            all_git_repos[tag.name] = {
                "version": version,
                "remote": tag.remote,
                "commit": tag.commit if tag.commit else "",
                "commit_date": tag.commit_date if hasattr(tag, "commit_date") and tag.commit_date else "",
                "tag": tag.tag if tag.tag else "",
                "purl": purl,
                "license": tag.license if tag.license else "",
                "supplier": tag.supplier if tag.supplier else "",
                "cpe": tag.cpe if hasattr(tag, "cpe") and tag.cpe else "",
                "aliases": tag.aliases if hasattr(tag, "aliases") and tag.aliases else [],
                "pedigree_ancestors": tag.pedigree_ancestors if hasattr(tag, "pedigree_ancestors") and tag.pedigree_ancestors else [],
                "pedigree_descendants": tag.pedigree_descendants if hasattr(tag, "pedigree_descendants") and tag.pedigree_descendants else [],
                "pedigree_variants": tag.pedigree_variants if hasattr(tag, "pedigree_variants") and tag.pedigree_variants else [],
                "pedigree_notes": tag.pedigree_notes if hasattr(tag, "pedigree_notes") and tag.pedigree_notes else "",
                "declared_by": module_name,
            }

        # Collect license info for bazel_dep modules, http_archive, git_repository, and crate deps
        for tag in mod.tags.license:
            dep_type = tag.type if hasattr(tag, "type") and tag.type else ""
            url = ""
            if hasattr(tag, "urls") and tag.urls:
                url = tag.urls[0]
            elif hasattr(tag, "url") and tag.url:
                url = tag.url
            remote = tag.remote if hasattr(tag, "remote") and tag.remote else ""

            explicit_version = tag.version if hasattr(tag, "version") and tag.version else ""
            supplier = tag.supplier if hasattr(tag, "supplier") and tag.supplier else ""
            cpe = tag.cpe if hasattr(tag, "cpe") and tag.cpe else ""
            aliases = tag.aliases if hasattr(tag, "aliases") and tag.aliases else []
            pedigree_ancestors = tag.pedigree_ancestors if hasattr(tag, "pedigree_ancestors") and tag.pedigree_ancestors else []
            pedigree_descendants = tag.pedigree_descendants if hasattr(tag, "pedigree_descendants") and tag.pedigree_descendants else []
            pedigree_variants = tag.pedigree_variants if hasattr(tag, "pedigree_variants") and tag.pedigree_variants else []
            pedigree_notes = tag.pedigree_notes if hasattr(tag, "pedigree_notes") and tag.pedigree_notes else ""

            if dep_type == "cargo":
                version = explicit_version if explicit_version else "unknown"
                all_crates[tag.name] = {
                    "version": version,
                    "purl": tag.purl if tag.purl else "pkg:cargo/{}@{}".format(tag.name, version),
                    "license": tag.license,
                    "supplier": supplier,
                    "cpe": cpe,
                    "aliases": aliases,
                    "pedigree_ancestors": pedigree_ancestors,
                    "pedigree_descendants": pedigree_descendants,
                    "pedigree_variants": pedigree_variants,
                    "pedigree_notes": pedigree_notes,
                }
            elif url or (explicit_version and not remote):
                version = explicit_version if explicit_version else _extract_version_from_url(url)
                purl = tag.purl if tag.purl else _generate_purl_from_url(url, tag.name, version)
                all_http_archives[tag.name] = {
                    "version": version or "unknown",
                    "url": url,
                    "purl": purl,
                    "license": tag.license,
                    "supplier": supplier,
                    "cpe": cpe,
                    "aliases": aliases,
                    "pedigree_ancestors": pedigree_ancestors,
                    "pedigree_descendants": pedigree_descendants,
                    "pedigree_variants": pedigree_variants,
                    "pedigree_notes": pedigree_notes,
                    "declared_by": module_name,
                }
            elif remote:
                version = explicit_version if explicit_version else "unknown"
                purl = tag.purl if tag.purl else _generate_purl_from_git(remote, tag.name, version)
                all_git_repos[tag.name] = {
                    "version": version,
                    "remote": remote,
                    "commit": "",
                    "tag": "",
                    "purl": purl,
                    "license": tag.license,
                    "supplier": supplier,
                    "cpe": cpe,
                    "aliases": aliases,
                    "pedigree_ancestors": pedigree_ancestors,
                    "pedigree_descendants": pedigree_descendants,
                    "pedigree_variants": pedigree_variants,
                    "pedigree_notes": pedigree_notes,
                    "declared_by": module_name,
                }
            else:
                all_licenses[tag.name] = {
                    "license": tag.license,
                    "supplier": supplier,
                    "purl": tag.purl if tag.purl else "",
                    "cpe": cpe,
                    "aliases": aliases,
                    "pedigree_ancestors": pedigree_ancestors,
                    "pedigree_descendants": pedigree_descendants,
                    "pedigree_variants": pedigree_variants,
                    "pedigree_notes": pedigree_notes,
                }

    # Apply license/supplier overrides to modules
    for name, license_info in all_licenses.items():
        if name in all_modules:
            all_modules[name]["license"] = license_info["license"]
            if license_info.get("supplier"):
                all_modules[name]["supplier"] = license_info["supplier"]
            if license_info.get("purl"):
                all_modules[name]["purl"] = license_info["purl"]
            if license_info.get("cpe"):
                all_modules[name]["cpe"] = license_info["cpe"]
            if license_info.get("aliases"):
                all_modules[name]["aliases"] = license_info["aliases"]
            if license_info.get("pedigree_ancestors"):
                all_modules[name]["pedigree_ancestors"] = license_info["pedigree_ancestors"]
            if license_info.get("pedigree_descendants"):
                all_modules[name]["pedigree_descendants"] = license_info["pedigree_descendants"]
            if license_info.get("pedigree_variants"):
                all_modules[name]["pedigree_variants"] = license_info["pedigree_variants"]
            if license_info.get("pedigree_notes"):
                all_modules[name]["pedigree_notes"] = license_info["pedigree_notes"]

    # Generate metadata JSON
    metadata_content = json.encode({
        "modules": all_modules,
        "http_archives": all_http_archives,
        "git_repositories": all_git_repos,
        "crates": all_crates,
        "licenses": all_licenses,
    })

    _sbom_metadata_repo(
        name = "sbom_metadata",
        metadata_content = metadata_content,
        tracked_modules = tracked_modules,
    )

# Tag for http_archive dependencies - mirrors http_archive attributes
_http_archive_tag = tag_class(
    doc = "SBOM metadata for http_archive dependency (mirrors http_archive attrs)",
    attrs = {
        "name": attr.string(mandatory = True, doc = "Repository name"),
        "urls": attr.string_list(doc = "Download URLs"),
        "url": attr.string(doc = "Single download URL (alternative to urls)"),
        "version": attr.string(doc = "Version (auto-extracted from URL if not provided)"),
        "sha256": attr.string(doc = "SHA256 checksum"),
        "license": attr.string(doc = "SPDX license identifier"),
        "supplier": attr.string(doc = "Supplier/organization name"),
        "purl": attr.string(doc = "Package URL (auto-generated if not provided)"),
        "cpe": attr.string(doc = "CPE identifier"),
        "aliases": attr.string_list(doc = "Alternate component names"),
        "pedigree_ancestors": attr.string_list(doc = "Pedigree ancestor identifiers (PURL or name)"),
        "pedigree_descendants": attr.string_list(doc = "Pedigree descendant identifiers (PURL or name)"),
        "pedigree_variants": attr.string_list(doc = "Pedigree variant identifiers (PURL or name)"),
        "pedigree_notes": attr.string(doc = "Pedigree notes"),
    },
)

# Tag for git_repository dependencies - mirrors git_repository attributes
_git_repository_tag = tag_class(
    doc = "SBOM metadata for git_repository dependency (mirrors git_repository attrs)",
    attrs = {
        "name": attr.string(mandatory = True, doc = "Repository name"),
        "remote": attr.string(mandatory = True, doc = "Git remote URL"),
        "commit": attr.string(doc = "Git commit hash"),
        "tag": attr.string(doc = "Git tag"),
        "commit_date": attr.string(doc = "Git commit date (ISO 8601)"),
        "license": attr.string(doc = "SPDX license identifier"),
        "supplier": attr.string(doc = "Supplier/organization name"),
        "purl": attr.string(doc = "Package URL (auto-generated if not provided)"),
        "cpe": attr.string(doc = "CPE identifier"),
        "aliases": attr.string_list(doc = "Alternate component names"),
        "pedigree_ancestors": attr.string_list(doc = "Pedigree ancestor identifiers (PURL or name)"),
        "pedigree_descendants": attr.string_list(doc = "Pedigree descendant identifiers (PURL or name)"),
        "pedigree_variants": attr.string_list(doc = "Pedigree variant identifiers (PURL or name)"),
        "pedigree_notes": attr.string(doc = "Pedigree notes"),
    },
)

# Tag to add license info to any dependency (bazel_dep, http_archive, git_repository, or crate)
_license_tag = tag_class(
    doc = "Add license/supplier metadata for any dependency",
    attrs = {
        "name": attr.string(mandatory = True, doc = "Dependency name"),
        "license": attr.string(mandatory = True, doc = "SPDX license identifier"),
        "supplier": attr.string(doc = "Supplier/organization name (e.g., 'Boost.org', 'Google LLC')"),
        "version": attr.string(doc = "Version string (for http_archive/git_repository/crate; auto-extracted for bazel_dep)"),
        "type": attr.string(doc = "Dependency type: 'cargo' for Rust crates (affects PURL generation). Leave empty for auto-detection."),
        "purl": attr.string(doc = "Override Package URL"),
        "url": attr.string(doc = "Download URL for http_archive (for PURL generation)"),
        "urls": attr.string_list(doc = "Download URLs for http_archive (for PURL generation)"),
        "remote": attr.string(doc = "Git remote URL for git_repository (for PURL generation)"),
        "cpe": attr.string(doc = "CPE identifier"),
        "aliases": attr.string_list(doc = "Alternate component names"),
        "pedigree_ancestors": attr.string_list(doc = "Pedigree ancestor identifiers (PURL or name)"),
        "pedigree_descendants": attr.string_list(doc = "Pedigree descendant identifiers (PURL or name)"),
        "pedigree_variants": attr.string_list(doc = "Pedigree variant identifiers (PURL or name)"),
        "pedigree_notes": attr.string(doc = "Pedigree notes"),
    },
)

# Tag to track a dependency module for automatic version extraction
_track_module_tag = tag_class(
    doc = "Track a bazel_dep module for automatic version extraction from its MODULE.bazel",
    attrs = {
        "name": attr.string(mandatory = True, doc = "Module name (as declared in bazel_dep)"),
    },
)

sbom_metadata = module_extension(
    implementation = _sbom_metadata_impl,
    tag_classes = {
        "http_archive": _http_archive_tag,
        "git_repository": _git_repository_tag,
        "license": _license_tag,
        "track_module": _track_module_tag,
    },
    doc = "Collects SBOM metadata from dependency declarations",
)
