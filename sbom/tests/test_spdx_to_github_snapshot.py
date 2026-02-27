"""Tests for SPDX 2.3 → GitHub Dependency Submission snapshot conversion."""

import unittest

from sbom.scripts.spdx_to_github_snapshot import convert_spdx_to_snapshot


def _make_spdx(
    packages: list[dict],
    relationships: list[dict] | None = None,
    doc_name: str = "test-sbom",
) -> dict:
    return {
        "spdxVersion": "SPDX-2.3",
        "name": doc_name,
        "SPDXID": "SPDXRef-DOCUMENT",
        "packages": packages,
        "relationships": relationships or [],
    }


def _cargo_pkg(
    spdx_id: str, name: str, version: str, purl: str | None = None
) -> dict:
    pkg: dict = {
        "SPDXID": spdx_id,
        "name": name,
        "versionInfo": version,
        "downloadLocation": "https://crates.io",
    }
    if purl:
        pkg["externalRefs"] = [
            {"referenceCategory": "PACKAGE-MANAGER", "referenceType": "purl", "referenceLocator": purl}
        ]
    return pkg


class TestConvertSpdxToSnapshot(unittest.TestCase):

    def _base_snapshot(self, spdx: dict, **kwargs) -> dict:
        return convert_spdx_to_snapshot(
            spdx=spdx,
            sha="abc123" * 6 + "ab",  # 38 chars, close enough for test
            ref="refs/heads/main",
            job_correlator="test-workflow_sbom",
            job_id="42",
            **kwargs,
        )

    def test_snapshot_top_level_fields(self):
        spdx = _make_spdx(packages=[])
        snapshot = self._base_snapshot(spdx)
        self.assertEqual(snapshot["version"], 0)
        self.assertIn("sha", snapshot)
        self.assertIn("ref", snapshot)
        self.assertIn("job", snapshot)
        self.assertIn("detector", snapshot)
        self.assertIn("scanned", snapshot)
        self.assertIn("manifests", snapshot)

    def test_detector_fields(self):
        spdx = _make_spdx(packages=[])
        snapshot = self._base_snapshot(spdx)
        detector = snapshot["detector"]
        self.assertEqual(detector["name"], "score-sbom-generator")
        self.assertIn("version", detector)
        self.assertIn("url", detector)

    def test_job_correlator(self):
        spdx = _make_spdx(packages=[])
        snapshot = self._base_snapshot(spdx)
        self.assertEqual(snapshot["job"]["correlator"], "test-workflow_sbom")
        self.assertEqual(snapshot["job"]["id"], "42")

    def test_packages_without_purl_are_excluded(self):
        root_pkg = _cargo_pkg("SPDXRef-root", "myapp", "1.0.0", purl="pkg:github/eclipse-score/myapp@1.0.0")
        no_purl_pkg = _cargo_pkg("SPDXRef-nopurl", "internal-tool", "0.1.0")
        spdx = _make_spdx(
            packages=[root_pkg, no_purl_pkg],
            relationships=[
                {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-root"},
            ],
        )
        snapshot = self._base_snapshot(spdx)
        manifest = next(iter(snapshot["manifests"].values()))
        resolved = manifest["resolved"]
        # no_purl_pkg has no PURL → excluded
        self.assertFalse(any("internal-tool" in k for k in resolved))

    def test_root_package_excluded_from_resolved(self):
        root_pkg = _cargo_pkg("SPDXRef-root", "myapp", "1.0.0", purl="pkg:github/eclipse-score/myapp@1.0.0")
        dep_pkg = _cargo_pkg("SPDXRef-serde", "serde", "1.0.228", purl="pkg:cargo/serde@1.0.228")
        spdx = _make_spdx(
            packages=[root_pkg, dep_pkg],
            relationships=[
                {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-root"},
                {"spdxElementId": "SPDXRef-root", "relationshipType": "DEPENDS_ON", "relatedSpdxElement": "SPDXRef-serde"},
            ],
        )
        snapshot = self._base_snapshot(spdx)
        manifest = next(iter(snapshot["manifests"].values()))
        resolved = manifest["resolved"]
        # Root package (myapp) should not appear in resolved
        self.assertFalse(any("myapp" in k for k in resolved))
        # Dep package should appear
        self.assertTrue(any("serde" in k for k in resolved))

    def test_direct_vs_indirect_relationship(self):
        root_pkg = _cargo_pkg("SPDXRef-root", "myapp", "1.0.0", purl="pkg:github/eclipse-score/myapp@1.0.0")
        direct_pkg = _cargo_pkg("SPDXRef-tokio", "tokio", "1.0.0", purl="pkg:cargo/tokio@1.0.0")
        indirect_pkg = _cargo_pkg("SPDXRef-mio", "mio", "0.8.0", purl="pkg:cargo/mio@0.8.0")
        spdx = _make_spdx(
            packages=[root_pkg, direct_pkg, indirect_pkg],
            relationships=[
                {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-root"},
                {"spdxElementId": "SPDXRef-root", "relationshipType": "DEPENDS_ON", "relatedSpdxElement": "SPDXRef-tokio"},
                {"spdxElementId": "SPDXRef-tokio", "relationshipType": "DEPENDS_ON", "relatedSpdxElement": "SPDXRef-mio"},
            ],
        )
        snapshot = self._base_snapshot(spdx)
        manifest = next(iter(snapshot["manifests"].values()))
        resolved = manifest["resolved"]

        tokio_entry = next(v for k, v in resolved.items() if "tokio" in k)
        mio_entry = next(v for k, v in resolved.items() if "mio" in k)

        self.assertEqual(tokio_entry["relationship"], "direct")
        self.assertEqual(mio_entry["relationship"], "indirect")

    def test_package_url_preserved(self):
        root_pkg = _cargo_pkg("SPDXRef-root", "myapp", "1.0.0", purl="pkg:github/eclipse-score/myapp@1.0.0")
        dep_pkg = _cargo_pkg("SPDXRef-serde", "serde", "1.0.228", purl="pkg:cargo/serde@1.0.228")
        spdx = _make_spdx(
            packages=[root_pkg, dep_pkg],
            relationships=[
                {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-root"},
                {"spdxElementId": "SPDXRef-root", "relationshipType": "DEPENDS_ON", "relatedSpdxElement": "SPDXRef-serde"},
            ],
        )
        snapshot = self._base_snapshot(spdx)
        manifest = next(iter(snapshot["manifests"].values()))
        resolved = manifest["resolved"]
        serde_entry = next(v for k, v in resolved.items() if "serde" in k)
        self.assertEqual(serde_entry["package_url"], "pkg:cargo/serde@1.0.228")

    def test_manifest_name_from_spdx_document_name(self):
        spdx = _make_spdx(packages=[], doc_name="my-sbom-component")
        snapshot = self._base_snapshot(spdx)
        self.assertIn("my-sbom-component", snapshot["manifests"])

    def test_empty_spdx_produces_empty_manifest(self):
        spdx = _make_spdx(packages=[])
        snapshot = self._base_snapshot(spdx)
        manifest = next(iter(snapshot["manifests"].values()))
        self.assertEqual(manifest["resolved"], {})

    def test_sha_and_ref_set_correctly(self):
        spdx = _make_spdx(packages=[])
        snapshot = convert_spdx_to_snapshot(
            spdx=spdx,
            sha="deadbeef" * 5,
            ref="refs/tags/v1.0.0",
            job_correlator="ci_sbom",
            job_id="99",
        )
        self.assertEqual(snapshot["sha"], "deadbeef" * 5)
        self.assertEqual(snapshot["ref"], "refs/tags/v1.0.0")

    def test_generic_purl_included(self):
        """pkg:generic/ PURLs (BCR modules) are accepted by GitHub Dependency Graph."""
        root_pkg = _cargo_pkg("SPDXRef-root", "myapp", "1.0.0", purl="pkg:github/eclipse-score/myapp@1.0.0")
        boost_pkg = _cargo_pkg("SPDXRef-boost", "boost.filesystem", "1.83.0", purl="pkg:generic/boost.filesystem@1.83.0")
        spdx = _make_spdx(
            packages=[root_pkg, boost_pkg],
            relationships=[
                {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-root"},
                {"spdxElementId": "SPDXRef-root", "relationshipType": "DEPENDS_ON", "relatedSpdxElement": "SPDXRef-boost"},
            ],
        )
        snapshot = self._base_snapshot(spdx)
        manifest = next(iter(snapshot["manifests"].values()))
        resolved = manifest["resolved"]
        boost_entry = next((v for k, v in resolved.items() if "boost" in k), None)
        self.assertIsNotNone(boost_entry)
        self.assertEqual(boost_entry["package_url"], "pkg:generic/boost.filesystem@1.83.0")


if __name__ == "__main__":
    unittest.main()
