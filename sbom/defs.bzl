"""Public API for SBOM generation.

This module provides the sbom() macro, which is the main entry point for
generating Software Bill of Materials for Bazel targets.

Example usage:
    load("@score_tooling//sbom:defs.bzl", "sbom")

    sbom(
        name = "product_sbom",
        targets = [
            "//feature_showcase/rust:orch_per_example",
            "//feature_showcase/rust:kyron_example",
        ],
        component_version = "1.0.0",
    )
"""

load("//sbom/internal:rules.bzl", "sbom_rule")

def sbom(
        name,
        targets,
        metadata_json = "@sbom_metadata//:metadata.json",
        dep_module_files = None,
        cdxgen_sbom = None,
        auto_cdxgen = False,
        cargo_lockfile = None,
        module_lockfiles = None,
        auto_crates_cache = True,
        output_formats = ["spdx", "cyclonedx"],
        producer_name = "Eclipse Foundation",
        producer_url = "https://projects.eclipse.org/projects/automotive.score",
        component_name = None,
        component_version = None,
        sbom_authors = None,
        generation_context = None,
        sbom_tools = None,
        namespace = None,
        exclude_patterns = None,
        **kwargs):
    """Generates SBOM for specified targets.

    This macro creates an SBOM (Software Bill of Materials) for the specified
    targets, traversing their transitive dependencies and generating output
    in SPDX 2.3 and/or CycloneDX 1.6 format.

    License metadata is collected automatically:
    - Rust crates: from crates_metadata.json cache (bundled with tooling)
    - C++ deps: from cpp_metadata.json cache (bundled with tooling)
    - Bazel modules: version/PURL auto-extracted from module graph

    Prerequisites:
        In your MODULE.bazel, you must enable the sbom_metadata extension:
        ```
        sbom_ext = use_extension("@score_tooling//sbom:extensions.bzl", "sbom_metadata")
        use_repo(sbom_ext, "sbom_metadata")
        ```

    Args:
        name: Rule name, also used as output filename prefix
        targets: List of targets to include in SBOM
        metadata_json: Label to the metadata.json file from sbom_metadata extension
        dep_module_files: MODULE.bazel files from dependency modules for automatic version extraction
        cdxgen_sbom: Optional label to CycloneDX JSON from cdxgen for C++ enrichment
        auto_cdxgen: Run cdxgen automatically when no cdxgen_sbom is provided
        cargo_lockfile: Optional Cargo.lock for crates metadata cache generation
        module_lockfiles: MODULE.bazel.lock files for crate metadata extraction (e.g., from score_crates and workspace)
        auto_crates_cache: Run crates metadata cache generation when cargo_lockfile or module_lockfiles is provided
        output_formats: List of formats to generate ("spdx", "cyclonedx")
        producer_name: SBOM producer organization name
        producer_url: SBOM producer URL
        component_name: Main component name (defaults to rule name)
        component_version: Component version string
        namespace: SBOM namespace URI (defaults to https://eclipse.dev/score)
        exclude_patterns: Repo patterns to exclude (e.g., build tools)
        **kwargs: Additional arguments passed to the underlying rule

    Outputs:
        {name}.spdx.json - SPDX 2.3 format (if "spdx" in output_formats)
        {name}.cdx.json - CycloneDX 1.6 format (if "cyclonedx" in output_formats)

    Example:
        # Single target SBOM
        sbom(
            name = "my_app_sbom",
            targets = ["//src:my_app"],
            component_version = "1.0.0",
        )

        # Multi-target SBOM
        sbom(
            name = "product_sbom",
            targets = [
                "//feature_showcase/rust:orch_per_example",
                "//feature_showcase/rust:kyron_example",
            ],
            component_name = "score_reference_integration",
            component_version = "0.5.0-beta",
        )
    """
    default_exclude_patterns = [
        "rules_rust",
        "rules_cc",
        "bazel_tools",
        "platforms",
        "bazel_skylib",
        "rules_python",
        "rules_proto",
        "protobuf",
        "local_config_",
        "remote_",
    ]

    sbom_rule(
        name = name,
        targets = targets,
        metadata_json = metadata_json,
        dep_module_files = dep_module_files if dep_module_files else [],
        cdxgen_sbom = cdxgen_sbom,
        auto_cdxgen = auto_cdxgen,
        cargo_lockfile = cargo_lockfile,
        module_lockfiles = module_lockfiles if module_lockfiles else [],
        auto_crates_cache = auto_crates_cache,
        output_formats = output_formats,
        producer_name = producer_name,
        producer_url = producer_url,
        component_name = component_name if component_name else name,
        component_version = component_version if component_version else "",
        sbom_authors = sbom_authors if sbom_authors else [],
        generation_context = generation_context if generation_context else "",
        sbom_tools = sbom_tools if sbom_tools else [],
        namespace = namespace if namespace else "https://eclipse.dev/score",
        exclude_patterns = exclude_patterns if exclude_patterns else default_exclude_patterns,
        **kwargs
    )
