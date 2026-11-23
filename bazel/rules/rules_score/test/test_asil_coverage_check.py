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

import io
import os
import sys
import tempfile
import unittest
from unittest import mock

import trlc_check_runner

# Bazel passes the requirements model files as arguments.
_RSL = next(arg for arg in sys.argv[1:] if arg.endswith(".rsl"))
sys.argv = sys.argv[:1]

# The plugins dependable_element passes to the runner.
_CHECKS = ("trlc_checks.asil_coverage_check", "trlc_checks.version_pin_check")


def _record(kind, name, safety, derived_from=()):
    lines = [
        "ScoreReq.%s %s {" % (kind, name),
        '    description = "Fixture."',
        "    safety = ScoreReq.Asil.%s" % safety,
        "    version = 1",
    ]
    if kind == "AssumedSystemReq":
        lines.append('    rationale = "Fixture."')
    if derived_from:
        lines.append("    derived_from = [%s]" % ", ".join(d + "@1" for d in derived_from))
    lines.append("}")
    return "\n".join(lines)


def _asr(name, safety):
    return _record("AssumedSystemReq", name, safety)


def _aou(name, safety):
    return _record("AoU", name, safety)


def _feat(name, safety, *parents):
    return _record("FeatReq", name, safety, parents)


def _comp(name, safety, *parents):
    return _record("CompReq", name, safety, parents)


class TestAsilCoverageCheck(unittest.TestCase):
    """Tests for the asil_coverage_check plugin run through trlc_check_runner."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)

    def _write(self, *records):
        path = os.path.join(self._tmp.name, "reqs.trlc")
        with open(path, "w", encoding="utf-8") as fh:
            fh.write("package Fixture\n\nimport ScoreReq\n\n")
            fh.write("\n\n".join(records) + "\n")
        return path

    def _main(self, *records, extra=(), checks=_CHECKS):
        log = os.path.join(self._tmp.name, "out.log")
        argv = ["--no-lint", "--brief", "--log", log, *extra]
        for check in checks:
            argv += ["--check", check]
        argv += [_RSL, self._write(*records)]
        with mock.patch.object(sys, "stderr", io.StringIO()):
            exit_code = trlc_check_runner.main(argv)
        with open(log, encoding="utf-8") as fh:
            return exit_code, fh.read()

    def _run(self, *records):
        exit_code, report = self._main(*records)
        return exit_code == 0, report

    def test_child_with_same_asil_passes(self):
        ok, _ = self._run(_asr("Asr", "B"), _feat("Feat", "B", "Asr"))
        self.assertTrue(ok)

    def test_parent_without_children_is_ignored(self):
        ok, _ = self._run(_asr("Asr", "D"))
        self.assertTrue(ok)

    def test_only_higher_asil_children_fail(self):
        ok, report = self._run(_asr("Asr", "B"), _feat("Feat", "D", "Asr"))
        self.assertFalse(ok)
        self.assertIn("Fixture.Asr has ASIL B", report)
        self.assertIn("Fixture.Feat (D)", report)

    def test_one_matching_child_among_several_passes(self):
        ok, _ = self._run(
            _asr("Asr", "B"),
            _feat("FeatB", "B", "Asr"),
            _feat("FeatD", "D", "Asr"),
        )
        self.assertTrue(ok)

    def test_feature_parent_checked_against_component_children(self):
        ok, report = self._run(
            _asr("Asr", "B"),
            _feat("Feat", "B", "Asr"),
            _comp("Comp", "D", "Feat"),
        )
        self.assertFalse(ok)
        self.assertIn("Fixture.Feat has ASIL B", report)
        self.assertNotIn("Fixture.Asr has", report)

    def test_component_directly_derived_from_asr(self):
        ok, report = self._run(_asr("Asr", "B"), _comp("Comp", "D", "Asr"))
        self.assertFalse(ok)
        self.assertIn("Fixture.Asr has ASIL B", report)

    def test_child_with_two_parents_covers_only_matching_one(self):
        ok, report = self._run(
            _asr("AsrB", "B"),
            _asr("AsrD", "D"),
            _feat("Feat", "D", "AsrB", "AsrD"),
        )
        self.assertFalse(ok)
        self.assertIn("Fixture.AsrB has ASIL B", report)
        self.assertNotIn("Fixture.AsrD has", report)

    def test_aou_parent_is_not_checked(self):
        ok, _ = self._run(_aou("Aou", "B"), _comp("Comp", "D", "Aou"))
        self.assertTrue(ok)

    def test_each_violating_parent_is_reported(self):
        ok, report = self._run(
            _asr("AsrA", "B"),
            _asr("AsrB", "B"),
            _feat("FeatA", "D", "AsrA"),
            _feat("FeatB", "D", "AsrB"),
        )
        self.assertFalse(ok)
        self.assertEqual(report.count("no downstream requirement with the same ASIL"), 2)

    def test_lower_asil_child_alongside_matching_child_passes(self):
        ok, _ = self._run(
            _asr("Asr", "D"),
            _feat("FeatD", "D", "Asr"),
            _feat("FeatB", "B", "Asr"),
        )
        self.assertTrue(ok)

    def test_only_lower_asil_children_fail(self):
        ok, report = self._run(_asr("Asr", "D"), _feat("Feat", "B", "Asr"))
        self.assertFalse(ok)
        self.assertIn("Fixture.Asr has ASIL D", report)

    def test_warn_only_reports_but_does_not_fail(self):
        exit_code, report = self._main(_asr("Asr", "B"), _feat("Feat", "D", "Asr"), extra=("--warn-only",))
        self.assertEqual(exit_code, 0)
        self.assertIn("Fixture.Asr has ASIL B", report)

    def test_log_is_echoed_to_stderr_on_failure(self):
        log = os.path.join(self._tmp.name, "out.log")
        path = self._write(_asr("Asr", "B"), _feat("Feat", "D", "Asr"))
        stderr = io.StringIO()
        argv = ["--log", log, "--check", _CHECKS[0], _RSL, path]
        with mock.patch.object(sys, "stderr", stderr):
            self.assertEqual(trlc_check_runner.main(argv), 1)
        self.assertIn("Fixture.Asr has ASIL B", stderr.getvalue())

    def test_unknown_plugin_is_rejected(self):
        with self.assertRaises(SystemExit) as raised:
            trlc_check_runner.main(["--check", "no_such_module", _RSL])
        self.assertIn("no_such_module", str(raised.exception))

    def test_unknown_plugin_function_is_rejected(self):
        with self.assertRaises(SystemExit) as raised:
            trlc_check_runner.main(["--check", "trlc_checks:no_such_function", _RSL])
        self.assertIn("no_such_function", str(raised.exception))

    def test_no_plugin_runs_unless_named(self):
        exit_code, _ = self._main(_asr("Asr", "B"), _feat("Feat", "D", "Asr"), checks=())
        self.assertEqual(exit_code, 0)

    def test_plugin_is_not_run_when_trlc_cannot_resolve(self):
        plugin = mock.Mock(return_value=True)
        incomplete = "ScoreReq.FeatReq Incomplete {\n    version = 1\n}"
        with mock.patch.object(trlc_check_runner, "_load_plugin", return_value=plugin):
            exit_code, _ = self._main(incomplete, checks=("x",))
        self.assertEqual(exit_code, 1)
        plugin.assert_not_called()

    def test_plugin_runs_after_model_check_failure_and_warn_only_keeps_exit_code(self):
        spec = os.path.join(self._tmp.name, "failing.rsl")
        with open(spec, "w", encoding="utf-8") as fh:
            fh.write(
                "package Failing\n"
                'type Marker "Marker." {\n    value "Value." String\n}\n'
                'checks Marker {\n    value == "ok", error "always fails", value\n}\n'
            )
        exit_code, report = self._main(
            "import Failing",
            'Failing.Marker Bad {\n    value = "bad"\n}',
            _asr("Asr", "B"),
            _feat("Feat", "D", "Asr"),
            extra=("--warn-only", spec),
        )
        self.assertEqual(exit_code, 1)
        self.assertIn("always fails", report)
        self.assertIn("Fixture.Asr has ASIL B", report)

    def test_explicit_check_named_twice_runs_once(self):
        plugin = mock.Mock(return_value=True)
        with mock.patch.object(trlc_check_runner, "_load_plugin", return_value=plugin):
            self._main(_asr("Asr", "B"), checks=("x", "y"))
        plugin.assert_called_once()

    def test_perform_checks_is_restored(self):
        original = trlc_check_runner.trlc_cli.Source_Manager.perform_checks
        self._main(_asr("Asr", "B"))
        self.assertIs(trlc_check_runner.trlc_cli.Source_Manager.perform_checks, original)

    def test_version_pins_match_referenced_records(self):
        exit_code, report = self._main(
            _asr("Asr", "B"),
            _feat("Feat", "B", "Asr"),
            _comp("Comp", "B", "Feat", "Asr", "Aou"),
            _aou("Aou", "B"),
        )
        self.assertEqual(exit_code, 0, report)

    def test_stale_and_future_requirement_pins_are_reported(self):
        exit_code, report = self._main(
            _asr("Asr", "B"),
            _feat("Feat", "B", "Asr").replace("Asr@1", "Asr@0"),
            _comp("Comp", "B", "Feat", "Aou").replace("Feat@1", "Feat@2").replace("Aou@1", "Aou@0"),
            _aou("Aou", "B"),
        )
        self.assertEqual(exit_code, 1)
        for expected in ("Fixture.Feat", "Fixture.Comp", "Fixture.Asr@0", "Fixture.Feat@2", "Fixture.Aou@0"):
            self.assertIn(expected, report)
        self.assertEqual(report.count("current version is 1"), 3)

    def test_cross_package_pin_and_development_warning(self):
        upstream = os.path.join(self._tmp.name, "upstream.trlc")
        with open(upstream, "w", encoding="utf-8") as fh:
            fh.write("package Upstream\nimport ScoreReq\n" + _asr("Asr", "B").replace("version = 1", "version = 2"))
        feat = _feat("Feat", "B", "Upstream.Asr")
        exit_code, report = self._main(
            "import Upstream",
            feat,
            extra=("--warn-only", upstream),
        )
        self.assertEqual(exit_code, 0)
        self.assertIn("Upstream.Asr@1", report)
        self.assertIn("current version is 2", report)

    def test_check_applies_to_extended_requirement_type(self):
        spec = os.path.join(self._tmp.name, "extra.rsl")
        with open(spec, "w", encoding="utf-8") as fh:
            fh.write("package Extra\nimport ScoreReq\ntype ExtendedFeat extends ScoreReq.FeatReq {}\n")
        feat = _feat("Feat", "B", "Asr").replace("ScoreReq.FeatReq", "Extra.ExtendedFeat").replace("Asr@1", "Asr@2")
        exit_code, report = self._main(
            "import Extra",
            _asr("Asr", "B"),
            feat,
            extra=(spec,),
        )
        self.assertEqual(exit_code, 1)
        self.assertIn("Fixture.Feat derived_from references Fixture.Asr@2", report)

    def test_assessment_measure_pins_are_not_checked(self):
        failure_mode = """ScoreReq.FailureMode Failure {
    description = "Fixture."
    safety = ScoreReq.Asil.B
    version = 1
    guidewords = [ScoreReq.Guideword.Corrupted]
    failureeffect = "Fixture."
}"""
        root_cause = """ScoreReq.RootCause Root {
    title = "Fixture."
    diagram = "fixture.puml"
    line = 1
    failure_modes = [Failure]
}"""
        assessment = """ScoreReq.RootCauseAssessment Assessment {
    root_causes = [Root]
    measures = [Aou@2, Comp@0]
    safety = ScoreReq.Asil.B
    rationale = "Fixture."
    version = 1
}"""
        exit_code, report = self._main(
            _asr("Asr", "B"),
            _comp("Comp", "B", "Asr"),
            _aou("Aou", "B"),
            failure_mode,
            root_cause,
            assessment,
        )
        self.assertEqual(exit_code, 0, report)


if __name__ == "__main__":
    unittest.main()
