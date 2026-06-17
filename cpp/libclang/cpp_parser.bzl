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
load("@bazel_skylib//rules:common_settings.bzl", "BuildSettingInfo")
load("@rules_cc//cc:find_cc_toolchain.bzl", "find_cc_toolchain", "use_cc_toolchain")
load("@rules_cc//cc/common:cc_common.bzl", "cc_common")

def _extract_files_from_attr(attr, attr_name):
    files = []
    for src in getattr(attr, attr_name, []):
        if hasattr(src, "files"):
            files.extend(src.files.to_list())
    return files

SourceFilesInfo = provider(fields = ["files", "inputs"])

def _cc_sources_aspect_impl(target, ctx):
    direct_srcs = _extract_files_from_attr(ctx.rule.attr, "srcs")
    direct_hdrs = _extract_files_from_attr(ctx.rule.attr, "hdrs")
    direct_textual_hdrs = _extract_files_from_attr(ctx.rule.attr, "textual_hdrs")

    transitive_inputs = []
    for dep in getattr(ctx.rule.attr, "deps", []):
        if SourceFilesInfo in dep:
            transitive_inputs.append(dep[SourceFilesInfo].inputs)

    if direct_srcs:
        files = direct_srcs
    else:
        files = []
        files.extend(direct_hdrs)
        files.extend(direct_textual_hdrs)

    direct_inputs = []
    direct_inputs.extend(direct_srcs)
    direct_inputs.extend(direct_hdrs)
    direct_inputs.extend(direct_textual_hdrs)

    return [
        SourceFilesInfo(
            files = depset(files),
            inputs = depset(direct_inputs, transitive = transitive_inputs),
        ),
    ]

cc_sources_aspect = aspect(
    implementation = _cc_sources_aspect_impl,
    attr_aspects = ["deps"],
)

CompilationFlagsInfo = provider(
    doc = "Collected compilation flags for a target and its transitive deps",
    fields = {
        "flags": "depset of compilation flags",
    },
)

def _collect_from_cc_info(target):
    flags = []

    if CcInfo in target:
        cc_info = target[CcInfo]

        if hasattr(cc_info, "compilation_context"):
            cc_ctx = cc_info.compilation_context

            flags.extend(["-D%s" % d for d in cc_ctx.defines.to_list()])
            flags.extend(["-D%s" % d for d in cc_ctx.local_defines.to_list()])
            flags.extend(["-I%s" % p for p in cc_ctx.includes.to_list()])
            flags.extend(["-iquote%s" % p for p in cc_ctx.quote_includes.to_list()])
            flags.extend(["-isystem%s" % p for p in cc_ctx.system_includes.to_list()])
            flags.extend(["-isystem%s" % p for p in cc_ctx.external_includes.to_list()])
            flags.extend(["-F%s" % p for p in cc_ctx.framework_includes.to_list()])

    return flags

def _compilation_flags_aspect_impl(target, ctx):
    transitive = []

    for dep in getattr(ctx.rule.attr, "deps", []):
        if CompilationFlagsInfo in dep:
            transitive.append(dep[CompilationFlagsInfo].flags)

    direct_flags = _collect_from_cc_info(target)

    return [
        CompilationFlagsInfo(
            flags = depset(direct_flags, transitive = transitive),
        ),
    ]

compilation_flags_aspect = aspect(
    implementation = _compilation_flags_aspect_impl,
    attr_aspects = ["deps"],
)

def _detect_standard_from_flags(ctx):
    """
    Fall back: compile the action's compile flags and look for -std=.

    This covers toolchains that express the standard via compiler flags
    rather than named features.
    """

    cc_toolchain = find_cc_toolchain(ctx)
    feature_configuration = cc_common.configure_features(
        ctx = ctx,
        cc_toolchain = cc_toolchain,
        # Request every standard feature so configure_features can see them.
        requested_features = ctx.features,
        unsupported_features = ctx.disabled_features,
    )
    compile_variables = cc_common.create_compile_variables(
        feature_configuration = feature_configuration,
        cc_toolchain = cc_toolchain,
        # We only care about the flag shape, not a real file.
        source_file = "/dev/null",
        output_file = "/dev/null",
    )
    flags = cc_common.get_memory_inefficient_command_line(
        feature_configuration = feature_configuration,
        action_name = "c++-compile",
        variables = compile_variables,
    )

    modern_default = "-std=c++11"

    for flag in reversed(flags):
        if flag.startswith("-std="):
            return flag
    return modern_default

def _collect_required_llvm_include_args(cxx_builtin_include_files, extra_config_site_files):
    """
    Build required libc++/clang builtin include flags from LLVM toolchain attributes.

    Derives all paths dynamically from the cxx_builtin_include and extra_config_site
    filegroups exposed by the LLVM toolchain

    Args:
        cxx_builtin_include_files: Files from @llvm_toolchain_llvm//:cxx_builtin_include,
            containing the libc++ headers directory (include/c++) and the clang resource
            include directory (lib/clang/<version>/include).
        extra_config_site_files: Files from @llvm_toolchain_llvm//:extra_config_site,
            containing the arch-specific __config_site file(s) used to locate the ABI
            include directory (include/<triple>/c++/v1).
    """
    libcxx_include = None
    resource_include = None

    for f in cxx_builtin_include_files:
        if "/lib/clang/" in f.path:
            resource_include = f.path
        elif f.path.endswith("/include/c++"):
            libcxx_include = f.path + "/v1"

    if not libcxx_include or not resource_include:
        fail("Could not derive LLVM include paths from cxx_builtin_include filegroup. " +
             "Got files: %s" % [f.path for f in cxx_builtin_include_files])

    resource_dir = resource_include.rpartition("/include")[0]

    result = [
        "-isystem",
        libcxx_include,
    ]

    if len(extra_config_site_files) > 1:
        fail("Expected at most one arch-specific __config_site file, got: %s" %
             [f.path for f in extra_config_site_files])

    for f in extra_config_site_files:
        result += ["-isystem", f.dirname]

    result += [
        "-resource-dir",
        resource_dir,
        "-isystem",
        resource_include,
    ]

    return result

def _cpp_parser_impl(ctx):
    output_dir = ctx.actions.declare_directory(
        ctx.label.name + "_result",
    )
    libclang = ctx.file._libclang

    args = []

    args += [
        "--output-dir",
        output_dir.path,
    ]

    if ctx.attr.emit_debug_json:
        args.append("--json")

    target_compilation_flags_list = ctx.attr.target[CompilationFlagsInfo].flags.to_list()

    cxx_builtin_include_files = ctx.attr._llvm_cxx_builtin_include.files.to_list()
    extra_config_site_files = ctx.attr._llvm_extra_config_site.files.to_list()
    llvm_include_args = _collect_required_llvm_include_args(cxx_builtin_include_files, extra_config_site_files)

    extra_args = [
        _detect_standard_from_flags(ctx),
        "-nostdinc++",
    ]
    extra_args += llvm_include_args
    extra_args += target_compilation_flags_list + ctx.attr.extra_args
    for ea in extra_args:
        args += ["--extra-arg", ea]

    target_source_files_info = ctx.attr.target[SourceFilesInfo]
    target_source_files_list = target_source_files_info.files.to_list()
    target_source_inputs_list = target_source_files_info.inputs.to_list()

    # extend args with input sources
    if SourceFilesInfo in ctx.attr.target:
        args += [file.path for file in target_source_files_list]

    inputs = [
        libclang,
    ] + target_source_inputs_list + cxx_builtin_include_files + extra_config_site_files

    ctx.actions.run(
        inputs = inputs,
        outputs = [output_dir],
        executable = ctx.executable.tool,
        tools = [ctx.attr.tool[DefaultInfo].files_to_run],
        arguments = args,
        env = {
            "LIBCLANG_PATH": libclang.dirname,
            "LIBCLANG_LOG": ctx.attr._log_level[BuildSettingInfo].value,
        },
        mnemonic = "CppAnalyze",
        # this is required to parse some system headers
        execution_requirements = {
            "no-sandbox": "1",
        },
        progress_message = "Running C++ AST analysis: %s" % ctx.label,
    )

    return DefaultInfo(
        files = depset([output_dir]),
        runfiles = ctx.runfiles(files = [output_dir]),
    )

cpp_parser = rule(
    implementation = _cpp_parser_impl,
    attrs = {
        "target": attr.label(
            aspects = [cc_sources_aspect, compilation_flags_aspect],
            mandatory = True,
        ),
        "tool": attr.label(
            executable = True,
            cfg = "exec",
            mandatory = True,
        ),
        "extra_args": attr.string_list(
            default = [],
        ),
        "emit_debug_json": attr.bool(
            default = False,
            doc = "Emit debug.json alongside the FlatBuffer output. Intended for tests/debugging.",
        ),
        "_libclang": attr.label(
            allow_single_file = True,
            default = "@llvm_toolchain_llvm//:lib/libclang.so",
        ),
        "_llvm_cxx_builtin_include": attr.label(
            default = "@llvm_toolchain_llvm//:cxx_builtin_include",
            doc = "LLVM toolchain filegroup containing the libc++ header directory (include/c++) " +
                  "and the clang resource include directory (lib/clang/<version>/include).",
        ),
        "_llvm_extra_config_site": attr.label(
            default = "@llvm_toolchain_llvm//:extra_config_site",
            doc = "LLVM toolchain filegroup containing the arch-specific __config_site file " +
                  "(include/<triple>/c++/v1/__config_site) used to locate the ABI include path.",
        ),
        "_log_level": attr.label(
            default = Label("//cpp/libclang:log_level"),
            doc = "Build setting that controls clang_rs_parser log level.",
        ),
    },
    toolchains = use_cc_toolchain(),
    fragments = ["cpp"],
)
