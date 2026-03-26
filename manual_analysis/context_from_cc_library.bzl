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

load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load(":manual_analysis.bzl", "ManualAnalysisContextInfo")

_CollectedCcLibrarySourcesInfo = provider(
    doc = "Internal provider with transitive cc_library context files.",
    fields = {
        "files": "depset of source/header files and compiled outputs from a cc_library dependency graph",
        "rules": "depset of serialized <label>\\t<canonical_form> entries for relevant attributes",
    },
)

_SRC_ATTRS = [
    "srcs",
    "data",
    "hdrs",
    "additional_compiler_inputs",
    "additional_linker_inputs",
    "module_interfaces",
    "textual_hdrs",
    "win_def_file",
]

_HASHED_ATTRS = [
    "name",
    "alwayslink",
    "conlyopts",
    "copts",
    "cxxopts",
    "defines",
    "include_prefix",
    "includes",
    "linkopts",
    "linkstamp",
    "linkstatic",
    "local_defines",
    "strip_include_prefix",
]

_DEPS_ATTRS = [
    "deps",
    "implementation_deps",
]

def _cc_library_direct_files(ctx):
    files = []
    for attr_name in _SRC_ATTRS:
        if hasattr(ctx.rule.files, attr_name):
            files.extend(getattr(ctx.rule.files, attr_name))
    return files

def _normalize_attr_value(value):
    if type(value) == type(""):
        return value
    if type(value) == type([]):
        return _normalize_list_attr_value(value)
    if type(value) == type(True) or type(value) == type(0) or value == None:
        return value
    return str(value)

def _normalize_list_attr_value(value):
    return [_normalize_attr_value(item) for item in value]

def _cc_library_direct_rules(target, ctx):
    """Build canonical attribute form for hashing in Python.

    Returns a serialized <label>\t<canonical_form> entry. The canonical_form
    will be hashed in Python (update_lock.py) using SHA256.
    """
    canonical_attrs = {}
    for attr_name in _HASHED_ATTRS:
        if hasattr(ctx.rule.attr, attr_name):
            canonical_attrs[attr_name] = _normalize_attr_value(getattr(ctx.rule.attr, attr_name))
        else:
            canonical_attrs[attr_name] = None

    canonical_form = json.encode(canonical_attrs)
    return ["{}\t{}".format(str(target.label), canonical_form)]

def _cc_library_transitive_files(ctx):
    files = []
    for attr in _DEPS_ATTRS:
        for dep in getattr(ctx.rule.attr, attr, []):
            if _CollectedCcLibrarySourcesInfo in dep:
                files.append(dep[_CollectedCcLibrarySourcesInfo].files)
    return files

def _cc_library_transitive_rules(ctx):
    rules = []
    for attr in _DEPS_ATTRS:
        for dep in getattr(ctx.rule.attr, attr, []):
            if _CollectedCcLibrarySourcesInfo in dep:
                rules.append(dep[_CollectedCcLibrarySourcesInfo].rules)
    return rules

def _collect_cc_library_sources_aspect_impl(target, ctx):
    return [_CollectedCcLibrarySourcesInfo(
        files = depset(
            direct = _cc_library_direct_files(ctx),
            transitive = _cc_library_transitive_files(ctx),
        ),
        rules = depset(
            direct = _cc_library_direct_rules(target, ctx),
            transitive = _cc_library_transitive_rules(ctx),
        ),
    )]

_collect_cc_library_sources_aspect = aspect(
    implementation = _collect_cc_library_sources_aspect_impl,
    attr_aspects = _DEPS_ATTRS,
    doc = "Collects transitive source/header files and compiled outputs from cc_library deps.",
)

def _manual_analysis_context_from_cc_library_impl(ctx):
    info = ctx.attr.library[_CollectedCcLibrarySourcesInfo]
    files = info.files
    rules = info.rules
    return [
        DefaultInfo(files = files),
        ManualAnalysisContextInfo(
            files = files,
            rules = rules,
        ),
    ]

manual_analysis_context_from_cc_library = rule(
    doc = "Creates ManualAnalysisContextInfo from a cc_library and its transitive deps.",
    implementation = _manual_analysis_context_from_cc_library_impl,
    attrs = {
        "library": attr.label(
            doc = "cc_library target used as analysis context root.",
            mandatory = True,
            providers = [CcInfo],
            aspects = [_collect_cc_library_sources_aspect],
        ),
    },
)
