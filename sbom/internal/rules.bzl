"""SBOM generation rule implementation.

This module contains the main _sbom_impl rule that combines data from
the aspect (target dependencies) with metadata from the module extension
to generate SPDX and CycloneDX format SBOMs.
"""

load(":aspect.bzl", "sbom_aspect")
load(":providers.bzl", "SbomDepsInfo")

def _sbom_impl(ctx):
    """Generates SBOM by combining aspect data with extension metadata.

    Args:
        ctx: The rule context

    Returns:
        DefaultInfo with generated SBOM files
    """

    # Collect all external repos used by targets
    all_external_repos = depset(transitive = [
        target[SbomDepsInfo].external_repos
        for target in ctx.attr.targets
    ])

    # Collect all transitive deps
    all_transitive_deps = depset(transitive = [
        target[SbomDepsInfo].transitive_deps
        for target in ctx.attr.targets
    ])

    # Collect external dependency edges
    all_external_dep_edges = depset(transitive = [
        target[SbomDepsInfo].external_dep_edges
        for target in ctx.attr.targets
    ])

    # Get the metadata JSON file from the extension
    metadata_file = ctx.file.metadata_json

    # Create input file with dependency info for Python generator
    deps_json = ctx.actions.declare_file(ctx.attr.name + "_deps.json")

    # Build target labels list
    target_labels = [str(t.label) for t in ctx.attr.targets]

    # Infer scan root for cdxgen:
    # - If all targets come from the same external repo, scan that repo tree.
    # - Otherwise scan the current execroot.
    target_repos = []
    for t in ctx.attr.targets:
        repo = t.label.workspace_name
        if repo and repo not in target_repos:
            target_repos.append(repo)
    cdxgen_scan_root = "."
    if len(target_repos) == 1:
        cdxgen_scan_root = "external/{}".format(target_repos[0])

    # Build exclude patterns list
    exclude_patterns = ctx.attr.exclude_patterns

    # Collect MODULE.bazel files from dependency modules for version extraction
    dep_module_paths = [f.path for f in ctx.files.dep_module_files]
    module_lock_paths = [f.path for f in ctx.files.module_lockfiles]

    deps_data = {
        "external_repos": all_external_repos.to_list(),
        "transitive_deps": [str(d) for d in all_transitive_deps.to_list()],
        "external_dep_edges": all_external_dep_edges.to_list(),
        "target_labels": target_labels,
        "exclude_patterns": exclude_patterns,
        "dep_module_files": dep_module_paths,
        "module_lockfiles": module_lock_paths,
        "config": {
            "producer_name": ctx.attr.producer_name,
            "producer_url": ctx.attr.producer_url,
            "component_name": ctx.attr.component_name if ctx.attr.component_name else ctx.attr.name,
            "component_version": ctx.attr.component_version,
            "namespace": ctx.attr.namespace,
            "sbom_authors": ctx.attr.sbom_authors,
            "generation_context": ctx.attr.generation_context,
            "sbom_tools": ctx.attr.sbom_tools,
        },
    }

    ctx.actions.write(
        output = deps_json,
        content = json.encode(deps_data),
    )

    # Declare outputs
    outputs = []
    args = ctx.actions.args()
    args.add("--input", deps_json)
    args.add("--metadata", metadata_file)

    if "spdx" in ctx.attr.output_formats:
        spdx_out = ctx.actions.declare_file(ctx.attr.name + ".spdx.json")
        outputs.append(spdx_out)
        args.add("--spdx-output", spdx_out)

    if "cyclonedx" in ctx.attr.output_formats:
        cdx_out = ctx.actions.declare_file(ctx.attr.name + ".cdx.json")
        outputs.append(cdx_out)
        args.add("--cyclonedx-output", cdx_out)

    # Build inputs list
    generator_inputs = [deps_json, metadata_file] + ctx.files.dep_module_files + ctx.files.module_lockfiles

    # Auto-generate crates metadata cache if enabled and a lockfile is provided
    crates_cache = None
    if (ctx.file.cargo_lockfile or ctx.files.module_lockfiles) and ctx.attr.auto_crates_cache:
        crates_cache = ctx.actions.declare_file(ctx.attr.name + "_crates_metadata.json")
        cache_inputs = [ctx.file._crates_cache_script]
        cache_cmd = "set -euo pipefail\npython3 {} {}".format(
            ctx.file._crates_cache_script.path,
            crates_cache.path,
        )
        if ctx.file.cargo_lockfile:
            cache_inputs.append(ctx.file.cargo_lockfile)
            cache_cmd += " --cargo-lock {}".format(ctx.file.cargo_lockfile.path)
        for lock in ctx.files.module_lockfiles:
            cache_inputs.append(lock)
            cache_cmd += " --module-lock {}".format(lock.path)
        ctx.actions.run_shell(
            inputs = cache_inputs,
            outputs = [crates_cache],
            command = cache_cmd,
            mnemonic = "CratesCacheGenerate",
            progress_message = "Generating crates metadata cache for %s" % ctx.attr.name,
            execution_requirements = {"requires-network": ""},
            use_default_shell_env = True,
        )

    # Add cdxgen SBOM if provided; otherwise auto-generate if enabled
    cdxgen_sbom = ctx.file.cdxgen_sbom
    if not cdxgen_sbom and ctx.attr.auto_cdxgen:
        cdxgen_sbom = ctx.actions.declare_file(ctx.attr.name + "_cdxgen.cdx.json")
        ctx.actions.run(
            outputs = [cdxgen_sbom],
            executable = ctx.executable._npm,
            arguments = [
                "exec",
                "--",
                "@cyclonedx/cdxgen",
                "-t",
                "cpp",
                "--deep",
                "-r",
                "-o",
                cdxgen_sbom.path,
                cdxgen_scan_root,
            ],
            mnemonic = "CdxgenGenerate",
            progress_message = "Generating cdxgen SBOM for %s" % ctx.attr.name,
            # cdxgen needs to recursively scan source trees. Running sandboxed with
            # only declared file inputs makes the scan effectively empty.
            execution_requirements = {"no-sandbox": "1"},
        )

    if cdxgen_sbom:
        args.add("--cdxgen-sbom", cdxgen_sbom)
        generator_inputs.append(cdxgen_sbom)

    if crates_cache:
        args.add("--crates-cache", crates_cache)
        generator_inputs.append(crates_cache)

    # Run Python generator
    ctx.actions.run(
        inputs = generator_inputs,
        outputs = outputs,
        executable = ctx.executable._generator,
        arguments = [args],
        mnemonic = "SbomGenerate",
        progress_message = "Generating SBOM for %s" % ctx.attr.name,
    )

    return [DefaultInfo(files = depset(outputs))]

sbom_rule = rule(
    implementation = _sbom_impl,
    attrs = {
        "targets": attr.label_list(
            mandatory = True,
            aspects = [sbom_aspect],
            doc = "Targets to generate SBOM for",
        ),
        "output_formats": attr.string_list(
            default = ["spdx", "cyclonedx"],
            doc = "Output formats: spdx, cyclonedx",
        ),
        "producer_name": attr.string(
            default = "Eclipse Foundation",
            doc = "SBOM producer organization name",
        ),
        "producer_url": attr.string(
            default = "https://projects.eclipse.org/projects/automotive.score",
            doc = "SBOM producer URL",
        ),
        "component_name": attr.string(
            doc = "Component name (defaults to rule name)",
        ),
        "component_version": attr.string(
            default = "",
            doc = "Component version",
        ),
        "sbom_authors": attr.string_list(
            default = [],
            doc = "SBOM author(s) (distinct from software producers)",
        ),
        "generation_context": attr.string(
            default = "",
            doc = "SBOM generation context: pre-build, build, post-build",
        ),
        "sbom_tools": attr.string_list(
            default = [],
            doc = "Additional SBOM generation tool names",
        ),
        "namespace": attr.string(
            default = "https://eclipse.dev/score",
            doc = "SBOM namespace URI",
        ),
        "exclude_patterns": attr.string_list(
            default = [
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
            ],
            doc = "External repo patterns to exclude (build tools)",
        ),
        "metadata_json": attr.label(
            mandatory = True,
            allow_single_file = [".json"],
            doc = "Metadata JSON file from sbom_metadata extension",
        ),
        "dep_module_files": attr.label_list(
            allow_files = True,
            default = [],
            doc = "MODULE.bazel files from dependency modules for automatic version extraction",
        ),
        "cargo_lockfile": attr.label(
            allow_single_file = True,
            doc = "Optional Cargo.lock file for automatic crate metadata extraction",
        ),
        "module_lockfiles": attr.label_list(
            allow_files = True,
            doc = "MODULE.bazel.lock files for crate metadata extraction (e.g., from score_crates and workspace)",
        ),
        "cdxgen_sbom": attr.label(
            allow_single_file = [".json"],
            doc = "Optional CycloneDX JSON from cdxgen for C++ dependency enrichment",
        ),
        "auto_cdxgen": attr.bool(
            default = False,
            doc = "Automatically run cdxgen when no cdxgen_sbom is provided",
        ),
        "_npm": attr.label(
            default = "//sbom:npm_wrapper",
            executable = True,
            cfg = "exec",
        ),
        "auto_crates_cache": attr.bool(
            default = True,
            doc = "Automatically build crates metadata cache when cargo_lockfile or module_lockfile is provided",
        ),
        "_crates_cache_script": attr.label(
            default = "//sbom/scripts:generate_crates_metadata_cache.py",
            allow_single_file = True,
        ),
        "_generator": attr.label(
            default = "//sbom/internal/generator:sbom_generator",
            executable = True,
            cfg = "exec",
        ),
    },
    doc = "Generates SBOM for specified targets in SPDX and CycloneDX formats",
)
