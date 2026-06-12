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

"""Repository rule that downloads and extracts a graphviz cmake-release .deb.

The upstream cmake deb uses data.tar.gz compression, which download_utils does
not support (it only handles .xz and .zst).  This rule uses the standard
`ar` + `tar` toolchain that is present on every Debian/Ubuntu host.

Note: because extraction shells out to host `ar` and `tar`, this rule is not
fully hermetic and assumes a Debian/Ubuntu-style builder.  It will fail on
hosts where those tools are unavailable.
"""

def _graphviz_deb_impl(ctx):
    """Download and extract a graphviz .deb package into an external repository."""

    # Step 1: download the .deb archive.
    deb_path = ctx.path("graphviz.deb")
    ctx.download(
        url = ctx.attr.urls,
        integrity = ctx.attr.integrity,
        output = deb_path,
    )

    work_dir = str(ctx.path("."))

    # Step 2: unpack data.tar.gz from the ar archive.
    # A Debian .deb is an ar archive containing control.tar.* and data.tar.*.
    result = ctx.execute(
        ["ar", "x", str(deb_path), "data.tar.gz"],
        working_directory = work_dir,
    )
    if result.return_code != 0:
        fail("Failed to extract data.tar.gz from deb: {}".format(result.stderr))

    # Step 3: extract data.tar.gz contents into the repository root.
    result = ctx.execute(
        ["tar", "-xzf", "data.tar.gz"],
        working_directory = work_dir,
    )
    if result.return_code != 0:
        fail("Failed to extract data.tar.gz: {}".format(result.stderr))

    # Clean up only the files we explicitly created.
    ctx.execute(["rm", "-f", str(deb_path), "data.tar.gz"], working_directory = work_dir)

    # Step 4: inject the BUILD file that exposes graphviz targets.
    ctx.file("BUILD", ctx.read(ctx.attr.build))

graphviz_deb = repository_rule(
    doc = "Downloads and extracts a graphviz cmake-release .deb into an external repository.",
    implementation = _graphviz_deb_impl,
    attrs = {
        "urls": attr.string_list(
            mandatory = True,
            doc = "List of mirror URLs for the graphviz .deb package.",
        ),
        "integrity": attr.string(
            mandatory = True,
            doc = "Subresource Integrity (SRI) checksum of the .deb archive.",
        ),
        "build": attr.label(
            mandatory = True,
            doc = "Label of the BUILD file template to inject into the extracted repository.",
        ),
    },
)
