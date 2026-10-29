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

import os
import sys
import tempfile
import types
import unittest

import safety_analysis_assembler as fa


class _Type:
    def __init__(self, name):
        self.name = name


class _Obj:
    def __init__(self, name, type_name, fields=None, file_name="x.trlc"):
        self.name = name
        self.n_typ = _Type(type_name)
        self._fields = fields or {}
        self.location = types.SimpleNamespace(file_name=file_name)

    def to_python_dict(self):
        return self._fields


class _FakeRenderer:
    """Minimal stand-in for ``TRLCRST`` exercising the assembler layout."""

    def __init__(self, objs):
        self._objs = objs

    def objects_by_fqn(self):
        return self._objs

    def render_records_to_string(self, fqns, fields):
        return "RECORDS(" + ",".join(fqns) + ")\n"

    def field_value_for(self, fqn, field_name, records=None):
        return self._objs[fqn].to_python_dict().get(field_name, "")


def _objs():
    """FailureMode/AoU records, a single RootCause for ``Lib.FM_A`` assessed by
    ``Lib.Asm_A`` (measure ``Lib.AoU_1``); ``Lib.FM_Orphan``/``Lib.AoU_Orphan``
    are not referenced by any root cause / assessment."""
    return {
        "Lib.FM_A": _Obj(
            "FM_A",
            "FailureMode",
            {
                "guidewords": ["LossOfFunction", "TooLate"],
                "safety": "B",
                "interface": ["Lib.Api", "Lib.Api2"],
                "failureeffect": "world ends",
                "description": "fm a description",
            },
        ),
        "Lib.FM_Orphan": _Obj("FM_Orphan", "FailureMode", {"safety": "QM", "guidewords": ["TooLate"]}),
        "Lib.AoU_1": _Obj("AoU_1", "AoU", {"safety": "B", "description": "aou one", "version": 3}),
        "Lib.AoU_Orphan": _Obj("AoU_Orphan", "AoU", {"safety": "D"}),
        "Lib.RC_A": _Obj(
            "RC_A",
            "RootCause",
            {"title": "Root cause A", "diagram": "fta_a.puml", "line": 2, "failure_modes": ["Lib.FM_A"]},
        ),
        "Lib.Asm_A": _assessment("Asm_A", ["Lib.RC_A"], ["Lib.AoU_1"], "asm a rationale"),
    }


def _assessment(name, root_causes, measures, rationale="rationale", file_name="x.trlc", safety="B"):
    return _Obj(
        name,
        "RootCauseAssessment",
        {
            "root_causes": root_causes,
            "measures": [{"item": m, "version": 1} for m in measures],
            "safety": safety,
            "rationale": rationale,
            "version": 1,
        },
        file_name=file_name,
    )


def _root_cause(name, line=3):
    return _Obj(
        name,
        "RootCause",
        {"title": f"Root cause {name}", "diagram": "fta_a.puml", "line": line, "failure_modes": ["Lib.FM_A"]},
    )


_DIAGRAMS = ["fta_a.puml"]


def _objs_no_fta():
    """FailureMode/AoU records only, no generated RootCause
    records at all (mirrors an ``safety_analysis()`` target with no ``root_causes`` diagrams)."""
    return {fqn: obj for fqn, obj in _objs().items() if obj.n_typ.name in ("FailureMode", "AoU")}


class AnchorTest(unittest.TestCase):
    def test_anchor_is_sanitised_lowercase(self):
        self.assertEqual(fa._anchor("Lib.FM_A"), "safety-analysis-lib-fm-a")

    def test_ref_targets_anchor(self):
        self.assertEqual(
            fa._ref("Lib.FM_A", "FM_A"),
            ":ref:`FM_A <safety-analysis-lib-fm-a>`",
        )

    def test_diagram_anchor_is_package_scoped_and_sanitised(self):
        self.assertEqual(fa._diagram_anchor("Lib", "fta_a.puml"), "safety-analysis-lib-diagram-fta-a-puml")

    def test_diagram_anchor_differs_per_package(self):
        self.assertNotEqual(fa._diagram_anchor("A", "fta.puml"), fa._diagram_anchor("B", "fta.puml"))

    def test_escape_inline_markup(self):
        self.assertEqual(fa._escape("a*b_c|d`e\\f"), "a\\*b\\_c\\|d\\`e\\\\f")


class ChainDerivationTest(unittest.TestCase):
    """Unit tests for the obj_map-driven link-derivation helpers."""

    def setUp(self):
        self.obj_map = _objs()

    def test_assessments_by_root_cause_maps_root_cause_to_assessment(self):
        self.assertEqual(fa._assessments_by_root_cause(self.obj_map), {"Lib.RC_A": ["Lib.Asm_A"]})

    def test_assessments_by_root_cause_maps_grouped_root_causes_to_one_assessment(self):
        obj_map = dict(self.obj_map)
        obj_map["Lib.RC_B"] = _root_cause("RC_B")
        obj_map["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A", "Lib.RC_B"], ["Lib.AoU_1"])
        self.assertEqual(
            fa._assessments_by_root_cause(obj_map),
            {"Lib.RC_A": ["Lib.Asm_A"], "Lib.RC_B": ["Lib.Asm_A"]},
        )

    def test_assessments_by_root_cause_ignores_repeated_root_cause_in_one_assessment(self):
        obj_map = dict(self.obj_map)
        obj_map["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A", "Lib.RC_A"], [])
        self.assertEqual(fa._assessments_by_root_cause(obj_map), {"Lib.RC_A": ["Lib.Asm_A"]})

    def test_measures_of_assessment_returns_measure_items(self):
        assessment = _assessment("Asm", ["Lib.RC_A"], ["Lib.AoU_1", "Lib.CR_1"])
        self.assertEqual(fa._measures_of_assessment(assessment), ["Lib.AoU_1", "Lib.CR_1"])

    def test_measures_of_assessment_without_measures_is_empty(self):
        self.assertEqual(fa._measures_of_assessment(_assessment("Asm", ["Lib.RC_A"], [])), [])

    def test_pinned_versions_maps_measure_to_pinned_version(self):
        assessment = _assessment("Asm", ["Lib.RC_A"], ["Lib.AoU_1", "Lib.CR_1"])
        self.assertEqual(fa._pinned_versions(assessment), {"Lib.AoU_1": 1, "Lib.CR_1": 1})

    def test_root_causes_for_failure_mode_finds_matching_root_causes(self):
        self.assertEqual(fa._root_causes_for_failure_mode(self.obj_map, "Lib.FM_A"), ["Lib.RC_A"])

    def test_root_causes_for_failure_mode_orphan_is_empty(self):
        self.assertEqual(fa._root_causes_for_failure_mode(self.obj_map, "Lib.FM_Orphan"), [])


class CheckInputsTest(unittest.TestCase):
    def test_consistent_inputs_have_no_errors(self):
        self.assertEqual(fa._check_inputs(_objs(), []), [])

    def test_root_cause_failure_mode_outside_failure_modes_is_reported(self):
        objs = {fqn: obj for fqn, obj in _objs().items() if fqn != "Lib.FM_A"}
        errors = fa._check_inputs(objs, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("Lib.RC_A", errors[0])
        self.assertIn("Lib.FM_A", errors[0])

    def test_assessment_in_assessments_file_is_accepted(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], ["Lib.AoU_1"], file_name="a.trlc")
        self.assertEqual(fa._check_inputs(objs, ["a.trlc"]), [])

    def test_non_assessment_in_assessments_file_is_reported(self):
        objs = _objs()
        objs["Lib.AoU_1"] = _Obj("AoU_1", "AoU", {"safety": "B"}, file_name="a.trlc")
        errors = fa._check_inputs(objs, ["a.trlc"])
        self.assertEqual(len(errors), 1)
        self.assertIn("AoU Lib.AoU_1", errors[0])

    def test_root_cause_in_assessments_file_is_reported(self):
        objs = _objs()
        objs["Lib.RC_A"] = _Obj(
            "RC_A",
            "RootCause",
            {"title": "Root cause A", "diagram": "fta_a.puml", "line": 2, "failure_modes": ["Lib.FM_A"]},
            file_name="a.trlc",
        )
        errors = fa._check_inputs(objs, ["a.trlc"])
        self.assertEqual(len(errors), 1)
        self.assertIn("RootCause Lib.RC_A", errors[0])

    def test_non_assessment_outside_assessments_file_is_accepted(self):
        objs = _objs()
        objs["Lib.AoU_1"] = _Obj("AoU_1", "AoU", {"safety": "B"}, file_name="m.trlc")
        self.assertEqual(fa._check_inputs(objs, ["a.trlc"]), [])

    def test_root_cause_in_several_assessments_is_reported(self):
        objs = _objs()
        objs["Lib.Asm_B"] = _assessment("Asm_B", ["Lib.RC_A"], [])
        errors = fa._check_inputs(objs, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("Lib.RC_A", errors[0])
        self.assertIn("Lib.Asm_A", errors[0])
        self.assertIn("Lib.Asm_B", errors[0])

    def test_grouped_assessment_is_accepted(self):
        objs = _objs()
        objs["Lib.RC_B"] = _root_cause("RC_B")
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A", "Lib.RC_B"], ["Lib.AoU_1"])
        self.assertEqual(fa._check_inputs(objs, []), [])

    def test_assessment_root_cause_not_generated_is_reported(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A", "Lib.RC_Foreign"], ["Lib.AoU_1"])
        errors = fa._check_inputs(objs, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("Lib.Asm_A", errors[0])
        self.assertIn("Lib.RC_Foreign", errors[0])

    def test_assessment_measure_outside_safety_measures_is_reported(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], ["Lib.AoU_1", "Lib.AoU_Foreign"])
        errors = fa._check_inputs(objs, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("Lib.Asm_A", errors[0])
        self.assertIn("Lib.AoU_Foreign", errors[0])

    def test_assessment_without_measures_is_accepted(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], [])
        self.assertEqual(fa._check_inputs(objs, []), [])

    def test_assessment_asil_below_failure_mode_is_reported(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], [], safety="QM")
        errors = fa._check_inputs(objs, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("Lib.Asm_A has ASIL QM, below ASIL B", errors[0])
        self.assertIn("Lib.FM_A", errors[0])
        self.assertIn("Lib.RC_A", errors[0])

    def test_assessment_asil_above_measure_is_reported(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], ["Lib.AoU_1"], safety="D")
        errors = fa._check_inputs(objs, [])
        self.assertEqual(len(errors), 1)
        self.assertIn("Lib.Asm_A has ASIL D, above ASIL B of measure Lib.AoU_1", errors[0])

    def test_assessment_asil_above_failure_mode_without_measures_is_accepted(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], [], safety="D")
        self.assertEqual(fa._check_inputs(objs, []), [])


class BuildBodyTest(unittest.TestCase):
    def setUp(self):
        self.renderer = _FakeRenderer(_objs())
        self.diagrams = _DIAGRAMS

    def test_overview_and_failure_mode_section_rendered(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams)
        self.assertIn("Title\n=====", body)
        self.assertIn("Overview", body)
        # Overview table lists each failure mode with its interface.
        overview = body.split("Failure Modes\n-------------")[0]
        self.assertIn("   * - Failure Mode\n     - Guidewords\n     - ASIL\n     - Interface\n", overview)
        self.assertIn(
            ":ref:`FM_A <safety-analysis-lib-fm-a>`\n     - ``LossOfFunction``, ``TooLate``\n     - B\n", overview
        )
        self.assertIn("     - ``Lib.Api``, ``Lib.Api2``\n", overview)
        # Top-level grouping sections.
        self.assertIn("Failure Modes\n-------------", body)
        # FM dropdown titled by its full fqn only (no ASIL in the heading).
        self.assertIn(".. dropdown:: Lib.FM_A\n", body)
        self.assertNotIn(".. dropdown:: Lib.FM_A :bdg", body)
        self.assertIn(":name: safety-analysis-lib-fm-a", body)
        # Attributes as a grid of cards; no inner requirement id.
        self.assertNotIn(".. requirement:definition::", body)
        self.assertIn(".. grid:: 2", body)
        # Guidewords / ASIL are bare centred grid items (no card chrome).
        self.assertIn(".. grid-item::", body)
        self.assertIn(":class: sd-text-center", body)
        self.assertIn(":bdg-info:`LossOfFunction` :bdg-info:`TooLate`", body)
        self.assertIn(":bdg-warning:`ASIL B`", body)
        self.assertNotIn(".. grid-item-card:: Guideword", body)
        self.assertIn(".. grid-item-card:: Interface", body)
        self.assertIn("``Lib.Api``, ``Lib.Api2``", body)
        # Description as a prominent card.
        self.assertIn(".. grid-item-card:: Description", body)
        self.assertIn("fm a description", body)
        # Root Causes rubric with one card per root cause: title, source, assessment.
        self.assertIn(".. rubric:: Root Causes", body)
        self.assertIn(".. grid-item-card:: Root cause A", body)
        self.assertIn("Source: :ref:`fta_a.puml <safety-analysis-lib-diagram-fta-a-puml>`, line 2", body)
        # Assessment: bold name and rationale in the root cause card.
        self.assertIn("**Asm_A**", body)
        self.assertIn("asm a rationale", body)
        self.assertNotIn("Not applicable", body)
        # Measure card: bold ID with inline kind + ASIL badges in the card body (no card title).
        self.assertIn(".. grid-item-card::\n", body)
        self.assertIn("**AoU_1**", body)
        self.assertIn(":bdg-info:`Assumption of Use`", body)
        self.assertIn(":bdg-warning:`ASIL B`", body)
        # Attribute grid uses a gutter to separate the badge row from the cards.
        self.assertIn(":gutter: 3", body)

    def test_fault_trees_section_shows_each_diagram_once(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams + self.diagrams)
        self.assertIn("Fault Trees\n-----------", body)
        self.assertIn(".. _safety-analysis-lib-diagram-fta-a-puml:", body)
        self.assertEqual(body.count(".. uml:: fta_a.puml"), 1)

    def test_unstaged_diagram_source_is_escaped_plain_text(self):
        objs = _objs()
        objs["Lib.RC_A"] = _Obj(
            "RC_A",
            "RootCause",
            {"title": "Root cause A", "diagram": "x*y.puml", "line": 2, "failure_modes": ["Lib.FM_A"]},
        )
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", self.diagrams)
        self.assertIn("Source: x\\*y.puml, line 2", body)

    def test_root_cause_without_assessment_is_flagged(self):
        objs = _objs()
        objs["Lib.RC_B"] = _root_cause("RC_B")
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", self.diagrams)
        self.assertEqual(body.count(":bdg-danger:`Not assessed`"), 1)

    def test_assessment_without_measures_is_rendered_as_not_applicable(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], [], "does not apply")
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", self.diagrams)
        self.assertEqual(body.count(":bdg-secondary:`Not applicable`"), 1)
        self.assertIn("does not apply", body)
        self.assertNotIn("Not assessed", body)

    def test_grouped_assessment_is_rendered_in_each_root_cause_card(self):
        objs = _objs()
        objs["Lib.RC_B"] = _root_cause("RC_B")
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A", "Lib.RC_B"], ["Lib.AoU_1"], "shared rationale")
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", self.diagrams)
        self.assertEqual(body.count("**Asm_A**"), 2)
        self.assertEqual(body.count("shared rationale"), 2)

    def test_assessment_card_shows_pinned_measure_version(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams)
        self.assertEqual(body.count(":bdg-light:`v1`"), 1)

    def test_assessment_card_shows_assessment_asil(self):
        objs = _objs()
        objs["Lib.Asm_A"] = _assessment("Asm_A", ["Lib.RC_A"], ["Lib.AoU_1"], safety="D")
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", self.diagrams)
        self.assertIn("**Asm_A** :bdg-danger:`ASIL D`", body)

    def test_measures_section_shows_record_version(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams)
        self.assertEqual(body.count(":bdg-light:`v3`"), 1)

    def test_measure_without_version_has_no_version_badge(self):
        objs = {"Lib.AoU_Orphan": _objs()["Lib.AoU_Orphan"]}
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", [])
        self.assertIn("**AoU_Orphan**", body)
        self.assertNotIn(":bdg-light:", body)

    def test_unlisted_measure_is_flagged_in_measures_section(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams)
        self.assertEqual(body.count(":bdg-warning:`Not in an assessment`"), 1)

    def test_root_cause_title_is_escaped(self):
        objs = _objs()
        objs["Lib.RC_A"] = _Obj(
            "RC_A",
            "RootCause",
            {"title": "a*b", "diagram": "fta_a.puml", "line": 2, "failure_modes": ["Lib.FM_A"]},
        )
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", self.diagrams)
        self.assertIn(".. grid-item-card:: a\\*b", body)

    def test_global_measures_section_lists_all(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams)
        self.assertIn("Safety Measures\n---------------", body)
        self.assertIn("**AoU_1**", body)
        self.assertIn("**AoU_Orphan**", body)
        self.assertIn(":bdg-info:`Assumption of Use`", body)

    def test_failure_mode_without_root_cause_has_no_root_causes_rubric(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams)
        self.assertIn(".. dropdown:: Lib.FM_Orphan\n", body)
        self.assertEqual(body.count(".. rubric:: Root Causes"), 1)

    def test_no_fault_trees_renders_all_failure_modes_without_fta(self):
        renderer = _FakeRenderer(_objs_no_fta())
        body = fa._build_body(renderer, "Title", "Lib", [])
        self.assertIn(".. dropdown:: Lib.FM_A\n", body)
        self.assertIn(".. dropdown:: Lib.FM_Orphan\n", body)
        self.assertNotIn(".. uml::", body)
        self.assertNotIn("Fault Trees", body)


# Minimal self-contained TRLC model: defines the FailureMode / AoU / CompReq /
# RootCause / RootCauseAssessment types the assembler keys
# on, so main() runs a real TRLCRST parse (catching contract drift the
# _FakeRenderer cannot). Everything lives in one package (unlike the real
# ScoreReq model) since this test does not need to exercise cross-package
# stub imports.
_RSL = """\
package TestSafetyAnalysis

enum Asil {
    QM
    B
    D
}

type ApiMethod {
    signatures String [1..*]
}

type FailureMode {
    guidewords optional String [1..*]
    safety Asil
    interface optional ApiMethod [1..*]
    failureeffect optional String
    description optional String
}

type RootCause {
    title String
    diagram String
    line Integer
    failure_modes FailureMode [1..*]
}

type AoU {
    safety Asil
    description optional String
}

type CompReq {
    safety Asil
    description optional String
}

tuple MeasureId {
    item [AoU, CompReq]
    separator @
    version Integer
}

type RootCauseAssessment {
    root_causes RootCause [1..*]
    measures MeasureId [0..*]
    safety Asil
    rationale Markup_String
    version Integer
}
"""

_FM_TRLC = """\
package TestSafetyAnalysis

FailureMode FmA {
    guidewords = ["TooLate"]
    safety = Asil.D
    interface = [TestSafetyAnalysis.ApiA]
    failureeffect = "downstream timeout"
    description = "fm a description"
}
"""

_API_TRLC = """\
package TestSafetyAnalysis

ApiMethod ApiA {
    signatures = ["void Lib::Api::A()"]
}
"""

_AOU_TRLC = """\
package TestSafetyAnalysis

AoU AouA {
    safety = Asil.D
    description = "aou a description"
}
"""

_COMPREQ_TRLC = """\
package TestSafetyAnalysis

CompReq CrB {
    safety = Asil.D
    description = "cr b description"
}
"""

_ASSESSMENTS_TRLC = """\
package TestSafetyAnalysis

RootCauseAssessment AsmA {
    root_causes = [TestSafetyAnalysis.RcA]
    measures = [TestSafetyAnalysis.AouA@1]
    safety = Asil.D
    rationale = "asm a rationale"
    version = 1
}

RootCauseAssessment AsmB {
    root_causes = [TestSafetyAnalysis.RcB]
    measures = [TestSafetyAnalysis.CrB@1]
    safety = Asil.D
    rationale = "asm b rationale"
    version = 1
}

RootCauseAssessment AsmNotApplicable {
    root_causes = [TestSafetyAnalysis.RcNotApplicable]
    measures = []
    safety = Asil.D
    rationale = "does not apply here"
    version = 1
}
"""

_DUPLICATE_ASSESSMENT = """
RootCauseAssessment AsmDuplicate {
    root_causes = [TestSafetyAnalysis.RcA]
    measures = []
    safety = Asil.D
    rationale = "duplicate"
    version = 1
}
"""

_FTA_TRLC = """\
package TestSafetyAnalysis

RootCause RcA {
    title = "Root cause A"
    diagram = "a.puml"
    line = 2
    failure_modes = [TestSafetyAnalysis.FmA]
}

RootCause RcB {
    title = "Root cause B"
    diagram = "a.puml"
    line = 3
    failure_modes = [TestSafetyAnalysis.FmA]
}

RootCause RcNotApplicable {
    title = "Root cause not applicable"
    diagram = "a.puml"
    line = 4
    failure_modes = [TestSafetyAnalysis.FmA]
}

RootCause RcUnassessed {
    title = "Root cause unassessed"
    diagram = "a.puml"
    line = 5
    failure_modes = [TestSafetyAnalysis.FmA]
}
"""


class MainIntegrationTest(unittest.TestCase):
    """End-to-end main(): real TRLCRST parse of failuremodes + assessments +
    safetymeasures + fta_events.trlc -> safety_analysis.rst."""

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
            api = self._write(tmp, "api.trlc", _API_TRLC)
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            aou = self._write(tmp, "aou.trlc", _AOU_TRLC)
            cr = self._write(tmp, "cr.trlc", _COMPREQ_TRLC)
            asm = self._write(tmp, "asm.trlc", _ASSESSMENTS_TRLC)
            fta = self._write(tmp, "fta_events.trlc", _FTA_TRLC)
            template = self._write(tmp, "tmpl.rst", "{body}\n")
            out = os.path.join(tmp, "safety_analysis.rst")

            self._run_main(
                [
                    "safety_analysis_assembler",
                    "--output",
                    out,
                    "--template",
                    template,
                    "--title",
                    "Test FMEA",
                    "--fta-package",
                    "TestSafetyAnalysis",
                    "--fta-events",
                    fta,
                    "--diagrams",
                    "a.puml",
                    "--failuremodes",
                    fm,
                    "--assessments",
                    asm,
                    "--safetymeasures",
                    aou,
                    cr,
                    "--dep-files",
                    rsl,
                    api,
                ]
            )

            with open(out, encoding="utf-8") as fh:
                rst = fh.read()

            # Title + overview table + FM dropdown + root causes + fault trees + safety measures.
            self.assertIn("Test FMEA", rst)
            self.assertIn("Overview", rst)
            self.assertIn(".. list-table::", rst)
            self.assertIn(".. dropdown:: TestSafetyAnalysis.FmA\n", rst)
            self.assertIn(":name: safety-analysis-testsafetyanalysis-fma", rst)
            self.assertIn(".. grid-item-card:: Description", rst)
            self.assertIn(".. rubric:: Root Causes", rst)
            self.assertIn(".. grid-item-card:: Root cause A", rst)
            self.assertIn(".. uml:: a.puml", rst)
            self.assertIn("Safety Measures", rst)
            # Real rendered record content (proves TRLCRST actually parsed).
            self.assertIn("fm a description", rst)
            self.assertIn("aou a description", rst)
            self.assertIn("asm a rationale", rst)
            # CompReq rendered as a kind-badged control measure.
            self.assertIn("cr b description", rst)
            self.assertIn(":bdg-success:`Control Measure`", rst)
            # The assessments pin both measures at version 1.
            self.assertEqual(rst.count(":bdg-light:`v1`"), 2)
            self.assertIn("**AsmA** :bdg-danger:`ASIL D`", rst)

            # RcA/RcB are assessed with measures, RcNotApplicable without
            # measures, RcUnassessed has no assessment at all.
            self.assertEqual(rst.count(":bdg-secondary:`Not applicable`"), 1)
            self.assertIn("does not apply here", rst)
            self.assertEqual(rst.count(":bdg-danger:`Not assessed`"), 1)
            self.assertEqual(rst.count(":bdg-warning:`Not in an assessment`"), 0)

    def _assemble(self, tmp, failuremodes=(), assessments=(), safetymeasures=(), spec=()):
        rsl = self._write(tmp, "types.rsl", _RSL)
        api = self._write(tmp, "api.trlc", _API_TRLC)
        fta = self._write(tmp, "fta_events.trlc", _FTA_TRLC)
        template = self._write(tmp, "tmpl.rst", "{body}\n")
        argv = [
            "safety_analysis_assembler",
            "--output",
            os.path.join(tmp, "safety_analysis.rst"),
            "--template",
            template,
            "--title",
            "T",
            "--fta-package",
            "TestSafetyAnalysis",
            "--fta-events",
            fta,
            "--failuremodes",
            *failuremodes,
            "--assessments",
            *assessments,
            "--safetymeasures",
            *safetymeasures,
            "--dep-files",
            rsl,
            api,
            *spec,
        ]
        self._run_main(argv)

    def test_root_cause_failure_mode_outside_failure_modes_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            with self.assertRaises(SystemExit) as ctx:
                self._assemble(tmp, spec=[fm])
            self.assertEqual(ctx.exception.code, 1)

    def test_non_assessment_in_assessments_file_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            aou = self._write(tmp, "aou.trlc", _AOU_TRLC)
            with self.assertRaises(SystemExit) as ctx:
                self._assemble(tmp, failuremodes=[fm], assessments=[aou])
            self.assertEqual(ctx.exception.code, 1)

    def test_assessment_asil_below_failure_mode_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            aou = self._write(tmp, "aou.trlc", _AOU_TRLC)
            cr = self._write(tmp, "cr.trlc", _COMPREQ_TRLC)
            asm = self._write(tmp, "asm.trlc", _ASSESSMENTS_TRLC.replace("safety = Asil.D", "safety = Asil.B"))
            with self.assertRaises(SystemExit) as ctx:
                self._assemble(tmp, failuremodes=[fm], assessments=[asm], safetymeasures=[aou, cr])
            self.assertEqual(ctx.exception.code, 1)

    def test_root_cause_in_several_assessments_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            aou = self._write(tmp, "aou.trlc", _AOU_TRLC)
            asm = self._write(tmp, "asm.trlc", _ASSESSMENTS_TRLC + _DUPLICATE_ASSESSMENT)
            with self.assertRaises(SystemExit) as ctx:
                self._assemble(tmp, failuremodes=[fm], assessments=[asm], safetymeasures=[aou])
            self.assertEqual(ctx.exception.code, 1)

    def test_measure_outside_safety_measures_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            aou = self._write(tmp, "aou.trlc", _AOU_TRLC)
            cr = self._write(tmp, "cr.trlc", _COMPREQ_TRLC)
            asm = self._write(tmp, "asm.trlc", _ASSESSMENTS_TRLC)
            # The AoU / CompReq are only parsed for reference resolution, so they are no own measures.
            with self.assertRaises(SystemExit) as ctx:
                self._assemble(tmp, failuremodes=[fm], assessments=[asm], spec=[aou, cr])
            self.assertEqual(ctx.exception.code, 1)

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
                        "--fta-package",
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
                        "--fta-package",
                        "T",
                        "--fta-events",
                        fta,
                    ]
                )
            self.assertEqual(ctx.exception.code, 1)


if __name__ == "__main__":
    unittest.main()
