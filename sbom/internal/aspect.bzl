"""Aspect to traverse and collect transitive dependencies of a target.

This aspect traverses the dependency graph of specified targets and collects
information about all dependencies, including external repositories, which
is essential for SBOM generation.
"""

load(":providers.bzl", "SbomDepsInfo")

def _sbom_aspect_impl(target, ctx):
    """Collects transitive dependency information for SBOM generation.

    Args:
        target: The target being analyzed
        ctx: The aspect context

    Returns:
        A list containing SbomDepsInfo provider
    """
    direct_deps = []
    transitive_deps_list = []
    external_repos_list = []
    external_repos_direct = []
    external_dep_edges_direct = []
    external_dep_edges_list = []

    # Get this target's label info
    label = target.label
    if label.workspace_name:
        # This is an external dependency
        external_repos_direct.append(label.workspace_name)
        from_repo = label.workspace_name
    else:
        from_repo = ""

    # Collect from rule attributes that represent dependencies
    dep_attrs = ["deps", "srcs", "data", "proc_macro_deps", "crate_root", "compile_data"]
    for attr_name in dep_attrs:
        if hasattr(ctx.rule.attr, attr_name):
            attr_val = getattr(ctx.rule.attr, attr_name)
            if type(attr_val) == "list":
                for dep in attr_val:
                    if hasattr(dep, "label"):
                        direct_deps.append(dep.label)
                        if from_repo and dep.label.workspace_name:
                            external_dep_edges_direct.append(
                                "{}::{}".format(from_repo, dep.label.workspace_name),
                            )
                        if SbomDepsInfo in dep:
                            # Propagate transitive deps from dependencies
                            transitive_deps_list.append(dep[SbomDepsInfo].transitive_deps)
                            external_repos_list.append(dep[SbomDepsInfo].external_repos)
                            external_dep_edges_list.append(dep[SbomDepsInfo].external_dep_edges)
            elif attr_val != None and hasattr(attr_val, "label"):
                # Single target attribute (e.g., crate_root)
                direct_deps.append(attr_val.label)
                if from_repo and attr_val.label.workspace_name:
                    external_dep_edges_direct.append(
                        "{}::{}".format(from_repo, attr_val.label.workspace_name),
                    )
                if SbomDepsInfo in attr_val:
                    transitive_deps_list.append(attr_val[SbomDepsInfo].transitive_deps)
                    external_repos_list.append(attr_val[SbomDepsInfo].external_repos)
                    external_dep_edges_list.append(attr_val[SbomDepsInfo].external_dep_edges)

    # Handle cc_library specific attributes
    cc_dep_attrs = ["hdrs", "textual_hdrs", "implementation_deps"]
    for attr_name in cc_dep_attrs:
        if hasattr(ctx.rule.attr, attr_name):
            attr_val = getattr(ctx.rule.attr, attr_name)
            if type(attr_val) == "list":
                for dep in attr_val:
                    if hasattr(dep, "label"):
                        direct_deps.append(dep.label)
                        if from_repo and dep.label.workspace_name:
                            external_dep_edges_direct.append(
                                "{}::{}".format(from_repo, dep.label.workspace_name),
                            )
                        if SbomDepsInfo in dep:
                            transitive_deps_list.append(dep[SbomDepsInfo].transitive_deps)
                            external_repos_list.append(dep[SbomDepsInfo].external_repos)
                            external_dep_edges_list.append(dep[SbomDepsInfo].external_dep_edges)

    return [SbomDepsInfo(
        direct_deps = depset(direct_deps),
        transitive_deps = depset(
            direct = [label],
            transitive = transitive_deps_list,
        ),
        external_repos = depset(
            direct = external_repos_direct,
            transitive = external_repos_list,
        ),
        external_dep_edges = depset(
            direct = external_dep_edges_direct,
            transitive = external_dep_edges_list,
        ),
    )]

sbom_aspect = aspect(
    implementation = _sbom_aspect_impl,
    attr_aspects = [
        "deps",
        "srcs",
        "data",
        "proc_macro_deps",
        "crate_root",
        "compile_data",
        "hdrs",
        "textual_hdrs",
        "implementation_deps",
    ],
    provides = [SbomDepsInfo],
    doc = "Traverses target dependencies and collects SBOM-relevant information",
)
