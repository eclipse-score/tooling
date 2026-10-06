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
"""Tests for the report-only metamodel drift detector."""

from __future__ import annotations

import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import Any

import yaml

from metamodel_drift import analyze, main


def _fixture(name: str) -> Path:
    workspace = os.environ.get("TEST_WORKSPACE", "rules_score_test")
    return Path(os.environ["TEST_SRCDIR"]) / workspace / "fixtures" / "metamodel_drift" / name


def _config(name: str) -> dict[str, Any]:
    with _fixture(name).open(encoding="utf-8") as stream:
        value = yaml.safe_load(stream)
    assert isinstance(value, dict)
    return value


class MetamodelDriftTest(unittest.TestCase):
    def _analyze(
        self,
        mapping: dict[str, Any] | None = None,
        metamodel: dict[str, Any] | None = None,
        copy_paths: list[tuple[str, str]] | None = None,
    ) -> dict[str, Any]:
        with tempfile.TemporaryDirectory() as directory:
            mapping_path = Path(directory) / "mapping.yaml"
            metamodel_path = Path(directory) / "metamodel.yaml"
            with mapping_path.open("w", encoding="utf-8") as stream:
                yaml.safe_dump(mapping if mapping is not None else _config("mapping.yaml"), stream)
            with metamodel_path.open("w", encoding="utf-8") as stream:
                yaml.safe_dump(
                    metamodel if metamodel is not None else _config("metamodel.yaml"),
                    stream,
                )
            report, _ = analyze(
                [str(_fixture("tiny.rsl"))],
                str(metamodel_path),
                str(mapping_path),
                compare_rsl=copy_paths,
            )
            primary_errors = self._findings(report, "PRIMARY_RSL_PARSE_ERROR")
            if primary_errors:
                raise AssertionError(primary_errors[0]["message"])
            return report

    @staticmethod
    def _findings(report: dict[str, Any], identifier: str) -> list[dict[str, Any]]:
        return [finding for finding in report["findings"] if finding["id"] == identifier]

    def test_mapping_and_metamodel_findings_include_positive_and_negative_cases(self) -> None:
        report = self._analyze()
        missing_options = self._findings(report, "MANDATORY_OPTION_MISSING")
        missing_links = self._findings(report, "MANDATORY_LINK_MISSING")
        self.assertIn("title", {finding["field"] for finding in missing_options})
        self.assertIn("must", {finding["field"] for finding in missing_links})
        self.assertTrue(
            any(
                finding["trlc_type"] == "Feature" and finding["field"] == "enabled"
                for finding in self._findings(report, "OPTIONAL_FIELD_FOR_MANDATORY_OPTION")
            )
        )

        mapping = _config("mapping.yaml")
        mapping["types"]["Feature"]["options"]["extra"] = "title"
        mapping["types"]["Feature"]["links"]["refs"] = "must"
        positive_report = self._analyze(mapping=mapping)
        self.assertNotIn(
            "title",
            {finding["field"] for finding in self._findings(positive_report, "MANDATORY_OPTION_MISSING")},
        )
        self.assertNotIn(
            "must",
            {finding["field"] for finding in self._findings(positive_report, "MANDATORY_LINK_MISSING")},
        )

    def test_enums_and_boolean_values_are_compared_with_enumerable_patterns(self) -> None:
        report = self._analyze()
        rejected = self._findings(report, "ENUM_VALUE_REJECTED")
        self.assertTrue(any("Blue" in finding["message"] for finding in rejected))
        unrepresentable = self._findings(report, "NEED_VALUE_UNREPRESENTABLE")
        self.assertTrue(
            any(finding["field"] == "color" and "green" in finding["message"] for finding in unrepresentable)
        )

    def test_union_tuple_links_report_rejected_and_unmapped_targets(self) -> None:
        report = self._analyze()
        rejected = self._findings(report, "LINK_TARGET_REJECTED")
        self.assertTrue(any(finding["field"] == "refs" and "TargetB" in finding["message"] for finding in rejected))
        self.assertFalse(self._findings(report, "LINK_TARGET_UNMAPPED"))

        mapping = _config("mapping.yaml")
        mapping["types"].pop("TargetB")
        mapping["unmapped_types"]["TargetB"] = "No need type is defined."
        unmapped_report = self._analyze(mapping=mapping)
        unmapped = self._findings(unmapped_report, "LINK_TARGET_UNMAPPED")
        self.assertTrue(any(finding["field"] == "refs" and "TargetB" in finding["message"] for finding in unmapped))

    def test_inherited_unmapped_fields_and_unmapped_record_types_are_reported(self) -> None:
        report = self._analyze()
        unmapped_fields = self._findings(report, "TRLC_FIELD_UNMAPPED")
        feature_fields = {finding["field"] for finding in unmapped_fields if finding["trlc_type"] == "Feature"}
        self.assertIn("inherited", feature_fields)
        self.assertIn("extra", feature_fields)
        self.assertTrue(
            any(finding["trlc_type"] == "Unmapped" for finding in self._findings(report, "UNMAPPED_TRLC_TYPE"))
        )

    def test_bad_mapping_keys_and_missing_types_are_reported(self) -> None:
        baseline = self._analyze()
        self.assertFalse(self._findings(baseline, "MAPPED_FIELD_NOT_FOUND"))
        self.assertFalse(self._findings(baseline, "MAPPED_NEED_TYPE_NOT_FOUND"))

        mapping = _config("mapping.yaml")
        mapping["types"]["Feature"]["options"]["ghost"] = "title"
        mapping["types"]["MissingType"] = {"need_type": "feature"}
        mapping["types"]["TargetB"]["need_type"] = "missing_need"
        report = self._analyze(mapping=mapping)
        self.assertTrue(self._findings(report, "MAPPED_FIELD_NOT_FOUND"))
        self.assertTrue(self._findings(report, "MAPPED_TRLC_TYPE_NOT_FOUND"))
        self.assertTrue(self._findings(report, "MAPPED_NEED_TYPE_NOT_FOUND"))

        mapping = _config("mapping.yaml")
        mapping["types"]["Feature"]["options"]["color"] = "not_an_option"
        mapping["types"]["Feature"]["links"]["refs"] = "not_a_link"
        mismatch_report = self._analyze(mapping=mapping)
        self.assertTrue(self._findings(mismatch_report, "MAPPED_OPTION_NOT_IN_NEED"))
        self.assertTrue(self._findings(mismatch_report, "MAPPED_LINK_NOT_IN_NEED"))
        self.assertFalse(self._findings(self._analyze(), "MAPPED_OPTION_NOT_IN_NEED"))
        self.assertFalse(self._findings(self._analyze(), "MAPPED_LINK_NOT_IN_NEED"))

    def test_base_options_are_merged_into_every_need_type(self) -> None:
        report = self._analyze()
        missing_base_options = [
            finding for finding in self._findings(report, "MANDATORY_OPTION_MISSING") if finding["field"] == "version"
        ]
        self.assertEqual([], missing_base_options)

    def test_option_enum_values_are_accepted_when_metamodel_enumerates_them(self) -> None:
        metamodel = _config("metamodel.yaml")
        metamodel["needs_types"]["feature"]["mandatory_options"]["color"] = "^(red|blue)$"
        report = self._analyze(metamodel=metamodel)
        self.assertFalse(self._findings(report, "ENUM_VALUE_REJECTED"))
        self.assertFalse(self._findings(report, "NEED_VALUE_UNREPRESENTABLE"))

    def test_link_patterns_unreachable_targets_and_multiplicity_are_reported(self) -> None:
        report = self._analyze()
        self.assertTrue(self._findings(report, "LINK_TARGET_UNREACHABLE"))
        self.assertTrue(self._findings(report, "LINK_MULTIPLICITY"))

        mapping = _config("mapping.yaml")
        mapping["types"]["Feature"]["links"]["optional_refs"] = "related"
        optional_link_report = self._analyze(mapping=mapping)
        self.assertFalse(
            any(
                finding["trlc_type"] == "Feature" and finding["field"] == "optional_refs"
                for finding in self._findings(optional_link_report, "OPTIONAL_FIELD_FOR_MANDATORY_OPTION")
            )
        )
        mapping["types"]["Feature"]["links"]["optional_refs"] = "must"
        mandatory_link_report = self._analyze(mapping=mapping)
        self.assertTrue(
            any(
                finding["trlc_type"] == "Feature" and finding["field"] == "optional_refs"
                for finding in self._findings(mandatory_link_report, "OPTIONAL_FIELD_FOR_MANDATORY_OPTION")
            )
        )

        metamodel = _config("metamodel.yaml")
        metamodel["needs_types"]["feature"]["optional_links"]["related"] = "target_.*"
        pattern_report = self._analyze(metamodel=metamodel)
        self.assertTrue(self._findings(pattern_report, "LINK_TARGET_PATTERN"))
        self.assertFalse(self._findings(pattern_report, "LINK_TARGET_REJECTED"))
        self.assertFalse(self._findings(pattern_report, "LINK_TARGET_UNREACHABLE"))

    def test_copy_drift_covers_types_fields_parent_bounds_optionality_and_enum_literals(self) -> None:
        primary_text = _fixture("tiny.rsl").read_text(encoding="utf-8")
        copy_text = (
            primary_text.replace("    Blue\n", "    Green\n")
            .replace("type Feature extends Common", "type Feature extends OtherCommon")
            .replace("refs TargetRef[1 .. *]", "refs TargetRef[0 .. 3]")
            .replace('extra "Extra value." String', 'extra "Extra value." Integer')
            .replace('maybe "Optional text." optional String', 'maybe "Optional text." String')
            .replace('    obsolete "Generated field." String\n', "")
            .replace(
                '    maybe "Optional text." String\n}',
                '    maybe "Optional text." String\n    added "Added field." String\n}',
            )
            .replace("type RemovedThing extends Common {\n}\n", "")
        )
        copy_text += "\ntype AddedThing extends Common {\n}\n"
        with tempfile.TemporaryDirectory() as directory:
            copy_path = Path(directory) / "copy.rsl"
            copy_path.write_text(copy_text, encoding="utf-8")
            report = self._analyze(copy_paths=[("vendor", str(copy_path))])

        identifiers = {finding["id"] for finding in self._findings_for_section(report, "copy:vendor")}
        self.assertTrue(
            {
                "RSL_COPY_TYPE_ADDED",
                "RSL_COPY_TYPE_REMOVED",
                "RSL_COPY_FIELD_ADDED",
                "RSL_COPY_FIELD_REMOVED",
                "RSL_COPY_FIELD_CHANGED",
                "RSL_COPY_ENUM_LITERAL_ADDED",
                "RSL_COPY_ENUM_LITERAL_REMOVED",
            }.issubset(identifiers)
        )

    def test_copy_parse_error_is_reported_and_does_not_stop_other_copies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            bad_path = Path(directory) / "broken.rsl"
            good_path = Path(directory) / "same.rsl"
            bad_path.write_text("package ScoreReq\ntype Broken extends {\n", encoding="utf-8")
            good_path.write_text(_fixture("tiny.rsl").read_text(encoding="utf-8"), encoding="utf-8")
            report = self._analyze(copy_paths=[("broken", str(bad_path)), ("same", str(good_path))])
        findings = self._findings(report, "RSL_COPY_PARSE_ERROR")
        self.assertEqual(1, len(findings))
        self.assertIn("broken", findings[0]["section"])
        self.assertFalse(self._findings_for_section(report, "copy:same"))

    def test_mapping_all_fields_removes_unmapped_warnings(self) -> None:
        mapping = _config("mapping.yaml")
        mapping["types"]["Feature"]["options"]["extra"] = "title"
        mapping["types"]["Feature"]["links"]["refs"] = "must"
        mapping["types"]["Feature"]["links"]["optional_refs"] = "related"
        mapping["types"]["Feature"]["ignored"].update(
            {
                "inherited": "Shared model field.",
                "maybe": "Intentionally optional.",
            }
        )
        mapping["unmapped_types"]["Unmapped"] = "Not exported in this test."
        mapping["unmapped_types"]["TargetB"] = "Temporarily not mapped."
        mapping["types"].pop("TargetB")
        report = self._analyze(mapping=mapping)
        self.assertFalse(self._findings(report, "TRLC_FIELD_UNMAPPED"))
        self.assertFalse(self._findings(report, "UNMAPPED_TRLC_TYPE"))

    def test_fail_on_threshold_and_json_report(self) -> None:
        mapping = _config("mapping.yaml")
        metamodel = _config("metamodel.yaml")
        mapping["types"]["Feature"]["options"]["extra"] = "title"
        mapping["types"]["Feature"]["links"]["refs"] = "must"
        mapping["types"]["Feature"]["links"]["optional_refs"] = "related"
        mapping["types"]["Feature"]["ignored"].update(
            {
                "inherited": "Shared model field.",
                "maybe": "Intentionally optional.",
            }
        )
        mapping["unmapped_types"]["Unmapped"] = "Not exported in this test."
        metamodel["needs_types"]["feature"]["mandatory_options"]["color"] = "^(red|blue)$"
        metamodel["needs_types"]["feature"]["mandatory_links"]["must"] = "target_a, target_b"
        metamodel["needs_types"]["feature"]["optional_links"]["related"] = "target_a, target_b, target_c"

        with tempfile.TemporaryDirectory() as directory:
            mapping_path = Path(directory) / "mapping.yaml"
            metamodel_path = Path(directory) / "metamodel.yaml"
            report_path = Path(directory) / "report.json"
            mapping_path.write_text(yaml.safe_dump(mapping), encoding="utf-8")
            metamodel_path.write_text(yaml.safe_dump(metamodel), encoding="utf-8")
            common = [
                "--rsl",
                str(_fixture("tiny.rsl")),
                "--metamodel",
                str(metamodel_path),
                "--mapping",
                str(mapping_path),
                "--report-json",
                str(report_path),
            ]
            results = []
            for fail_on in ("never", "warning", "error"):
                stdout = io.StringIO()
                with contextlib.redirect_stdout(stdout):
                    results.append(main(common + ["--fail-on", fail_on]))
                self.assertIn("# Metamodel Drift Report", stdout.getvalue())
            self.assertEqual([0, 1, 0], results)
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertIsInstance(report["findings"], list)
            self.assertEqual(1, report["summary"]["warning"])

    @staticmethod
    def _findings_for_section(report: dict[str, Any], section: str) -> list[dict[str, Any]]:
        return [finding for finding in report["findings"] if finding["section"] == section]


if __name__ == "__main__":
    unittest.main()
