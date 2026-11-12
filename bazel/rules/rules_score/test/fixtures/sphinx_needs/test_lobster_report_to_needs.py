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
"""Tests for lobster_report_to_needs."""

import json
import os
import tempfile
import unittest

from lobster_report_to_needs import ReportError, convert, main, short_hash


def _item(name, status="ok", text=None, ref_up=None):
    return {
        "tag": f"gtest {name}",
        "location": {"kind": "file", "file": "score/bits/bits_test.cpp", "line": 12, "column": None},
        "name": name,
        "ref_up": ref_up or [],
        "framework": "GoogleTest",
        "kind": "test",
        "text": text,
        "status": status,
    }


REPORT = {
    "schema": "lobster-report",
    "version": 2,
    "levels": [
        {
            "name": "Component Requirements",
            "kind": "requirements",
            "items": [{"tag": "req comp_req__bits__set@1", "name": "comp_req__bits__set", "kind": "CompReq"}],
        },
        {
            "name": "Unit Test",
            "kind": "activity",
            "items": [
                _item(
                    "SetBit:WithUInt8",
                    text=":Fullyverifies: comp_req__bits__set\n:Description: Sets a bit.\n"
                    ":Testtype: requirements-based\n:Derivationtechnique: boundary-values",
                    ref_up=["req comp_req__bits__set"],
                ),
                _item("SetBit:Broken", status="fail", ref_up=["req comp_req__bits__set@1", "req comp_req__bits__get"]),
                _item("SetBit:Disabled", status="not run"),
            ],
        },
    ],
}


def _needs(report=REPORT):
    return convert(report, "bits")["versions"]["1.0"]["needs"]


class ConvertTest(unittest.TestCase):
    def test_only_activities_become_testcase_needs(self):
        needs = _needs()
        self.assertEqual(len(needs), 3)
        self.assertEqual({n["type"] for n in needs.values()}, {"testcase"})

    def test_fault_tree_root_causes_are_no_testcases(self):
        root_causes = {"name": "Root Causes", "kind": "activity", "items": [_item("Fta.Crash")]}
        report = dict(REPORT, levels=REPORT["levels"] + [root_causes])
        self.assertEqual(len(_needs(report)), 3)

    def test_testcase_uses_the_docs_as_code_test_xml_scheme(self):
        name = "SetBit__WithUInt8"
        need = _needs()[f"testcase__{name}_{short_hash('score/bits/bits_test.cpp' + name)}"]
        self.assertEqual(need["name"], name)
        self.assertEqual(need["file"], "score/bits/bits_test.cpp")
        self.assertEqual(need["line"], "12")
        self.assertEqual(need["result"], "passed")
        self.assertEqual(need["fully_verifies"], ["comp_req__bits__set"])
        self.assertEqual(need["partially_verifies"], [])
        self.assertEqual(need["test_type"], "requirements-based")
        self.assertEqual(need["derivation_technique"], "boundary-values")

    def test_short_hash_matches_docs_as_code(self):
        # Value of score_source_code_linker.xml_parser.short_hash for the same input.
        self.assertEqual(
            short_hash("score/bitmanipulation/bit_manipulation_test.cpp" + "Byte__CanBeConstructedFromTwoBigHalfBytes"),
            "dmiiu",
        )

    def test_lobster_tracing_without_verifies_property_becomes_partially_verifies(self):
        need = next(n for n in _needs().values() if n["name"] == "SetBit__Broken")
        self.assertEqual(need["result"], "failed")
        self.assertEqual(need["partially_verifies"], ["comp_req__bits__set", "comp_req__bits__get"])

    def test_not_run_is_skipped(self):
        need = next(n for n in _needs().values() if n["name"] == "SetBit__Disabled")
        self.assertEqual(need["result"], "skipped")

    def test_other_schema_is_an_error(self):
        with self.assertRaises(ReportError):
            convert({"schema": "lobster-act-trace"}, "bits")


class MainTest(unittest.TestCase):
    def test_writes_needs_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            report = os.path.join(tmp, "report.json")
            out = os.path.join(tmp, "needs.json")
            with open(report, "w", encoding="utf-8") as f:
                json.dump(REPORT, f)
            self.assertEqual(main([report, "--out", out, "--project", "bits"]), 0)
            with open(out, encoding="utf-8") as f:
                data = json.load(f)
            self.assertEqual(data["current_version"], "1.0")
            self.assertEqual(data["project"], "bits")
            self.assertEqual(data["versions"]["1.0"]["needs_amount"], 3)

    def test_without_report_writes_empty_needs_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "needs.json")
            self.assertEqual(main(["--out", out]), 0)
            with open(out, encoding="utf-8") as f:
                self.assertEqual(json.load(f)["versions"]["1.0"]["needs"], {})


if __name__ == "__main__":
    unittest.main()
