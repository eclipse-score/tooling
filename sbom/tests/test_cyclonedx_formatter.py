"""Tests for CycloneDX 1.6 formatter."""

import unittest
from datetime import datetime, timezone

from sbom.internal.generator.cyclonedx_formatter import (
    generate_cyclonedx,
    _normalize_spdx_license,
)


class TestCycloneDXFormatter(unittest.TestCase):
    """Tests for CycloneDX 1.6 generation."""

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
                "source": "crates.io",
            },
            {
                "name": "serde",
                "version": "1.0.0",
                "purl": "pkg:cargo/serde@1.0.0",
                "type": "library",
                "license": "MIT OR Apache-2.0",
                "source": "crates.io",
            },
        ]

    def test_generate_cyclonedx_structure(self):
        """Test that generated CycloneDX has correct structure."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        self.assertEqual(cdx["bomFormat"], "CycloneDX")
        self.assertEqual(cdx["specVersion"], "1.6")
        self.assertIn("serialNumber", cdx)
        self.assertTrue(cdx["serialNumber"].startswith("urn:uuid:"))
        self.assertEqual(cdx["version"], 1)

    def test_generate_cyclonedx_metadata(self):
        """Test that CycloneDX metadata is correct."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        metadata = cdx["metadata"]
        self.assertEqual(metadata["timestamp"], self.timestamp)
        self.assertIn("tools", metadata)
        self.assertIn("component", metadata)

        root_component = metadata["component"]
        self.assertEqual(root_component["name"], "test-component")
        self.assertEqual(root_component["version"], "1.0.0")
        self.assertEqual(root_component["type"], "application")

    def test_generate_cyclonedx_components(self):
        """Test that components are properly added."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        components = cdx["components"]
        self.assertEqual(len(components), 2)

        component_names = {c["name"] for c in components}
        self.assertEqual(component_names, {"tokio", "serde"})

    def test_generate_cyclonedx_component_details(self):
        """Test that component details are correct."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        tokio = next(c for c in cdx["components"] if c["name"] == "tokio")

        self.assertEqual(tokio["version"], "1.10.0")
        self.assertEqual(tokio["type"], "library")
        self.assertEqual(tokio["purl"], "pkg:cargo/tokio@1.10.0")
        self.assertIn("bom-ref", tokio)

    def test_generate_cyclonedx_licenses(self):
        """Test that licenses are properly set."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        tokio = next(c for c in cdx["components"] if c["name"] == "tokio")

        self.assertIn("licenses", tokio)
        self.assertEqual(len(tokio["licenses"]), 1)
        self.assertEqual(tokio["licenses"][0]["license"]["id"], "MIT")

    def test_generate_cyclonedx_dependencies(self):
        """Test that dependencies are created."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        dependencies = cdx["dependencies"]

        # Should have root + 2 component dependency entries
        self.assertEqual(len(dependencies), 3)

        # Find root dependency
        root_dep = next(d for d in dependencies if d["ref"] == "test-component@1.0.0")
        self.assertEqual(len(root_dep["dependsOn"]), 2)

    def test_generate_cyclonedx_external_references(self):
        """Test that external references are added for crates.io sources."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        tokio = next(c for c in cdx["components"] if c["name"] == "tokio")

        self.assertIn("externalReferences", tokio)
        ext_refs = tokio["externalReferences"]

        distribution_ref = next(
            (r for r in ext_refs if r["type"] == "distribution"), None
        )
        self.assertIsNotNone(distribution_ref)
        self.assertIn("crates.io", distribution_ref["url"])

    def test_generate_cyclonedx_cratesio_external_ref_from_source_field(self):
        """Crates with source=crates.io get a distribution externalReference URL."""
        components = [
            {
                "name": "serde",
                "version": "1.0.228",
                "purl": "pkg:cargo/serde@1.0.228",
                "type": "library",
                "license": "MIT OR Apache-2.0",
                "source": "crates.io",
            }
        ]
        cdx = generate_cyclonedx(components, self.config, self.timestamp)
        serde = next(c for c in cdx["components"] if c["name"] == "serde")
        ext_refs = serde.get("externalReferences", [])
        dist_ref = next((r for r in ext_refs if r["type"] == "distribution"), None)
        self.assertIsNotNone(
            dist_ref, "Expected distribution externalReference for crates.io crate"
        )
        self.assertIn("crates.io/crates/serde/1.0.228", dist_ref["url"])

    def test_generate_cyclonedx_schema_url_uses_https(self):
        """Test that $schema URL uses https:// not http://."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)
        self.assertTrue(
            cdx["$schema"].startswith("https://"),
            f"$schema should use https://, got: {cdx['$schema']}",
        )

    def test_generate_cyclonedx_with_empty_components(self):
        """Test generating CycloneDX with no components."""
        cdx = generate_cyclonedx([], self.config, self.timestamp)

        self.assertEqual(len(cdx["components"]), 0)
        self.assertEqual(len(cdx["dependencies"]), 1)  # Just root

    def test_generate_cyclonedx_bom_refs_unique(self):
        """Test that bom-refs are unique across components."""
        cdx = generate_cyclonedx(self.components, self.config, self.timestamp)

        bom_refs = [c["bom-ref"] for c in cdx["components"]]
        self.assertEqual(len(bom_refs), len(set(bom_refs)))


class TestNormalizeSpdxLicenseCdx(unittest.TestCase):
    """Verify lowercase operator normalization for CycloneDX formatter."""

    def test_lowercase_or_normalized(self):
        self.assertEqual(
            _normalize_spdx_license("Apache-2.0 or MIT"), "Apache-2.0 OR MIT"
        )

    def test_gpl_or_later_not_mangled(self):
        self.assertEqual(
            _normalize_spdx_license("GPL-2.0-or-later"), "GPL-2.0-or-later"
        )

    def test_lowercase_or_routes_to_expression_field(self):
        """'Apache-2.0 or MIT' from dash-license-scan must use expression field, not license.id."""
        config = {
            "component_name": "test",
            "component_version": "1.0",
            "producer_name": "Test",
            "namespace": "https://example.com",
        }
        timestamp = "2024-01-01T00:00:00+00:00"
        components = [
            {
                "name": "serde",
                "version": "1.0.228",
                "purl": "pkg:cargo/serde@1.0.228",
                "type": "library",
                "license": "Apache-2.0 or MIT",
            }
        ]
        cdx = generate_cyclonedx(components, config, timestamp)
        serde = next(c for c in cdx["components"] if c["name"] == "serde")
        licenses = serde.get("licenses", [])
        self.assertEqual(len(licenses), 1)
        # Must use 'expression' field with uppercase OR, not 'license.id'
        self.assertIn(
            "expression", licenses[0], "compound license must use 'expression' field"
        )
        self.assertEqual(licenses[0]["expression"], "Apache-2.0 OR MIT")
        self.assertNotIn("license", licenses[0])


if __name__ == "__main__":
    unittest.main()
