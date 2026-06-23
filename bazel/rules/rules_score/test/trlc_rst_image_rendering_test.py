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
"""Integration test: Markdown image syntax in a requirement description is
rendered to a ``.. image::`` RST directive by trlc_rst.

Uses the TRLCRST Python API directly against the local trlc checkout so that
changes to ``trlc_rst.py`` are picked up immediately without a git push.
"""

import os
import tempfile
import unittest

from trlc_rst import TRLCRST


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


class TestImageRenderingIntegration(unittest.TestCase):
    """Verify that Markdown ``![alt](path)`` in a description is rendered to
    a ``.. image::`` RST directive with a ``:alt:`` option."""

    def setUp(self):
        self._tmpdir = tempfile.mkdtemp()
        self._out = os.path.join(self._tmpdir, "output.rst")
        self._fixture_trlc = _runfile(
            "fixtures",
            "image_requirements",
            "image_requirements.trlc",
        )
        self._fixture_rsl = _runfile(
            "fixtures",
            "image_requirements",
            "schema.rsl",
        )

    def _render(self) -> str:
        renderer = TRLCRST(
            source_files=[self._fixture_trlc],
            dep_files=[self._fixture_rsl],
        )
        renderer.render(self._out)
        with open(self._out, encoding="utf-8") as f:
            return f.read()

    def test_image_directive_emitted(self):
        """Both SVG and PNG image directives appear in the rendered RST."""
        rendered = self._render()
        self.assertIn(".. image:: diagrams/overview.svg", rendered)
        self.assertIn(".. image:: diagrams/overview.png", rendered)

    def test_alt_text_emitted(self):
        """The ``:alt:`` option is emitted with the Markdown alt text."""
        rendered = self._render()
        self.assertIn(":alt: System overview", rendered)
        self.assertIn(":alt: System overview PNG", rendered)

    def test_markdown_syntax_not_in_output(self):
        """The raw ``![...]()`` Markdown syntax must not appear in the output."""
        self.assertNotIn("![", self._render())

    def test_requirement_without_image_unaffected(self):
        """Requirements without an image render normally; two images total."""
        content = self._render()
        self.assertIn("The system shall operate without images.", content)
        self.assertEqual(content.count(".. image::"), 2)


if __name__ == "__main__":
    unittest.main()
