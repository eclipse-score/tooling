"""Tests for BCR known-license resolution in sbom_generator.

These tests verify that C++ modules from the Bazel Central Registry
(e.g. boost.*) receive correct license data even when cdxgen and
lockfile parsing cannot provide it.
"""

import unittest

from sbom.internal.generator.sbom_generator import (
    BCR_KNOWN_LICENSES,
    apply_known_licenses,
    resolve_component,
)


class TestBcrKnownLicenses(unittest.TestCase):
    """Verify the BCR_KNOWN_LICENSES table contents."""

    def test_boost_entry_exists(self):
        self.assertIn("boost", BCR_KNOWN_LICENSES)
        self.assertEqual(BCR_KNOWN_LICENSES["boost"]["license"], "BSL-1.0")

    def test_all_entries_have_license(self):
        for name, info in BCR_KNOWN_LICENSES.items():
            self.assertTrue(
                info.get("license"),
                f"BCR_KNOWN_LICENSES['{name}'] has no license",
            )


class TestApplyKnownLicenses(unittest.TestCase):
    """Tests for apply_known_licenses()."""

    # -- BCR known-license fallback -------------------------------------------

    def test_boost_submodule_gets_license(self):
        """boost.config should inherit BSL-1.0 from the 'boost' BCR entry."""
        metadata = {
            "modules": {
                "boost.config": {"version": "1.87.0", "purl": "pkg:bazel/boost.config@1.87.0"},
            },
            "licenses": {},
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["boost.config"]["license"], "BSL-1.0")
        self.assertEqual(metadata["modules"]["boost.config"]["supplier"], "Boost.org")

    def test_multiple_boost_submodules(self):
        """All boost.* sub-modules should receive BSL-1.0."""
        names = [
            "boost.config", "boost.assert", "boost.mp11", "boost.container",
            "boost.interprocess", "boost.core", "boost.predef",
        ]
        metadata = {
            "modules": {
                n: {"version": "1.87.0", "purl": f"pkg:bazel/{n}@1.87.0"}
                for n in names
            },
            "licenses": {},
        }
        apply_known_licenses(metadata)

        for n in names:
            self.assertEqual(
                metadata["modules"][n]["license"], "BSL-1.0",
                f"{n} should have BSL-1.0 license",
            )

    def test_exact_bcr_match(self):
        """A module matching a BCR key exactly gets the license."""
        metadata = {
            "modules": {
                "abseil-cpp": {"version": "20230802.0", "purl": "pkg:bazel/abseil-cpp@20230802.0"},
            },
            "licenses": {},
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["abseil-cpp"]["license"], "Apache-2.0")

    def test_unknown_module_unchanged(self):
        """Modules not in BCR_KNOWN_LICENSES remain without a license."""
        metadata = {
            "modules": {
                "some_unknown_lib": {"version": "1.0.0", "purl": "pkg:bazel/some_unknown_lib@1.0.0"},
            },
            "licenses": {},
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["some_unknown_lib"].get("license", ""), "")

    # -- Explicit license overrides (sbom_ext.license) ------------------------

    def test_explicit_license_override(self):
        """User-declared license in metadata['licenses'] takes priority."""
        metadata = {
            "modules": {
                "boost.config": {"version": "1.87.0", "purl": "pkg:bazel/boost.config@1.87.0"},
            },
            "licenses": {
                "boost.config": {"license": "MIT", "supplier": "Custom"},
            },
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["boost.config"]["license"], "MIT")
        self.assertEqual(metadata["modules"]["boost.config"]["supplier"], "Custom")

    def test_parent_license_override(self):
        """Parent-level license declaration covers all sub-modules."""
        metadata = {
            "modules": {
                "boost.config": {"version": "1.87.0", "purl": "pkg:bazel/boost.config@1.87.0"},
                "boost.container": {"version": "1.87.0", "purl": "pkg:bazel/boost.container@1.87.0"},
            },
            "licenses": {
                "boost": {"license": "BSL-1.0-custom", "supplier": "My Boost Fork"},
            },
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["boost.config"]["license"], "BSL-1.0-custom")
        self.assertEqual(metadata["modules"]["boost.container"]["license"], "BSL-1.0-custom")

    def test_explicit_beats_parent(self):
        """Exact-name license takes priority over parent-level declaration."""
        metadata = {
            "modules": {
                "boost.config": {"version": "1.87.0", "purl": "pkg:bazel/boost.config@1.87.0"},
            },
            "licenses": {
                "boost": {"license": "BSL-1.0", "supplier": "Boost.org"},
                "boost.config": {"license": "MIT-override", "supplier": "Override"},
            },
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["boost.config"]["license"], "MIT-override")

    def test_explicit_beats_bcr_known(self):
        """User-declared license overrides the BCR known-license database."""
        metadata = {
            "modules": {
                "boost.config": {"version": "1.87.0", "purl": "pkg:bazel/boost.config@1.87.0"},
            },
            "licenses": {
                "boost": {"license": "Apache-2.0", "supplier": "Custom Boost"},
            },
        }
        apply_known_licenses(metadata)

        # User's declaration should win over BCR_KNOWN_LICENSES["boost"]
        self.assertEqual(metadata["modules"]["boost.config"]["license"], "Apache-2.0")

    # -- Preserves existing data ----------------------------------------------

    def test_existing_license_not_overwritten(self):
        """Modules that already have a license are not modified."""
        metadata = {
            "modules": {
                "boost.config": {
                    "version": "1.87.0",
                    "purl": "pkg:bazel/boost.config@1.87.0",
                    "license": "Already-Set",
                    "supplier": "Original",
                },
            },
            "licenses": {},
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["boost.config"]["license"], "Already-Set")
        self.assertEqual(metadata["modules"]["boost.config"]["supplier"], "Original")

    def test_supplier_not_overwritten_when_present(self):
        """Existing supplier is preserved even when license is filled from BCR."""
        metadata = {
            "modules": {
                "boost.config": {
                    "version": "1.87.0",
                    "purl": "pkg:bazel/boost.config@1.87.0",
                    "supplier": "My Custom Supplier",
                },
            },
            "licenses": {},
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["boost.config"]["license"], "BSL-1.0")
        self.assertEqual(metadata["modules"]["boost.config"]["supplier"], "My Custom Supplier")

    # -- Edge cases -----------------------------------------------------------

    def test_empty_metadata(self):
        """Empty metadata does not raise."""
        metadata = {}
        apply_known_licenses(metadata)  # Should not raise

    def test_no_licenses_key(self):
        """Missing 'licenses' key does not raise."""
        metadata = {
            "modules": {
                "boost.config": {"version": "1.87.0", "purl": "pkg:bazel/boost.config@1.87.0"},
            },
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["boost.config"]["license"], "BSL-1.0")

    def test_module_without_dot_not_treated_as_parent(self):
        """A module name without dots only matches exact BCR entries."""
        metadata = {
            "modules": {
                "zlib": {"version": "1.3.1", "purl": "pkg:bazel/zlib@1.3.1"},
            },
            "licenses": {},
        }
        apply_known_licenses(metadata)

        self.assertEqual(metadata["modules"]["zlib"]["license"], "Zlib")


class TestResolveComponentWithLicenses(unittest.TestCase):
    """Verify that resolve_component returns licenses from metadata modules."""

    def test_module_with_license_from_apply(self):
        """After apply_known_licenses, resolve_component picks up the license."""
        metadata = {
            "modules": {
                "boost.config": {
                    "version": "1.87.0",
                    "purl": "pkg:bazel/boost.config@1.87.0",
                    "license": "BSL-1.0",
                    "supplier": "Boost.org",
                },
            },
            "licenses": {},
        }
        comp = resolve_component("boost.config+", metadata)

        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "boost.config")
        self.assertEqual(comp["license"], "BSL-1.0")


if __name__ == "__main__":
    unittest.main()
