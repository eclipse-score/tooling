# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
Test specification rule for S-CORE projects.

A ``test_specification`` holds one formal test specification written in RST
(Test Goal, Test Focus, Test Steps). Test cases reference it by its Bazel
label via ``RecordProperty("test-spec", "//pkg:name")`` instead of
Given/When/Then. The RST format is validated at build time.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo", "TestSpecificationInfo")

def _repo_relative_label(label):
    """Render *label* as ``//pkg:name``, relative to its own repository.

    Independent of the canonical repository name, so the result does not change
    when the spec is built as part of an external dependency.
    """
    return "//{}:{}".format(label.package, label.name)

def _test_specification_impl(ctx):
    src = ctx.file.src
    spec_json = ctx.actions.declare_file(ctx.label.name + ".test_spec.json")

    args = ctx.actions.args()
    args.add("--src", src)
    args.add("--ref", _repo_relative_label(ctx.label))
    args.add("--alias", _repo_relative_label(src.owner))
    args.add("--out", spec_json)

    ctx.actions.run(
        executable = ctx.executable._parse_test_spec,
        inputs = [src],
        outputs = [spec_json],
        arguments = [args],
        mnemonic = "TestSpecificationCheck",
        progress_message = "Checking test specification %{label}",
    )

    return [
        DefaultInfo(files = depset([spec_json])),
        TestSpecificationInfo(src = src, spec_json = spec_json),
        SphinxSourcesInfo(
            srcs = depset([src]),
            deps = depset([src]),
            aux_srcs = depset(),
        ),
        # Validate the format whenever a dependent (e.g. a unit) is built,
        # even if nothing else consumes the JSON in that build.
        OutputGroupInfo(_validation = depset([spec_json])),
    ]

test_specification = rule(
    implementation = _test_specification_impl,
    doc = """Formal test specification in RST, referenced by test cases via RecordProperty("test-spec", "<label>").

The file must contain exactly one title section with the subsections
``Test Goal`` (mandatory), ``Test Focus`` (optional) and ``Test Steps``
(mandatory, one subsection per step with the fields ``:Precondition:``
(optional), ``:Action:`` and ``:Expected:``). A malformed file fails the build.

Example:
    ```python
    test_specification(
        name = "conn_fault_spec",
        src = "conn_fault_spec.rst",
    )

    unit(
        name = "my_unit",
        ...
        tests = [":my_unit_test"],
        test_specs = [":conn_fault_spec"],
    )
    ```
""",
    attrs = {
        "src": attr.label(
            allow_single_file = [".rst"],
            mandatory = True,
            doc = "RST file holding exactly one test specification.",
        ),
        "_parse_test_spec": attr.label(
            default = Label("//bazel/rules/rules_score/src/test_case_coverage:parse_test_spec"),
            executable = True,
            cfg = "exec",
        ),
    },
)
