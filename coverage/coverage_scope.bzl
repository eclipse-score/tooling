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

"""
Coverage scope rule for deriving file-level allowlists from implementation targets.

This rule uses an aspect to traverse the build graph starting from the listed
implementation targets (cc_library, rust_library, rust_binary) through their
transitive deps. At each node it collects the actual source files.

- cc_library (and rust_library, which also provides CcInfo in rules_rust
  0.68.x): source files come from the srcs/hdrs attributes; static archives
  (including the rlib exposed as a .a symlink) are collected for baseline
  (zero-coverage) reporting.
- rust_binary (no CcInfo): source files come from CrateInfo.srcs; the
  coverage-built executable itself serves as the baseline object.

The resulting allowlist contains one source file path per line (relative to the
workspace root). The coverage reporter uses this to restrict reports to exactly
the files that are part of the covered implementation.
"""

load("@rules_rust//rust:rust_common.bzl", "CrateInfo")

# =============================================================================
# Provider to carry collected source file paths through the aspect
# =============================================================================

_CoverageScopeInfo = provider(
    doc = "Carries source file paths and object files collected by the coverage scope aspect.",
    fields = {
        "source_files": "Depset of source file path strings (workspace-relative).",
        "object_files": "Depset of compiled archive/executable File objects for baseline coverage.",
    },
)

# =============================================================================
# Aspect: traverses library/binary deps to collect files
# =============================================================================

def _coverage_scope_aspect_impl(target, ctx):
    """Collects source file paths and archive files from the build graph."""
    direct_files = []
    direct_archives = []
    transitive = []
    transitive_archives = []

    # At cc_library / rust_library targets (rust_library provides CcInfo with
    # its rlib exposed as a .a symlink): collect srcs, hdrs, and static archive
    if CcInfo in target:
        for attr_name in ["srcs", "hdrs"]:
            if hasattr(ctx.rule.attr, attr_name):
                for src in getattr(ctx.rule.attr, attr_name):
                    for f in src.files.to_list():
                        if not f.path.startswith("external/") and f.is_source:
                            direct_files.append(f.short_path)

        # Only collect workspace-internal labels and archives
        if not str(target.label).startswith("@@") or str(target.label).startswith("@@//"):
            # Collect .a archive files for baseline coverage.
            for linker_input in target[CcInfo].linking_context.linker_inputs.to_list():
                for lib in linker_input.libraries:
                    for archive in [lib.static_library, lib.pic_static_library]:
                        if archive and "/external/" not in archive.path and not archive.path.startswith("external/"):
                            direct_archives.append(archive)
                            break
    elif CrateInfo in target:
        # rust_binary: no CcInfo, collect .rs sources from CrateInfo and use the
        # coverage-built executable as the baseline object.
        for f in target[CrateInfo].srcs.to_list():
            if not f.path.startswith("external/") and f.is_source:
                direct_files.append(f.short_path)
        out = target[CrateInfo].output
        if out and "/external/" not in out.path and not out.path.startswith("external/"):
            direct_archives.append(out)

    # Propagate from children traversed by the aspect
    for attr_name in ["components", "implementation", "deps", "implementation_deps", "exported_deps"]:
        if hasattr(ctx.rule.attr, attr_name):
            for dep in getattr(ctx.rule.attr, attr_name):
                if _CoverageScopeInfo in dep:
                    transitive.append(dep[_CoverageScopeInfo].source_files)
                    transitive_archives.append(dep[_CoverageScopeInfo].object_files)

    return [_CoverageScopeInfo(
        source_files = depset(direct_files, transitive = transitive),
        object_files = depset(direct_archives, transitive = transitive_archives),
    )]

_coverage_scope_aspect = aspect(
    implementation = _coverage_scope_aspect_impl,
    attr_aspects = ["components", "implementation", "deps", "implementation_deps", "exported_deps"],
    doc = "Traverses cc_library/rust_library/rust_binary hierarchy to collect implementation source files.",
)

# =============================================================================
# Rule: aggregates aspect results into an allowlist file
# =============================================================================

def _coverage_scope_impl(ctx):
    """Aggregates source file paths from all deps and writes allowlist + baseline objects."""
    all_files = {}
    all_objects = []

    for dep in ctx.attr.deps:
        if _CoverageScopeInfo in dep:
            for path in dep[_CoverageScopeInfo].source_files.to_list():
                if path:
                    all_files[path] = True
            all_objects.append(dep[_CoverageScopeInfo].object_files)

    sorted_files = sorted(all_files.keys())
    object_depset = depset(transitive = all_objects)

    # Write the allowlist file
    output = ctx.actions.declare_file(ctx.attr.name + "_allowlist.txt")
    ctx.actions.write(
        output = output,
        content = "\n".join(sorted_files) + "\n" if sorted_files else "",
    )

    # Write archive file paths for baseline coverage (reporter uses these as --object args)
    archive_paths = sorted(set([f.short_path for f in object_depset.to_list()]))
    objects_output = ctx.actions.declare_file(ctx.attr.name + "_objects.txt")
    ctx.actions.write(
        output = objects_output,
        content = "\n".join(archive_paths) + "\n" if archive_paths else "",
    )

    return [
        DefaultInfo(files = depset([output, objects_output], transitive = [object_depset])),
        OutputGroupInfo(
            allowlist = depset([output]),
            objects = depset([objects_output]),
            object_files = object_depset,
        ),
    ]

def _coverage_transition_impl(settings, attr):
    # This dictionary modifies the build configuration
    return {
        "//command_line_option:collect_code_coverage": True,
    }

# Define the transition
coverage_transition = transition(
    implementation = _coverage_transition_impl,
    inputs = [],
    outputs = ["//command_line_option:collect_code_coverage"],
)

def _coverage_wrapper_impl(ctx):
    # Forward the executable or providers from the underlying target
    actual_target = ctx.attr.actual[0]
    return [actual_target[DefaultInfo]]

# Define a rule that applies the transition to its 'actual' dependency
coverage_wrapper = rule(
    implementation = _coverage_wrapper_impl,
    attrs = {
        "actual": attr.label(
            mandatory = True,
            cfg = coverage_transition,  # Applying the transition here
        ),
        # Mandatory attribute needed when a rule uses a transition
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)

coverage_scope = rule(
    implementation = _coverage_scope_impl,
    doc = """Generates a file-level coverage allowlist from implementation targets.

    Uses an aspect to traverse the listed targets (cc_library, rust_library,
    rust_binary) and their transitive deps, collecting all source files
    (srcs + hdrs / CrateInfo.srcs). Outputs a text file with one
    workspace-relative file path per line.

    This allowlist is consumed by the coverage reporter to restrict coverage
    reporting to exactly the source files that are part of the implementation.
    """,
    attrs = {
        "deps": attr.label_list(
            mandatory = True,
            aspects = [_coverage_scope_aspect],
            cfg = coverage_transition,
            doc = "Implementation targets whose transitive deps define the coverage scope.",
        ),
    },
)
