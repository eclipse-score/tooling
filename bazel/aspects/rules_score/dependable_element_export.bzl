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

"""CLI-level aspect exporting dependable_element (rules_score SEooC) artifacts.

The aspect exposes every dependable_element (rules_score SEooC)'s own doc/report
artifacts as a dedicated output group. It no-ops for any target that isn't a
dependable_element, so it is safe to apply unconditionally on //...
Every `dependable_element`'s own doc/report artifacts become an explicitly requestable output group of that *same* build invocation - without changing what is built for any other target.

Rather than merely re-exposing each dependable_element's existing output files as-is, this
aspect *relocates/copies* them under a synthetic `_dependable_element_export/` path segment to simplify downloading it from remote-execution or uploading it in CI.

Usage:

bazel build //<your_target(s)> \
    --aspects=@score_tooling//bazel/aspects/rules_score:dependable_element_export.bzl%dependable_element_export_aspect \
    --output_groups=+dependable_element_export \
    --remote_download_regex=".*/_dependable_element_export/.*"

See README.md next to this file for details.
"""

load("@score_tooling//bazel/rules/rules_score:providers.bzl", "DependableElementInfo")

_EXPORT_DIR = "_dependable_element_export"

def _dependable_element_export_aspect_impl(target, ctx):
    """Re-export a dependable_element's own default outputs under a uniform path segment.

    No-ops for any target that isn't a dependable_element (identified via the
    `DependableElementInfo` provider that `dependable_element()` forwards for exactly this kind
    of cross-target lookup - see rules_score's own use of it for integrity-level checks).
    """
    if DependableElementInfo not in target:
        return []

    # Own package prefix of the file's short_path, if present, is stripped so the relocated
    # path doesn't redundantly nest the package path inside itself a second time.
    own_package_prefix = ctx.label.package + "/"

    exported_files = []
    for f in target[DefaultInfo].files.to_list():
        short_path = f.short_path
        if short_path.startswith("../"):
            # Drop external repo prefix.
            relative_path = short_path.split("/", 2)[2]
        elif short_path.startswith(own_package_prefix):
            relative_path = short_path[len(own_package_prefix):]
        else:
            relative_path = short_path

        # Some dependable_element outputs are tree artifacts (e.g. a Sphinx HTML directory),
        # not plain files - handled with a directory copy instead of a single-file one.
        if f.is_directory:
            exported = ctx.actions.declare_directory(_EXPORT_DIR + "/" + relative_path)
            ctx.actions.run_shell(
                inputs = [f],
                outputs = [exported],
                command = "mkdir -p \"$2\" && cp -r \"$1\"/. \"$2\"/",
                arguments = [f.path, exported.path],
                mnemonic = "DependableElementExportCopyDir",
                progress_message = "Exporting dependable_element directory " + f.short_path,
            )
        else:
            exported = ctx.actions.declare_file(_EXPORT_DIR + "/" + relative_path)
            ctx.actions.run_shell(
                inputs = [f],
                outputs = [exported],
                command = "mkdir -p \"$(dirname \"$2\")\" && cp \"$1\" \"$2\"",
                arguments = [f.path, exported.path],
                mnemonic = "DependableElementExportCopyFile",
                progress_message = "Exporting dependable_element file " + f.short_path,
            )
        exported_files.append(exported)

    return [
        OutputGroupInfo(
            dependable_element_export = depset(exported_files),
        ),
    ]

dependable_element_export_aspect = aspect(
    implementation = _dependable_element_export_aspect_impl,
    doc = (
        "Exports dependable_element (rules_score SEooC) default outputs (doc/report " +
        "artifacts), relocated (via copy actions) under a uniform " +
        "'_dependable_element_export/' path segment, as a dedicated " +
        "'dependable_element_export' output group - for use as an additive `--aspects=` flag " +
        "on an otherwise unrelated build."
    ),
)
