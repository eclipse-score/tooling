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
load("@aspect_rules_lint//format:defs.bzl", "format_multirun", "format_test")

# Maps a user-facing language name to the (attribute, formatter label) pair that
# is forwarded to aspect_rules_lint's format_multirun / format_test rules.
_FORMATTERS = {
    "rust": ("rust", "@score_tooling//format_checker:rustfmt_with_policies"),
    "starlark": ("starlark", "@buildifier_prebuilt//:buildifier"),
    "cpp": ("cc", "@llvm_toolchain//:clang-format"),
}

# Languages enabled when the caller does not specify a selection. C++ is left
# out so that projects without C++ code do not depend on the LLVM toolchain.
_DEFAULT_LANGUAGES = ["rust", "starlark"]

def use_format_targets(fix_name = "format.fix", check_name = "format.check", languages = None):
    """Registers format.fix and format.check targets for the selected languages.

    Args:
        fix_name: Name of the format_multirun target that applies formatting.
        check_name: Name of the format_test target that checks formatting.
        languages: List of language names to enable. Supported values are the
            keys of _FORMATTERS.
    """
    if languages == None:
        languages = _DEFAULT_LANGUAGES

    formatters = {}
    for language in languages:
        if language not in _FORMATTERS:
            fail("Unsupported format language '{}'. Supported languages are: {}.".format(
                language,
                ", ".join(sorted(_FORMATTERS.keys())),
            ))
        attribute, label = _FORMATTERS[language]
        formatters[attribute] = label

    format_multirun(
        name = fix_name,
        visibility = ["//visibility:public"],
        tags = ["manual"],
        **formatters
    )

    format_test(
        name = check_name,
        no_sandbox = True,
        workspace = "//:MODULE.bazel",
        tags = ["manual"],
        visibility = ["//visibility:public"],
        **formatters
    )
