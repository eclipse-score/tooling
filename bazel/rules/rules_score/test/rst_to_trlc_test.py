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

"""Unit tests for the RST-to-TRLC converter.

Tests are structured around the four S-CORE requirement types:
  - assumed_system_req  →  ScoreReq.AssumedSystemReq
  - feat_req                       →  ScoreReq.FeatReq
  - comp_req                       →  ScoreReq.CompReq
  - aou_req                        →  ScoreReq.AoU
"""

import tempfile
import unittest
from io import StringIO
from pathlib import Path

from rst_to_trlc import (
    _ALLOWED_RST_ATTRS,
    _collect_body,
    _collect_refs,
    _escape,
    convert,
    DIRECTIVE_TO_TRLC,
    parse_directives,
    render_trlc,
    SAFETY_MAP,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _rst(*lines: str) -> str:
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# DIRECTIVE_TO_TRLC – only S-CORE types are present
# ---------------------------------------------------------------------------


class TestDirectiveToTrlc(unittest.TestCase):
    def test_assumed_system_req(self):
        self.assertEqual(
            DIRECTIVE_TO_TRLC["assumed_system_req"], "ScoreReq.AssumedSystemReq"
        )

    def test_feat_req(self):
        self.assertEqual(DIRECTIVE_TO_TRLC["feat_req"], "ScoreReq.FeatReq")

    def test_comp_req(self):
        self.assertEqual(DIRECTIVE_TO_TRLC["comp_req"], "ScoreReq.CompReq")

    def test_aou_req(self):
        self.assertEqual(DIRECTIVE_TO_TRLC["aou_req"], "ScoreReq.AoU")

    def test_non_score_directives_not_present(self):
        """Directives outside the S-CORE model must not appear in the map."""
        for alias in (
            "asr_req",
            "feature_req",
            "component_req",
            "assumption_of_use",
            "comp_saf_fmea",
            "comp_arc_sta",
            "stkh_req",
        ):
            self.assertNotIn(alias, DIRECTIVE_TO_TRLC, f"{alias} should not be mapped")


# ---------------------------------------------------------------------------
# ALLOWED_RST_ATTRS – whitelist is complete and minimal
# ---------------------------------------------------------------------------


class TestAllowedRstAttrs(unittest.TestCase):
    def test_required_attrs_present(self):
        for attr in (
            "id",
            "safety",
            "satisfies",
            "derived_from",
            "rationale",
            "version",
        ):
            self.assertIn(attr, _ALLOWED_RST_ATTRS)

    def test_sphinx_only_attrs_absent(self):
        """Sphinx-needs-only attributes must not be in the whitelist."""
        for attr in (
            "reqtype",
            "security",
            "valid_from",
            "valid_until",
            "belongs_to",
            "tags",
            "fulfils",
            "mitigates",
            "status",
        ):
            self.assertNotIn(attr, _ALLOWED_RST_ATTRS, f"{attr} should be ignored")


# ---------------------------------------------------------------------------
# SAFETY_MAP – only QM, B, D exist in ScoreReq.Asil
# ---------------------------------------------------------------------------


class TestSafetyMap(unittest.TestCase):
    def test_qm(self):
        self.assertEqual(SAFETY_MAP["QM"], "ScoreReq.Asil.QM")

    def test_asil_b(self):
        self.assertEqual(SAFETY_MAP["ASIL_B"], "ScoreReq.Asil.B")

    def test_asil_d(self):
        self.assertEqual(SAFETY_MAP["ASIL_D"], "ScoreReq.Asil.D")

    def test_asil_a(self):
        self.assertEqual(SAFETY_MAP["ASIL_A"], "ScoreReq.Asil.A")

    def test_asil_c(self):
        self.assertEqual(SAFETY_MAP["ASIL_C"], "ScoreReq.Asil.C")

    def test_all_five_levels_present(self):
        """All five ASIL levels must be present in the map."""
        for key in ("QM", "ASIL_A", "ASIL_B", "ASIL_C", "ASIL_D"):
            self.assertIn(key, SAFETY_MAP)


# ---------------------------------------------------------------------------
# parse_directives – S-CORE directive recognition
# ---------------------------------------------------------------------------


class TestParseDirectives(unittest.TestCase):
    # --- Assumed System Requirements ---

    def test_parses_assumed_system_req(self):
        rst = _rst(
            ".. assumed_system_req:: Minimal Interface",
            "   :id: asr_req__test__001",
            "   :safety: ASIL_B",
            "   :status: valid",
            "",
            "   The system shall provide a minimal interface.",
        )
        result = parse_directives(rst)
        self.assertEqual(len(result), 1)
        item = result[0]
        self.assertEqual(item["directive"], "assumed_system_req")
        self.assertEqual(item["title"], "Minimal Interface")
        self.assertEqual(item["fields"]["id"], "asr_req__test__001")
        self.assertEqual(item["fields"]["safety"], "ASIL_B")
        self.assertIn("minimal interface", item["body"])

    def test_parses_assumed_system_req_with_rationale(self):
        rst = _rst(
            ".. assumed_system_req:: With Rationale",
            "   :id: asr_req__test__003",
            "   :safety: QM",
            "   :rationale: Needed for safety analysis.",
            "",
            "   The system shall do something.",
        )
        result = parse_directives(rst)
        self.assertEqual(
            result[0]["fields"]["rationale"], "Needed for safety analysis."
        )

    # --- Feature Requirements ---

    def test_parses_feat_req(self):
        rst = _rst(
            ".. feat_req:: Mock Interface",
            "   :id: feat_req__test__001",
            "   :safety: ASIL_B",
            "   :satisfies: asr_req__test__001",
            "",
            "   The component shall provide a mock interface.",
        )
        result = parse_directives(rst)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["directive"], "feat_req")
        self.assertEqual(result[0]["fields"]["satisfies"], "asr_req__test__001")

    def test_parses_feat_req_with_derived_from(self):
        """derived_from is an alternative RST field name for satisfies."""
        rst = _rst(
            ".. feat_req:: Another Feature",
            "   :id: feat_req__test__002",
            "   :safety: ASIL_B",
            "   :derived_from: asr_req__test__001",
            "",
            "   Body.",
        )
        result = parse_directives(rst)
        self.assertEqual(result[0]["fields"]["derived_from"], "asr_req__test__001")

    # --- Component Requirements ---

    def test_parses_comp_req(self):
        rst = _rst(
            ".. comp_req:: Return Value",
            "   :id: comp_req__test__001",
            "   :safety: ASIL_B",
            "   :satisfies: feat_req__test__001",
            "",
            "   The function shall return 42.",
        )
        result = parse_directives(rst)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["directive"], "comp_req")

    def test_parses_comp_req_without_satisfies(self):
        """satisfies is optional for comp_req."""
        rst = _rst(
            ".. comp_req:: Standalone",
            "   :id: comp_req__test__002",
            "   :safety: QM",
            "",
            "   Body.",
        )
        result = parse_directives(rst)
        self.assertEqual(len(result), 1)
        self.assertNotIn("satisfies", result[0]["fields"])

    # --- Assumptions of Use ---

    def test_parses_aou_req(self):
        rst = _rst(
            ".. aou_req:: Operating Conditions",
            "   :id: aou_req__test__001",
            "   :safety: ASIL_B",
            "",
            "   The SEooC shall operate within defined conditions.",
        )
        result = parse_directives(rst)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["directive"], "aou_req")

    # --- General behaviour ---

    def test_ignores_non_score_directives(self):
        rst = _rst(
            ".. image:: diagram.png",
            ".. note::",
            "   Some note.",
            "",
            ".. feat_req:: Real Req",
            "   :id: feat_req__test__001",
            "   :safety: QM",
            "",
            "   Body.",
        )
        result = parse_directives(rst)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["directive"], "feat_req")

    def test_ignores_stkh_req(self):
        """stkh_req has no corresponding TRLC type and must be ignored."""
        rst = _rst(
            ".. stkh_req:: Platform Requirement",
            "   :id: stkh_req__platform__001",
            "   :safety: ASIL_B",
            "",
            "   Body.",
        )
        result = parse_directives(rst)
        self.assertEqual(result, [])

    def test_returns_empty_for_plain_rst(self):
        self.assertEqual(parse_directives("Plain RST text without directives.\n"), [])

    def test_parses_multiple_directives(self):
        rst = _rst(
            ".. assumed_system_req:: First",
            "   :id: asr_req__test__001",
            "   :safety: QM",
            "",
            "   First body.",
            "",
            ".. feat_req:: Second",
            "   :id: feat_req__test__001",
            "   :safety: ASIL_B",
            "   :satisfies: asr_req__test__001",
            "",
            "   Second body.",
        )
        result = parse_directives(rst)
        self.assertEqual(len(result), 2)
        self.assertEqual(result[0]["directive"], "assumed_system_req")
        self.assertEqual(result[1]["directive"], "feat_req")

    def test_sphinx_only_fields_collected_but_not_in_whitelist(self):
        """Fields like reqtype/security are collected but must not affect TRLC output."""
        rst = _rst(
            ".. feat_req:: Has Sphinx Fields",
            "   :id: feat_req__test__001",
            "   :reqtype: Functional",
            "   :security: NO",
            "   :safety: ASIL_B",
            "   :valid_from: v0.0.1",
            "   :belongs_to: feat__some_feature",
            "   :satisfies: asr_req__test__001",
            "",
            "   Body.",
        )
        result = parse_directives(rst)
        fields = result[0]["fields"]
        # Fields are parsed from RST...
        self.assertIn("reqtype", fields)
        self.assertIn("security", fields)
        # ...but none of the non-whitelisted ones appear in _ALLOWED_RST_ATTRS
        for key in ("reqtype", "security", "valid_from", "belongs_to"):
            self.assertNotIn(key, _ALLOWED_RST_ATTRS)

    def test_strips_rst_bold_markup_from_body(self):
        rst = _rst(
            ".. assumed_system_req:: Markup",
            "   :id: asr_req__test__001",
            "   :safety: QM",
            "",
            "   The system shall be **robust** and **fast**.",
        )
        result = parse_directives(rst)
        self.assertNotIn("**", result[0]["body"])
        self.assertIn("robust", result[0]["body"])


# ---------------------------------------------------------------------------
# render_trlc – TRLC output for each S-CORE type
# ---------------------------------------------------------------------------


class TestRenderTrlc(unittest.TestCase):
    def _single(self, directive, fields=None, body="The component shall do something."):
        base = {"id": f"{directive}__test__001", "safety": "ASIL_B"}
        if fields:
            base.update(fields)
        return [{"directive": directive, "title": "Test", "fields": base, "body": body}]

    # --- AssumedSystemReq ---

    def test_assumed_system_req_produces_type(self):
        out = render_trlc(self._single("assumed_system_req"), "Pkg", "")
        self.assertIn("ScoreReq.AssumedSystemReq", out)

    def test_assumed_system_req_adds_rationale_placeholder_when_absent(self):
        out = render_trlc(self._single("assumed_system_req"), "Pkg", "")
        self.assertIn('rationale   = "TODO: add rationale"', out)

    def test_assumed_system_req_uses_rationale_from_rst_when_present(self):
        items = self._single(
            "assumed_system_req", {"rationale": "Needed for ISO 26262 compliance."}
        )
        out = render_trlc(items, "Pkg", "")
        self.assertIn('"Needed for ISO 26262 compliance."', out)
        self.assertNotIn("TODO: add rationale", out)

    def test_assumed_system_req_rationale_is_escaped(self):
        items = self._single("assumed_system_req", {"rationale": 'Has "quotes"'})
        out = render_trlc(items, "Pkg", "")
        self.assertIn('\\"quotes\\"', out)

    # --- FeatReq ---

    def test_feat_req_produces_feat_req_type(self):
        out = render_trlc(self._single("feat_req"), "Pkg", "")
        self.assertIn("ScoreReq.FeatReq", out)

    def test_feat_req_no_rationale(self):
        out = render_trlc(self._single("feat_req"), "Pkg", "")
        self.assertNotIn("rationale", out)

    def test_feat_req_satisfies_maps_to_derived_from(self):
        items = self._single("feat_req", {"satisfies": "asr_req__test__001"})
        out = render_trlc(items, "FeatPkg", "AsrPkg")
        self.assertIn("derived_from", out)
        self.assertIn("AsrPkg.asr_req__test__001@1", out)

    def test_feat_req_derived_from_field_also_works(self):
        items = self._single("feat_req", {"derived_from": "asr_req__test__001"})
        out = render_trlc(items, "FeatPkg", "AsrPkg")
        self.assertIn("AsrPkg.asr_req__test__001@1", out)

    def test_feat_req_imports_ref_package_when_refs_present(self):
        items = self._single("feat_req", {"satisfies": "asr_req__test__001"})
        out = render_trlc(items, "FeatPkg", "AsrPkg")
        self.assertIn("import AsrPkg", out)

    # --- CompReq ---

    def test_comp_req_produces_comp_req_type(self):
        out = render_trlc(self._single("comp_req"), "Pkg", "")
        self.assertIn("ScoreReq.CompReq", out)

    def test_comp_req_satisfies_maps_to_derived_from(self):
        items = self._single("comp_req", {"satisfies": "feat_req__test__001"})
        out = render_trlc(items, "CompPkg", "FeatPkg")
        self.assertIn("derived_from", out)
        self.assertIn("FeatPkg.feat_req__test__001@1", out)

    def test_comp_req_without_satisfies_has_no_derived_from(self):
        out = render_trlc(self._single("comp_req"), "Pkg", "")
        self.assertNotIn("derived_from", out)

    # --- AoU ---

    def test_aou_req_produces_aou_type(self):
        out = render_trlc(self._single("aou_req"), "Pkg", "")
        self.assertIn("ScoreReq.AoU", out)

    def test_aou_req_no_rationale(self):
        out = render_trlc(self._single("aou_req"), "Pkg", "")
        self.assertNotIn("rationale", out)

    # --- Ignored Sphinx-only attributes ---

    def test_sphinx_only_fields_do_not_appear_in_output(self):
        """reqtype, security, valid_from, belongs_to must be silently dropped."""
        items = self._single(
            "feat_req",
            {
                "satisfies": "asr_req__test__001",
                "reqtype": "Functional",
                "security": "NO",
                "valid_from": "v0.0.1",
                "valid_until": "v1.0.0",
                "belongs_to": "feat__some_feature",
                "tags": "important",
            },
        )
        out = render_trlc(items, "FeatPkg", "AsrPkg")
        for field in (
            "reqtype",
            "security",
            "valid_from",
            "valid_until",
            "belongs_to",
            "tags",
        ):
            self.assertNotIn(
                field, out, f"'{field}' should be dropped from TRLC output"
            )

    # --- General TRLC structure ---

    def test_output_contains_package_declaration(self):
        out = render_trlc(self._single("assumed_system_req"), "MyPkg", "")
        self.assertIn("package MyPkg", out)

    def test_output_imports_score_req(self):
        out = render_trlc(self._single("assumed_system_req"), "MyPkg", "")
        self.assertIn("import ScoreReq", out)

    def test_no_extra_import_when_no_refs(self):
        out = render_trlc(self._single("assumed_system_req"), "MyPkg", "SomePkg")
        self.assertNotIn("import SomePkg", out)

    def test_safety_asil_a_written_as_a(self):
        items = [
            {
                "directive": "assumed_system_req",
                "title": "T",
                "fields": {"id": "x", "safety": "ASIL_A"},
                "body": "b.",
            }
        ]
        out = render_trlc(items, "P", "")
        self.assertIn("ScoreReq.Asil.A", out)

    def test_description_escapes_double_quotes(self):
        items = [
            {
                "directive": "assumed_system_req",
                "title": "T",
                "fields": {"id": "x", "safety": "QM"},
                "body": 'He said "hi".',
            }
        ]
        out = render_trlc(items, "P", "")
        self.assertIn('\\"hi\\"', out)

    def test_uses_id_as_record_name(self):
        out = render_trlc(self._single("assumed_system_req"), "P", "")
        self.assertIn("assumed_system_req__test__001", out)

    def test_derives_name_from_title_when_no_id(self):
        items = [
            {
                "directive": "assumed_system_req",
                "title": "My Cool Req",
                "fields": {"safety": "QM"},
                "body": "b.",
            }
        ]
        out = render_trlc(items, "P", "")
        self.assertIn("My_Cool_Req", out)

    def test_version_defaults_to_1(self):
        out = render_trlc(self._single("assumed_system_req"), "P", "")
        self.assertIn("version     = 1", out)

    def test_empty_list_produces_header_only(self):
        out = render_trlc([], "EmptyPkg", "")
        self.assertIn("package EmptyPkg", out)
        self.assertNotIn("description", out)


# ---------------------------------------------------------------------------
# _collect_refs – only satisfies and derived_from are cross-references
# ---------------------------------------------------------------------------


class TestCollectRefs(unittest.TestCase):
    def test_satisfies_is_a_ref_field(self):
        self.assertEqual(
            _collect_refs({"satisfies": "asr_req__test__001"}), ["asr_req__test__001"]
        )

    def test_derived_from_is_a_ref_field(self):
        self.assertEqual(
            _collect_refs({"derived_from": "asr_req__test__001"}),
            ["asr_req__test__001"],
        )

    def test_comma_separated_refs(self):
        self.assertEqual(
            _collect_refs({"satisfies": "req_001, req_002"}),
            ["req_001", "req_002"],
        )

    def test_fulfils_is_not_a_ref_field(self):
        """fulfils is not part of the S-CORE process and must not produce refs."""
        self.assertEqual(_collect_refs({"fulfils": "some_req"}), [])

    def test_mitigates_is_not_a_ref_field(self):
        """mitigates is a String field on AoU/CompReq, not a cross-reference."""
        self.assertEqual(_collect_refs({"mitigates": "some_req"}), [])

    def test_returns_empty_when_no_ref_fields(self):
        self.assertEqual(_collect_refs({"safety": "QM", "reqtype": "Functional"}), [])


# ---------------------------------------------------------------------------
# _collect_body – blank-line handling consistency
# ---------------------------------------------------------------------------


class TestCollectBody(unittest.TestCase):
    def test_collects_indented_body(self):
        lines = ["   Body text.", ""]
        body, _ = _collect_body(lines, 0)
        self.assertEqual(body, "Body text.")

    def test_stops_at_unindented_line(self):
        lines = ["   Indented.", "Unindented."]
        body, i = _collect_body(lines, 0)
        self.assertEqual(body, "Indented.")
        self.assertEqual(i, 1)

    def test_interior_blank_line_no_double_space(self):
        lines = ["   First.", "", "   Second.", ""]
        body, _ = _collect_body(lines, 0)
        self.assertNotIn("  ", body)
        self.assertIn("First.", body)
        self.assertIn("Second.", body)

    def test_early_return_and_normal_exit_consistent(self):
        """Both return paths must filter empty parts identically."""
        lines_early = ["   A.", "", "Unindented"]
        body_early, _ = _collect_body(lines_early, 0)

        lines_normal = ["   A.", "Unindented"]
        body_normal, _ = _collect_body(lines_normal, 0)

        self.assertEqual(body_early, body_normal)


# ---------------------------------------------------------------------------
# _escape
# ---------------------------------------------------------------------------


class TestEscape(unittest.TestCase):
    def test_double_quotes(self):
        self.assertEqual(_escape('say "hi"'), 'say \\"hi\\"')

    def test_backslashes(self):
        self.assertEqual(_escape("a\\b"), "a\\\\b")

    def test_no_change_on_clean_string(self):
        self.assertEqual(_escape("hello"), "hello")


# ---------------------------------------------------------------------------
# convert – integration across all S-CORE types
# ---------------------------------------------------------------------------


class TestConvert(unittest.TestCase):
    def _convert(self, rst_content: str, filename="req.rst", **kwargs) -> str:
        with tempfile.TemporaryDirectory() as tmpdir:
            src = Path(tmpdir) / filename
            src.write_text(rst_content, encoding="utf-8")
            out = Path(tmpdir) / (src.stem + ".trlc")
            convert(src, out, **kwargs)
            return out.read_text(encoding="utf-8")

    def test_assumed_system_req_round_trip(self):
        rst = (
            ".. assumed_system_req:: Min Interface\n"
            "   :id: asr_req__test__001\n"
            "   :safety: ASIL_B\n"
            "\n"
            "   The system shall provide a minimal interface.\n"
        )
        out = self._convert(rst)
        self.assertIn("ScoreReq.AssumedSystemReq", out)
        self.assertIn("asr_req__test__001", out)
        self.assertIn("ScoreReq.Asil.B", out)
        self.assertIn("rationale", out)

    def test_assumed_system_req_with_rationale_from_rst(self):
        rst = (
            ".. assumed_system_req:: Min Interface\n"
            "   :id: asr_req__test__001\n"
            "   :safety: ASIL_B\n"
            "   :rationale: Required by ISO 26262.\n"
            "\n"
            "   The system shall provide a minimal interface.\n"
        )
        out = self._convert(rst)
        self.assertIn("Required by ISO 26262.", out)
        self.assertNotIn("TODO: add rationale", out)

    def test_feat_req_with_satisfies(self):
        rst = (
            ".. feat_req:: Mock Interface\n"
            "   :id: feat_req__test__001\n"
            "   :safety: ASIL_B\n"
            "   :satisfies: asr_req__test__001\n"
            "\n"
            "   The component shall provide a mock interface.\n"
        )
        out = self._convert(rst, ref_package="AsrPkg")
        self.assertIn("ScoreReq.FeatReq", out)
        self.assertIn("AsrPkg.asr_req__test__001@1", out)

    def test_feat_req_sphinx_attrs_not_in_output(self):
        rst = (
            ".. feat_req:: Full Template\n"
            "   :id: feat_req__test__001\n"
            "   :reqtype: Functional\n"
            "   :security: NO\n"
            "   :safety: ASIL_B\n"
            "   :satisfies: asr_req__test__001\n"
            "   :valid_from: v0.0.1\n"
            "   :belongs_to: feat__some_feature\n"
            "   :status: invalid\n"
            "\n"
            "   The component shall provide a mock interface.\n"
        )
        out = self._convert(rst, ref_package="AsrPkg")
        for field in ("reqtype", "security", "valid_from", "belongs_to", "status"):
            self.assertNotIn(field, out)

    def test_comp_req_round_trip(self):
        rst = (
            ".. comp_req:: Return Value\n"
            "   :id: comp_req__test__001\n"
            "   :safety: ASIL_B\n"
            "   :satisfies: feat_req__test__001\n"
            "\n"
            "   The function shall return 42.\n"
        )
        out = self._convert(rst, ref_package="FeatPkg")
        self.assertIn("ScoreReq.CompReq", out)
        self.assertIn("FeatPkg.feat_req__test__001@1", out)

    def test_aou_req_round_trip(self):
        rst = (
            ".. aou_req:: Operating Conditions\n"
            "   :id: aou_req__test__001\n"
            "   :safety: ASIL_B\n"
            "\n"
            "   The SEooC shall operate within defined conditions.\n"
        )
        out = self._convert(rst)
        self.assertIn("ScoreReq.AoU", out)
        self.assertNotIn("rationale", out)

    def test_package_name_derived_from_file_stem(self):
        rst = (
            ".. assumed_system_req:: Pkg Test\n"
            "   :id: asr_req__test__001\n"
            "   :safety: QM\n"
            "\n"
            "   Body.\n"
        )
        out = self._convert(rst, filename="my_requirements.rst")
        self.assertIn("package MyRequirements", out)

    def test_empty_rst_warns_and_returns_zero(self):
        import sys

        buf = StringIO()
        old_stderr, sys.stderr = sys.stderr, buf
        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                src = Path(tmpdir) / "empty.rst"
                src.write_text("No directives here.\n", encoding="utf-8")
                count = convert(src, Path(tmpdir) / "empty.trlc")
        finally:
            sys.stderr = old_stderr
        self.assertEqual(count, 0)
        self.assertIn("WARNING", buf.getvalue())

    def test_stkh_req_is_skipped(self):
        rst = (
            ".. stkh_req:: Platform Requirement\n"
            "   :id: stkh_req__platform__001\n"
            "   :safety: ASIL_B\n"
            "\n"
            "   The platform shall do something.\n"
        )
        import sys

        buf = StringIO()
        old_stderr, sys.stderr = sys.stderr, buf
        try:
            out = self._convert(rst)
        finally:
            sys.stderr = old_stderr
        # stkh_req has no TRLC mapping → treated as no directives found
        self.assertIn("WARNING", buf.getvalue())
        self.assertNotIn("stkh_req", out)


if __name__ == "__main__":
    unittest.main()
