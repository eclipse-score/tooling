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

load("@bazel_binaries//:defs.bzl", "bazel_binaries")
load(
    "@rules_bazel_integration_test//bazel_integration_test:defs.bzl",
    "bazel_integration_test",
)

_INHERITED_ENV = [
    "http_proxy",
    "https_proxy",
    "no_proxy",
    "all_proxy",
    "ftp_proxy",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "ALL_PROXY",
    "FTP_PROXY",
    "L4_CACHE_DIR",
    "L4_DOWNLOADER_CONFIG",
]

_TAGS = [
    "external",
    "no-sandbox",
    "requires-network",
]

def l4_test(
        name,
        module,
        overrides = [],
        cases = None,
        variant_build = [],
        timeout = "eternal",
        **kwargs):
    """Runs a consumer module in a nested Bazel process.

    Args:
      name: Test target name.
      module: Name of the consumer module (a bazel_dep with local_path_override).
      overrides: Modules the consumer overrides with local paths, as in its own
        MODULE.bazel. score_tooling is always overridden.
      cases: Optional TSV file; runs its cases instead of the consumer's own tests.
      variant_build: Optional target patterns and flags for an additional build.
      timeout: Bazel test timeout.
      **kwargs: Passed to bazel_integration_test; tags are added to the defaults.
    """
    tags = _TAGS + kwargs.pop("tags", [])
    override_modules = ["score_tooling"] + overrides
    override_values = [
        "{}=$(rootpath @{}//:MODULE.bazel)".format(module_name, module_name)
        for module_name in override_modules
    ]
    data = [
        "@{}//:MODULE.bazel".format(module_name)
        for module_name in override_modules
    ]
    env = {"OVERRIDE_MODULES": " ".join(override_values)}

    if variant_build:
        env["L4_VARIANT_BUILD"] = " ".join(variant_build)

    if cases != None:
        data.append(cases)
        env["L4_CASES"] = "$(rootpath {})".format(cases)

    bazel_integration_test(
        name = name,
        bazel_binaries = bazel_binaries,
        bazel_version = bazel_binaries.versions.current,
        test_runner = Label("//:run_child"),
        # Canonical runfiles directory of a bzlmod module repository.
        workspace_path = module + "+",
        workspace_files = ["@{}//:MODULE.bazel".format(module)],
        data = data,
        env = env,
        additional_env_inherit = _INHERITED_ENV,
        tags = tags,
        timeout = timeout,
        **kwargs
    )
