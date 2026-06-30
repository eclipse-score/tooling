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
"""Unit tests for the FMEA page assembler layout logic."""

import json
import os
import sys
import tempfile
import unittest

import fmea_assembler as fa


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

    def render_table_to_string(
        self, columns, fqns=None, name_header="Name", link_fn=None
    ):
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
        "Lib.FM_Orphan": _Obj(
            "FM_Orphan", "FailureMode", {"safety": "QM", "guideword": "TooLate"}
        ),
        "Lib.CM_1": _Obj(
            "CM_1", "ControlMeasure", {"safety": "B", "description": "cm one"}
        ),
        "Lib.CM_Orphan": _Obj("CM_Orphan", "ControlMeasure", {"safety": "D"}),
    }


class AnchorTest(unittest.TestCase):
    def test_anchor_is_sanitised_lowercase(self):
        self.assertEqual(fa._anchor("Lib.FM_A"), "fmea-lib-fm-a")

    def test_ref_targets_anchor(self):
        self.assertEqual(
            fa._ref("Lib.FM_A", "FM_A"),
            ":ref:`FM_A <fmea-lib-fm-a>`",
        )


class BuildBodyTest(unittest.TestCase):
    def setUp(self):
        self.renderer = _FakeRenderer(_objs())
        self.chains = [
            {
                "fm_fqn": "Lib.FM_A",
                "fm_name": "Failure Mode A",
                "puml": "fta_a.puml",
                "control_measures": ["Lib.CM_1"],
            }
        ]

    def test_overview_and_chain_section_rendered(self):
        body = fa._build_body(self.renderer, self.chains, "Title")
        self.assertIn("Title\n=====", body)
        self.assertIn("Overview", body)
        # Top-level grouping sections.
        self.assertIn("Failure Modes\n-------------", body)
        # FM dropdown titled by its full fqn only (no ASIL in the heading).
        self.assertIn(".. dropdown:: Lib.FM_A\n", body)
        self.assertNotIn(".. dropdown:: Lib.FM_A :bdg", body)
        self.assertIn(":name: fmea-lib-fm-a", body)
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
        self.assertIn(".. rubric:: Control Measures", body)
        # CM card: bold ID with inline ASIL badge in the card body (no card title).
        self.assertIn(".. grid-item-card::\n", body)
        self.assertIn("**CM_1**", body)
        self.assertIn(":bdg-warning:`ASIL B`", body)
        # Attribute grid uses a gutter to separate the badge row from the cards.
        self.assertIn(":gutter: 3", body)

    def test_global_control_measures_table_lists_all(self):
        body = fa._build_body(self.renderer, self.chains, "Title")
        self.assertIn("Control Measures\n----------------", body)
        self.assertIn("TABLE[Control Measure]", body)
        self.assertIn("CM_1", body)
        self.assertIn("CM_Orphan", body)

    def test_orphan_failure_mode_rendered_without_fta(self):
        body = fa._build_body(self.renderer, self.chains, "Title")
        # Orphan FM still appears as a dropdown, but with no FTA / RCA rubric.
        self.assertIn(".. dropdown:: Lib.FM_Orphan\n", body)
        self.assertEqual(body.count(".. rubric:: Root Cause Analysis"), 1)

    def test_no_chains_renders_all_failure_modes(self):
        body = fa._build_body(self.renderer, [], "Title")
        self.assertIn(".. dropdown:: Lib.FM_A\n", body)
        self.assertIn(".. dropdown:: Lib.FM_Orphan\n", body)
        self.assertNotIn(".. uml::", body)

    def test_chain_with_unknown_fm_fqn_logs_warning(self):
        chains = [
            {
                "fm_fqn": "Lib.NoSuchFM",
                "fm_name": "Unknown",
                "puml": "x.puml",
                "control_measures": [],
            }
        ]
        import logging as _logging

        with self.assertLogs("fmea_assembler", level=_logging.WARNING) as log:
            body = fa._build_body(self.renderer, chains, "Title")
        self.assertTrue(any("Lib.NoSuchFM" in m for m in log.output))
        # The unknown FM is skipped; the known FMs still render.
        self.assertIn(".. dropdown:: Lib.FM_A\n", body)
        self.assertNotIn(".. dropdown:: Lib.NoSuchFM\n", body)


class ChainValidationTest(unittest.TestCase):
    """A malformed chain entry must fail loudly, not crash with a bare KeyError."""

    def setUp(self):
        self.renderer = _FakeRenderer(_objs())

    def test_chain_missing_fm_fqn_raises_valueerror(self):
        chains = [{"puml": "a.puml", "control_measures": []}]
        with self.assertRaises(ValueError) as ctx:
            fa._build_body(self.renderer, chains, "Title")
        self.assertIn("missing required key", str(ctx.exception))

    def test_chain_missing_puml_raises_valueerror(self):
        chains = [{"fm_fqn": "Lib.FM_A", "control_measures": []}]
        with self.assertRaises(ValueError):
            fa._build_body(self.renderer, chains, "Title")


# Minimal self-contained TRLC model: defines the FailureMode / ControlMeasure
# types the assembler keys on, so main() runs a real TRLCRST parse (catching
# contract drift the _FakeRenderer cannot).
_RSL = """\
package TestFmea

type FailureMode {
    guideword optional String
    safety optional String
    interface optional String
    failureeffect optional String
    description optional String
}

type ControlMeasure {
    safety optional String
    description optional String
}
"""

_FM_TRLC = """\
package TestFmea

FailureMode FmA {
    guideword = "TooLate"
    safety = "ASIL_D"
    interface = "Lib.Api"
    failureeffect = "downstream timeout"
    description = "fm a description"
}
"""

_CM_TRLC = """\
package TestFmea

ControlMeasure CmA {
    safety = "ASIL_D"
    description = "cm a description"
}
"""


class MainIntegrationTest(unittest.TestCase):
    """End-to-end main(): real TRLCRST parse + chains JSON -> fmea.rst."""

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
            cm = self._write(tmp, "cm.trlc", _CM_TRLC)
            template = self._write(tmp, "tmpl.rst", "{body}\n")
            chains = self._write(
                tmp,
                "chains.json",
                json.dumps(
                    [
                        {
                            "fm_fqn": "TestFmea.FmA",
                            "fm_name": "Fm A",
                            "puml": "a.puml",
                            "control_measures": ["TestFmea.CmA"],
                        }
                    ]
                ),
            )
            out = os.path.join(tmp, "fmea.rst")

            self._run_main(
                [
                    "fmea_assembler",
                    "--output",
                    out,
                    "--template",
                    template,
                    "--title",
                    "Test FMEA",
                    "--chains",
                    chains,
                    "--failuremodes",
                    fm,
                    "--controlmeasures",
                    cm,
                    "--spec",
                    rsl,
                ]
            )

            with open(out, encoding="utf-8") as fh:
                rst = fh.read()

            # Title + overview table + FM dropdown + inline FTA + control measures.
            self.assertIn("Test FMEA", rst)
            self.assertIn("Overview", rst)
            self.assertIn(".. list-table::", rst)
            self.assertIn(".. dropdown:: TestFmea.FmA\n", rst)
            self.assertIn(":name: fmea-testfmea-fma", rst)
            self.assertIn(".. grid-item-card:: Description", rst)
            self.assertIn(".. rubric:: Root Cause Analysis", rst)
            self.assertIn(".. uml:: a.puml", rst)
            self.assertIn("Control Measures", rst)
            # Real rendered record content (proves TRLCRST actually parsed).
            self.assertIn("fm a description", rst)
            self.assertIn("cm a description", rst)

    def test_malformed_chains_json_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            template = self._write(tmp, "tmpl.rst", "{body}\n")
            bad = self._write(tmp, "chains.json", "{ this is not json")
            out = os.path.join(tmp, "fmea.rst")
            with self.assertRaises(SystemExit) as ctx:
                self._run_main(
                    [
                        "fmea_assembler",
                        "--output",
                        out,
                        "--template",
                        template,
                        "--title",
                        "T",
                        "--chains",
                        bad,
                    ]
                )
            self.assertEqual(ctx.exception.code, 1)

    def test_missing_body_placeholder_exits_nonzero(self):
        with tempfile.TemporaryDirectory() as tmp:
            # Template without the required {body} placeholder.
            template = self._write(tmp, "tmpl.rst", "no placeholder here\n")
            chains = self._write(tmp, "chains.json", "[]")
            out = os.path.join(tmp, "fmea.rst")
            with self.assertRaises(SystemExit) as ctx:
                self._run_main(
                    [
                        "fmea_assembler",
                        "--output",
                        out,
                        "--template",
                        template,
                        "--title",
                        "T",
                        "--chains",
                        chains,
                    ]
                )
            self.assertEqual(ctx.exception.code, 1)


if __name__ == "__main__":
    unittest.main()
