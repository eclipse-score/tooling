"""Integration tests that generate real SBOMs from reference_integration fixtures.

What this file tests
---------------------
Generates SPDX 2.3 and CycloneDX 1.6 SBOMs for three real S-CORE components
using fixture files extracted directly from the reference_integration workspace:

  score_baselibs  — C++ foundational libraries (Boost, nlohmann_json, …)
  score_kyron     — Rust kyron framework (iceoryx2-qnx8, serde, syn, …)
  score_orchestrator — Rust orchestration layer (kyron + tracing + postcard, …)

Fixtures are stored in sbom/tests/fixtures/ and include:
  *_input.json             — Real Bazel aspect output (external_repos, config, …)
  sbom_metadata.json       — Real sbom_metadata Bazel extension output
  crates_metadata.json     — 288-crate dash-license-scan + crates.io cache
  *_cdxgen.cdx.json        — Real cdxgen C++-scan output for each component
  reference_integration.MODULE.bazel.lock  — Minimal MODULE.bazel.lock slice

SPDX 2.3 structural rules validated per https://spdx.github.io/spdx-spec/v2.3/
CycloneDX 1.6 structural rules validated per https://cyclonedx.org/docs/1.6/json/

Online validation against https://sbomgenerator.com/tools/validator is performed
automatically in test_online_validator_accepts_all_sboms and skipped gracefully
when the service is unreachable (e.g. offline CI environments).

Bazel target : //sbom/tests:test_real_sbom_integration
Run          : bazel test //sbom/tests:test_real_sbom_integration
               pytest sbom/tests/test_real_sbom_integration.py -v
"""

import json
import os
import re
import shutil
import tempfile
import unittest
import unittest.mock
import urllib.request
from pathlib import Path

from sbom.internal.generator.sbom_generator import main

FIXTURES = Path(__file__).parent / "fixtures"


def _load_fixture(name: str) -> dict:
    with open(FIXTURES / name, encoding="utf-8") as f:
        return json.load(f)


def _write_json(path: str, data: dict) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f)


class TestRealSbomGeneration(unittest.TestCase):
    """SBOM generation and structural validation using real reference_integration fixtures."""

    def setUp(self):
        self.tmpdir = tempfile.mkdtemp(prefix="sbom_real_")
        self._lock_path = str(FIXTURES / "reference_integration.MODULE.bazel.lock")
        self._meta_path = str(FIXTURES / "sbom_metadata.json")
        self._crates_path = str(FIXTURES / "crates_metadata.json")

    def tearDown(self):
        shutil.rmtree(self.tmpdir, ignore_errors=True)

    # -----------------------------------------------------------------------
    # Helpers
    # -----------------------------------------------------------------------

    def _run(
        self,
        input_fixture: str,
        cdxgen_fixture: str | None = None,
    ) -> tuple[dict, dict]:
        """Load fixture, run sbom_generator.main(), return (spdx_doc, cdx_doc)."""
        input_data = _load_fixture(input_fixture)

        # Substitute the sentinel lockfile path with the real fixture path.
        input_data["module_lockfiles"] = [self._lock_path]

        input_path = os.path.join(self.tmpdir, "input.json")
        spdx_path = os.path.join(self.tmpdir, "out.spdx.json")
        cdx_path = os.path.join(self.tmpdir, "out.cdx.json")

        _write_json(input_path, input_data)

        argv = [
            "sbom_generator.py",
            "--input",
            input_path,
            "--metadata",
            self._meta_path,
            "--spdx-output",
            spdx_path,
            "--cyclonedx-output",
            cdx_path,
            "--crates-cache",
            self._crates_path,
        ]
        if cdxgen_fixture:
            argv += ["--cdxgen-sbom", str(FIXTURES / cdxgen_fixture)]

        with unittest.mock.patch("sys.argv", argv):
            rc = main()

        self.assertEqual(rc, 0, "sbom_generator.main() must return 0")

        with open(spdx_path, encoding="utf-8") as f:
            spdx = json.load(f)
        with open(cdx_path, encoding="utf-8") as f:
            cdx = json.load(f)

        return spdx, cdx

    # ── SPDX 2.3 structural validator ──────────────────────────────────────

    def _assert_valid_spdx(
        self,
        spdx: dict,
        component_name: str,
        expected_dep_names: list[str],
    ) -> None:
        """Assert SPDX 2.3 structural validity per the specification."""
        # Top-level required fields
        self.assertEqual(spdx["spdxVersion"], "SPDX-2.3")
        self.assertEqual(spdx["dataLicense"], "CC0-1.0")
        self.assertEqual(spdx["SPDXID"], "SPDXRef-DOCUMENT")
        self.assertIn("documentNamespace", spdx)
        self.assertIn("creationInfo", spdx)
        self.assertIn("packages", spdx)
        self.assertIn("relationships", spdx)

        ns = spdx["documentNamespace"]
        self.assertRegex(ns, r"^https?://", "documentNamespace must be a URI")

        ci = spdx["creationInfo"]
        self.assertIn("created", ci)
        self.assertIn("creators", ci)
        self.assertIsInstance(ci["creators"], list)
        self.assertTrue(ci["creators"], "creators must not be empty")

        pkgs = spdx["packages"]
        self.assertIsInstance(pkgs, list)
        self.assertGreater(len(pkgs), 1, "Must have root + at least one dep package")

        spdx_id_pattern = re.compile(r"^SPDXRef-[a-zA-Z0-9.\-]+$")

        # Root package
        root = next((p for p in pkgs if p.get("SPDXID") == "SPDXRef-RootPackage"), None)
        self.assertIsNotNone(root, "Root package SPDXRef-RootPackage must exist")
        self.assertEqual(root["name"], component_name)

        # All packages
        all_spdx_ids: set[str] = set()
        for pkg in pkgs:
            name = pkg.get("name", "")
            self.assertTrue(name, f"Package name must not be empty: {pkg}")

            sid = pkg.get("SPDXID", "")
            self.assertRegex(
                sid, spdx_id_pattern, f"Invalid SPDXID on package {name!r}"
            )
            self.assertNotIn(sid, all_spdx_ids, f"Duplicate SPDXID: {sid!r}")
            all_spdx_ids.add(sid)

            self.assertIn("versionInfo", pkg, f"Missing versionInfo on {name!r}")
            self.assertIn(
                "downloadLocation", pkg, f"Missing downloadLocation on {name!r}"
            )
            self.assertIn("filesAnalyzed", pkg, f"Missing filesAnalyzed on {name!r}")
            self.assertFalse(
                pkg["filesAnalyzed"], f"filesAnalyzed must be False on {name!r}"
            )
            self.assertIn(
                "licenseConcluded", pkg, f"Missing licenseConcluded on {name!r}"
            )
            self.assertIn(
                "licenseDeclared", pkg, f"Missing licenseDeclared on {name!r}"
            )
            self.assertIn("copyrightText", pkg, f"Missing copyrightText on {name!r}")

            # checksums entries must have algorithm + value
            for chk in pkg.get("checksums", []):
                self.assertIn("algorithm", chk)
                self.assertIn("checksumValue", chk)
                if chk["algorithm"] == "SHA256":
                    self.assertRegex(
                        chk["checksumValue"],
                        r"^[0-9a-f]{64}$",
                        f"SHA256 value on {name!r} must be 64 lowercase hex digits",
                    )

        # LicenseRef-* identifiers in packages must be declared
        licenseref_re = re.compile(r"LicenseRef-[A-Za-z0-9\-.]+")
        used_refs: set[str] = set()
        for pkg in pkgs:
            for field in ("licenseConcluded", "licenseDeclared"):
                used_refs.update(licenseref_re.findall(pkg.get(field, "")))
        if used_refs:
            declared = {
                e["licenseId"] for e in spdx.get("hasExtractedLicensingInfos", [])
            }
            for ref in used_refs:
                self.assertIn(
                    ref,
                    declared,
                    f"LicenseRef {ref!r} used but not declared in hasExtractedLicensingInfos",
                )

        # Relationships: at least DESCRIBES + one DEPENDS_ON
        rels = spdx["relationships"]
        self.assertIsInstance(rels, list)
        rel_types = {r["relationshipType"] for r in rels}
        self.assertIn("DESCRIBES", rel_types)
        self.assertIn("DEPENDS_ON", rel_types)

        # All relationship element IDs must reference known SPDXIDs
        doc_spdx_ids = all_spdx_ids | {"SPDXRef-DOCUMENT"}
        for rel in rels:
            for field in ("spdxElementId", "relatedSpdxElement"):
                self.assertIn(
                    rel[field],
                    doc_spdx_ids,
                    f"Relationship references unknown SPDXID {rel[field]!r}",
                )

        # Spot-check: expected dependency names must appear
        dep_names = {
            p["name"] for p in pkgs if p.get("SPDXID") != "SPDXRef-RootPackage"
        }
        for dep in expected_dep_names:
            self.assertIn(
                dep, dep_names, f"Expected dep {dep!r} not found in SPDX packages"
            )

    # ── CycloneDX 1.6 structural validator ─────────────────────────────────

    def _assert_valid_cdx(
        self,
        cdx: dict,
        component_name: str,
        expected_dep_names: list[str],
    ) -> None:
        """Assert CycloneDX 1.6 structural validity per the specification."""
        self.assertEqual(cdx["bomFormat"], "CycloneDX")
        self.assertEqual(cdx["specVersion"], "1.6")
        self.assertIn("serialNumber", cdx)
        self.assertRegex(
            cdx["serialNumber"],
            r"^urn:uuid:[0-9a-f-]{36}$",
            "serialNumber must be a URN UUID",
        )
        self.assertIsInstance(cdx.get("version"), int)

        # metadata
        meta = cdx.get("metadata", {})
        self.assertIn("timestamp", meta)
        self.assertIn("tools", meta)
        self.assertIn("component", meta)

        mc = meta["component"]
        self.assertEqual(mc["name"], component_name)
        self.assertIn("type", mc)
        self.assertIn("version", mc)
        self.assertIn("bom-ref", mc)

        # components
        comps = cdx.get("components", [])
        self.assertIsInstance(comps, list)
        self.assertGreater(len(comps), 0, "components must not be empty")

        CDX_TYPES = {
            "application",
            "library",
            "framework",
            "container",
            "device",
            "firmware",
            "file",
            "operating-system",
            "device-driver",
            "platform",
            "machine-learning-model",
            "data",
        }

        bom_refs: list[str] = []
        for comp in comps:
            name = comp.get("name", "")
            self.assertTrue(name, f"Component name must not be empty: {comp}")
            self.assertIn("type", comp, f"Missing type on {name!r}")
            self.assertIn("version", comp, f"Missing version on {name!r}")
            self.assertIn("bom-ref", comp, f"Missing bom-ref on {name!r}")
            self.assertIn(
                comp["type"],
                CDX_TYPES,
                f"Unknown CDX type {comp['type']!r} on {name!r}",
            )
            bom_refs.append(comp["bom-ref"])

            # hashes entries must have alg + content
            for h in comp.get("hashes", []):
                self.assertIn("alg", h)
                self.assertIn("content", h)

            # licenses must be a list of licence or expression objects
            for lic_entry in comp.get("licenses", []):
                self.assertTrue(
                    "license" in lic_entry or "expression" in lic_entry,
                    f"License entry on {name!r} must have 'license' or 'expression': {lic_entry}",
                )

        # bom-refs must be unique across all components
        self.assertEqual(
            len(bom_refs),
            len(set(bom_refs)),
            f"Duplicate bom-refs found: {[r for r in bom_refs if bom_refs.count(r) > 1]}",
        )

        # dependencies: root must depend on at least one component
        deps = cdx.get("dependencies", [])
        self.assertIsInstance(deps, list)
        root_dep = next((d for d in deps if d.get("ref") == mc["bom-ref"]), None)
        self.assertIsNotNone(root_dep, "Root component must have a dependency entry")
        self.assertGreater(
            len(root_dep.get("dependsOn", [])),
            0,
            "Root component must depend on at least one component",
        )

        # Spot-check: expected dependency names must appear
        comp_names = {c["name"] for c in comps}
        for dep in expected_dep_names:
            self.assertIn(
                dep, comp_names, f"Expected dep {dep!r} not found in CDX components"
            )

    # ── sbomgenerator.com online validator ─────────────────────────────────

    def _validate_online(self, content: str, fmt: str) -> dict | None:
        """POST content to sbomgenerator.com/tools/validator/validate.

        Returns the parsed JSON response dict on success, or None when the
        service is unreachable (network error, timeout, non-200 response).
        Never raises — callers must handle the None case.

        Args:
            content: Serialised SBOM string (JSON).
            fmt:     Format string accepted by the API: ``"spdx"`` or ``"cyclonedx"``.
        """
        payload = json.dumps(
            {
                "sbom_data": content,
                "format": fmt,
                "options": {
                    "strict": True,
                    "bestPractices": True,
                    "validatePurls": True,
                    "checkLicenses": True,
                },
            }
        ).encode()
        req = urllib.request.Request(
            "https://sbomgenerator.com/tools/validator/validate",
            data=payload,
            headers={
                "Content-Type": "application/json",
                "User-Agent": "Mozilla/5.0",
                "Referer": "https://sbomgenerator.com/tools/validator",
                "Origin": "https://sbomgenerator.com",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.loads(resp.read())
        except Exception:
            return None

    # -----------------------------------------------------------------------
    # Test cases
    # -----------------------------------------------------------------------

    def test_baselibs_spdx_and_cdx_are_valid(self):
        """score_baselibs SBOM (Boost + nlohmann_json) passes structural validation."""
        spdx, cdx = self._run("baselibs_input.json")

        self._assert_valid_spdx(
            spdx,
            component_name="score_baselibs",
            expected_dep_names=["boost.config", "boost.assert", "boost.mp11"],
        )
        self._assert_valid_cdx(
            cdx,
            component_name="score_baselibs",
            expected_dep_names=["boost.config", "boost.assert", "boost.mp11"],
        )

    def test_kyron_spdx_and_cdx_are_valid(self):
        """score_kyron SBOM (iceoryx2, serde, syn, …) passes structural validation."""
        spdx, cdx = self._run(
            "kyron_input.json", cdxgen_fixture="kyron_cdxgen.cdx.json"
        )

        self._assert_valid_spdx(
            spdx,
            component_name="score_kyron",
            expected_dep_names=["serde", "syn"],
        )
        self._assert_valid_cdx(
            cdx,
            component_name="score_kyron",
            expected_dep_names=["serde", "syn"],
        )

    def test_orchestrator_spdx_and_cdx_are_valid(self):
        """score_orchestrator SBOM (kyron + tracing + postcard, …) passes structural validation."""
        spdx, cdx = self._run(
            "orchestrator_input.json",
            cdxgen_fixture="orchestrator_cdxgen.cdx.json",
        )

        self._assert_valid_spdx(
            spdx,
            component_name="score_orchestrator",
            expected_dep_names=["serde", "postcard", "tracing"],
        )
        self._assert_valid_cdx(
            cdx,
            component_name="score_orchestrator",
            expected_dep_names=["serde", "postcard", "tracing"],
        )

    def test_baselibs_package_count(self):
        """score_baselibs SBOM must contain the expected number of packages."""
        spdx, cdx = self._run("baselibs_input.json")
        pkgs = spdx["packages"]
        comps = cdx["components"]
        # root + 19 deps (Boost sub-libs + nlohmann_json + acl-deb)
        self.assertEqual(len(pkgs), 20, f"Expected 20 SPDX packages, got {len(pkgs)}")
        self.assertEqual(
            len(comps), 19, f"Expected 19 CDX components, got {len(comps)}"
        )

    def test_kyron_crate_license_enrichment(self):
        """Rust crates in score_kyron SBOM must have license data from crates_metadata.json."""
        spdx, _ = self._run("kyron_input.json", cdxgen_fixture="kyron_cdxgen.cdx.json")
        pkgs = spdx["packages"]
        serde = next((p for p in pkgs if p["name"] == "serde"), None)
        self.assertIsNotNone(serde, "serde package must exist in kyron SBOM")
        self.assertNotEqual(
            serde["licenseConcluded"],
            "NOASSERTION",
            "serde must have a resolved license from crates_metadata.json",
        )
        self.assertIn("Apache-2.0", serde["licenseConcluded"])

    def test_orchestrator_crate_checksum_present(self):
        """Crates with known checksums must have checksums in the SPDX output."""
        spdx, _ = self._run(
            "orchestrator_input.json",
            cdxgen_fixture="orchestrator_cdxgen.cdx.json",
        )
        pkgs = spdx["packages"]
        crates_with_checksum = [
            p
            for p in pkgs
            if p.get("checksums") and p.get("SPDXID") != "SPDXRef-RootPackage"
        ]
        self.assertGreater(
            len(crates_with_checksum),
            50,
            f"Expected >50 crates with checksums, got {len(crates_with_checksum)}",
        )

    def test_lockfile_enriches_module_version(self):
        """MODULE.bazel.lock fixture must enrich boost.config with version from BCR URL."""
        spdx, _ = self._run("baselibs_input.json")
        pkgs = spdx["packages"]
        boost_config = next((p for p in pkgs if p["name"] == "boost.config"), None)
        self.assertIsNotNone(boost_config)
        self.assertNotEqual(
            boost_config["versionInfo"],
            "unknown",
            "boost.config version must be extracted from MODULE.bazel.lock",
        )

    def test_spdx_licenseref_declarations(self):
        """All LicenseRef-* identifiers used in SPDX packages must be declared."""
        for fixture in (
            "baselibs_input.json",
            "kyron_input.json",
            "orchestrator_input.json",
        ):
            with self.subTest(fixture=fixture):
                spdx, _ = self._run(fixture)
                licenseref_re = re.compile(r"LicenseRef-[A-Za-z0-9\-.]+")
                used: set[str] = set()
                for pkg in spdx["packages"]:
                    for field in ("licenseConcluded", "licenseDeclared"):
                        used.update(licenseref_re.findall(pkg.get(field, "")))
                if used:
                    declared = {
                        e["licenseId"]
                        for e in spdx.get("hasExtractedLicensingInfos", [])
                    }
                    self.assertEqual(
                        used,
                        used & declared,
                        f"Undeclared LicenseRef-* in {fixture}: {used - declared}",
                    )

    def test_cdx_bom_refs_are_unique(self):
        """All CycloneDX bom-ref values must be unique within each document."""
        for fixture, cdxgen in [
            ("baselibs_input.json", None),
            ("kyron_input.json", "kyron_cdxgen.cdx.json"),
            ("orchestrator_input.json", "orchestrator_cdxgen.cdx.json"),
        ]:
            with self.subTest(fixture=fixture):
                _, cdx = self._run(fixture, cdxgen_fixture=cdxgen)
                refs = [c["bom-ref"] for c in cdx["components"]]
                self.assertEqual(
                    len(refs), len(set(refs)), f"Duplicate bom-refs in {fixture}"
                )

    def test_all_spdx_ids_reference_valid_nodes(self):
        """Relationship element IDs must reference only packages defined in the document."""
        for fixture in (
            "baselibs_input.json",
            "kyron_input.json",
            "orchestrator_input.json",
        ):
            with self.subTest(fixture=fixture):
                spdx, _ = self._run(fixture)
                valid_ids = {p["SPDXID"] for p in spdx["packages"]} | {
                    "SPDXRef-DOCUMENT"
                }
                for rel in spdx["relationships"]:
                    self.assertIn(
                        rel["spdxElementId"],
                        valid_ids,
                        f"Dangling spdxElementId in {fixture}: {rel['spdxElementId']!r}",
                    )
                    self.assertIn(
                        rel["relatedSpdxElement"],
                        valid_ids,
                        f"Dangling relatedSpdxElement in {fixture}: {rel['relatedSpdxElement']!r}",
                    )

    def test_online_validator_accepts_all_sboms(self):
        """SPDX and CycloneDX outputs pass sbomgenerator.com/tools/validator.

        Posts each generated SBOM to https://sbomgenerator.com/tools/validator/validate
        and asserts that it is reported as valid with zero errors.

        Skipped automatically (per subtest) when the service is unreachable so
        that offline CI environments are not broken by network failures.
        """
        cases = [
            ("baselibs_input.json", None, "score_baselibs"),
            ("kyron_input.json", "kyron_cdxgen.cdx.json", "score_kyron"),
            (
                "orchestrator_input.json",
                "orchestrator_cdxgen.cdx.json",
                "score_orchestrator",
            ),
        ]
        for input_fixture, cdxgen_fixture, component_name in cases:
            spdx, cdx = self._run(input_fixture, cdxgen_fixture=cdxgen_fixture)
            for content, fmt in [
                (json.dumps(spdx), "spdx"),
                (json.dumps(cdx), "cyclonedx"),
            ]:
                with self.subTest(component=component_name, format=fmt):
                    result = self._validate_online(content, fmt)
                    if result is None:
                        self.skipTest(
                            "sbomgenerator.com is unreachable — skipping online validation"
                        )
                    self.assertTrue(
                        result.get("valid"),
                        f"{component_name} {fmt}: validator reports invalid — "
                        f"errors: {result.get('errors', [])}",
                    )
                    self.assertEqual(
                        result.get("errors", []),
                        [],
                        f"{component_name} {fmt}: unexpected errors from validator: "
                        f"{result.get('errors', [])}",
                    )
