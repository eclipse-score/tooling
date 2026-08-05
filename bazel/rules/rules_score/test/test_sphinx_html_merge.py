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
"""Tests for sphinx_html_merge."""

import tempfile
import unittest
from pathlib import Path

from sphinx_html_merge import merge_html_dirs


def _write(path: Path, content: str = "") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


class TestMergeHtmlDirs(unittest.TestCase):
    """Tests for merge_html_dirs / copy_html_files."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.output = self.root / "output"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_nested_dep_page_rewrites_sibling_link_with_full_depth(self) -> None:
        """Regression test: a dependency's link to a sibling module from a
        page nested two directories deep must climb back out to the merged
        site root before descending into the sibling, not just one level.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")

        dep_a = self.root / "dep_a"
        _write(
            dep_a / "sub" / "deep" / "page.html",
            '<a href="dep_b/index.html">link</a>',
        )

        dep_b = self.root / "dep_b"
        _write(dep_b / "index.html", "<html></html>")

        merge_html_dirs(
            self.output,
            main,
            [("dep_a", dep_a), ("dep_b", dep_b)],
        )

        content = (self.output / "dep_a" / "sub" / "deep" / "page.html").read_text()
        self.assertIn('href="../../../dep_b/index.html"', content)

    def test_static_and_module_rewrites_agree_on_depth(self) -> None:
        """The _static rewrite and the sibling-module rewrite must use the
        same depth computation; a page linking to both should get the same
        number of '../' for each.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")

        dep_a = self.root / "dep_a"
        _write(
            dep_a / "sub" / "page.html",
            '<link href="_static/theme.css"><a href="dep_b/index.html">link</a>',
        )

        dep_b = self.root / "dep_b"
        _write(dep_b / "index.html", "<html></html>")

        merge_html_dirs(
            self.output,
            main,
            [("dep_a", dep_a), ("dep_b", dep_b)],
        )

        content = (self.output / "dep_a" / "sub" / "page.html").read_text()
        self.assertIn('href="../../_static/theme.css"', content)
        self.assertIn('href="../../dep_b/index.html"', content)

    def test_single_dependency_still_drops_own_static(self) -> None:
        """Regression test: with exactly one dependency there are no sibling
        modules to rewrite links for, but the dependency's own _static/ must
        still be dropped in favor of the merged site's shared _static/ — this
        must not depend on whether the sibling set happens to be non-empty.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")
        _write(main / "_static" / "theme.css", "/* main theme */")

        dep_a = self.root / "dep_a"
        _write(dep_a / "index.html", "<html></html>")
        _write(dep_a / "_static" / "theme.css", "/* dep theme, must be dropped */")

        merge_html_dirs(self.output, main, [("dep_a", dep_a)])

        self.assertFalse((self.output / "dep_a" / "_static").exists())
        self.assertTrue((self.output / "_static" / "theme.css").exists())

    def test_doctrees_are_never_published(self) -> None:
        """Regression test: Sphinx's build cache (.doctrees) must never be
        copied into the merged, published site — for the main module or for
        any dependency.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")
        _write(main / ".doctrees" / "index.doctree", "pickled-state")

        dep_a = self.root / "dep_a"
        _write(dep_a / "index.html", "<html></html>")
        _write(dep_a / ".doctrees" / "index.doctree", "pickled-state")

        merge_html_dirs(self.output, main, [("dep_a", dep_a)])

        self.assertFalse((self.output / ".doctrees").exists())
        self.assertFalse((self.output / "dep_a" / ".doctrees").exists())

    def test_sources_dir_is_still_published(self) -> None:
        """_sources/ backs Sphinx's "view page source" links and is real HTML
        output, unlike .doctrees — it must survive the merge.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")
        _write(main / "_sources" / "index.rst.txt", "Index\n=====\n")

        merge_html_dirs(self.output, main, [])

        self.assertTrue((self.output / "_sources" / "index.rst.txt").exists())

    def test_extra_static_copied_after_main(self) -> None:
        """extra_static entries must land under output/_static/ and are
        copied after the main module, so they can override theme defaults.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")
        _write(main / "_static" / "logo.svg", "main-logo")

        extra = self.root / "custom_logo.svg"
        _write(extra, "custom-logo")

        merge_html_dirs(
            self.output,
            main,
            [],
            extra_static=[(str(extra), "logo.svg")],
        )

        self.assertEqual((self.output / "_static" / "logo.svg").read_text(), "custom-logo")

    def test_deps_are_always_a_flat_transitive_set_no_nested_skip_needed(self) -> None:
        """sphinx_module.bzl always passes --dep as the transitive closure
        (SphinxModuleInfo.transitive_modules), each pointing at that module's
        own_html_dir — a module's own, unmerged Sphinx output, never another
        module's already-merged tree. There is therefore nothing nested to
        skip: this merge script has no special-casing for a dep's
        subdirectory happening to share a name with another dep, and copies
        it verbatim. Guards against reintroducing the old nested-sibling-skip
        heuristic, which existed only because deps used to be each other's
        *merged* trees (recursively containing further-nested deps) before
        the merge was flattened.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")

        dep_a = self.root / "dep_a"
        _write(dep_a / "index.html", "<html></html>")
        # Coincidentally named the same as another --dep entry below. Under
        # the old nested-merge design this could only happen via actual
        # nesting and had to be skipped; under the flat design it's just a
        # same-named subdirectory in dep_a's own tree, copied like any other.
        _write(dep_a / "dep_b" / "page.html", "<html>dep_a's own dep_b/ subdirectory</html>")

        dep_b = self.root / "dep_b"
        _write(dep_b / "index.html", "<html>canonical dep_b</html>")

        merge_html_dirs(
            self.output,
            main,
            [("dep_a", dep_a), ("dep_b", dep_b)],
        )

        self.assertEqual(
            (self.output / "dep_a" / "dep_b" / "page.html").read_text(),
            "<html>dep_a's own dep_b/ subdirectory</html>",
        )
        self.assertEqual(
            (self.output / "dep_b" / "index.html").read_text(),
            "<html>canonical dep_b</html>",
        )

    def test_main_page_links_to_dependency_are_not_rewritten(self) -> None:
        """Documents a current limitation: only dependency HTML
        (is_dependency=True) gets its links rewritten. A main-module page
        that links directly to a dependency keeps an unqualified href
        regardless of how deep the main page is nested, so such links must
        already be authored relative to the merged site root (e.g. via
        sphinx-needs external_needs base_url) rather than as a plain relative
        path - the merge step will not fix them up.
        """
        main = self.root / "main"
        _write(
            main / "guide" / "page.html",
            '<a href="dep_a/index.html">link</a>',
        )

        dep_a = self.root / "dep_a"
        _write(dep_a / "index.html", "<html></html>")

        merge_html_dirs(self.output, main, [("dep_a", dep_a)])

        content = (self.output / "guide" / "page.html").read_text()
        self.assertIn('href="dep_a/index.html">link', content)

    def test_literal_href_text_in_code_block_is_not_rewritten(self) -> None:
        """Regression test: a <pre>/<code> block showing example markup that
        happens to contain the literal text `href="dep_b/..."` (e.g. a code
        sample explaining how to write a link) must not be mistaken for a
        real navigational attribute and rewritten - only genuine anchors
        outside the protected block are link-rewriting targets.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")

        dep_a = self.root / "dep_a"
        _write(
            dep_a / "sub" / "page.html",
            '<pre>&lt;a href="dep_b/index.html"&gt;example&lt;/a&gt;</pre><a href="dep_b/index.html">real link</a>',
        )

        dep_b = self.root / "dep_b"
        _write(dep_b / "index.html", "<html></html>")

        merge_html_dirs(
            self.output,
            main,
            [("dep_a", dep_a), ("dep_b", dep_b)],
        )

        content = (self.output / "dep_a" / "sub" / "page.html").read_text()
        # The real anchor outside <pre> is rewritten for the new nesting depth.
        self.assertIn('<a href="../../dep_b/index.html">real link</a>', content)
        # The escaped example text inside <pre> is left exactly as authored.
        self.assertIn(
            '<pre>&lt;a href="dep_b/index.html"&gt;example&lt;/a&gt;</pre>',
            content,
        )

    def test_href_text_in_script_block_is_not_rewritten(self) -> None:
        """Regression test: a JS string literal inside <script> that contains
        `href="..."` (e.g. code building a link programmatically) must not be
        rewritten by the HTML link-fixing regexes - <script> bodies are
        opaque to them.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")

        dep_a = self.root / "dep_a"
        _write(
            dep_a / "sub" / "page.html",
            '<script>var link = \'<a href="dep_b/x.html">\';</script><a href="dep_b/index.html">real link</a>',
        )

        dep_b = self.root / "dep_b"
        _write(dep_b / "index.html", "<html></html>")

        merge_html_dirs(
            self.output,
            main,
            [("dep_a", dep_a), ("dep_b", dep_b)],
        )

        content = (self.output / "dep_a" / "sub" / "page.html").read_text()
        self.assertIn('<a href="../../dep_b/index.html">real link</a>', content)
        self.assertIn("var link = '<a href=\"dep_b/x.html\">';", content)

    def test_script_src_attribute_is_still_rewritten(self) -> None:
        """Regression test: a real `<script src="_static/...">` tag - e.g.
        Sphinx's own documentation_options.js / theme.js includes - has its
        src attribute in the OPENING TAG, not the body. Protecting the whole
        <script>...</script> element (including the opening tag) would hide
        this attribute from the rewriting regexes and silently break every
        JS asset link on dependency pages. Only the body must be protected.
        """
        main = self.root / "main"
        _write(main / "index.html", "<html></html>")

        dep_a = self.root / "dep_a"
        _write(
            dep_a / "sub" / "page.html",
            '<script src="../_static/documentation_options.js"></script>'
            "<script>var link = '<a href=\"dep_b/x.html\">';</script>",
        )

        merge_html_dirs(self.output, main, [("dep_a", dep_a)])

        content = (self.output / "dep_a" / "sub" / "page.html").read_text()
        # The real src attribute on the opening tag is rewritten for the new depth.
        self.assertIn('<script src="../../_static/documentation_options.js"></script>', content)
        # The unrelated script body's example text is still left untouched.
        self.assertIn("var link = '<a href=\"dep_b/x.html\">';", content)


if __name__ == "__main__":
    unittest.main()
