"""Providers for SBOM data propagation.

This module defines the providers used to pass SBOM-related information
between different phases of the build:
- SbomDepsInfo: Collected by aspect - deps of a specific target
- SbomMetadataInfo: Collected by extension - metadata for all modules
"""

# Collected by aspect - deps of a specific target
SbomDepsInfo = provider(
    doc = "Transitive dependency information for SBOM generation",
    fields = {
        "direct_deps": "depset of direct dependency labels",
        "transitive_deps": "depset of all transitive dependency labels",
        "external_repos": "depset of external repository names used",
        "external_dep_edges": "depset of external repo dependency edges (from::to)",
    },
)

# Collected by extension - metadata for all modules
SbomMetadataInfo = provider(
    doc = "Metadata about all available modules/crates",
    fields = {
        "modules": "dict of module_name -> {version, commit, registry, purl}",
        "crates": "dict of crate_name -> {version, checksum, purl}",
        "http_archives": "dict of repo_name -> {url, version, sha256, purl}",
    },
)
