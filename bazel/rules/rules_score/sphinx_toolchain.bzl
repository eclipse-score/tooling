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
"""Sphinx toolchain: the SphinxInfo provider, the sphinx_toolchain rule, and
the score_sphinx_toolchain() convenience macro for consumers."""

load("@rules_python//python:py_binary.bzl", "py_binary")
load("@sphinxdocs//sphinxdocs:sphinx.bzl", "sphinx_build_binary")

SphinxInfo = provider(
    doc = "Provider for Sphinx Toolchain",
    fields = {
        "sphinx": "sphinx executable",
        "conf_template": "template for conf.py",
    },
)

def _sphinx_toolchain_impl(ctx):
    toolchain_info = platform_common.ToolchainInfo(
        sphinxinfo = SphinxInfo(
            sphinx = ctx.attr.sphinx,
            conf_template = ctx.attr.conf_template,
        ),
    )
    return [toolchain_info]

sphinx_toolchain = rule(
    implementation = _sphinx_toolchain_impl,
    attrs = {
        "sphinx": attr.label(
            default = Label("//bazel/rules/rules_score:raw_build"),
        ),
        "conf_template": attr.label(
            allow_single_file = True,
            default = Label("//bazel/rules/rules_score:templates/conf.template.py"),
        ),
    },
)

def score_sphinx_toolchain(
        name,
        extra_deps = [],
        deps = None,
        extra_data = [],
        conf_template = None,
        package_collisions = None,
        py_binary_rule = py_binary,
        **kwargs):
    """Declares a sphinx_toolchain, reusing score_tooling's default build binary.

    Emits `<name>_binary` (the Sphinx build py_binary, built via
    @rules_python//sphinxdocs:sphinx.bzl's `sphinx_build_binary` from
    rules_python's own sphinx_build.py -- persistent-worker/param-file
    support come from there, not from anything score_tooling ports or
    re-implements), `<name>_info` (the `sphinx_toolchain` target), and
    `<name>` (the `toolchain()` itself). Register the result yourself so the
    root module's registration takes precedence over score_tooling's own
    default:

        register_toolchains("//:<name>")

    Args:
        name: Name of the emitted `toolchain()` target.
        extra_deps: Extend mode. Extra Python deps added on top of
            `@score_tooling//bazel/rules/rules_score:sphinx_base_deps` — the
            same deps used by score_tooling's own default toolchain. Mutually
            exclusive with `deps`.
        deps: Replace mode. Exact list of Python deps for the Sphinx build
            binary, bypassing `sphinx_base_deps` entirely. Use this when a
            package version conflicts with the shared deps (e.g. a separate
            pip hub). Mutually exclusive with `extra_deps`.
        extra_data: Extra data files/targets for the Sphinx build binary.
        conf_template: Label of a conf.py template. Defaults to score_tooling's
            generic template if not given.
        package_collisions: Forwarded to the generated py_binary, if
            `py_binary_rule` accepts it (e.g. aspect_rules_py's `py_binary`,
            which uses it to control venv-collision handling). The default
            `py_binary_rule` (rules_python's own) has no such attr --
            passing this without also overriding `py_binary_rule` fails with
            a clear error instead of a raw "unexpected keyword" from Bazel.
        py_binary_rule: {type}`callable` A `py_binary`-compatible rule/macro
            used to build the Sphinx build binary, forwarded to
            `sphinx_build_binary`. Defaults to rules_python's own `py_binary`.
            Pass aspect_rules_py's `py_binary` (or another py_binary-compatible
            rule) here if a consumer's deps need its venv semantics -- e.g.
            deps whose imports rely on aspect's runfiles layout rather than
            rules_python's.
        **kwargs: Forwarded to the `toolchain()` target (e.g. `visibility`,
            `exec_compatible_with`, `target_compatible_with`).
    """
    if deps != None and extra_deps:
        fail("score_sphinx_toolchain: pass either `deps` (replace mode) or " +
             "`extra_deps` (extend mode) for target '%s', not both" % name)
    if package_collisions != None and py_binary_rule == py_binary:
        fail("score_sphinx_toolchain: package_collisions has no effect with " +
             "the default py_binary_rule (rules_python's py_binary has no " +
             "such attr) for target '%s' -- pass py_binary_rule explicitly " % name +
             "(e.g. aspect_rules_py's py_binary) alongside package_collisions, " +
             "or drop package_collisions.")

    binary_deps = deps if deps != None else (
        ["@score_tooling//bazel/rules/rules_score:sphinx_base_deps"] + extra_deps
    )

    binary_kwargs = dict(
        data = extra_data,
        deps = binary_deps,
        visibility = ["//visibility:private"],
    )
    if package_collisions != None:
        binary_kwargs["package_collisions"] = package_collisions

    sphinx_build_binary(
        name = name + "_binary",
        py_binary_rule = py_binary_rule,
        **binary_kwargs
    )

    toolchain_kwargs = {}
    if conf_template != None:
        toolchain_kwargs["conf_template"] = conf_template

    sphinx_toolchain(
        name = name + "_info",
        sphinx = ":" + name + "_binary",
        **toolchain_kwargs
    )

    native.toolchain(
        name = name,
        toolchain = ":" + name + "_info",
        toolchain_type = "@score_tooling//bazel/rules/rules_score:toolchain_type",
        **kwargs
    )
