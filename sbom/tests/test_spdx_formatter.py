"""Tests for SPDX 2.3 formatter."""

import unittest
from datetime import datetime, timezone

from sbom.internal.generator.spdx_formatter import generate_spdx, _normalize_spdx_license


class TestSpdxFormatter(unittest.TestCase):
    """Tests for SPDX 2.3 generation."""

    def setUp(self):
        """Set up test fixtures."""
        self.timestamp = datetime(
            2024, 1, 15, 12, 0, 0, tzinfo=timezone.utc
        ).isoformat()
        self.config = {
            "component_name": "test-component",
            "component_version": "1.0.0",
            "producer_name": "Eclipse Foundation",
            "producer_url": "https://eclipse.dev/score",
            "namespace": "https://eclipse.dev/score",
        }
        self.components = [
            {
                "name": "tokio",
                "version": "1.10.0",
                "purl": "pkg:cargo/tokio@1.10.0",
                "type": "library",
                "license": "MIT",
            },
            {
                "name": "serde",
                "version": "1.0.0",
                "purl": "pkg:cargo/serde@1.0.0",
                "type": "library",
                "license": "MIT OR Apache-2.0",
            },
        ]

    def test_generate_spdx_structure(self):
        """Test that generated SPDX has correct structure."""
        spdx = generate_spdx(self.components, self.config, self.timestamp)

        self.assertEqual(spdx["spdxVersion"], "SPDX-2.3")
        self.assertEqual(spdx["dataLicense"], "CC0-1.0")
        self.assertEqual(spdx["SPDXID"], "SPDXRef-DOCUMENT")
        self.assertIn("documentNamespace", spdx)
        self.assertIn("packages", spdx)
        self.assertIn("relationships", spdx)

    def test_generate_spdx_document_info(self):
        """Test that SPDX document has correct metadata."""
        spdx = generate_spdx(self.components, self.config, self.timestamp)

        self.assertEqual(spdx["name"], "SBOM for test-component")
        creation_info = spdx["creationInfo"]
        self.assertEqual(creation_info["created"], self.timestamp)
        creators = creation_info["creators"]
        self.assertIn("Organization: Eclipse Foundation", creators)
        self.assertIn("Tool: score-sbom-generator", creators)

    def test_generate_spdx_components(self):
        """Test that components are properly added to SPDX."""
        spdx = generate_spdx(self.components, self.config, self.timestamp)

        packages = spdx["packages"]
        # root package + 2 components
        self.assertEqual(len(packages), 3)

    def test_generate_spdx_relationships(self):
        """Test that dependency relationships are created."""
        spdx = generate_spdx(self.components, self.config, self.timestamp)

        relationships = spdx["relationships"]
        # DESCRIBES + 2 DEPENDS_ON
        describes = [r for r in relationships if r["relationshipType"] == "DESCRIBES"]
        depends_on = [r for r in relationships if r["relationshipType"] == "DEPENDS_ON"]

        self.assertEqual(len(describes), 1)
        self.assertEqual(len(depends_on), 2)

    def test_generate_spdx_with_empty_components(self):
        """Test generating SPDX with no components."""
        spdx = generate_spdx([], self.config, self.timestamp)

        packages = spdx["packages"]
        # Only root package
        self.assertEqual(len(packages), 1)

    def test_generate_spdx_component_purl(self):
        """Test that component PURLs are properly set."""
        spdx = generate_spdx(self.components, self.config, self.timestamp)

        packages = spdx["packages"]
        tokio_pkg = next((p for p in packages if p["name"] == "tokio"), None)

        self.assertIsNotNone(tokio_pkg)
        ext_refs = tokio_pkg.get("externalRefs", [])
        purl_ref = next(
            (r for r in ext_refs if r.get("referenceType") == "purl"),
            None,
        )
        self.assertIsNotNone(purl_ref)
        self.assertEqual(purl_ref["referenceLocator"], "pkg:cargo/tokio@1.10.0")


    def test_generate_spdx_component_checksum(self):
        """Test that SHA-256 checksums are emitted when available."""
        components_with_hash = [
            {
                "name": "serde",
                "version": "1.0.0",
                "purl": "pkg:cargo/serde@1.0.0",
                "type": "library",
                "license": "MIT OR Apache-2.0",
                "checksum": "abc123def456abc123def456abc123def456abc123def456abc123def456abcd",
            }
        ]
        spdx = generate_spdx(components_with_hash, self.config, self.timestamp)

        packages = spdx["packages"]
        serde_pkg = next((p for p in packages if p["name"] == "serde"), None)
        self.assertIsNotNone(serde_pkg)
        self.assertIn("checksums", serde_pkg)
        self.assertEqual(len(serde_pkg["checksums"]), 1)
        self.assertEqual(serde_pkg["checksums"][0]["algorithm"], "SHA256")
        self.assertEqual(
            serde_pkg["checksums"][0]["checksumValue"],
            "abc123def456abc123def456abc123def456abc123def456abc123def456abcd",
        )

    def test_generate_spdx_no_checksum_when_absent(self):
        """Test that checksums field is absent when no checksum available."""
        spdx = generate_spdx(self.components, self.config, self.timestamp)

        packages = spdx["packages"]
        tokio_pkg = next((p for p in packages if p["name"] == "tokio"), None)
        self.assertIsNotNone(tokio_pkg)
        self.assertNotIn("checksums", tokio_pkg)


class TestNormalizeSpdxLicense(unittest.TestCase):
    """Tests for SPDX boolean operator normalization."""

    def test_lowercase_or_uppercased(self):
        self.assertEqual(_normalize_spdx_license("Apache-2.0 or MIT"), "Apache-2.0 OR MIT")

    def test_lowercase_and_uppercased(self):
        self.assertEqual(_normalize_spdx_license("MIT and Apache-2.0"), "MIT AND Apache-2.0")

    def test_lowercase_with_uppercased(self):
        self.assertEqual(_normalize_spdx_license("GPL-2.0 with Classpath-exception-2.0"), "GPL-2.0 WITH Classpath-exception-2.0")

    def test_already_uppercase_unchanged(self):
        self.assertEqual(_normalize_spdx_license("Apache-2.0 OR MIT"), "Apache-2.0 OR MIT")

    def test_gpl_or_later_identifier_not_mangled(self):
        """GPL-2.0-or-later has '-or-' (hyphen-delimited) — must not be uppercased."""
        self.assertEqual(_normalize_spdx_license("GPL-2.0-or-later"), "GPL-2.0-or-later")

    def test_mixed_compound_expression(self):
        self.assertEqual(
            _normalize_spdx_license("(Apache-2.0 or MIT) and Unicode-DFS-2016"),
            "(Apache-2.0 OR MIT) AND Unicode-DFS-2016",
        )

    def test_empty_string(self):
        self.assertEqual(_normalize_spdx_license(""), "")

    def test_single_license_unchanged(self):
        self.assertEqual(_normalize_spdx_license("MIT"), "MIT")

    def test_lowercase_operator_in_spdx_output_end_to_end(self):
        """Verify that lowercase 'or' from dash-license-scan is normalized in SPDX output."""
        config = {
            "component_name": "test",
            "component_version": "1.0",
            "producer_name": "Test",
            "namespace": "https://example.com",
        }
        timestamp = "2024-01-01T00:00:00+00:00"
        components = [{"name": "serde", "version": "1.0.228", "license": "Apache-2.0 or MIT"}]
        spdx = generate_spdx(components, config, timestamp)
        serde_pkg = next(p for p in spdx["packages"] if p["name"] == "serde")
        self.assertEqual(serde_pkg["licenseConcluded"], "Apache-2.0 OR MIT")
        self.assertEqual(serde_pkg["licenseDeclared"], "Apache-2.0 OR MIT")


if __name__ == "__main__":
    unittest.main()
