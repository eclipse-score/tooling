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
"""Unit tests for the safety-analysis page assembler layout logic."""

import json
import os
import sys
import tempfile
import unittest

import safety_analysis_assembler as fa


class _Type:
    def __init__(self, name):
        self.name = name


class _Obj:
    def __init__(self, name, type_name, fields=None):
        self.name = name
        self.n_typ = _Type(type_name)
        self._fields = fields or {}

    def to_python_dict(self):
        return self._fields


class _FakeRenderer:
    """Minimal stand-in for ``TRLCRST`` exercising the assembler layout."""

    def __init__(self, objs):
        self._objs = objs

    def objects_by_fqn(self):
        return self._objs

    def render_table_to_string(self, columns, fqns=None, name_header="Name", link_fn=None):
        if fqns is None:
            fqns = list(self._objs)
        if link_fn is not None:
            rows = "\n".join(link_fn(f, self._objs[f].name) for f in fqns)
        else:
            rows = "\n".join(self._objs[f].name for f in fqns)
        return f"TABLE[{name_header}]\n{rows}\n"

    def render_records_to_string(self, fqns, fields):
        return "RECORDS(" + ",".join(fqns) + ")\n"

    def field_value_for(self, fqn, field_name, records=None):
        return self._objs[fqn].to_python_dict().get(field_name, "")


def _objs():
    """FailureMode/Mitigation records plus a single FtaFailureMode/RootCause pair
    covering ``Lib.FM_A`` (via ``Lib.CM_1``); ``Lib.FM_Orphan``/``Lib.CM_Orphan``
    are not referenced by any generated stub."""
    return {
        "Lib.FM_A": _Obj(
            "FM_A",
            "FailureMode",
            {
                "guideword": "LossOfFunction",
                "safety": "B",
                "interface": "Lib.Api",
                "failureeffect": "world ends",
                "description": "fm a description",
            },
        ),
        "Lib.FM_Orphan": _Obj("FM_Orphan", "FailureMode", {"safety": "QM", "guideword": "TooLate"}),
        "Lib.CM_1": _Obj(
            "CM_1",
            "Mitigation",
            {"safety": "B", "description": "cm one", "root_causes": ["Lib.RC_A"]},
        ),
        "Lib.CM_Orphan": _Obj("CM_Orphan", "Mitigation", {"safety": "D"}),
        "Lib.FFM_A": _Obj(
            "FFM_A",
            "FtaFailureMode",
            {"title": "Failure mode A", "diagram": "fta_a.puml", "line": 1, "failure_modes": ["Lib.FM_A"]},
        ),
        "Lib.RC_A": _Obj(
            "RC_A",
            "RootCause",
            {"title": "Root cause A", "diagram": "fta_a.puml", "line": 2, "failure_modes": ["Lib.FFM_A"]},
        ),
    }


def _objs_no_fta():
    """FailureMode/Mitigation records only, no generated FtaFailureMode/RootCause
    stubs at all (mirrors an ``safety_analysis()`` target with no ``root_causes`` diagrams)."""
    return {fqn: obj for fqn, obj in _objs().items() if obj.n_typ.name in ("FailureMode", "Mitigation")}


class AnchorTest(unittest.TestCase):
    def test_anchor_is_sanitised_lowercase(self):
        self.assertEqual(fa._anchor("Lib.FM_A"), "safety-analysis-lib-fm-a")

    def test_ref_targets_anchor(self):
        self.assertEqual(
            fa._ref("Lib.FM_A", "FM_A"),
            ":ref:`FM_A <safety-analysis-lib-fm-a>`",
        )


class ChainDerivationTest(unittest.TestCase):
    """Unit tests for the obj_map-driven chain-derivation helpers."""

    def setUp(self):
        self.obj_map = _objs()

    def test_root_causes_for_fta_failure_mode_finds_matching_root_causes(self):
        self.assertEqual(fa._root_causes_for_fta_failure_mode(self.obj_map, "Lib.FFM_A"), ["Lib.RC_A"])

    def test_root_causes_for_fta_failure_mode_empty_when_unreferenced(self):
        self.assertEqual(fa._root_causes_for_fta_failure_mode(self.obj_map, "Lib.NoSuchTE"), [])

    def test_measures_for_root_cause_finds_matching_measures(self):
        self.assertEqual(fa._measures_for_root_cause(self.obj_map, "Lib.RC_A"), ["Lib.CM_1"])

    def test_measures_for_root_cause_includes_aou_type(self):
        obj_map = dict(self.obj_map)
        obj_map["Lib.Aou_1"] = _Obj("Aou_1", "AoU", {"root_causes": ["Lib.RC_A"]})
        self.assertCountEqual(
            fa._measures_for_root_cause(obj_map, "Lib.RC_A"),
            ["Lib.CM_1", "Lib.Aou_1"],
        )

    def test_measures_for_root_cause_includes_compreq_type(self):
        obj_map = dict(self.obj_map)
        obj_map["Lib.Cr_1"] = _Obj(
            "Cr_1",
            "CompReq",
            {"derived_from": [{"item": "Lib.RC_A", "version": None}]},
        )
        self.assertCountEqual(
            fa._measures_for_root_cause(obj_map, "Lib.RC_A"),
            ["Lib.CM_1", "Lib.Cr_1"],
        )

    def test_measures_for_root_cause_compreq_other_union_members_ignored(self):
        obj_map = dict(self.obj_map)
        # A CompReq derived from a FeatReq/AssumedSystemReq/AoU (not this
        # RootCause) must not count as covering it.
        obj_map["Lib.Cr_Unrelated"] = _Obj(
            "Cr_Unrelated",
            "CompReq",
            {"derived_from": [{"item": "Lib.SomeFeatReq", "version": 1}]},
        )
        self.assertEqual(fa._measures_for_root_cause(obj_map, "Lib.RC_A"), ["Lib.CM_1"])

    def test_chains_for_failure_mode_covered(self):
        chains = fa._chains_for_failure_mode(self.obj_map, "Lib.FM_A")
        self.assertEqual(len(chains), 1)
        self.assertEqual(chains[0]["ffm_fqn"], "Lib.FFM_A")
        self.assertEqual(chains[0]["puml"], "fta_a.puml")
        self.assertEqual(chains[0]["measures"], ["Lib.CM_1"])

    def test_chains_for_failure_mode_orphan_is_empty(self):
        self.assertEqual(fa._chains_for_failure_mode(self.obj_map, "Lib.FM_Orphan"), [])


class UncoveredRootCausesTest(unittest.TestCase):
    def test_covered_root_cause_is_not_reported(self):
        obj_map = _objs()
        self.assertEqual(fa._uncovered_root_causes(obj_map), [])

    def test_uncovered_root_cause_is_reported(self):
        obj_map = _objs()
        obj_map["Lib.RC_Orphan"] = _Obj(
            "RC_Orphan", "RootCause", {"title": "Orphan", "diagram": "fta_a.puml", "line": 3, "failure_modes": []}
        )
        self.assertEqual(fa._uncovered_root_causes(obj_map), ["Lib.RC_Orphan"])


class BuildBodyTest(unittest.TestCase):
    def setUp(self):
        self.renderer = _FakeRenderer(_objs())

    def test_overview_and_chain_section_rendered(self):
        body = fa._build_body(self.renderer, "Title")
        self.assertIn("Title\n=====", body)
        self.assertIn("Overview", body)
        # Top-level grouping sections.
        self.assertIn("Failure Modes\n-------------", body)
        # FM dropdown titled by its full fqn only (no ASIL in the heading).
        self.assertIn(".. dropdown:: Lib.FM_A\n", body)
        self.assertNotIn(".. dropdown:: Lib.FM_A :bdg", body)
        self.assertIn(":name: safety-analysis-lib-fm-a", body)
        # Attributes as a grid of cards; no inner requirement id.
        self.assertNotIn(".. requirement:definition::", body)
        self.assertIn(".. grid:: 2", body)
        # Guideword / ASIL are bare centred grid items (no card chrome).
        self.assertIn(".. grid-item::", body)
        self.assertIn(":class: sd-text-center", body)
        self.assertIn(":bdg-info:`LossOfFunction`", body)
        self.assertIn(":bdg-warning:`ASIL B`", body)
        self.assertNotIn(".. grid-item-card:: Guideword", body)
        self.assertIn(".. grid-item-card:: Interface", body)
        # Description as a prominent card.
        self.assertIn(".. grid-item-card:: Description", body)
        self.assertIn("fm a description", body)
        # Root Cause Analysis rubric over the inline FTA + per-FM CM cards.
        self.assertIn(".. rubric:: Root Cause Analysis", body)
        self.assertIn(".. uml:: fta_a.puml", body)
        self.assertIn(".. rubric:: Safety Measures", body)
        # CM card: bold ID with inline kind + ASIL badges in the card body (no card title).
        self.assertIn(".. grid-item-card::\n", body)
        self.assertIn("**CM_1**", body)
        self.assertIn(":bdg-warning:`Mitigation`", body)
        self.assertIn(":bdg-warning:`ASIL B`", body)
        # Attribute grid uses a gutter to separate the badge row from the cards.
        self.assertIn(":gutter: 3", body)

    def test_global_measures_section_lists_all(self):
        body = fa._build_body(self.renderer, "Title")
        self.assertIn("Safety Measures\n---------------", body)
        self.assertIn("**CM_1**", body)
        self.assertIn("**CM_Orphan**", body)
        self.assertIn(":bdg-warning:`Mitigation`", body)

    def test_orphan_failure_mode_rendered_without_fta(self):
        body = fa._build_body(self.renderer, "Title")
        # Orphan FM still appears as a dropdown, but with no FTA / RCA rubric.
        self.assertIn(".. dropdown:: Lib.FM_Orphan\n", body)
        self.assertEqual(body.count(".. rubric:: Root Cause Analysis"), 1)

    def test_no_fta_failure_modes_renders_all_failure_modes_without_fta(self):
        renderer = _FakeRenderer(_objs_no_fta())
        body = fa._build_body(renderer, "Title")
        self.assertIn(".. dropdown:: Lib.FM_A\n", body)
        self.assertIn(".. dropdown:: Lib.FM_Orphan\n", body)
        self.assertNotIn(".. uml::", body)


# Minimal self-contained TRLC model: defines the FailureMode / SafetyMeasure /
# Mitigation / AoU / CompReq / FtaFailureMode / RootCause types the assembler keys
# on, so main() runs a real TRLCRST parse (catching contract drift the
# _FakeRenderer cannot). Everything lives in one package (unlike the real
# ScoreReq model) since this test does not need to exercise cross-package
# stub imports.
_RSL = """\
package TestSafetyAnalysis

type FailureMode {
    guideword optional String
    safety optional String
    interface optional String
    failureeffect optional String
    description optional String
}

abstract type FtaEvent {
    title String
    diagram String
    line Integer
}

type FtaFailureMode extends FtaEvent {
    failure_modes FailureMode [1..*]
}

type RootCause extends FtaEvent {
    failure_modes FtaFailureMode [1..*]
}

abstract type SafetyMeasure {
    safety optional String
    description optional String
    root_causes RootCause [1..*]
}

type Mitigation extends SafetyMeasure {
}

type AoU {
    safety optional String
    description optional String
    root_causes optional RootCause [1..*]
}

tuple CompReqSourceId {
    item RootCause
    separator @
    version optional Integer
}

type CompReq {
    safety optional String
    description optional String
    derived_from CompReqSourceId [1..*]
}
"""

_FM_TRLC = """\
package TestSafetyAnalysis

FailureMode FmA {
    guideword = "TooLate"
    safety = "ASIL_D"
    interface = "Lib.Api"
    failureeffect = "downstream timeout"
    description = "fm a description"
}
"""

_MITIGATION_TRLC = """\
package TestSafetyAnalysis

Mitigation CmA {
    safety = "ASIL_D"
    description = "cm a description"
    root_causes = [TestSafetyAnalysis.RcA]
}
"""

_MEASURES_TRLC = """\
package TestSafetyAnalysis

CompReq CrB {
    description = "cr b description"
    derived_from = [TestSafetyAnalysis.RcB@1]
}
"""

_FTA_TRLC = """\
package TestSafetyAnalysis

FtaFailureMode FfmA {
    title = "Failure mode A"
    diagram = "a.puml"
    line = 1
    failure_modes = [TestSafetyAnalysis.FmA]
}

RootCause RcA {
    title = "Root cause A"
    diagram = "a.puml"
    line = 2
    failure_modes = [TestSafetyAnalysis.FfmA]
}

RootCause RcB {
    title = "Root cause B"
    diagram = "a.puml"
    line = 3
    failure_modes = [TestSafetyAnalysis.FfmA]
}

RootCause RcUncovered {
    title = "Root cause uncovered"
    diagram = "a.puml"
    line = 4
    failure_modes = [TestSafetyAnalysis.FfmA]
}
"""


class MainIntegrationTest(unittest.TestCase):
    """End-to-end main(): real TRLCRST parse of failuremodes + safetymeasures +
    measures + fta_events.trlc -> safety_analysis.rst (+ uncovered_root_causes.json)."""

    def _write(self, directory, name, content):
        path = os.path.join(directory, name)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(content)
        return path

    def _run_main(self, argv):
        saved = sys.argv
        try:
            sys.argv = argv
            fa.main()
        finally:
            sys.argv = saved

    def test_full_page_assembled_from_real_trlc(self):
        with tempfile.TemporaryDirectory() as tmp:
            rsl = self._write(tmp, "types.rsl", _RSL)
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            cm = self._write(tmp, "cm.trlc", _MITIGATION_TRLC)
            measures = self._write(tmp, "measures.trlc", _MEASURES_TRLC)
            fta = self._write(tmp, "fta_events.trlc", _FTA_TRLC)
            template = self._write(tmp, "tmpl.rst", "{body}\n")
            out = os.path.join(tmp, "safety_analysis.rst")
            uncovered_out = os.path.join(tmp, "uncovered_root_causes.json")

            self._run_main(
                [
                    "safety_analysis_assembler",
                    "--output",
                    out,
                    "--template",
                    template,
                    "--title",
                    "Test FMEA",
                    "--fta-events",
                    fta,
                    "--failuremodes",
                    fm,
                    "--safetymeasures",
                    cm,
                    "--measures",
                    measures,
                    "--spec",
                    rsl,
                    "--uncovered-root-causes-output",
                    uncovered_out,
                ]
            )

            with open(out, encoding="utf-8") as fh:
                rst = fh.read()

            # Title + overview table + FM dropdown + inline FTA + safety measures.
            self.assertIn("Test FMEA", rst)
            self.assertIn("Overview", rst)
            self.assertIn(".. list-table::", rst)
            self.assertIn(".. dropdown:: TestSafetyAnalysis.FmA\n", rst)
            self.assertIn(":name: safety-analysis-testsafetyanalysis-fma", rst)
            self.assertIn(".. grid-item-card:: Description", rst)
            self.assertIn(".. rubric:: Root Cause Analysis", rst)
            self.assertIn(".. uml:: a.puml", rst)
            self.assertIn("Safety Measures", rst)
            # Real rendered record content (proves TRLCRST actually parsed).
            self.assertIn("fm a description", rst)
            self.assertIn("cm a description", rst)
            # CompReq (from --measures) rendered as a kind-badged control measure.
            self.assertIn("cr b description", rst)
            self.assertIn(":bdg-success:`Control Measure`", rst)

            # RcA is covered by the Mitigation, RcB by the CompReq; RcUncovered
            # is addressed by neither and must be the sole coverage gap.
            with open(uncovered_out, encoding="utf-8") as fh:
                uncovered = json.load(fh)
            self.assertEqual(uncovered, ["TestSafetyAnalysis.RcUncovered"])

    def test_malformed_fta_events_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            template = self._write(tmp, "tmpl.rst", "{body}\n")
            bad = self._write(tmp, "fta_events.trlc", "this is not valid trlc @@@ syntax")
            out = os.path.join(tmp, "safety_analysis.rst")
            with self.assertRaises(SystemExit) as ctx:
                self._run_main(
                    [
                        "safety_analysis_assembler",
                        "--output",
                        out,
                        "--template",
                        template,
                        "--title",
                        "T",
                        "--fta-events",
                        bad,
                    ]
                )
            self.assertEqual(ctx.exception.code, 1)

    def test_missing_body_placeholder_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            # Template without the required {body} placeholder.
            template = self._write(tmp, "tmpl.rst", "no placeholder here\n")
            # Trivial stub, matching what safety_analysis.bzl emits when there are no
            # root_causes diagrams at all.
            fta = self._write(tmp, "fta_events.trlc", "// no root cause diagrams\npackage T\n")
            out = os.path.join(tmp, "safety_analysis.rst")
            with self.assertRaises(SystemExit) as ctx:
                self._run_main(
                    [
                        "safety_analysis_assembler",
                        "--output",
                        out,
                        "--template",
                        template,
                        "--title",
                        "T",
                        "--fta-events",
                        fta,
                    ]
                )
            self.assertEqual(ctx.exception.code, 1)


if __name__ == "__main__":
    unittest.main()
