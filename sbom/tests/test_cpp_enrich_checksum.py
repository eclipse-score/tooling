"""Tests for enrich_components_from_cpp_cache() and the no-manual-curation rule.

What this file tests
---------------------
enrich_components_from_cpp_cache() — field propagation
  - SHA-256 checksum is copied from cache to a component that has none.
  - An existing checksum on the component is never overwritten.
  - A cache entry with no checksum leaves the component's checksum empty.
  - Components with no matching cache entry are left unchanged.
  - Normalised-name matching: nlohmann_json (underscore) matches
    nlohmann-json (hyphen) cache entry.
  - Parent-name matching: boost.config component matches a "boost" cache entry.

No-manual-curation rule (on-disk cpp_metadata.json)
  - cpp_metadata.json must be empty ({}); any entry signals a policy violation.
    All C++ metadata must be produced by generate_cpp_metadata_cache.py from
    cdxgen output, never written by hand.
  - Belt-and-suspenders: even if the file is non-empty, no SBOM field
    (checksum, license, supplier, version, purl, description) may appear.

Bazel target : //sbom/tests:test_cpp_enrich_checksum
Run          : bazel test //sbom/tests:test_cpp_enrich_checksum
               pytest sbom/tests/test_cpp_enrich_checksum.py -v
"""

import json
import pathlib
import unittest

from sbom.internal.generator.sbom_generator import enrich_components_from_cpp_cache

# SBOM fields that must never appear as manually-curated static values.
# If any of these appear in cpp_metadata.json they were hand-written and must
# be removed. The only valid sources are automated tooling (cdxgen, lockfiles).
_SBOM_FIELDS = {"checksum", "license", "supplier", "version", "purl", "description"}


class TestCppEnrichChecksumPropagation(unittest.TestCase):
    """enrich_components_from_cpp_cache field propagation mechanics.

    These tests exercise the code path using synthetic cache data generated
    by cdxgen (not manually written). The logic itself is valid — the
    restriction is on what may appear in the on-disk cpp_metadata.json.
    """

    def _run(self, components, cpp_components):
        return enrich_components_from_cpp_cache(components, cpp_components, {})

    def test_checksum_propagated_when_component_has_none(self):
        """SHA-256 from the cdxgen-generated cache is copied to a component with no checksum."""
        sha = "a22461d13119ac5c78f205d3df1db13403e58ce1bb1794edc9313677313f4a9d"
        components = [{"name": "nlohmann-json", "version": "3.11.3", "checksum": ""}]
        cpp_cache = [{"name": "nlohmann-json", "version": "3.11.3", "checksum": sha}]

        result = self._run(components, cpp_cache)

        self.assertEqual(result[0]["checksum"], sha)

    def test_checksum_not_overwritten_when_already_present(self):
        """An existing checksum on a component is preserved — cache is skipped."""
        existing = "aaaa" * 16
        cache_sha = "bbbb" * 16
        components = [
            {"name": "flatbuffers", "version": "25.2.10", "checksum": existing}
        ]
        cpp_cache = [{"name": "flatbuffers", "checksum": cache_sha}]

        result = self._run(components, cpp_cache)

        self.assertEqual(result[0]["checksum"], existing)

    def test_no_checksum_in_cache_leaves_component_without_checksum(self):
        """When the cache entry has no checksum the component remains without one."""
        components = [{"name": "boost", "version": "1.87.0", "checksum": ""}]
        cpp_cache = [{"name": "boost", "license": "BSL-1.0"}]

        result = self._run(components, cpp_cache)

        self.assertEqual(result[0]["checksum"], "")

    def test_component_without_matching_cache_entry_unchanged(self):
        """A component with no matching cache entry is not modified."""
        components = [{"name": "some-unknown-lib", "checksum": ""}]
        cpp_cache = [{"name": "nlohmann-json", "checksum": "aaaa"}]

        result = self._run(components, cpp_cache)

        self.assertEqual(result[0]["checksum"], "")

    def test_checksum_propagated_via_normalised_name(self):
        """nlohmann_json (underscore) component matches nlohmann-json cache entry."""
        sha = "a22461d13119ac5c78f205d3df1db13403e58ce1bb1794edc9313677313f4a9d"
        components = [{"name": "nlohmann_json", "checksum": ""}]
        cpp_cache = [{"name": "nlohmann-json", "checksum": sha}]

        result = self._run(components, cpp_cache)

        self.assertEqual(result[0]["checksum"], sha)

    def test_checksum_propagated_via_parent_match(self):
        """boost.config component matches the 'boost' cache entry."""
        sha = "deadbeef" * 8
        components = [{"name": "boost.config", "checksum": ""}]
        cpp_cache = [{"name": "boost", "checksum": sha}]

        result = self._run(components, cpp_cache)

        self.assertEqual(result[0]["checksum"], sha)


class TestNoManualFallbackInCppMetadata(unittest.TestCase):
    """Enforce the no-manual-fallback requirement on the on-disk cache.

    MUST REQUIREMENT: cpp_metadata.json must never contain manually-curated
    SBOM field values. The file must either be empty ({}) or contain only
    entries generated automatically by generate_cpp_metadata_cache.py from
    cdxgen output.

    Rationale: A manually-written value is tied to a specific version string
    in the file. If the workspace resolves a different version of that library,
    the value silently describes the wrong artifact — an incorrect SBOM entry
    is worse than an absent one. All SBOM fields must trace back to an
    automated source (cdxgen scan, MODULE.bazel.lock, http_archive sha256).

    Known violations still to be resolved:
    - BCR_KNOWN_LICENSES dict in sbom_generator.py (manual license/supplier
      lookup for BCR C++ modules — must be replaced by automated BCR metadata
      fetching or removed).
    """

    _CACHE_PATH = pathlib.Path(__file__).parent.parent / "cpp_metadata.json"

    def setUp(self):
        self._data = json.loads(self._CACHE_PATH.read_text(encoding="utf-8"))

    def test_cpp_metadata_json_is_empty(self):
        """cpp_metadata.json must be empty.

        Any entry in this file was written by hand. All C++ metadata must be
        produced by automated tooling at build time (cdxgen via auto_cdxgen,
        or lockfile parsing). If you need to populate this file, run:

            npx @cyclonedx/cdxgen -t cpp --deep -r -o cdxgen_output.cdx.json
            python3 tooling/sbom/scripts/generate_cpp_metadata_cache.py \\
                cdxgen_output.cdx.json tooling/sbom/cpp_metadata.json
        """
        self.assertEqual(
            self._data,
            {},
            "cpp_metadata.json must be empty. Found manually-curated entries: "
            + ", ".join(self._data.keys())
            + ". Remove them — use generate_cpp_metadata_cache.py to populate "
            "this file from cdxgen output instead.",
        )

    def test_no_sbom_fields_in_any_entry(self):
        """No entry in cpp_metadata.json may contain any SBOM metadata field.

        This is a belt-and-suspenders check: even if the file is non-empty
        (which the previous test already flags), no SBOM field value may be
        manually written. Automated generation via generate_cpp_metadata_cache.py
        is the only permitted source.
        """
        for lib, entry in self._data.items():
            manually_present = _SBOM_FIELDS & set(entry.keys())
            with self.subTest(lib=lib):
                self.assertFalse(
                    manually_present,
                    f"cpp_metadata.json['{lib}'] contains manually-curated SBOM "
                    f"fields: {manually_present}. All SBOM fields must come from "
                    f"automated sources only.",
                )
