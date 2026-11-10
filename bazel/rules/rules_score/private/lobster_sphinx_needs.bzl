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

"""Subrule converting Sphinx-Needs ``needs.json`` files to LOBSTER items.

Counterpart of ``subrule_lobster_trlc``: the resulting ``.lobster`` file holds
the same kind of requirement items, so it can be passed on wherever a
``lobster-trlc`` output is expected.
"""

def needs_json_labels(needs):
    """Map ``docs_bundle`` labels to their ``needs.json`` export.

    A ``docs_bundle`` of score_docs_as_code exports the Needs of its own sources
    as ``<name>.__internal__.needs_local``. Labels of ``.json`` files and of
    ``needs_json_file`` targets are kept unchanged.

    Args:
        needs: Labels of ``docs_bundle`` targets or ``needs.json`` files.

    Returns:
        List of labels producing ``needs.json`` files.
    """
    result = []
    for label in needs:
        label_str = str(label)
        target = label_str.rsplit(":", 1)[-1]
        if target.endswith(".json") or target == "needs_json_file" or "__internal__" in target:
            result.append(label)
        else:
            result.append(label_str + ".__internal__.needs_local")
    return result

def _lobster_sphinx_needs_impl(ctx, needs_files, config, rst_basename, title, _lobster_sphinx_needs):
    lobster_file = ctx.actions.declare_file(ctx.label.name + "_needs.lobster")
    rst_file = ctx.actions.declare_file(rst_basename)

    args = ctx.actions.args()
    args.add("--config", config)
    args.add("--out", lobster_file)
    args.add("--rst-out", rst_file)
    args.add("--title", title)
    args.add_all(needs_files)

    ctx.actions.run(
        executable = _lobster_sphinx_needs,
        inputs = needs_files + [config],
        outputs = [lobster_file, rst_file],
        arguments = [args],
        mnemonic = "LobsterSphinxNeeds",
        progress_message = "lobster-sphinx-needs %s" % lobster_file.short_path,
    )
    return lobster_file, rst_file

subrule_lobster_sphinx_needs = subrule(
    implementation = _lobster_sphinx_needs_impl,
    attrs = {
        "_lobster_sphinx_needs": attr.label(
            default = Label("//bazel/rules/rules_score:lobster_sphinx_needs"),
            executable = True,
            cfg = "exec",
        ),
    },
)
