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
    """FailureMode/Mitigation records plus a single RootCause for ``Lib.FM_A``
    addressed by ``Lib.CM_1``; ``Lib.FM_Orphan``/``Lib.CM_Orphan`` are not
    referenced by any root cause."""
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
        "Lib.RC_A": _Obj(
            "RC_A",
            "RootCause",
            {"title": "Root cause A", "diagram": "fta_a.puml", "line": 2, "failure_modes": ["Lib.FM_A"]},
        ),
    }


_DIAGRAMS = ["fta_a.puml"]


def _objs_no_fta():
    """FailureMode/Mitigation records only, no generated RootCause
    records at all (mirrors an ``safety_analysis()`` target with no ``root_causes`` diagrams)."""
    return {fqn: obj for fqn, obj in _objs().items() if obj.n_typ.name in ("FailureMode", "Mitigation")}


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

    def test_mitigation_in_mitigations_file_is_accepted(self):
        objs = _objs()
        objs["Lib.CM_1"] = _Obj("CM_1", "Mitigation", {"root_causes": ["Lib.RC_A"]}, file_name="m.trlc")
        self.assertEqual(fa._check_inputs(objs, ["m.trlc"]), [])

    def test_non_mitigation_in_mitigations_file_is_reported(self):
        objs = _objs()
        objs["Lib.Aou_1"] = _Obj("Aou_1", "AoU", {"root_causes": ["Lib.RC_A"]}, file_name="m.trlc")
        errors = fa._check_inputs(objs, ["m.trlc"])
        self.assertEqual(len(errors), 1)
        self.assertIn("AoU Lib.Aou_1", errors[0])

    def test_non_mitigation_outside_mitigations_file_is_accepted(self):
        objs = _objs()
        objs["Lib.Aou_1"] = _Obj("Aou_1", "AoU", {"root_causes": ["Lib.RC_A"]}, file_name="a.trlc")
        self.assertEqual(fa._check_inputs(objs, ["m.trlc"]), [])


class BuildBodyTest(unittest.TestCase):
    def setUp(self):
        self.renderer = _FakeRenderer(_objs())
        self.diagrams = _DIAGRAMS

    def test_overview_and_failure_mode_section_rendered(self):
        body = fa._build_body(self.renderer, "Title", "Lib", self.diagrams)
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
        # Root Causes rubric with one card per root cause: title, source, measures.
        self.assertIn(".. rubric:: Root Causes", body)
        self.assertIn(".. grid-item-card:: Root cause A", body)
        self.assertIn("Source: :ref:`fta_a.puml <safety-analysis-lib-diagram-fta-a-puml>`, line 2", body)
        # Measure card: bold ID with inline kind + ASIL badges in the card body (no card title).
        self.assertIn(".. grid-item-card::\n", body)
        self.assertIn("**CM_1**", body)
        self.assertIn(":bdg-warning:`Mitigation`", body)
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

    def test_root_cause_without_measure_is_flagged(self):
        objs = _objs()
        objs["Lib.RC_B"] = _Obj(
            "RC_B",
            "RootCause",
            {"title": "Root cause B", "diagram": "fta_a.puml", "line": 3, "failure_modes": ["Lib.FM_A"]},
        )
        body = fa._build_body(_FakeRenderer(objs), "Title", "Lib", self.diagrams)
        self.assertEqual(body.count(":bdg-danger:`No safety measure`"), 1)

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
        self.assertIn("**CM_1**", body)
        self.assertIn("**CM_Orphan**", body)
        self.assertIn(":bdg-warning:`Mitigation`", body)

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


# Minimal self-contained TRLC model: defines the FailureMode / SafetyMeasure /
# Mitigation / AoU / CompReq / RootCause types the assembler keys
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

type RootCause {
    title String
    diagram String
    line Integer
    failure_modes FailureMode [1..*]
}

abstract type SafetyMeasure {
    safety optional String
    description optional String
    root_causes RootCause [1..*]
}

type Mitigation extends SafetyMeasure {
}

type AoU extends SafetyMeasure {
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

_COMPREQ_TRLC = """\
package TestSafetyAnalysis

CompReq CrB {
    description = "cr b description"
    derived_from = [TestSafetyAnalysis.RcB@1]
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

RootCause RcUncovered {
    title = "Root cause uncovered"
    diagram = "a.puml"
    line = 4
    failure_modes = [TestSafetyAnalysis.FmA]
}
"""


class MainIntegrationTest(unittest.TestCase):
    """End-to-end main(): real TRLCRST parse of failuremodes + safetymeasures +
    fta_events.trlc -> safety_analysis.rst."""

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
            cr = self._write(tmp, "cr.trlc", _COMPREQ_TRLC)
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
                    "--mitigations",
                    cm,
                    "--safetymeasures",
                    cr,
                    "--dep-files",
                    rsl,
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
            self.assertIn("cm a description", rst)
            # CompReq rendered as a kind-badged control measure.
            self.assertIn("cr b description", rst)
            self.assertIn(":bdg-success:`Control Measure`", rst)

            # RcA is addressed by the Mitigation, RcB by the CompReq; RcUncovered
            # by neither and is the only root cause flagged.
            self.assertEqual(rst.count(":bdg-danger:`No safety measure`"), 1)

    def _assemble(self, tmp, failuremodes=(), mitigations=(), spec=()):
        rsl = self._write(tmp, "types.rsl", _RSL)
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
            "--mitigations",
            *mitigations,
            "--dep-files",
            rsl,
            *spec,
        ]
        self._run_main(argv)

    def test_root_cause_failure_mode_outside_failure_modes_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            with self.assertRaises(SystemExit) as ctx:
                self._assemble(tmp, spec=[fm])
            self.assertEqual(ctx.exception.code, 1)

    def test_non_mitigation_in_mitigations_file_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            fm = self._write(tmp, "fm.trlc", _FM_TRLC)
            aou = self._write(
                tmp,
                "aou.trlc",
                "package TestSafetyAnalysis\n\nAoU AouA {\n    root_causes = [TestSafetyAnalysis.RcA]\n}\n",
            )
            with self.assertRaises(SystemExit) as ctx:
                self._assemble(tmp, failuremodes=[fm], mitigations=[aou])
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
