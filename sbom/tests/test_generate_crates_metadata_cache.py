"""Tests for generate_crates_metadata_cache.py.

These tests verify the core parsing and data transformation functions
used to extract Rust crate license metadata via dash-license-scan.
"""

import json
import os
import tempfile
import unittest

# The script lives under sbom/scripts/ and is not a regular Python package.
# Import functions by adding the scripts directory to sys.path.
import sys

sys.path.insert(
    0,
    os.path.join(os.path.dirname(__file__), "..", "scripts"),
)

from generate_crates_metadata_cache import (
    build_dash_coordinates,
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


class TestBuildDashCoordinates(unittest.TestCase):
    """Tests for build_dash_coordinates — coordinate string construction."""

    def test_basic_coordinate_building(self):
        """Crate data produces correct coordinate strings."""
        crates = {
            "serde": {"name": "serde", "version": "1.0.228", "checksum": "abc123"},
            "tokio": {"name": "tokio", "version": "1.10.0", "checksum": "def456"},
        }
        coords = build_dash_coordinates(crates)

        self.assertEqual(len(coords), 2)
        self.assertIn("crate/cratesio/-/serde/1.0.228", coords)
        self.assertIn("crate/cratesio/-/tokio/1.10.0", coords)

    def test_empty_crates(self):
        """Empty crates dict produces empty coordinates list."""
        coords = build_dash_coordinates({})
        self.assertEqual(coords, [])

    def test_coordinates_are_sorted(self):
        """Coordinates are sorted by crate name."""
        crates = {
            "z-crate": {"name": "z-crate", "version": "1.0.0", "checksum": ""},
            "a-crate": {"name": "a-crate", "version": "2.0.0", "checksum": ""},
        }
        coords = build_dash_coordinates(crates)

        self.assertEqual(coords[0], "crate/cratesio/-/a-crate/2.0.0")
        self.assertEqual(coords[1], "crate/cratesio/-/z-crate/1.0.0")

    def test_hyphenated_crate_name(self):
        """Crate names with hyphens are preserved in coordinates."""
        crates = {
            "iceoryx2-bb-lock-free": {
                "name": "iceoryx2-bb-lock-free",
                "version": "0.7.0",
                "checksum": "",
            },
        }
        coords = build_dash_coordinates(crates)

        self.assertEqual(coords[0], "crate/cratesio/-/iceoryx2-bb-lock-free/0.7.0")


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
    handles the output format of the Eclipse dash-licenses JAR, which
    is the format that build_dash_coordinates + JAR invocation produces.
    """

    def _write_summary(self, content: str) -> str:
        fd, path = tempfile.mkstemp(suffix=".txt")
        with os.fdopen(fd, "w") as f:
            f.write(content)
        self.addCleanup(os.unlink, path)
        return path

    def test_coordinates_match_summary_format(self):
        """Coordinates built by build_dash_coordinates match the format
        that parse_dash_summary expects in the JAR output."""
        crates = {
            "serde": {"name": "serde", "version": "1.0.228", "checksum": "abc"},
            "tokio": {"name": "tokio", "version": "1.10.0", "checksum": "def"},
        }

        # Build coordinates (what we send to the JAR)
        coords = build_dash_coordinates(crates)
        self.assertEqual(coords[0], "crate/cratesio/-/serde/1.0.228")
        self.assertEqual(coords[1], "crate/cratesio/-/tokio/1.10.0")

        # Simulate JAR summary output (what the JAR would produce)
        summary = (
            "crate/cratesio/-/serde/1.0.228, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/tokio/1.10.0, MIT, approved, clearlydefined\n"
        )
        path = self._write_summary(summary)
        license_map = parse_dash_summary(path)

        # Verify licenses are correctly mapped back to crate names
        self.assertEqual(license_map["serde"], "Apache-2.0 OR MIT")
        self.assertEqual(license_map["tokio"], "MIT")

        # Verify all crates got licenses
        for name in crates:
            self.assertIn(name, license_map, f"Missing license for crate: {name}")

    def test_kyron_style_crates(self):
        """Verify license extraction works for crates typical in the score_kyron module."""
        crates = {
            "proc-macro2": {"name": "proc-macro2", "version": "1.0.92", "checksum": ""},
            "quote": {"name": "quote", "version": "1.0.37", "checksum": ""},
            "syn": {"name": "syn", "version": "2.0.96", "checksum": ""},
            "iceoryx2": {"name": "iceoryx2", "version": "0.7.0", "checksum": ""},
        }

        coords = build_dash_coordinates(crates)
        self.assertEqual(len(coords), 4)

        # Simulate JAR output
        summary = (
            "crate/cratesio/-/proc-macro2/1.0.92, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/quote/1.0.37, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/syn/2.0.96, Apache-2.0 OR MIT, approved, clearlydefined\n"
            "crate/cratesio/-/iceoryx2/0.7.0, Apache-2.0 OR MIT, approved, clearlydefined\n"
        )
        path = self._write_summary(summary)
        license_map = parse_dash_summary(path)

        # All crates should have licenses
        for name in crates:
            self.assertIn(name, license_map, f"Missing license for {name}")
            self.assertTrue(license_map[name], f"Empty license for {name}")
