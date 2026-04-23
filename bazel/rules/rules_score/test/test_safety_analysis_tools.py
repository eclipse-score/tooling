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

"""Tests for safety_analysis_tools."""

import json
import logging
import os
import sys
import tempfile
import unittest

from safety_analysis_tools import (
    LOBSTER_GENERATOR,
    LOBSTER_SCHEMA,
    LOBSTER_VERSION,
    _is_valid_trlc_fqn,
    _parse_quoted_args,
    create_lobster_output,
    extract_fta_items,
    main,
    preprocess_puml,
)

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

SAMPLE_FTA = """\
@startuml

!include fta_metamodel.puml

$TopEvent("Failure takes over", "Pkg.TopFailure")
$OrGate("OG1", "Pkg.TopFailure")
$BasicEvent("Bad luck", "Pkg.BadLuck", "OG1")
$IntermediateEvent("Angry", "IEF", "OG1")
$AndGate("AG2", "IEF")
$BasicEvent("No Cookies", "Pkg.NoCookies", "AG2")
$BasicEvent("No Coffee", "Pkg.NoCoffee", "AG2")

@enduml
"""

EMPTY_FTA = """\
@startuml
' No events here
@enduml
"""

SAMPLE_METAMODEL = """\
@startuml

' AND gate sprite
sprite $and <svg>placeholder</svg>

!procedure $TopEvent($name, $alias)
  rectangle "$name" as $alias
!endprocedure

!procedure $BasicEvent($name, $alias, $connection)
  "$name" as $alias
  $alias -u-> $connection
!endprocedure

@enduml
"""


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_file(content: str, directory: str, name: str) -> str:
    path = os.path.join(directory, name)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(content)
    return path


# ---------------------------------------------------------------------------
# _parse_quoted_args
# ---------------------------------------------------------------------------


class TestParseQuotedArgs(unittest.TestCase):
    def test_returns_none_when_proc_absent(self):
        self.assertIsNone(_parse_quoted_args("$OrGate()", "$TopEvent"))

    def test_extracts_two_args(self):
        self.assertEqual(
            _parse_quoted_args('$TopEvent("My name", "Pkg.Record")', "$TopEvent"),
            ["My name", "Pkg.Record"],
        )

    def test_extracts_three_args(self):
        self.assertEqual(
            _parse_quoted_args(
                '$BasicEvent("Bad luck", "Pkg.BadLuck", "OG1")', "$BasicEvent"
            ),
            ["Bad luck", "Pkg.BadLuck", "OG1"],
        )

    def test_returns_none_on_missing_open_paren(self):
        self.assertIsNone(_parse_quoted_args("$TopEvent", "$TopEvent"))

    def test_returns_none_on_no_quoted_args(self):
        self.assertIsNone(_parse_quoted_args("$TopEvent()", "$TopEvent"))

    def test_ignores_content_after_close_paren(self):
        line = '$TopEvent("a", "b") $BasicEvent("c", "d", "e")'
        self.assertEqual(_parse_quoted_args(line, "$TopEvent"), ["a", "b"])


# ---------------------------------------------------------------------------
# _is_valid_trlc_fqn
# ---------------------------------------------------------------------------


class TestIsValidTrlcFqn(unittest.TestCase):
    def test_valid(self):
        self.assertTrue(_is_valid_trlc_fqn("Pkg.Record"))
        self.assertTrue(_is_valid_trlc_fqn("SampleLib.FailureMode"))

    def test_no_dot(self):
        self.assertFalse(_is_valid_trlc_fqn("NoDotsHere"))

    def test_too_many_dots(self):
        self.assertFalse(_is_valid_trlc_fqn("A.B.C"))

    def test_empty_string(self):
        self.assertFalse(_is_valid_trlc_fqn(""))

    def test_dot_only(self):
        self.assertFalse(_is_valid_trlc_fqn("."))


# ---------------------------------------------------------------------------
# extract_fta_items
# ---------------------------------------------------------------------------


class TestExtractFtaItems(unittest.TestCase):
    def test_extracts_top_event(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            items = extract_fta_items(path)

            top_events = [i for i in items if i["kind"] == "TopEvent"]
            self.assertEqual(len(top_events), 1)
            self.assertEqual(top_events[0]["name"], "Pkg.TopFailure")
            self.assertEqual(top_events[0]["tag"], "fta Pkg.TopFailure")
            self.assertIn("req Pkg.TopFailure", top_events[0]["refs"])

    def test_extracts_basic_events(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            items = extract_fta_items(path)

            basic_aliases = {i["name"] for i in items if i["kind"] == "BasicEvent"}
            self.assertEqual(
                basic_aliases, {"Pkg.BadLuck", "Pkg.NoCookies", "Pkg.NoCoffee"}
            )

    def test_ignores_gates_and_intermediate_events(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            items = extract_fta_items(path)
            self.assertEqual({i["kind"] for i in items}, {"TopEvent", "BasicEvent"})

    def test_refs_point_to_alias(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            for item in extract_fta_items(path):
                self.assertIn(f"req {item['name']}", item["refs"])

    def test_location_contains_file_and_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            for item in extract_fta_items(path):
                self.assertEqual(item["location"]["kind"], "file")
                self.assertEqual(item["location"]["file"], path)
                self.assertGreater(item["location"]["line"], 0)

    def test_one_match_per_line(self):
        content = '$TopEvent("name", "A.B") $BasicEvent("x", "C.D", "conn")\n'
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(content, tmp, "fta.puml")
            items = extract_fta_items(path)
            self.assertEqual(len(items), 1)
            self.assertEqual(items[0]["kind"], "TopEvent")

    def test_empty_diagram_returns_empty_list(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(EMPTY_FTA, tmp, "fta.puml")
            self.assertEqual(extract_fta_items(path), [])

    def test_missing_file_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(OSError):
                extract_fta_items(os.path.join(tmp, "nonexistent.puml"))

    def test_invalid_alias_logs_warning(self):
        content = '$TopEvent("bad alias", "NoDotsHere")\n'
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(content, tmp, "fta.puml")
            with self.assertLogs(level="WARNING") as log:
                extract_fta_items(path)
            self.assertTrue(any("does not look like a valid" in m for m in log.output))

    def test_valid_alias_produces_no_fqn_warning(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            with self.assertLogs(level="WARNING") as log:
                logging.getLogger().warning(
                    "sentinel"
                )  # ensure assertLogs doesn't fail
                extract_fta_items(path)
            self.assertFalse(any("does not look like a valid" in m for m in log.output))


# ---------------------------------------------------------------------------
# preprocess_puml
# ---------------------------------------------------------------------------


class TestPreprocessPuml(unittest.TestCase):
    def _write_metamodel(self, directory: str) -> str:
        return _write_file(SAMPLE_METAMODEL, directory, "fta_metamodel.puml")

    def test_inlines_metamodel_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            meta = self._write_metamodel(tmp)
            src = _write_file(
                '@startuml\n!include fta_metamodel.puml\n$TopEvent("x", "A.B")\n@enduml\n',
                tmp,
                "fta.puml",
            )
            out = os.path.join(tmp, "out.puml")
            preprocess_puml(src, meta, out)

            with open(out) as fh:
                content = fh.read()
            self.assertNotIn("!include fta_metamodel.puml", content)
            self.assertIn("!procedure $TopEvent", content)
            self.assertIn("sprite $and", content)
            self.assertIn('$TopEvent("x", "A.B")', content)

    def test_metamodel_markers_are_stripped(self):
        with tempfile.TemporaryDirectory() as tmp:
            meta = self._write_metamodel(tmp)
            src = _write_file(
                "@startuml\n!include fta_metamodel.puml\n@enduml\n",
                tmp,
                "fta.puml",
            )
            out = os.path.join(tmp, "out.puml")
            preprocess_puml(src, meta, out)
            with open(out) as fh:
                self.assertNotIn("@startuml\n@startuml", fh.read())

    def test_diagram_without_include_is_unchanged(self):
        original = '@startuml\n$TopEvent("x", "A.B")\n@enduml\n'
        with tempfile.TemporaryDirectory() as tmp:
            meta = self._write_metamodel(tmp)
            src = _write_file(original, tmp, "fta.puml")
            out = os.path.join(tmp, "out.puml")
            preprocess_puml(src, meta, out)
            with open(out) as fh:
                self.assertEqual(fh.read(), original)


# ---------------------------------------------------------------------------
# create_lobster_output
# ---------------------------------------------------------------------------


class TestCreateLobsterOutput(unittest.TestCase):
    def test_envelope_fields(self):
        items = [{"tag": "fta A.B", "name": "A.B"}]
        output = create_lobster_output(items)
        self.assertEqual(output["generator"], LOBSTER_GENERATOR)
        self.assertEqual(output["schema"], LOBSTER_SCHEMA)
        self.assertEqual(output["version"], LOBSTER_VERSION)
        self.assertEqual(output["data"], items)

    def test_empty_items(self):
        self.assertEqual(create_lobster_output([])["data"], [])


# ---------------------------------------------------------------------------
# main (CLI integration)
# ---------------------------------------------------------------------------


class TestMain(unittest.TestCase):
    """Integration tests for the flat CLI: --metamodel --output-dir --lobster inputs..."""

    def _run_main(self, tmp: str, *puml_paths: str) -> dict:
        """Invoke main() and return the parsed lobster JSON dict."""
        meta = _write_file(SAMPLE_METAMODEL, tmp, "fta_metamodel.puml")
        out_dir = os.path.join(tmp, "preprocessed")
        lobster_path = os.path.join(tmp, "out.lobster")

        saved = sys.argv
        try:
            sys.argv = [
                "safety_analysis_tools",
                "--metamodel",
                meta,
                "--output-dir",
                out_dir,
                "--lobster",
                lobster_path,
                *puml_paths,
            ]
            main()
        finally:
            sys.argv = saved

        with open(lobster_path, encoding="utf-8") as fh:
            return json.load(fh)

    def test_produces_valid_lobster_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            puml = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            data = self._run_main(tmp, puml)
            self.assertEqual(data["schema"], LOBSTER_SCHEMA)
            self.assertEqual(len(data["data"]), 4)  # 1 TopEvent + 3 BasicEvents

    def test_produces_preprocessed_puml_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            puml = _write_file(SAMPLE_FTA, tmp, "fta.puml")
            self._run_main(tmp, puml)

            preprocessed = os.path.join(tmp, "preprocessed", "fta.puml")
            self.assertTrue(os.path.exists(preprocessed))
            with open(preprocessed) as fh:
                content = fh.read()
            self.assertNotIn("!include fta_metamodel.puml", content)
            self.assertIn("!procedure $TopEvent", content)

    def test_multiple_inputs_aggregate_lobster_items(self):
        with tempfile.TemporaryDirectory() as tmp:
            p1 = _write_file(SAMPLE_FTA, tmp, "a.puml")
            p2 = _write_file(SAMPLE_FTA, tmp, "b.puml")
            data = self._run_main(tmp, p1, p2)
            self.assertEqual(len(data["data"]), 8)  # 4 items × 2 files


if __name__ == "__main__":
    unittest.main()
