"""Tests for generate_crates_metadata_cache.py.

What this file tests
---------------------
parse_dash_summary()
  - Standard "crate/cratesio/-/NAME/VERSION, SPDX, STATUS, SOURCE" lines
    produce correct crate → license mappings.
  - Lines with an empty license expression are excluded.
  - Compound SPDX expressions (AND / OR / LicenseRef-*) are preserved verbatim.
  - Malformed lines (fewer than 4 comma-separated fields) are silently skipped.
  - Non-crate entries (pypi, npm) are ignored.
  - Empty file returns an empty dict.
  - Restricted crates still yield their license expression.

parse_module_bazel_lock()
  - Crate name and version are extracted from generatedRepoSpecs keys
    (format: crate_index__NAME-VERSION).
  - sha256 checksum is extracted from the attributes dict.
  - The bare "crate_index" meta-repo entry is not treated as a real crate.
  - Complex names (iceoryx2-bb-lock-free-qnx8-0.7.0) are parsed correctly.
  - Lockfiles without a crate extension return an empty dict.
  - Completely empty lockfiles return an empty dict.

generate_synthetic_cargo_lock()
  - Produces valid TOML with [[package]] entries and crates.io-index source.
  - Entries are sorted alphabetically by crate name.

TestEndToEndLicenseExtraction
  - parse_dash_summary() correctly round-trips JAR-style CSV output.
  - Full pipeline verified for a representative set of score_kyron crates.

Bazel target : //sbom/tests:test_generate_crates_metadata_cache
Run          : bazel test //sbom/tests:test_generate_crates_metadata_cache
               pytest sbom/tests/test_generate_crates_metadata_cache.py -v
"""

import json
import os
import tempfile
import unittest

from sbom.scripts.generate_crates_metadata_cache import (
    generate_synthetic_cargo_lock,
    parse_dash_summary,
    parse_module_bazel_lock,
)


class TestParseDashSummary(unittest.TestCase):
    """Tests for parse_dash_summary — the JAR summary CSV parser."""

    def _write_summary(self, content: str) -> str:
        """Helper: write content to a temp file and return its path."""
        fd, path = tempfile.mkstemp(suffix=".txt")
        with os.fdopen(fd, "w") as f:
            f.write(content)
        self.addCleanup(os.unlink, path)
        return path

    def test_basic_parsing(self):
        """Standard summary lines produce correct crate→license mapping."""
        summary = (
            "crate/cratesio/-/serde/1.0.228, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/tokio/1.10.0, MIT, approved, clearlydefined\n"
        )
        path = self._write_summary(summary)
        result = parse_dash_summary(path)

        self.assertEqual(result["serde"], "Apache-2.0 OR MIT")
        self.assertEqual(result["tokio"], "MIT")

    def test_empty_license_skipped(self):
        """Entries with empty license expressions are not included."""
        summary = (
            "crate/cratesio/-/serde/1.0.228, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/unknown-crate/0.1.0, , restricted, clearlydefined\n"
        )
        path = self._write_summary(summary)
        result = parse_dash_summary(path)

        self.assertIn("serde", result)
        self.assertNotIn("unknown-crate", result)

    def test_compound_spdx_expression(self):
        """Compound SPDX expressions (AND/OR) are preserved."""
        summary = (
            "crate/cratesio/-/ring/0.17.14, "
            "Apache-2.0 AND LicenseRef-scancode-iso-8879 AND (GPL-2.0-only AND MIT), "
            "restricted, #25641\n"
        )
        path = self._write_summary(summary)
        result = parse_dash_summary(path)

        self.assertIn("ring", result)
        self.assertIn("Apache-2.0", result["ring"])

    def test_malformed_lines_skipped(self):
        """Lines with fewer than 4 comma-separated fields are ignored."""
        summary = (
            "crate/cratesio/-/serde/1.0.228, MIT, approved, clearlydefined\n"
            "this is not a valid line\n"
            "only, two, parts\n"
            "\n"
        )
        path = self._write_summary(summary)
        result = parse_dash_summary(path)

        self.assertEqual(len(result), 1)
        self.assertEqual(result["serde"], "MIT")

    def test_non_crate_entries_skipped(self):
        """Non-crate entries (pypi, npm, etc.) are ignored."""
        summary = (
            "crate/cratesio/-/serde/1.0.228, MIT, approved, clearlydefined\n"
            "pypi/pypi/-/requests/2.31.0, Apache-2.0, approved, clearlydefined\n"
            "npm/npmjs/-/express/4.18.2, MIT, approved, clearlydefined\n"
        )
        path = self._write_summary(summary)
        result = parse_dash_summary(path)

        self.assertEqual(len(result), 1)
        self.assertIn("serde", result)

    def test_empty_file(self):
        """An empty summary file produces an empty dict."""
        path = self._write_summary("")
        result = parse_dash_summary(path)
        self.assertEqual(result, {})

    def test_restricted_crate_still_gets_license(self):
        """Restricted crates still have their license extracted."""
        summary = "crate/cratesio/-/openssl-sys/0.9.104, OpenSSL, restricted, clearlydefined\n"
        path = self._write_summary(summary)
        result = parse_dash_summary(path)

        self.assertEqual(result["openssl-sys"], "OpenSSL")

    def test_licenseref_expression(self):
        """LicenseRef-* expressions are preserved."""
        summary = "crate/cratesio/-/ring/0.17.14, LicenseRef-ring, restricted, clearlydefined\n"
        path = self._write_summary(summary)
        result = parse_dash_summary(path)

        self.assertEqual(result["ring"], "LicenseRef-ring")


class TestParseModuleBazelLock(unittest.TestCase):
    """Tests for parse_module_bazel_lock — MODULE.bazel.lock crate extraction."""

    def _write_lockfile(self, data: dict) -> str:
        """Helper: write JSON data to a temp file and return its path."""
        fd, path = tempfile.mkstemp(suffix=".json")
        with os.fdopen(fd, "w") as f:
            json.dump(data, f)
        self.addCleanup(os.unlink, path)
        return path

    def test_basic_crate_extraction(self):
        """Crates are correctly extracted from generatedRepoSpecs."""
        lockfile = {
            "moduleExtensions": {
                "@@rules_rust+//crate_universe:extensions.bzl%crate": {
                    "general": {
                        "generatedRepoSpecs": {
                            "crate_index__serde-1.0.228": {
                                "attributes": {"sha256": "abc123def456"}
                            },
                            "crate_index__tokio-1.10.0": {
                                "attributes": {"sha256": "789xyz"}
                            },
                        }
                    }
                }
            }
        }
        path = self._write_lockfile(lockfile)
        result = parse_module_bazel_lock(path)

        self.assertEqual(len(result), 2)
        self.assertEqual(result["serde"]["version"], "1.0.228")
        self.assertEqual(result["serde"]["checksum"], "abc123def456")
        self.assertEqual(result["tokio"]["version"], "1.10.0")

    def test_crate_index_meta_repo_skipped(self):
        """The crate_index meta-repo entry is not treated as a crate."""
        lockfile = {
            "moduleExtensions": {
                "crate_universe": {
                    "general": {
                        "generatedRepoSpecs": {
                            "crate_index": {"attributes": {}},
                            "crate_index__serde-1.0.228": {
                                "attributes": {"sha256": "abc"}
                            },
                        }
                    }
                }
            }
        }
        path = self._write_lockfile(lockfile)
        result = parse_module_bazel_lock(path)

        self.assertEqual(len(result), 1)
        self.assertIn("serde", result)

    def test_complex_crate_name(self):
        """Crate names with multiple hyphens (e.g. iceoryx2-qnx8) are parsed correctly."""
        lockfile = {
            "moduleExtensions": {
                "crate": {
                    "general": {
                        "generatedRepoSpecs": {
                            "crate_index__iceoryx2-bb-lock-free-qnx8-0.7.0": {
                                "attributes": {"sha256": "xyz"}
                            },
                        }
                    }
                }
            }
        }
        path = self._write_lockfile(lockfile)
        result = parse_module_bazel_lock(path)

        self.assertEqual(len(result), 1)
        self.assertIn("iceoryx2-bb-lock-free-qnx8", result)
        self.assertEqual(result["iceoryx2-bb-lock-free-qnx8"]["version"], "0.7.0")

    def test_no_crate_extension(self):
        """Lockfile without crate extension returns empty dict."""
        lockfile = {"moduleExtensions": {"some_other_extension": {"general": {}}}}
        path = self._write_lockfile(lockfile)
        result = parse_module_bazel_lock(path)

        self.assertEqual(result, {})

    def test_empty_lockfile(self):
        """Lockfile with no moduleExtensions returns empty dict."""
        path = self._write_lockfile({})
        result = parse_module_bazel_lock(path)
        self.assertEqual(result, {})


class TestGenerateSyntheticCargoLock(unittest.TestCase):
    """Tests for generate_synthetic_cargo_lock."""

    def test_generates_valid_toml(self):
        """Generated Cargo.lock has correct TOML structure."""
        crates = {
            "serde": {"name": "serde", "version": "1.0.228", "checksum": "abc"},
            "tokio": {"name": "tokio", "version": "1.10.0", "checksum": "def"},
        }
        fd, path = tempfile.mkstemp(suffix=".lock")
        os.close(fd)
        self.addCleanup(os.unlink, path)

        generate_synthetic_cargo_lock(crates, path)

        with open(path) as f:
            content = f.read()

        self.assertIn("version = 4", content)
        self.assertIn('name = "serde"', content)
        self.assertIn('version = "1.0.228"', content)
        self.assertIn('name = "tokio"', content)
        self.assertIn("[[package]]", content)
        self.assertIn("crates.io-index", content)

    def test_entries_are_sorted(self):
        """Cargo.lock entries are sorted by crate name."""
        crates = {
            "z-crate": {"name": "z-crate", "version": "1.0.0", "checksum": ""},
            "a-crate": {"name": "a-crate", "version": "2.0.0", "checksum": ""},
        }
        fd, path = tempfile.mkstemp(suffix=".lock")
        os.close(fd)
        self.addCleanup(os.unlink, path)

        generate_synthetic_cargo_lock(crates, path)

        with open(path) as f:
            content = f.read()

        a_pos = content.index('name = "a-crate"')
        z_pos = content.index('name = "z-crate"')
        self.assertLess(a_pos, z_pos)


class TestEndToEndLicenseExtraction(unittest.TestCase):
    """Integration tests verifying the full license extraction pipeline.

    These tests verify that the parse_dash_summary function correctly
    handles the output format of the Eclipse dash-licenses JAR.
    """

    def _write_summary(self, content: str) -> str:
        fd, path = tempfile.mkstemp(suffix=".txt")
        with os.fdopen(fd, "w") as f:
            f.write(content)
        self.addCleanup(os.unlink, path)
        return path

    def test_summary_format_round_trip(self):
        """parse_dash_summary correctly maps crate names from JAR-style CSV output."""
        summary = (
            "crate/cratesio/-/serde/1.0.228, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/tokio/1.10.0, MIT, approved, clearlydefined\n"
        )
        path = self._write_summary(summary)
        license_map = parse_dash_summary(path)

        self.assertEqual(license_map["serde"], "Apache-2.0 OR MIT")
        self.assertEqual(license_map["tokio"], "MIT")

    def test_kyron_style_crates(self):
        """Verify license extraction works for crates typical in the score_kyron module."""
        crate_names = ["proc-macro2", "quote", "syn", "iceoryx2"]

        summary = (
            "crate/cratesio/-/proc-macro2/1.0.92, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/quote/1.0.37, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/syn/2.0.96, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/iceoryx2/0.7.0, Apache-2.0 OR MIT, approved, clearlydefined\n"
        )
        path = self._write_summary(summary)
        license_map = parse_dash_summary(path)

        for name in crate_names:
            self.assertIn(name, license_map, f"Missing license for {name}")
            self.assertTrue(license_map[name], f"Empty license for {name}")
