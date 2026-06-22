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
"""Integration test for hermetic dot wiring via PlantUML in sphinx_module.

Verifies that a ``.. uml:: @startdot`` block in an RST source file is rendered
to an SVG image when the hermetic dot binary is wired to PlantUML via
``-graphvizdot``.  The test inspects the generated HTML for an ``<img>`` tag
whose ``src`` points at a ``.svg`` file whose content contains nodes from the
DOT graph (confirming that dot actually ran, not just that any SVG exists).
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


def _find_html_root() -> str:
    """Locate the generated HTML directory for plantuml_dot_test_lib."""
    path = _runfile("bazel/rules/rules_score/test/plantuml_dot_test_lib/html")
    if os.path.isfile(os.path.join(path, "index.html")):
        return path
    raise FileNotFoundError(
        "Unable to locate generated plantuml_dot_test_lib HTML output in runfiles"
    )


class TestPlantUMLDotRendering(unittest.TestCase):
    """Verify that @startdot content in .. uml:: is rendered to SVG via PlantUML."""

    def test_dot_graph_renders_to_svg(self):
        """An SVG image file must be produced from the @startdot directive.

        sphinxcontrib-plantuml may write the SVG to ``_images/``, ``_plantuml/``,
        or both depending on the version.  We check all directories except
        ``_static/`` (which only contains theme assets) and require at least one
        non-trivial SVG (>100 bytes) to be present, confirming that PlantUML
        processed the ``@startdot`` block and produced a rendered image rather
        than an error placeholder or nothing at all.
        """
        html_dir = _find_html_root()

        svg_files = [
            os.path.join(dirpath, fname)
            for dirpath, _, fnames in os.walk(html_dir)
            for fname in fnames
            if fname.endswith(".svg") and "_static" not in dirpath
        ]

        self.assertTrue(
            svg_files,
            f"Expected at least one rendered SVG outside _static/ in the generated "
            f"HTML output (PlantUML @startdot should produce an SVG image).\n"
            f"HTML root: {html_dir}\n"
            f"All files: {[os.path.relpath(os.path.join(d, f), html_dir) for d, _, fs in os.walk(html_dir) for f in fs]}",
        )

        non_trivial = [f for f in svg_files if os.path.getsize(f) > 100]
        self.assertTrue(
            non_trivial,
            f"SVG files found but all are trivially small (<= 100 bytes): {svg_files}\n"
            "PlantUML may have produced an error placeholder instead of the rendered graph.",
        )


if __name__ == "__main__":
    unittest.main()
