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
# Copyright 2023 The Bazel Authors. All rights reserved.
# https://github.com/bazel-contrib/rules_python/blob/release/1.8/sphinxdocs/private/sphinx.bzl

# ======================================================================================
# Helpers
# ======================================================================================
load("@bazel_skylib//lib:paths.bzl", "paths")
load("@rules_python//python:py_binary.bzl", "py_binary")
load("@rules_python//sphinxdocs:sphinx_docs_library.bzl", "sphinx_docs_library")
load("@rules_python//sphinxdocs/private:sphinx_docs_library_info.bzl", "SphinxDocsLibraryInfo")
load("//bazel/rules/rules_score:providers.bzl", "FilteredExecpathInfo", "SphinxIndexFileInfo", "SphinxModuleInfo", "SphinxNeedsInfo")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

_SPHINX_SERVE_MAIN_SRC = Label("@rules_python//sphinxdocs/private:sphinx_server.py")

# Maps the //bazel/rules/rules_score:verbosity build setting (see
# verbosity.bzl) to the sphinx-build CLI flags that achieve it. Lives here in
# Starlark rather than in a wrapper script: the Sphinx build binary is
# rules_python's own sphinx_build.py (see score_sphinx_toolchain), which
# takes plain sphinx-build argv with no score-specific flags of its own.
_SPHINX_VERBOSITY_FLAGS = {
    "warn": ["-q"],
    "info": [],
    "debug": ["-vv"],
    "trace": ["-vvv"],
}

def _get_index_file(ctx):
    """Extract the index file from the index attribute.

    If the target provides SphinxIndexFileInfo, use that. Otherwise expect
    exactly one file and use it directly.
    """
    target = ctx.attr.index
    if SphinxIndexFileInfo in target:
        return target[SphinxIndexFileInfo].index_file
    files = target.files.to_list()
    if len(files) != 1:
        fail("'index' target must provide SphinxIndexFileInfo or produce exactly one file, got %d files" % len(files))
    return files[0]

def _create_config_py(ctx, builder, project_name, output_prefix):
    """Generate the conf.py configuration file for one Sphinx builder pass.

    Args:
        ctx: Rule context
        builder: The Sphinx builder this conf.py is generated for (e.g.
            "needs" or "html"). Substituted as {BUILDER} so the template can
            scope builder-specific behavior (see suppress_warnings_for_builder
            in sphinx_conf_helpers.py).
        project_name: Value substituted as {PROJECT_NAME} (Sphinx's `project`
            config value).
        output_prefix: Directory prefix the generated conf.py is declared
            under. Must match the consuming _score_needs/_score_html target's
            own output prefix so conf.py lands alongside that target's other
            generated files (e.g. needs_external_needs.json, whose
            confdir-relative lookup depends on this — see
            bazel_sphinx_needs.py's base_dir doc).
    """
    sphinx_toolchain = ctx.toolchains["//bazel/rules/rules_score:toolchain_type"].sphinxinfo
    config_file = ctx.actions.declare_file(output_prefix + "/conf.py")
    template = sphinx_toolchain.conf_template.files.to_list()[0]

    # Read template and substitute PROJECT_NAME / BUILDER
    ctx.actions.expand_template(
        template = template,
        output = config_file,
        substitutions = {
            "{PROJECT_NAME}": project_name,
            "{BUILDER}": builder,
        },
    )
    return config_file

def _score_conf_impl(ctx):
    config_file = _create_config_py(ctx, ctx.attr.builder, ctx.attr.project_name, ctx.attr.output_prefix)
    return [DefaultInfo(files = depset([config_file]))]

_score_conf = rule(
    implementation = _score_conf_impl,
    doc = "Generates the conf.py for one Sphinx builder pass of a sphinx_module. " +
          "Its own target so consumers can inspect/override conf.py generation " +
          "independently of the needs/html build steps that consume it.",
    attrs = {
        "builder": attr.string(
            mandatory = True,
            values = ["needs", "html"],
            doc = "The Sphinx builder this conf.py is generated for.",
        ),
        "project_name": attr.string(
            mandatory = True,
            doc = "Value substituted as {PROJECT_NAME} (Sphinx's `project` config value).",
        ),
        "output_prefix": attr.string(
            mandatory = True,
            doc = "Directory prefix the generated conf.py is declared under.",
        ),
    },
    toolchains = ["//bazel/rules/rules_score:toolchain_type"],
)

# ======================================================================================
# Common attributes for Sphinx rules
# ======================================================================================
sphinx_rule_attrs = dict(
    {
        "srcs": attr.label_list(
            allow_files = True,
            doc = "List of source files for the Sphinx documentation.",
        ),
        "index": attr.label(
            allow_files = [".rst"],
            doc = "Index file (index.rst) for the Sphinx documentation.",
            mandatory = True,
        ),
        "deps": attr.label_list(
            doc = "List of other sphinx_module targets this module depends on for intersphinx.",
        ),
        "conf": attr.label(
            allow_single_file = ["conf.py"],
            mandatory = True,
            doc = "The _score_conf target providing this pass's generated conf.py.",
        ),
        "_plantuml": attr.label(
            default = Label("//third_party/plantuml:plantuml"),
            executable = True,
            cfg = "exec",
        ),
        "_graphviz": attr.label(
            default = Label("//third_party/docs_runtime:dot"),
            executable = True,
            cfg = "exec",
        ),
        "_fta_metamodel": attr.label(
            default = Label("//plantuml:fta_metamodel"),
            allow_files = True,
            doc = "Directory containing fta_metamodel.puml, passed to PlantUML via " +
                  "-Dplantuml.include.path so FTA diagrams can resolve !include fta_metamodel.puml.",
        ),
        "_plantuml_fontconfig": attr.label(
            default = Label("//third_party/plantuml:fontconfig_fallback"),
            allow_files = True,
            doc = "Directory containing fontconfig.properties.tpl and the bundled " +
                  "LiberationSans-Regular.ttf fallback font, passed to PlantUML via " +
                  "-Dsun.awt.fontconfig so it gets usable text metrics even when the " +
                  "execution environment has no native fontconfig library/fonts.",
        ),
        "allow_persistent_workers": attr.bool(
            default = False,
            doc = "(experimental) If true, allow Bazel to run this pass's Sphinx build " +
                  "action as a persistent worker (rules_python's sphinxdocs Worker " +
                  "protocol), improving incremental-build performance. Does not affect " +
                  "the HTML merge step, which never invokes Sphinx. Has no effect on the " +
                  "needs pass, which never opts into worker mode regardless of this " +
                  "attr's value -- see _score_needs_impl's worker_enabled comment. Known " +
                  "gap even where honored (HTML pass): rules_python's Worker only " +
                  "additively copies worker_outdir into the Bazel-declared output dir " +
                  "(shutil.copytree(..., dirs_exist_ok=True) in sphinxdocs/private/" +
                  "sphinx_build.py), so a page whose source doc was removed can survive " +
                  "as stale output across worker-reused builds until a clean build. " +
                  "Sphinx's own incremental build doesn't purge it from worker_outdir " +
                  "either (env.clear_doc() only removes the doc from env state, not the " +
                  "file from disk). Fix belongs upstream; do not flip this on for real " +
                  "use before it lands and a soak test exists.",
        ),
    },
    **VERBOSITY_ATTR
)

def _worker_execution_requirements(worker_enabled):
    """Execution requirements enabling the Bazel persistent-worker protocol.

    `worker_enabled` is the same boolean `_add_sphinx_args` uses to decide
    whether to keep `--jobs auto` -- one flag, computed once per call site,
    drives both. Bazel/RBE may still override these and fall back to
    one-shot execution even when this returns the worker dict.
    """
    if not worker_enabled:
        return {}
    return {
        "supports-workers": "1",
        "requires-worker-protocol": "json",
    }

def _add_sphinx_args(args, *, source_dir, output_dir, config_dir, builder, log_level, worker_enabled):
    """Append the shared sphinx-build positional/flag arguments to `args`.

    Positional order matters: `@rules_python//sphinxdocs/private:sphinx_build.py`'s
    persistent-worker `Worker._prepare_sphinx` hardcodes `arguments[0]`/`[1]`
    as srcdir/outdir, so these two MUST be the first two args added -- callers
    must call this before adding any extra_opts.

    `worker_enabled` must be the exact value passed to
    `_worker_execution_requirements` for the same action -- see that
    function's docstring.
    """
    args.add(source_dir)
    args.add(output_dir)
    args.add("-c", config_dir)
    args.add("-b", builder)
    args.add("-T")  # show details in case of errors in extensions

    if not worker_enabled:
        # --jobs auto forks a subprocess pool per invocation. Inside a
        # long-lived persistent worker process that would compound across
        # requests instead of being torn down between them, and the
        # extension stack's parallel_read_safe claims (sphinx-needs in
        # particular) haven't been soak-tested under worker reuse. Parallel
        # read is therefore one-shot-execution only; see also the needs
        # pass, which never sets worker_enabled True in the first place
        # (_score_needs_impl).
        args.add("--jobs", "auto")

    # Doctree dir lives outside output_dir (a sibling), suffixed by builder
    # so distinct passes sharing an output_dir prefix (e.g. a future pass
    # added under the same <name>/ directory as "html") never collide --
    # today's needs/html split already differs by directory, but that's
    # incidental, not a mechanism this depends on. Living outside output_dir
    # also means it survives the worker's per-request output_dir
    # redirection (see Worker._prepare_sphinx) and Bazel's own re-creation
    # of declared output dirs between one-shot invocations.
    args.add("--doctree-dir", output_dir + "_" + builder + "_doctrees")
    args.add_all(_SPHINX_VERBOSITY_FLAGS.get(log_level, []))

def _hermetic_tool_env(ctx):
    """Compute the env vars that give conf.py hermetic access to plantuml/graphviz.

    Returns both the execroot-relative path (for `os.path.abspath()` at process
    start, while cwd is still the execroot) and an analysis-time-stable
    rlocation key (no exec-config hash) for diagnostic logging. See
    docs/tooling_architecture.rst §"Hermetic tool path resolution".

    The returned files list (first return value) must be added to the calling
    action's `inputs` -- it covers both the FTA metamodel include and the
    PlantUML fontconfig fallback (font + template), neither of which is
    otherwise reachable from the `tools` attr's executables alone.
    """
    gv_short = ctx.executable._graphviz.short_path
    graphviz_rloc = gv_short[3:] if gv_short.startswith("../") else ctx.workspace_name + "/" + gv_short
    pl_short = ctx.executable._plantuml.short_path
    plantuml_rloc = pl_short[3:] if pl_short.startswith("../") else ctx.workspace_name + "/" + pl_short
    fta_metamodel_files = ctx.files._fta_metamodel
    fta_metamodel_dir = fta_metamodel_files[0].dirname if fta_metamodel_files else ""
    fontconfig_files = ctx.files._plantuml_fontconfig
    fontconfig_dir = fontconfig_files[0].dirname if fontconfig_files else ""
    hermetic_files = fta_metamodel_files + fontconfig_files
    return hermetic_files, {
        "PLANTUML_BIN": ctx.executable._plantuml.path,
        "PLANTUML_BIN_RLOC": plantuml_rloc,
        "GRAPHVIZ_DOT": ctx.executable._graphviz.path,
        "GRAPHVIZ_DOT_RLOC": graphviz_rloc,
        "FTA_METAMODEL_DIR": fta_metamodel_dir,
        "PLANTUML_FONTCONFIG_DIR": fontconfig_dir,
    }

def _needs_output_prefix(name):
    """Derive the `_score_needs` output prefix from its target name.

    `sphinx_module` always names this target `<name>_needs`; stripping the
    suffix (rather than replacing all occurrences of "_needs") keeps a module
    named e.g. "foo_needs_bar" from producing a mismatched output path.
    """
    return name.removesuffix("_needs")

# ======================================================================================
# Rule implementations
# ======================================================================================
def _score_needs_impl(ctx):
    sphinx_toolchain = ctx.toolchains["//bazel/rules/rules_score:toolchain_type"].sphinxinfo
    output_path = ctx.label.name + "/needs.json"
    needs_output = ctx.actions.declare_file(output_path)

    # Config file is generated by a standalone _score_conf target (see
    # sphinx_module() macro), not inline here.
    config_file = ctx.file.conf

    # Phase 1: Build needs.json (without external needs).
    # The needs builder (sphinx-needs NeedsBuilder) only collects `.. need::`
    # directives — it is blind to the custom trlc `RequirementsDomain` and its
    # `.. requirement:definition::` directives.  Generated/external files
    # (renamed_srcs, docs_library_deps) are therefore not needed here.  Their
    # toctree entries would produce toc.not_readable warnings because Sphinx's
    # source root is the original docs/ checkout; those are suppressed in
    # conf.template.py (safe: the HTML phase relocates everything so it never
    # emits toc.not_readable).
    needs_inputs = ctx.files.srcs + [config_file]
    output_dir = needs_output.dirname

    # The needs pass reads directly from the unrelocated source checkout
    # (source_dir below is paths.dirname() of the raw index file, not a
    # relocated/staged path) -- unlike the HTML pass, which reads a tree
    # generated under bazel-out. Upstream's persistent-worker protocol
    # (Worker._prepare_sphinx in @rules_python//sphinxdocs/private:
    # sphinx_build.py) writes a "_bazel_worker_request_info.json" file into
    # srcdir on every request. With --worker_sandboxing off (Bazel's
    # default) and execroot source paths symlinked straight into the real
    # checkout, that write would land in the actual source tree, not a
    # build-only directory. So the needs pass never opts into worker mode,
    # regardless of allow_persistent_workers, until srcdir here is a
    # generated tree too (i.e. once both passes share one relocated source
    # tree -- see sphinx_source_tree adoption).
    worker_enabled = False

    args = ctx.actions.args()
    args.use_param_file("@%s", use_always = True)
    args.set_param_file_format("multiline")
    _add_sphinx_args(
        args,
        source_dir = paths.dirname(_get_index_file(ctx).path),
        output_dir = output_dir,
        config_dir = paths.dirname(config_file.path),
        builder = "needs",
        log_level = get_log_level(ctx),
        worker_enabled = worker_enabled,
    )

    hermetic_tool_files, action_env = _hermetic_tool_env(ctx)
    ctx.actions.run(
        inputs = needs_inputs + hermetic_tool_files,
        outputs = [needs_output],
        arguments = [args],
        env = action_env,
        execution_requirements = _worker_execution_requirements(worker_enabled),
        mnemonic = "SphinxNeedsBuild",
        progress_message = "Generating needs.json for: %s" % ctx.label.name,
        executable = sphinx_toolchain.sphinx.files_to_run.executable,
        tools = [
            sphinx_toolchain.sphinx.files_to_run,
            ctx.attr._plantuml.files_to_run,
            ctx.attr._graphviz.files_to_run,
        ],
    )
    transitive_needs = [dep[SphinxNeedsInfo].needs_json_files for dep in ctx.attr.deps if SphinxNeedsInfo in dep]
    needs_json_files = depset([needs_output], transitive = transitive_needs)

    # Self-inclusive union (mirrors SphinxModuleInfo.transitive_modules): each
    # dep's own needs_modules already contains itself, so unioning deps'
    # depsets yields the full flat closure without this module needing to add
    # each dep individually on top. Consumed by _score_html_impl to build
    # needs_external_needs.json from every module transitively required, not
    # just direct deps -- otherwise a :need: reference more than one hop away
    # can never resolve.
    transitive_needs_modules = [dep[SphinxNeedsInfo].needs_modules for dep in ctx.attr.deps if SphinxNeedsInfo in dep]
    needs_modules = depset(
        [struct(name = _needs_output_prefix(ctx.label.name), needs_json_file = needs_output)],
        transitive = transitive_needs_modules,
    )
    return [
        DefaultInfo(
            files = needs_json_files,
        ),
        SphinxNeedsInfo(
            needs_json_file = needs_output,  # Direct file only
            needs_json_files = needs_json_files,  # Transitive depset
            needs_modules = needs_modules,  # Transitive, self-inclusive, keyed by base module name
        ),
    ]

def _score_html_impl(ctx):
    """Implementation for building a Sphinx module with two-phase build.
    Phase 1: Generate needs.json for this module and collect from all deps
    Phase 2: Generate HTML with external needs and merge all dependency HTML
    """
    args = ctx.actions.args()  # Args passed to the Sphinx build action
    args.use_param_file("@%s", use_always = True)
    args.set_param_file_format("multiline")

    # Expand location references in extra_opts and collect as sphinx arguments.
    # targets must include all labels referenced via $(location ...) / $(execpaths ...).
    location_targets = ctx.attr.srcs + ctx.attr.docs_library_deps
    source_prefix = ctx.label.name

    sphinx_toolchain = ctx.toolchains["//bazel/rules/rules_score:toolchain_type"].sphinxinfo

    # Built from the full transitive closure (each direct needs-dep's own
    # SphinxNeedsInfo.needs_modules is already self-inclusive), not just
    # direct deps -- a :need: reference more than one hop away could
    # otherwise never resolve. base_url = module.name (no path prefix) is
    # only truthful because the HTML merge below is flat: every transitive
    # module lands at that exact depth-1 path in the published site,
    # regardless of how many dependency hops away it is.
    transitive_needs_modules = depset(
        transitive = [dep[SphinxNeedsInfo].needs_modules for dep in ctx.attr.needs if SphinxNeedsInfo in dep],
    ).to_list()
    needs_external_needs = {
        module.name: {
            "base_url": module.name,  # Relative path to the subdirectory where dep HTML is copied
            "json_path": module.needs_json_file.path,
            "id_prefix": "",
            "css_class": "",
            "version": "1.0",
        }
        for module in transitive_needs_modules
    }
    needs_external_needs_json = ctx.actions.declare_file(ctx.label.name + "/needs_external_needs.json")
    ctx.actions.write(
        output = needs_external_needs_json,
        content = json.encode_indent(needs_external_needs, indent = "  "),
    )
    sphinx_source_files = []

    # Materialize a file under the `_sources` dir
    def _relocate(source_file, dest_path = None):
        if not dest_path:
            dest_path = source_file.short_path.removeprefix(ctx.attr.strip_prefix)
        dest_path = paths.join(source_prefix, dest_path)
        if source_file.is_directory:
            dest_file = ctx.actions.declare_directory(dest_path)
        else:
            dest_file = ctx.actions.declare_file(dest_path)
        ctx.actions.symlink(
            output = dest_file,
            target_file = source_file,
            progress_message = "Symlinking Sphinx source %{input} to %{output}",
        )
        sphinx_source_files.append(dest_file)
        return dest_file

    for t in ctx.attr.docs_library_deps:
        info = t[SphinxDocsLibraryInfo]
        for entry in info.transitive.to_list():
            for original in entry.files:
                new_path = entry.prefix + original.short_path.removeprefix(entry.strip_prefix)
                _relocate(original, new_path)
    for src_target, dest in ctx.attr.renamed_srcs.items():
        src_files = src_target[DefaultInfo].files.to_list()
        if len(src_files) != 1:
            fail("renamed_srcs entry must be exactly 1 file, got %d files: %s" % (len(src_files), src_files))
        _relocate(src_files[0], dest)

    # Config file is generated by a standalone _score_conf target (see
    # sphinx_module() macro), not inline here.
    config_file = ctx.file.conf

    # Sphinx only accepts a single directory to read its doc sources from.
    # Because plain files and generated files are in different directories,
    # we need to merge the two into a single directory.
    index_source_file = _get_index_file(ctx)

    # An index also listed in renamed_srcs would relocate to two different
    # destinations under the two relocation formulas below (srcs: strip
    # ctx.attr.strip_prefix from short_path; renamed_srcs: the dict's own
    # explicit destination path), so --index_file would end up pointing at
    # the wrong one -- surfacing as an opaque "index file does not exist"
    # from the Sphinx build action instead of a clear build-time error.
    # docs_library_deps can't be checked here: its relocation is
    # provider-driven, not visible until the srcs loop below has already
    # run, so that combination remains an unguarded gap.
    for renamed_src_target in ctx.attr.renamed_srcs.keys():
        if renamed_src_target.label == ctx.attr.index.label:
            fail(
                "sphinx_module '{}': 'index' ({}) must not also appear as a renamed_srcs key -- ".format(
                    ctx.label.name,
                    ctx.attr.index.label,
                ) +
                "srcs and renamed_srcs relocate a file to two different destinations, so " +
                "--index_file would point at the wrong one. Remove it from renamed_srcs.",
            )

    relocated_index_file = ""
    for orig_file in ctx.files.srcs:
        dest = _relocate(orig_file)
        if orig_file.path == index_source_file.path:
            relocated_index_file = dest.path

    if not relocated_index_file:
        fail(
            "sphinx_module '{}': 'index' ({}) did not resolve to a relocated path -- ".format(
                ctx.label.name,
                ctx.attr.index.label,
            ) +
            "its file must also appear in 'srcs'. An index reachable only via 'renamed_srcs' " +
            "or 'docs_library_deps' is not currently supported.",
        )

    sphinx_html_output = ctx.actions.declare_directory(ctx.label.name + "/_html")

    # The HTML pass reads a relocated tree generated under bazel-out (built
    # above via _relocate), so a worker-mode write into "srcdir" -- the
    # request-info file Worker._prepare_sphinx drops there -- lands in a
    # build-only directory, never the real checkout. Safe to honor the attr
    # here; see _score_needs_impl's worker_enabled comment for why the needs
    # pass can't do the same yet.
    worker_enabled = ctx.attr.allow_persistent_workers

    # Positional/builder args must come first (see _add_sphinx_args' docstring)
    # -- extra_opts are appended after.
    _add_sphinx_args(
        args,
        source_dir = paths.dirname(relocated_index_file),
        output_dir = sphinx_html_output.path,
        config_dir = paths.dirname(config_file.path),
        builder = "html",
        log_level = get_log_level(ctx),
        worker_enabled = worker_enabled,
    )

    # Process extra_opts targets: these are rule targets (e.g. filter_execpath)
    # providing FilteredExecpathInfo with resolved Sphinx arguments.
    filtered_files = []
    for target in ctx.attr.extra_opts_targets:
        info = target[FilteredExecpathInfo]
        args.add(info.arg)
        filtered_files.append(info.matched_file)
    for opt in ctx.attr.extra_opts:
        # Standard extra_opts: expand locations and pass through
        args.add(ctx.expand_location(opt, targets = location_targets))

    # Build HTML with external needs
    html_inputs = sphinx_source_files + ctx.files.needs + filtered_files + [config_file, needs_external_needs_json]

    # Use the hermetic graphviz wrapper that executes `/usr/bin/dot` inside the
    # docs_runtime sysroot via exec_in_sysroot.
    hermetic_tool_files, action_env = _hermetic_tool_env(ctx)

    ctx.actions.run(
        inputs = html_inputs + hermetic_tool_files,
        outputs = [sphinx_html_output],
        arguments = [args],
        env = action_env,
        execution_requirements = _worker_execution_requirements(worker_enabled),
        mnemonic = "SphinxHtmlBuild",
        progress_message = "Building HTML: %s" % ctx.label.name,
        executable = sphinx_toolchain.sphinx.files_to_run.executable,
        tools = [
            sphinx_toolchain.sphinx.files_to_run,
            ctx.attr._plantuml.files_to_run,
            ctx.attr._graphviz.files_to_run,
        ],
    )

    # Create final HTML output directory with dependencies using Python merge script
    html_output = ctx.actions.declare_directory(ctx.label.name + "/html")

    # Build arguments for the merge script
    merge_args = [
        "--output",
        html_output.path,
        "--main",
        sphinx_html_output.path,
        "--log-level",
        get_log_level(ctx),
    ]
    merge_inputs = [sphinx_html_output]

    # Every module transitively required by this one, each contributing its
    # own_html_dir (never another module's already-merged html_dir), so each
    # lands exactly once, at depth 1, regardless of how many paths reach it
    # in a diamond dependency graph (see SphinxModuleInfo.transitive_modules).
    transitive_modules_from_deps = depset(
        transitive = [dep[SphinxModuleInfo].transitive_modules for dep in ctx.attr.deps if SphinxModuleInfo in dep],
    ).to_list()
    for module in transitive_modules_from_deps:
        merge_inputs.append(module.own_html_dir)
        merge_args.extend(["--dep", module.name + ":" + module.own_html_dir.path])

    # Auto-detect static files from srcs: any file whose short_path contains
    # '/_static/' is a static asset that Sphinx may not copy correctly in the
    # Bazel sandbox (confdir != srcdir prevents html_static_path from resolving).
    # Copy them explicitly into output/_static/ via the merge step.
    for orig_file in ctx.files.srcs:
        path = orig_file.short_path
        static_marker = "/_static/"
        if static_marker in path:
            subpath = path[path.index(static_marker) + len(static_marker):]
            merge_args.extend(["--extra-static", orig_file.path + ":" + subpath])
            merge_inputs.append(orig_file)

    # Merging html files
    ctx.actions.run(
        inputs = merge_inputs,
        outputs = [html_output],
        arguments = merge_args,
        mnemonic = "SphinxHtmlMerge",
        progress_message = "Merging HTML with dependencies for %s" % ctx.label.name,
        executable = ctx.executable._html_merge_tool,
        tools = [ctx.attr._html_merge_tool.files_to_run],
    )
    return [
        DefaultInfo(files = depset([html_output])),
        SphinxModuleInfo(
            html_dir = html_output,
            own_html_dir = sphinx_html_output,
            # Reuses the already-flattened transitive_modules_from_deps list
            # collected above for the merge step, plus this module itself.
            transitive_modules = depset(
                [struct(name = ctx.label.name, own_html_dir = sphinx_html_output)] +
                transitive_modules_from_deps,
            ),
        ),
        OutputGroupInfo(
            sphinx_sources = depset([config_file] + sphinx_source_files),
        ),
    ]

# ======================================================================================
# Rule definitions
# ======================================================================================
_score_needs = rule(
    implementation = _score_needs_impl,
    attrs = sphinx_rule_attrs,
    toolchains = ["//bazel/rules/rules_score:toolchain_type"],
)
_score_html = rule(
    implementation = _score_html_impl,
    attrs = dict(
        sphinx_rule_attrs,
        _html_merge_tool = attr.label(
            default = Label("//bazel/rules/rules_score:sphinx_html_merge"),
            executable = True,
            cfg = "exec",
        ),
        strip_prefix = attr.string(doc = "Prefix to remove from input file paths."),
        docs_library_deps = attr.label_list(
            doc = "List of sphinx_docs_library targets to include as source files with prefix/strip_prefix handling.",
        ),
        needs = attr.label_list(
            allow_files = True,
            doc = "Submodule symbols.needs targets for this module.",
        ),
        extra_opts_targets = attr.label_list(
            providers = [FilteredExecpathInfo],
            doc = "Label targets that resolve to extra Sphinx arguments at analysis time. " +
                  "Target must provide FilteredExecpathInfo.",
        ),
        extra_opts = attr.string_list(
            doc = "Regular additional string options to pass onto Sphinx.",
        ),
        renamed_srcs = attr.label_keyed_string_dict(
            allow_files = True,
            doc = "Doc source files that are renamed. Keys are file labels, values are " +
                  "destination paths relative to the Sphinx source root. Exactly one " +
                  "file per label. Mirrors sphinx_docs.renamed_srcs from rules_python.",
        ),
    ),
    toolchains = ["//bazel/rules/rules_score:toolchain_type"],
)

# ======================================================================================
# Rule wrappers
# ======================================================================================
def _copy_propagating_kwargs(from_kwargs):
    """Return the subset of macro kwargs that must stay consistent across
    sibling targets with a dependency relationship.

    Deliberately excludes `visibility`: callers of this helper want their
    generated sub-target to NOT inherit the macro's own (often public)
    visibility.
    """
    into_kwargs = {}
    for attr in ("testonly", "tags", "compatible_with", "restricted_to", "target_compatible_with"):
        if attr in from_kwargs:
            into_kwargs[attr] = from_kwargs[attr]
    return into_kwargs

def sphinx_module(
        name,
        srcs,
        index,
        deps = [],
        docs_library_deps = [],
        renamed_srcs = {},
        strip_prefix = None,
        extra_opts = [],
        extra_opts_targets = [],
        allow_persistent_workers = False,
        testonly = False,
        **kwargs):
    """Build a Sphinx module with transitive HTML dependencies.
    This rule builds documentation modules into complete HTML sites with
    transitive dependency collection. Each dependency's HTML is copied into a
    <dep_name>/ subdirectory of the merged site for intersphinx/sphinx-needs
    cross-referencing.

    Generates targets:
    * `<name>`: The merged HTML site (this module's own HTML plus every
      transitive dependency's HTML, each under a `<dep_name>/` subdirectory).
    * `<name>.serve`: A binary that locally serves `<name>`'s HTML output,
      for previewing docs during development (`bazel run //:<name>.serve`).
    * `<name>_needs`: This module's `needs.json` build (see SphinxNeedsInfo).

    Args:
        name: Name of the target
        srcs: List of source files (.rst, .md) with index file first
        index: Label to index.rst file
        deps: List of other sphinx_module targets this module depends on
        docs_library_deps: {type}`list[label]` of {obj}`sphinx_docs_library` targets.
        renamed_srcs: {type}`dict[label, str]` Doc source files that are renamed
                    on their way into the Sphinx source tree.
        strip_prefix: {type}`str | None` A prefix to remove from the file paths of the
                    source files. e.g., given `//sphinxdocs/docs:foo.md`, stripping `docs/` makes
                    Sphinx see `foo.md` in its generated source directory. If not
                specified (None, the default), {any}`native.package_name` + "/" is
                used. Pass "" explicitly to strip nothing -- unlike a plain string
                default, None lets that explicit "" survive, since "" and "not
                specified" are different intents.
        extra_opts: {type}`list[str]` Additional string options to pass onto Sphinx building.
                    On each provided option, a location expansion is performed.
                    See {any}`ctx.expand_location`.
        extra_opts_targets: {type}`list[label]` Label targets that resolve to extra Sphinx
                    arguments at analysis time. Each target must provide FilteredExecpathInfo
                    (e.g. filter_execpath targets).
        allow_persistent_workers: {type}`bool` (experimental) If true, allow Bazel to run the
                    HTML build action as a persistent worker for faster incremental builds.
                    Has no effect on the needs.json build, which never runs as a worker (its
                    source directory is the real checkout, not a generated tree -- see
                    _score_needs_impl), or on the HTML merge step, which never invokes Sphinx.
        visibility: Bazel visibility
    """
    package = native.package_name()
    resolved_strip_prefix = strip_prefix if strip_prefix != None else (package + "/" if package else "")

    # conf.py generation is a private implementation detail consumed only by
    # the sibling _score_needs/_score_html targets below (same package) --
    # _copy_propagating_kwargs both drops visibility and narrows to the
    # attrs that must actually stay consistent across the two.
    conf_kwargs = _copy_propagating_kwargs(kwargs)

    _score_conf(
        name = name + "_needs_conf",
        builder = "needs",
        project_name = (name + "_needs").replace("_", " ").title(),
        output_prefix = name + "_needs",
        testonly = testonly,
        visibility = ["//visibility:private"],
        **conf_kwargs
    )
    _score_conf(
        name = name + "_conf",
        builder = "html",
        project_name = name.replace("_", " ").title(),
        output_prefix = name,
        testonly = testonly,
        visibility = ["//visibility:private"],
        **conf_kwargs
    )
    _score_needs(
        name = name + "_needs",
        srcs = srcs,
        index = index,
        deps = [d + "_needs" for d in deps],
        conf = name + "_needs_conf",
        allow_persistent_workers = allow_persistent_workers,
        testonly = testonly,
        **kwargs
    )
    _score_html(
        name = name,
        srcs = srcs,
        index = index,
        deps = deps,
        docs_library_deps = docs_library_deps,
        renamed_srcs = renamed_srcs,
        strip_prefix = resolved_strip_prefix,
        needs = [d + "_needs" for d in deps],
        conf = name + "_conf",
        extra_opts = extra_opts,
        extra_opts_targets = extra_opts_targets,
        allow_persistent_workers = allow_persistent_workers,
        testonly = testonly,
        **kwargs
    )

    serve_kwargs = _copy_propagating_kwargs(kwargs)
    serve_kwargs["tags"] = list(serve_kwargs.get("tags") or []) + ["manual"]
    py_binary(
        name = name + ".serve",
        srcs = [_SPHINX_SERVE_MAIN_SRC],
        main = _SPHINX_SERVE_MAIN_SRC,
        data = [name],
        args = ["$(execpath {})".format(name)],
        testonly = testonly,
        **serve_kwargs
    )
