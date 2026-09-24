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
"""Shared Starlark helper wiring the ``dedupe_aou_trlc`` tool into a rule action.

An AoU's TRLC identity (package + record name) is deliberately preserved
verbatim by ``filter_forwarded_trlc.py`` when it retypes a chain-forwarded
AoU to ``ScoreReq.ReceivedAoU`` -- this is what lets a ``derived_from``
reference stay valid no matter how many hops of forwarding it has been
through. The unavoidable consequence is that the *same* AoU identity can
legitimately appear in more than one ``.trlc`` file that end up merged into a
single TRLC parse/check -- most commonly in a diamond dependency shape (a
target depends both directly on an AoU's original owner and, transitively,
on an intermediate element that chain-forwards that same AoU). TRLC's own
duplicate-definition check keys on ``(package, name)`` alone, not on
declared type, and rejects this outright.

``dedupe_aou_trlc_files`` runs the ``dedupe_aou_trlc`` tool (see
``src/dedupe_aou_trlc.py``) over a list of files whenever there is more than
one, so any such duplicate AoU/ReceivedAoU identity is collapsed down to a
single declaration before those files are merged for a TRLC
parse/render/check. Used by both ``dependable_element.bzl`` (deduplicating
what it received from its own ``deps`` before chain-forwarding) and
``requirements.bzl`` (deduplicating what a `feature_requirements`/
`component_requirements`/`assumed_system_requirements`/`assumptions_of_use`
target's own ``deps`` expose, which is where the diamond shape most commonly
surfaces for a *downstream* consumer).
"""

def dedupe_aou_trlc_files(ctx, tool, files, output_subdir):
    """Deduplicate AoU/ReceivedAoU TRLC records across a list of files.

    Args:
        ctx: Rule context (used for ``ctx.actions`` and ``ctx.label``).
        tool: ``executable`` File for the ``dedupe_aou_trlc`` tool (an
            attribute resolved via ``ctx.executable.<attr_name>``).
        files: List of ``File`` to deduplicate. Returned unchanged (no
            action is run) if it has fewer than two entries -- a single
            file cannot contain a cross-file duplicate.
        output_subdir: Subdirectory name (relative to ``ctx.label.name``)
            to declare the deduplicated output files under. Callers using
            this helper more than once within the same rule implementation
            must pass a distinct value each time to avoid output path
            collisions.

    Returns:
        A list of ``File``, order-aligned with ``files``: either ``files``
        itself unchanged (fewer than two entries), or a matching list of
        freshly declared, deduplicated output files.
    """
    if len(files) < 2:
        return files

    outputs = [
        ctx.actions.declare_file("{}/{}/{}_{}".format(ctx.label.name, output_subdir, i, f.basename))
        for i, f in enumerate(files)
    ]

    args = ctx.actions.args()
    args.add_all("--inputs", files)
    args.add_all("--outputs", outputs)

    ctx.actions.run(
        inputs = files,
        outputs = outputs,
        executable = tool,
        arguments = [args],
        progress_message = "Deduplicating AoU TRLC records for %s" % ctx.label.name,
        mnemonic = "AoUTrlcDedupe",
    )

    return outputs
