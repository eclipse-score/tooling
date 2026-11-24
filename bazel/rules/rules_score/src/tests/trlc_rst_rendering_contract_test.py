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
"""Contract test: Markdown image syntax in a requirement description is
rendered to a ``.. image::`` RST directive by the pinned ``trlc_rst``.

The rules_score requirement pages rely on this rendering, so a trlc bump that
changes it fails here.
"""

import os
import shutil
import tempfile
import unittest

from python.runfiles import runfiles
from trlc_rst import TRLCRST


def _runfile(env_var: str) -> str:
    """Resolve the runfile whose rlocation path Bazel passes in ``env_var``."""
    rf = runfiles.Create()
    assert rf is not None
    path = rf.Rlocation(os.environ[env_var])
    if path is None or not os.path.exists(path):
        raise FileNotFoundError(f"Runfile for {env_var} not found: {path}")
    return path


class TestImageRenderingIntegration(unittest.TestCase):
    """Verify that Markdown ``![alt](path)`` in a description is rendered to
    a ``.. image::`` RST directive with a ``:alt:`` option."""

    def setUp(self):
        self._tmpdir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self._tmpdir)
        self._out = os.path.join(self._tmpdir, "output.rst")
        self._fixture_trlc = _runfile("IMAGE_REQUIREMENTS_TRLC")
        self._fixture_rsl = _runfile("IMAGE_REQUIREMENTS_RSL")

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
