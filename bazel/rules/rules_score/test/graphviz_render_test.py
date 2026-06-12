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
"""Integration test for hermetic graphviz rendering in sphinx_module.

Verifies that the sphinx.ext.graphviz extension successfully renders diagrams
to SVG when using hermetic graphviz bundled via download_deb.
"""

import os
import unittest


def _runfile(*parts: str) -> str:
    """Locate a file in the Bazel runfiles tree."""
    srcdir = os.environ["TEST_SRCDIR"]
    workspace = os.environ.get("TEST_WORKSPACE", "_main")
    for candidate in [
        os.path.join(srcdir, workspace, *parts),
        os.path.join(srcdir, *parts),
    ]:
        if os.path.exists(candidate):
            return candidate
    raise FileNotFoundError(
        f"Runfile not found: {os.path.join(*parts)}\n"
        f"  Searched under TEST_SRCDIR={srcdir}"
    )


def _find_generated_html_root() -> str:
    """Locate the generated HTML directory for graphviz_test_lib in runfiles.

    Depending on Bazel/runfiles layout, the directory artifact may appear either
    at ``.../graphviz_test_lib/html`` or at ``.../graphviz_test_lib``.
    """
    for parts in [
        ("bazel/rules/rules_score/test/graphviz_test_lib/html",),
        ("bazel/rules/rules_score/test/graphviz_test_lib",),
    ]:
        try:
            root = _runfile(*parts)
        except FileNotFoundError:
            continue
        if os.path.exists(os.path.join(root, "index.html")):
            return root

    raise FileNotFoundError(
        "Unable to locate generated graphviz_test_lib HTML output in runfiles"
    )


class TestGraphvizRendering(unittest.TestCase):
    """Verify that sphinx.ext.graphviz renders an SVG artifact."""

    def test_graphviz_renders_svg(self):
        """Test that graphviz output is rendered as SVG in generated HTML."""
        html_dir = _find_generated_html_root()

        svg_files = [
            file_name
            for _, _, files in os.walk(html_dir)
            for file_name in files
            if file_name.endswith(".svg")
        ]

        self.assertTrue(
            svg_files,
            "Generated HTML output should include at least one rendered graphviz SVG artifact",
        )


if __name__ == "__main__":
    unittest.main()
