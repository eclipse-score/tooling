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
"""Tests for aou_forwarding_to_trlc."""

import tempfile
import unittest

import yaml
from trlc.ast import Record_Object
from trlc.errors import Message_Handler
from trlc.trlc import Source_Manager

from aou_forwarding_to_trlc import (
    _match_forwarded_entries,
    _read_package_name,
    _serialize_aou,
    build_forwarded_files,
    load_aou_records,
    parse_forwarding_yaml,
)

_MINIMAL_SPEC = """
package ScoreReq

enum Asil {
    QM
    B
    D
}

enum Status {
    valid
    invalid
}

abstract type Requirement {
    description Markup_String
    version     Integer
    note        optional String
    status      Status
    freeze status = Status.valid
}

abstract type RequirementSafety extends Requirement {
    safety Asil
}

type ControlMeasure extends RequirementSafety {
    mitigates optional String
}

type AoU extends ControlMeasure {
}
"""


def _write_tmp(content: str, suffix: str) -> str:
    f = tempfile.NamedTemporaryFile(mode="w", suffix=suffix, delete=False)
    f.write(content)
    f.close()
    return f.name


def _write_yaml(data: dict) -> str:
    f = tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False)
    yaml.dump(data, f)
    f.close()
    return f.name


class TestParseForwardingYaml(unittest.TestCase):
    """Tests for parse_forwarding_yaml (mirrors aou_forwarding_to_lobster's)."""

    def test_valid_yaml(self) -> None:
        path = _write_yaml({"forwarded_aous": [{"aou_id": "Pkg.AoU1", "justification": "reason"}]})
        result = parse_forwarding_yaml(path)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["aou_id"], "Pkg.AoU1")
        self.assertEqual(result[0]["justification"], "reason")

    def test_missing_forwarded_aous_key(self) -> None:
        path = _write_yaml({"wrong_key": []})
        with self.assertRaises(SystemExit):
            parse_forwarding_yaml(path)

    def test_missing_aou_id(self) -> None:
        path = _write_yaml({"forwarded_aous": [{"justification": "r"}]})
        with self.assertRaises(SystemExit):
            parse_forwarding_yaml(path)

    def test_missing_justification(self) -> None:
        path = _write_yaml({"forwarded_aous": [{"aou_id": "Foo.Bar"}]})
        with self.assertRaises(SystemExit):
            parse_forwarding_yaml(path)


class TestLoadAouRecords(unittest.TestCase):
    """Tests for load_aou_records."""

    def setUp(self) -> None:
        self.spec_path = _write_tmp(_MINIMAL_SPEC, ".rsl")

    def test_loads_records_from_single_file(self) -> None:
        trlc_path = _write_tmp(
            """
            package Pkg
            import ScoreReq

            ScoreReq.AoU Sample {
                description = "desc"
                version = 1
                safety = ScoreReq.Asil.B
            }
            """,
            ".trlc",
        )
        records = load_aou_records([self.spec_path], [trlc_path])
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0].name, "Sample")

    def test_loads_records_from_multiple_files(self) -> None:
        path1 = _write_tmp(
            'package A\nimport ScoreReq\nScoreReq.AoU One { description = "d" version = 1 safety = ScoreReq.Asil.B }\n',
            ".trlc",
        )
        path2 = _write_tmp(
            'package B\nimport ScoreReq\nScoreReq.AoU Two { description = "d" version = 1 safety = ScoreReq.Asil.D }\n',
            ".trlc",
        )
        records = load_aou_records([self.spec_path], [path1, path2])
        self.assertEqual({r.name for r in records}, {"One", "Two"})

    def test_invalid_trlc_raises(self) -> None:
        bad_path = _write_tmp("package Pkg\nthis is not valid trlc\n", ".trlc")
        with self.assertRaises(SystemExit):
            load_aou_records([self.spec_path], [bad_path])


class TestMatchForwardedEntries(unittest.TestCase):
    """Tests for _match_forwarded_entries."""

    def setUp(self) -> None:
        spec_path = _write_tmp(_MINIMAL_SPEC, ".rsl")
        trlc_path = _write_tmp(
            """
            package Pkg
            import ScoreReq

            ScoreReq.AoU AoU1 {
                description = "desc"
                version = 1
                safety = ScoreReq.Asil.B
            }
            """,
            ".trlc",
        )
        self.records: list[Record_Object] = load_aou_records([spec_path], [trlc_path])

    def test_matches_by_base_id(self) -> None:
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        matched = _match_forwarded_entries(entries, self.records)
        self.assertEqual(len(matched), 1)
        self.assertEqual(matched[0][1].name, "AoU1")

    def test_matches_by_versioned_id(self) -> None:
        entries = [{"aou_id": "Pkg.AoU1@1", "justification": "reason"}]
        matched = _match_forwarded_entries(entries, self.records)
        self.assertEqual(len(matched), 1)

    def test_nonexistent_aou_id_raises(self) -> None:
        entries = [{"aou_id": "Pkg.DoesNotExist", "justification": "reason"}]
        with self.assertRaises(SystemExit):
            _match_forwarded_entries(entries, self.records)


class TestSerializeAou(unittest.TestCase):
    """Tests for _serialize_aou, including a round-trip through the parser."""

    def setUp(self) -> None:
        self.spec_path = _write_tmp(_MINIMAL_SPEC, ".rsl")

    def _parse_single(self, trlc_text: str) -> Record_Object:
        trlc_path = _write_tmp(trlc_text, ".trlc")
        records = load_aou_records([self.spec_path], [trlc_path])
        self.assertEqual(len(records), 1)
        return records[0]

    def test_round_trip_minimal_fields(self) -> None:
        record = self._parse_single(
            'package Pkg\nimport ScoreReq\nScoreReq.AoU AoU1 { description = "desc" version = 1 safety = ScoreReq.Asil.B }\n'
        )
        rendered = _serialize_aou(record)
        reparsed = self._parse_single(f"package Pkg\nimport ScoreReq\n\n{rendered}\n")
        self.assertEqual(reparsed.to_python_dict()["description"], "desc")
        self.assertEqual(reparsed.to_python_dict()["safety"], "B")

    def test_round_trip_optional_fields_present(self) -> None:
        record = self._parse_single(
            "package Pkg\nimport ScoreReq\n"
            'ScoreReq.AoU AoU1 { description = "desc" version = 2 note = "a note" '
            'safety = ScoreReq.Asil.D mitigates = "some hazard" }\n'
        )
        rendered = _serialize_aou(record)
        reparsed = self._parse_single(f"package Pkg\nimport ScoreReq\n\n{rendered}\n")
        fields = reparsed.to_python_dict()
        self.assertEqual(fields["note"], "a note")
        self.assertEqual(fields["mitigates"], "some hazard")
        self.assertEqual(fields["version"], 2)

    def test_round_trip_preserves_special_characters(self) -> None:
        record = self._parse_single(
            'package Pkg\nimport ScoreReq\nScoreReq.AoU AoU1 { description = "has \\"quotes\\" and {braces}" version = 1 safety = ScoreReq.Asil.B }\n'
        )
        rendered = _serialize_aou(record)
        reparsed = self._parse_single(f"package Pkg\nimport ScoreReq\n\n{rendered}\n")
        self.assertEqual(reparsed.to_python_dict()["description"], 'has "quotes" and {braces}')

    def test_omits_status_field(self) -> None:
        record = self._parse_single(
            'package Pkg\nimport ScoreReq\nScoreReq.AoU AoU1 { description = "desc" version = 1 safety = ScoreReq.Asil.B }\n'
        )
        rendered = _serialize_aou(record)
        self.assertNotIn("status", rendered)


class TestBuildForwardedFiles(unittest.TestCase):
    """Tests for build_forwarded_files."""

    def setUp(self) -> None:
        self.spec_path = _write_tmp(_MINIMAL_SPEC, ".rsl")

    def test_single_file_with_match(self) -> None:
        trlc_path = _write_tmp(
            'package Pkg\nimport ScoreReq\nScoreReq.AoU AoU1 { description = "desc" version = 1 safety = ScoreReq.Asil.B }\n',
            ".trlc",
        )
        records = load_aou_records([self.spec_path], [trlc_path])
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        output = build_forwarded_files(entries, records, [trlc_path])
        self.assertIn("package Pkg", output[trlc_path])
        self.assertIn("AoU1", output[trlc_path])

    def test_file_with_no_match_is_empty_but_valid(self) -> None:
        trlc_path = _write_tmp(
            'package Pkg\nimport ScoreReq\nScoreReq.AoU AoU1 { description = "desc" version = 1 safety = ScoreReq.Asil.B }\n',
            ".trlc",
        )
        records = load_aou_records([self.spec_path], [trlc_path])
        output = build_forwarded_files([], records, [trlc_path])
        self.assertEqual(output[trlc_path], "package Pkg\n")
        # An empty-but-valid package file must itself still parse.
        reparsed_path = _write_tmp(output[trlc_path], ".trlc")
        mh = Message_Handler()
        sm = Source_Manager(mh)
        sm.register_file(self.spec_path)
        sm.register_file(reparsed_path)
        self.assertIsNotNone(sm.process())

    def test_multiple_files_positionally_paired(self) -> None:
        path1 = _write_tmp(
            'package A\nimport ScoreReq\nScoreReq.AoU One { description = "d" version = 1 safety = ScoreReq.Asil.B }\n',
            ".trlc",
        )
        path2 = _write_tmp(
            'package B\nimport ScoreReq\nScoreReq.AoU Two { description = "d" version = 1 safety = ScoreReq.Asil.D }\n',
            ".trlc",
        )
        records = load_aou_records([self.spec_path], [path1, path2])
        entries = [{"aou_id": "A.One", "justification": "reason"}]
        output = build_forwarded_files(entries, records, [path1, path2])
        self.assertIn("One", output[path1])
        self.assertEqual(output[path2], "package B\n")

    def test_file_with_no_aou_records_falls_back_to_package_from_text(self) -> None:
        # A file with zero AoU records is legitimate: e.g. an upstream chain-
        # forwarding stage's own empty-but-valid `package X` placeholder,
        # re-forwarded through another level. Its package name must be read
        # from its text rather than off a (nonexistent) Record_Object.
        empty_trlc_path = _write_tmp("package Empty\n", ".trlc")
        real_trlc_path = _write_tmp(
            'package Pkg\nimport ScoreReq\nScoreReq.AoU AoU1 { description = "desc" version = 1 safety = ScoreReq.Asil.B }\n',
            ".trlc",
        )
        records = load_aou_records([self.spec_path], [empty_trlc_path, real_trlc_path])
        output = build_forwarded_files([], records, [empty_trlc_path, real_trlc_path])
        self.assertEqual(output[empty_trlc_path], "package Empty\n")
        self.assertEqual(output[real_trlc_path], "package Pkg\n")


class TestReadPackageName(unittest.TestCase):
    """Tests for the _read_package_name text-fallback helper."""

    def test_finds_package_declaration(self) -> None:
        path = _write_tmp("package Foo\n", ".trlc")
        self.assertEqual(_read_package_name(path), "Foo")

    def test_raises_when_no_package_declaration_present(self) -> None:
        path = _write_tmp("this file has no package declaration\n", ".trlc")
        with self.assertRaises(SystemExit):
            _read_package_name(path)


if __name__ == "__main__":
    unittest.main()
