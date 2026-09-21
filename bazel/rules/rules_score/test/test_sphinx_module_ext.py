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
"""Unit tests for sphinx_module_ext's confdir-aware needs loading."""

import json
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from sphinx_module_ext import (
    init_external_needs,
    render_directory_labels_without_links,
)


class TestInitExternalNeeds(unittest.TestCase):
    """Tests for init_external_needs's "config-inited" listener."""

    def test_uses_app_confdir_not_cwd(self) -> None:
        """Regression test: init_external_needs is invoked by Sphinx after its
        own chdir(confdir) (scoped to evaluating conf.py) has already been
        undone, so it must resolve needs_external_needs.json via app.confdir
        rather than the process's current working directory."""
        with tempfile.TemporaryDirectory() as tmp:
            confdir = Path(tmp) / "confdir"
            confdir.mkdir()
            (confdir / "needs_external_needs.json").write_text(
                json.dumps({"dep": {"json_path": "x", "version": "1.0"}}),
                encoding="utf-8",
            )
            elsewhere = Path(tmp) / "elsewhere"
            elsewhere.mkdir()
            old_cwd = Path.cwd()
            os.chdir(elsewhere)
            try:
                app = SimpleNamespace(confdir=str(confdir))
                config = SimpleNamespace()

                init_external_needs(app, config)
            finally:
                os.chdir(old_cwd)

            self.assertEqual(len(config.needs_external_needs), 1)


class TestRenderDirectoryLabelsWithoutLinks(unittest.TestCase):
    """Tests for render_directory_labels_without_links's html-page-context
    listener, which turns sidebar links to generated directory-index pages
    into plain labels while leaving links to real content pages untouched."""

    @staticmethod
    def _fake_app(metadata: dict, uri_by_docname: dict) -> SimpleNamespace:
        return SimpleNamespace(
            env=SimpleNamespace(metadata=metadata),
            builder=SimpleNamespace(
                get_relative_uri=lambda _pagename, docname: uri_by_docname[docname],
            ),
        )

    def test_strips_link_only_for_marked_directory_index_pages(self) -> None:
        """Only docnames carrying the score-directory-index metadata marker
        should have their sidebar links replaced with plain <span> labels;
        an ordinary hand-authored page named index.rst (no marker) must keep
        its normal <a> link, proving the fix no longer matches on
        URL/filename shape alone."""
        app = self._fake_app(
            metadata={
                "static/index": {"score-directory-index": ""},
                "static/fixtures/index": {"score-directory-index": ""},
                "unrelated/index": {},
            },
            uri_by_docname={
                "static/index": "static/index.html",
                "static/fixtures/index": "static/fixtures/index.html",
                "unrelated/index": "unrelated/index.html",
            },
        )

        def fake_toctree(*_args: object, **_kwargs: object) -> str:
            return (
                '<a href="static/index.html">Static Design</a>'
                '<a href="static/fixtures/index.html">Fixtures</a>'
                '<a href="unrelated/index.html">Unrelated</a>'
            )

        context = {"toctree": fake_toctree}
        render_directory_labels_without_links(app, "root", "page.html", context, None)

        rendered = context["toctree"]()
        self.assertNotIn('href="static/index.html"', rendered)
        self.assertNotIn('href="static/fixtures/index.html"', rendered)
        self.assertIn("<span>Static Design</span>", rendered)
        self.assertIn("<span>Fixtures</span>", rendered)
        self.assertIn('<a href="unrelated/index.html">Unrelated</a>', rendered)

    def test_noop_when_no_directory_index_pages_marked(self) -> None:
        """With no marked docnames, the toctree renderer is left untouched
        (not even wrapped), so unrelated builds without generated directory
        indexes pay no cost and see no behavior change."""
        app = self._fake_app(metadata={}, uri_by_docname={})

        def fake_toctree(*_args: object, **_kwargs: object) -> str:
            return '<a href="page.html">Page</a>'

        context = {"toctree": fake_toctree}
        render_directory_labels_without_links(app, "root", "page.html", context, None)

        self.assertIs(context["toctree"], fake_toctree)

    def test_wraps_generate_toctree_html_for_pydata_theme(self) -> None:
        """pydata_sphinx_theme's sidebar renders via "generate_toctree_html"
        instead of "toctree" -- regression test proving that renderer is
        also wrapped, not just "toctree" (which only sphinx_rtd_theme uses)."""
        app = self._fake_app(
            metadata={"static/index": {"score-directory-index": ""}},
            uri_by_docname={"static/index": "static/index.html"},
        )

        def fake_generate_toctree_html(*_args: object, **_kwargs: object) -> str:
            return '<a href="static/index.html">Static Design</a>'

        context = {"generate_toctree_html": fake_generate_toctree_html}
        render_directory_labels_without_links(app, "root", "page.html", context, None)

        rendered = context["generate_toctree_html"]()
        self.assertNotIn('href="static/index.html"', rendered)
        self.assertIn("<span>Static Design</span>", rendered)


if __name__ == "__main__":
    unittest.main()
