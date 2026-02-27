# ======================================================================================
# Providers
# ======================================================================================

SphinxModuleInfo = provider(
    doc = "Provider for Sphinx HTML module documentation",
    fields = {
        "html_dir": "Directory containing HTML files",
    },
)

SphinxNeedsInfo = provider(
    doc = "Provider for sphinx-needs info",
    fields = {
        "needs_json_file": "Direct needs.json file for this module",
        "needs_json_files": "Depset of needs.json files including transitive dependencies",
    },
)

# ======================================================================================
# Helpers
# ======================================================================================
def _create_config_py(ctx):
    """Get or generate the conf.py configuration file.

    Args:
        ctx: Rule context
    """
    if ctx.attr.config:
        config_file = ctx.attr.config.files.to_list()[0]
    else:
        config_file = ctx.actions.declare_file(ctx.label.name + "/conf.py")
        template = ctx.file._config_template

        # Read template and substitute PROJECT_NAME
        ctx.actions.expand_template(
            template = template,
            output = config_file,
            substitutions = {
                "{PROJECT_NAME}": ctx.label.name.replace("_", " ").title(),
            },
        )
    return config_file

# ======================================================================================
# Common attributes for Sphinx rules
# ======================================================================================
sphinx_rule_attrs = {
    "srcs": attr.label_list(
        allow_files = True,
        doc = "List of source files for the Sphinx documentation.",
    ),
    "sphinx": attr.label(
        doc = "The Sphinx build binary to use.",
        mandatory = True,
        executable = True,
        cfg = "exec",
    ),
    "config": attr.label(
        allow_files = [".py"],
        doc = "Configuration file (conf.py) for the Sphinx documentation. If not provided, a default config will be generated.",
        mandatory = False,
    ),
    "index": attr.label(
        allow_files = [".rst"],
        doc = "Index file (index.rst) for the Sphinx documentation.",
        mandatory = True,
    ),
    "deps": attr.label_list(
        doc = "List of other sphinx_module targets this module depends on for intersphinx.",
    ),
    "_config_template": attr.label(
        default = Label("//bazel/rules/rules_score:templates/conf.template.py"),
        allow_single_file = True,
        doc = "Template for generating default conf.py",
    ),
    "_html_merge_tool": attr.label(
        default = Label("//bazel/rules/rules_score:sphinx_html_merge"),
        executable = True,
        cfg = "exec",
        doc = "Tool for merging HTML directories",
    ),
}

# ======================================================================================
# Rule implementations
# ======================================================================================
def _score_needs_impl(ctx):
    output_path = ctx.label.name.replace("_needs", "") + "/needs.json"
    needs_output = ctx.actions.declare_file(output_path)

    # Get config file (generate or use provided)
    config_file = _create_config_py(ctx)

    # Phase 1: Build needs.json (without external needs)
    needs_inputs = ctx.files.srcs + [config_file]

    if ctx.attr.config:
        needs_inputs = needs_inputs + ctx.files.config

    needs_args = [
        "--index_file",
        ctx.attr.index.files.to_list()[0].path,
        "--output_dir",
        needs_output.dirname,
        "--config",
        config_file.path,
        "--builder",
        "needs",
    ]

    ctx.actions.run(
        inputs = needs_inputs,
        outputs = [needs_output],
        arguments = needs_args,
        progress_message = "Generating needs.json for: %s" % ctx.label.name,
        executable = ctx.executable.sphinx,
    )

    transitive_needs = [dep[SphinxNeedsInfo].needs_json_files for dep in ctx.attr.deps if SphinxNeedsInfo in dep]
    needs_json_files = depset([needs_output], transitive = transitive_needs)

    return [
        DefaultInfo(
            files = needs_json_files,
        ),
        SphinxNeedsInfo(
            needs_json_file = needs_output,  # Direct file only
            needs_json_files = needs_json_files,  # Transitive depset
        ),
    ]

def _score_html_impl(ctx):
    """Implementation for building a Sphinx module with two-phase build.

    Phase 1: Generate needs.json for this module and collect from all deps
    Phase 2: Generate HTML with external needs and merge all dependency HTML
    """

    # Collect all transitive dependencies with deduplication
    modules = []

    needs_external_needs = {}
    for dep in ctx.attr.needs:
        if SphinxNeedsInfo in dep:
            dep_name = dep.label.name.replace("_needs", "")
            needs_external_needs[dep.label.name] = {
                "base_url": dep_name,  # Relative path to the subdirectory where dep HTML is copied
                "json_path": dep[SphinxNeedsInfo].needs_json_file.path,  # Use direct file
                "id_prefix": "",
                "css_class": "",
            }

    for dep in ctx.attr.deps:
        if SphinxModuleInfo in dep:
            modules.extend([dep[SphinxModuleInfo].html_dir])

    needs_external_needs_json = ctx.actions.declare_file(ctx.label.name + "/needs_external_needs.json")

    ctx.actions.write(
        output = needs_external_needs_json,
        content = json.encode_indent(needs_external_needs, indent = "  "),
    )

    # Read template and substitute PROJECT_NAME
    config_file = ctx.actions.declare_file(ctx.label.name + "/conf.py")
    template = ctx.file._config_template

    ctx.actions.expand_template(
        template = template,
        output = config_file,
        substitutions = {
            "{PROJECT_NAME}": ctx.label.name.replace("_", " ").title(),
        },
    )

    # Build HTML with external needs
    html_inputs = ctx.files.srcs + ctx.files.needs + [config_file, needs_external_needs_json]
    sphinx_html_output = ctx.actions.declare_directory(ctx.label.name + "/_html")
    html_args = [
        "--index_file",
        ctx.attr.index.files.to_list()[0].path,
        "--output_dir",
        sphinx_html_output.path,
        "--config",
        config_file.path,
        "--builder",
        "html",
    ]

    ctx.actions.run(
        inputs = html_inputs,
        outputs = [sphinx_html_output],
        arguments = html_args,
        progress_message = "Building HTML: %s" % ctx.label.name,
        executable = ctx.executable.sphinx,
    )

    # Create final HTML output directory with dependencies using Python merge script
    html_output = ctx.actions.declare_directory(ctx.label.name + "/html")

    # Build arguments for the merge script
    merge_args = [
        "--output",
        html_output.path,
        "--main",
        sphinx_html_output.path,
    ]

    merge_inputs = [sphinx_html_output]

    # Add each dependency
    for dep in ctx.attr.deps:
        if SphinxModuleInfo in dep:
            dep_html_dir = dep[SphinxModuleInfo].html_dir
            dep_name = dep.label.name
            merge_inputs.append(dep_html_dir)
            merge_args.extend(["--dep", dep_name + ":" + dep_html_dir.path])

    # Merging html files
    ctx.actions.run(
        inputs = merge_inputs,
        outputs = [html_output],
        arguments = merge_args,
        progress_message = "Merging HTML with dependencies for %s" % ctx.label.name,
        executable = ctx.executable._html_merge_tool,
    )

    return [
        DefaultInfo(files = depset(ctx.files.needs + [html_output])),
        SphinxModuleInfo(
            html_dir = html_output,
        ),
    ]

# ======================================================================================
# Rule definitions
# ======================================================================================

_score_needs = rule(
    implementation = _score_needs_impl,
    attrs = sphinx_rule_attrs,
)

_score_html = rule(
    implementation = _score_html_impl,
    attrs = dict(sphinx_rule_attrs, needs = attr.label_list(
        allow_files = True,
        doc = "Submodule symbols.needs targets for this module.",
    )),
)

# ======================================================================================
# Rule wrappers
# ======================================================================================

def sphinx_module(
        name,
        srcs,
        index,
        config = None,
        deps = [],
        sphinx = Label("//bazel/rules/rules_score:score_build"),
        testonly = False,
        visibility = ["//visibility:public"]):
    """Build a Sphinx module with transitive HTML dependencies.

    This rule builds documentation modules into complete HTML sites with
    transitive dependency collection. All dependencies are automatically
    included in a modules/ subdirectory for intersphinx cross-referencing.

    Args:
        name: Name of the target
        srcs: List of source files (.rst, .md) with index file first
        index: Label to index.rst file
        config: Label to conf.py configuration file (optional, will be auto-generated if not provided)
        deps: List of other sphinx_module targets this module depends on
        sphinx: Label to sphinx build binary (default: :sphinx_build)
        visibility: Bazel visibility
    """
    _score_needs(
        name = name + "_needs",
        srcs = srcs,
        config = config,
        index = index,
        deps = [d + "_needs" for d in deps],
        sphinx = sphinx,
        testonly = testonly,
        visibility = visibility,
    )

    _score_html(
        name = name,
        srcs = srcs,
        config = config,
        index = index,
        deps = deps,
        needs = [d + "_needs" for d in deps],
        sphinx = sphinx,
        testonly = testonly,
        visibility = visibility,
    )
