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
"""Tests for generate_cpp_metadata_cache.py — convert_cdxgen_to_cache().

What this file tests
---------------------
Basic extraction
  - name and version are extracted; version defaults to "unknown" when absent.
  - Components with no name are silently skipped.
  - Multiple components all appear in the cache.
  - Empty components list returns an empty dict.

License field extraction
  - SPDX ID from license.id.
  - Fallback to license.name when id is absent.
  - Top-level expression field for compound SPDX expressions.
  - license.id takes priority over license.name in the same entry.
  - Compound AND expressions are preserved verbatim.
  - Components with no license produce no "license" key in the cache entry.

Supplier extraction
  - From supplier.name.
  - Fallback to publisher field when supplier is absent.
  - Components with neither produce no "supplier" key.

PURL and URL
  - purl is copied directly from the component.
  - No purl field → no "purl" key in the cache entry.
  - URL extracted from externalReferences type = website, vcs, or distribution
    (first matching entry wins).
  - No externalReferences → no "url" key.

Description
  - description is extracted when present.
  - No description → no "description" key.

Bazel target : //sbom/tests:test_generate_cpp_metadata_cache
Run          : bazel test //sbom/tests:test_generate_cpp_metadata_cache
               pytest sbom/tests/test_generate_cpp_metadata_cache.py -v
"""

import json
import os
import tempfile
import unittest

from sbom.scripts.generate_cpp_metadata_cache import convert_cdxgen_to_cache


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_cdx_doc(components: list) -> dict:
    """Build a minimal valid CycloneDX document wrapping the given components."""
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "components": components,
    }


def _write_cdx(components: list) -> tuple[str, str]:
    """Write a CycloneDX document to a temp file; return (fd_path, cleanup_path)."""
    data = _make_cdx_doc(components)
    fd, path = tempfile.mkstemp(suffix=".cdx.json")
    with os.fdopen(fd, "w") as f:
        json.dump(data, f)
    return path


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestConvertCdxgenToCacheBasic(unittest.TestCase):
    """Basic field extraction from a cdxgen CycloneDX document."""

    def setUp(self):
        self._cleanup: list[str] = []

    def tearDown(self):
        for path in self._cleanup:
            try:
                os.unlink(path)
            except FileNotFoundError:
                pass

    def _convert(self, components: list) -> dict:
        path = _write_cdx(components)
        self._cleanup.append(path)
        return convert_cdxgen_to_cache(path)

    def test_basic_name_and_version(self):
        result = self._convert([{"name": "nlohmann-json", "version": "3.11.3"}])
        self.assertIn("nlohmann-json", result)
        self.assertEqual(result["nlohmann-json"]["version"], "3.11.3")

    def test_version_defaults_to_unknown(self):
        result = self._convert([{"name": "some-lib"}])
        self.assertEqual(result["some-lib"]["version"], "unknown")

    def test_multiple_components(self):
        result = self._convert(
            [
                {"name": "lib_a", "version": "1.0"},
                {"name": "lib_b", "version": "2.0"},
                {"name": "lib_c", "version": "3.0"},
            ]
        )
        self.assertEqual(len(result), 3)
        self.assertIn("lib_a", result)
        self.assertIn("lib_b", result)
        self.assertIn("lib_c", result)

    def test_entry_with_no_name_skipped(self):
        """Components without a name must not appear in the cache."""
        result = self._convert(
            [{"version": "1.0", "licenses": [{"license": {"id": "MIT"}}]}]
        )
        self.assertEqual(result, {})

    def test_empty_components_list(self):
        result = self._convert([])
        self.assertEqual(result, {})


class TestConvertCdxgenToCacheLicense(unittest.TestCase):
    """License field extraction — license.id, license.name, and expression."""

    def setUp(self):
        self._cleanup: list[str] = []

    def tearDown(self):
        for path in self._cleanup:
            try:
                os.unlink(path)
            except FileNotFoundError:
                pass

    def _convert(self, components: list) -> dict:
        path = _write_cdx(components)
        self._cleanup.append(path)
        return convert_cdxgen_to_cache(path)

    def test_license_from_license_id(self):
        result = self._convert(
            [
                {
                    "name": "zlib",
                    "version": "1.3.1",
                    "licenses": [{"license": {"id": "Zlib"}}],
                }
            ]
        )
        self.assertEqual(result["zlib"]["license"], "Zlib")

    def test_license_from_license_name_fallback(self):
        """When license.id is absent, license.name is used as the identifier."""
        result = self._convert(
            [
                {
                    "name": "curl",
                    "version": "8.0.0",
                    "licenses": [{"license": {"name": "curl/libcurl"}}],
                }
            ]
        )
        self.assertEqual(result["curl"]["license"], "curl/libcurl")

    def test_license_from_expression(self):
        result = self._convert(
            [
                {
                    "name": "openssl",
                    "version": "3.0.0",
                    "licenses": [{"expression": "Apache-2.0 OR OpenSSL"}],
                }
            ]
        )
        self.assertEqual(result["openssl"]["license"], "Apache-2.0 OR OpenSSL")

    def test_license_id_takes_priority_over_name(self):
        """license.id is checked before license.name."""
        result = self._convert(
            [
                {
                    "name": "mylib",
                    "version": "1.0",
                    "licenses": [{"license": {"id": "MIT", "name": "MIT License"}}],
                }
            ]
        )
        self.assertEqual(result["mylib"]["license"], "MIT")

    def test_no_license_field_absent_from_cache(self):
        result = self._convert([{"name": "no-license-lib", "version": "1.0"}])
        self.assertNotIn("license", result["no-license-lib"])

    def test_compound_spdx_expression(self):
        result = self._convert(
            [
                {
                    "name": "dual-licensed",
                    "version": "1.0",
                    "licenses": [{"expression": "Apache-2.0 AND MIT"}],
                }
            ]
        )
        self.assertEqual(result["dual-licensed"]["license"], "Apache-2.0 AND MIT")


class TestConvertCdxgenToCacheSupplier(unittest.TestCase):
    """Supplier extraction from supplier.name and publisher fallback."""

    def setUp(self):
        self._cleanup: list[str] = []

    def tearDown(self):
        for path in self._cleanup:
            try:
                os.unlink(path)
            except FileNotFoundError:
                pass

    def _convert(self, components: list) -> dict:
        path = _write_cdx(components)
        self._cleanup.append(path)
        return convert_cdxgen_to_cache(path)

    def test_supplier_from_supplier_name(self):
        result = self._convert(
            [
                {
                    "name": "abseil-cpp",
                    "version": "20230802.0",
                    "supplier": {"name": "Google LLC"},
                }
            ]
        )
        self.assertEqual(result["abseil-cpp"]["supplier"], "Google LLC")

    def test_supplier_from_publisher_fallback(self):
        """If supplier.name is absent, publisher field is used as the supplier."""
        result = self._convert(
            [
                {
                    "name": "flatbuffers",
                    "version": "25.2.10",
                    "publisher": "Google",
                }
            ]
        )
        self.assertEqual(result["flatbuffers"]["supplier"], "Google")

    def test_no_supplier_field_absent_from_cache(self):
        result = self._convert([{"name": "anon-lib", "version": "1.0"}])
        self.assertNotIn("supplier", result["anon-lib"])


class TestConvertCdxgenToCachePurlAndUrl(unittest.TestCase):
    """PURL and external URL extraction."""

    def setUp(self):
        self._cleanup: list[str] = []

    def tearDown(self):
        for path in self._cleanup:
            try:
                os.unlink(path)
            except FileNotFoundError:
                pass

    def _convert(self, components: list) -> dict:
        path = _write_cdx(components)
        self._cleanup.append(path)
        return convert_cdxgen_to_cache(path)

    def test_purl_extracted(self):
        result = self._convert(
            [
                {
                    "name": "nlohmann-json",
                    "version": "3.11.3",
                    "purl": "pkg:generic/nlohmann-json@3.11.3",
                }
            ]
        )
        self.assertEqual(
            result["nlohmann-json"]["purl"], "pkg:generic/nlohmann-json@3.11.3"
        )

    def test_no_purl_field_absent_from_cache(self):
        result = self._convert([{"name": "no-purl-lib", "version": "1.0"}])
        self.assertNotIn("purl", result["no-purl-lib"])

    def test_url_from_website_external_reference(self):
        result = self._convert(
            [
                {
                    "name": "zlib",
                    "version": "1.3.1",
                    "externalReferences": [
                        {"type": "website", "url": "https://zlib.net"},
                    ],
                }
            ]
        )
        self.assertEqual(result["zlib"]["url"], "https://zlib.net")

    def test_url_from_vcs_external_reference(self):
        result = self._convert(
            [
                {
                    "name": "my-lib",
                    "version": "1.0",
                    "externalReferences": [
                        {"type": "vcs", "url": "https://github.com/example/my-lib"},
                    ],
                }
            ]
        )
        self.assertEqual(result["my-lib"]["url"], "https://github.com/example/my-lib")

    def test_url_from_distribution_external_reference(self):
        result = self._convert(
            [
                {
                    "name": "dist-lib",
                    "version": "1.0",
                    "externalReferences": [
                        {
                            "type": "distribution",
                            "url": "https://releases.example.com/dist-lib",
                        },
                    ],
                }
            ]
        )
        self.assertEqual(
            result["dist-lib"]["url"], "https://releases.example.com/dist-lib"
        )

    def test_no_url_field_absent_when_no_external_refs(self):
        result = self._convert([{"name": "local-lib", "version": "1.0"}])
        self.assertNotIn("url", result["local-lib"])


class TestConvertCdxgenToCacheDescription(unittest.TestCase):
    """Description field extraction."""

    def setUp(self):
        self._cleanup: list[str] = []

    def tearDown(self):
        for path in self._cleanup:
            try:
                os.unlink(path)
            except FileNotFoundError:
                pass

    def _convert(self, components: list) -> dict:
        path = _write_cdx(components)
        self._cleanup.append(path)
        return convert_cdxgen_to_cache(path)

    def test_description_extracted(self):
        result = self._convert(
            [
                {
                    "name": "boost",
                    "version": "1.87.0",
                    "description": "Boost C++ Libraries",
                }
            ]
        )
        self.assertEqual(result["boost"]["description"], "Boost C++ Libraries")

    def test_no_description_field_absent(self):
        result = self._convert([{"name": "lib-no-desc", "version": "1.0"}])
        self.assertNotIn("description", result["lib-no-desc"])
