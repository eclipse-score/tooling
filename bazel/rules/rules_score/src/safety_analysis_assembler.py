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
"""Assemble a failure-mode-centric ``safety_analysis.rst`` page.

The page is pivoted around the safety chain: an overview summary table followed
by one section per failure mode, each containing the failure-mode detail and one
card per root cause (``RootCause``) of that failure mode, holding the rationale
and the measures (``AoU``, ``CompReq``) of its ``RootCauseAssessment``.  A
"Fault Trees" section shows every fault-tree diagram once.  Failure modes
without a root cause and measures no assessment lists still render, so nothing
is dropped.

Each root cause is listed by exactly one ``RootCauseAssessment`` (``root_causes``
field), which names the measures addressing it (``measures`` field: an ``AoU``
or a ``CompReq`` and its version).  A root cause without an assessment renders
as "Not assessed"; an assessment without measures as "Not applicable".

All links (``RootCause`` -> ``FailureMode``, assessment -> ``RootCause`` /
measure) are regular, strongly-typed TRLC references (see
``puml_fta::render_trlc_stub``), so a single in-process TRLC parse (via the
extended ``TRLCRST`` library) backs the whole page.  Coverage of the links is
checked by lobster; this tool only rejects inconsistent inputs.
"""

import argparse
import dataclasses
import logging
import os
import re
import sys

from trlc_rst import TRLCRST, TRLCParseError

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

# Overview summary table (one row per failure mode): header and field of each
# column after the failure-mode name.
_OVERVIEW_COLUMNS = (
    ("Guidewords", "guidewords"),
    ("ASIL", "safety"),
    ("Interface", "interface"),
)

_OVERVIEW_TITLE = "Overview"
_FAILURE_MODES_TITLE = "Failure Modes"
_FAULT_TREES_TITLE = "Fault Trees"
_SAFETY_MEASURES_TITLE = "Safety Measures"
_ROOT_CAUSES_TITLE = "Root Causes"

# TRLC record types an assessment lists as measures addressing a root cause.
_MEASURE_KINDS = ("AoU", "CompReq")

# Measure kind -> sphinx-design badge role and human-readable label.
_MEASURE_KIND_BADGE = {
    "AoU": "bdg-info",
    "CompReq": "bdg-success",
}
_MEASURE_KIND_LABEL = {
    "AoU": "Assumption of Use",
    "CompReq": "Control Measure",
}

# ASIL value -> sphinx-design badge role (severity-coloured).
_ASIL_BADGE = {
    "QM": "bdg-secondary",
    "B": "bdg-warning",
    "D": "bdg-danger",
}
# ASIL values from lowest to highest.
_ASIL_ORDER = ("QM", "B", "D")
_DEFAULT_BADGE = "bdg-secondary"
_VERSION_BADGE = "bdg-light"
_GUIDEWORD_BADGE = "bdg-info"


def _heading(text: str, char: str) -> str:
    return f"{text}\n{char * len(text)}\n"


def _indent(text: str, n: int = 3) -> str:
    """Indent every non-empty line of *text* by *n* spaces (for nesting under a
    directive); blank lines are kept empty."""
    pad = " " * n
    return "\n".join(pad + line if line.strip() else "" for line in text.splitlines())


def _anchor(fqn: str) -> str:
    """Sphinx cross-reference label derived from a fully-qualified name."""
    return "safety-analysis-" + re.sub(r"[^0-9a-zA-Z]+", "-", fqn).strip("-").lower()


def _ref(fqn: str, name: str) -> str:
    return _ref_text(_anchor(fqn), name)


def _ref_text(anchor: str, name: str) -> str:
    return f":ref:`{name} <{anchor}>`"


def _diagram_anchor(fta_package: str, diagram: str) -> str:
    """Label for a fault-tree diagram; scoped by package so equal basenames never clash."""
    return _anchor(f"{fta_package} diagram {diagram}")


def _escape(text: str) -> str:
    """Escape RST inline-markup characters in free text (e.g. a root-cause title)."""
    return re.sub(r"([\\*`_|])", r"\\\1", text)


@dataclasses.dataclass
class _Directive:
    """A renderable RST / sphinx-design directive node.

    Indentation and blank-line separation are handled centrally in
    :meth:`render`, so element builders stay declarative trees instead of
    hand-concatenated strings.  ``body`` items may be nested ``_Directive``\\ s
    or raw RST string blocks (e.g. a trlc_rst-rendered table or description).
    """

    name: str
    arg: str = ""
    options: dict = dataclasses.field(default_factory=dict)
    body: list = dataclasses.field(default_factory=list)

    def render(self) -> str:
        lines = [f".. {self.name}::" + (f" {self.arg}" if self.arg else "")]
        for key, value in self.options.items():
            lines.append(f"   :{key}: {value}")
        blocks = [b for b in (_render_block(x) for x in self.body if x) if b]
        if blocks:
            lines.append("")
            lines.append(_indent("\n\n".join(blocks)))
        return "\n".join(lines)


def _render_block(block) -> str:
    """Render a nested directive node or a raw RST string block."""
    if isinstance(block, _Directive):
        return block.render()
    return str(block).rstrip("\n")


# --- sphinx-design element builders (declarative; no manual indentation) ----


def _grid(items: list, columns: int = 2, gutter: int | None = None) -> _Directive:
    options = {} if gutter is None else {"gutter": gutter}
    return _Directive("grid", str(columns), options, list(items))


def _grid_item(body, options: dict | None = None) -> _Directive:
    """A bare grid cell (no card chrome)."""
    return _Directive("grid-item", options=options or {}, body=[body])


def _card(title: str, body) -> _Directive:
    """A ``grid-item-card``; *title* may be empty for a header-less card."""
    return _Directive("grid-item-card", title, body=body if isinstance(body, list) else [body])


def _badge(role: str, text: str) -> str:
    return f":{role}:`{text}`"


def _asil_badge(safety: str) -> str:
    return _badge(_ASIL_BADGE.get(safety, _DEFAULT_BADGE), f"ASIL {safety}")


# ---------------------------------------------------------------------------
# Record queries
# ---------------------------------------------------------------------------


def _measures_of_assessment(assessment: object) -> list[str]:
    """Fqns of the measures an assessment lists (``MeasureId.item``)."""
    return [m.get("item") for m in assessment.to_python_dict().get("measures") or []]


def _pinned_versions(assessment: object) -> dict[str, int]:
    """``{measure fqn: version}`` an assessment pins (``MeasureId.version``)."""
    return {m.get("item"): m.get("version") for m in assessment.to_python_dict().get("measures") or []}


def _assessments_by_root_cause(obj_map: dict) -> dict[str, list[str]]:
    """``{root cause fqn: [assessment fqn, ...]}`` over all ``RootCauseAssessment`` records."""
    result: dict[str, list[str]] = {}
    for fqn, obj in obj_map.items():
        if obj.n_typ.name != "RootCauseAssessment":
            continue
        for rc_fqn in dict.fromkeys(obj.to_python_dict().get("root_causes") or []):
            result.setdefault(rc_fqn, []).append(fqn)
    return result


# ---------------------------------------------------------------------------
# Element renderers — each returns a directive node (or None)
# ---------------------------------------------------------------------------


def _code_list(values: list[str]) -> str:
    """*values* (record names, enum literals) as comma-separated literals."""
    return ", ".join(f"``{value}``" for value in values)


def _attr_grid(obj: object) -> _Directive | None:
    """FM attributes: guidewords/ASIL as centred chips (no card chrome),
    interface/failure-effect as titled cards; a gutter separates the rows.
    """
    fields = obj.to_python_dict()
    items = []
    guidewords = fields.get("guidewords")
    if guidewords:
        chips = " ".join(_badge(_GUIDEWORD_BADGE, guideword) for guideword in guidewords)
        items.append(_grid_item(chips, {"class": "sd-text-center"}))
    safety = fields.get("safety")
    if safety:
        items.append(_grid_item(_asil_badge(safety), {"class": "sd-text-center"}))
    interface = fields.get("interface")
    if interface:
        items.append(_card("Interface", _code_list(interface)))
    failure_effect = fields.get("failureeffect")
    if failure_effect:
        items.append(_card("Failure Effect", failure_effect))
    return _grid(items, columns=2, gutter=3) if items else None


def _description_card(renderer: TRLCRST, fqn: str) -> _Directive | None:
    """Description as a prominent card in a ``grid:: 1`` so its borders align
    with the attribute grid above."""
    description = renderer.field_value_for(fqn, "description")
    if not description:
        return None
    return _grid([_card("Description", description)], columns=1)


def _measure_card(
    renderer: TRLCRST, fqn: str, obj: object, unused: bool = False, version: int | None = None
) -> _Directive:
    """One measure card: bold ID with an inline kind badge, an ASIL badge and a
    version badge (if present), then description.

    Both the ID line and the description are direct text content of the card
    (not nested grids), so they align at the same indentation as the Description
    card body above.  An *unused* measure, which no assessment lists, is flagged.
    *version* is the version an assessment pins; it defaults to the record's own.
    """
    kind = obj.n_typ.name
    fields = obj.to_python_dict()
    kind_badge = _badge(_MEASURE_KIND_BADGE.get(kind, _DEFAULT_BADGE), _MEASURE_KIND_LABEL.get(kind, kind))
    safety = fields.get("safety", "")
    safety_badge = " " + _asil_badge(safety) if safety else ""
    shown_version = version if version is not None else fields.get("version")
    version_badge = " " + _badge(_VERSION_BADGE, f"v{shown_version}") if shown_version is not None else ""
    unused_badge = " " + _badge("bdg-warning", "Not in an assessment") if unused else ""
    header_text = f"**{obj.name}** {kind_badge}{safety_badge}{version_badge}{unused_badge}"
    description = renderer.field_value_for(fqn, "description")
    body: list = [header_text]
    if description:
        body.append(description)
    return _card("", body)


def _measures_grid(
    renderer: TRLCRST,
    obj_map: dict,
    measures: list[str],
    unused: set[str] | None = None,
    versions: dict[str, int] | None = None,
) -> _Directive | None:
    cards = [
        _measure_card(renderer, fqn, obj_map[fqn], fqn in (unused or ()), (versions or {}).get(fqn))
        for fqn in measures
        if fqn in obj_map
    ]
    return _grid(cards, columns=1) if cards else None


def _assessment_blocks(renderer: TRLCRST, obj_map: dict, assessment_fqn: str | None) -> list:
    """Card body for the assessment of a root cause: name, rationale and measures."""
    if assessment_fqn is None:
        return [_badge("bdg-danger", "Not assessed")]
    assessment = obj_map[assessment_fqn]
    measures = _measures_of_assessment(assessment)
    header = f"**{assessment.name}** {_asil_badge(assessment.to_python_dict()['safety'])}"
    if not measures:
        header += " " + _badge("bdg-secondary", "Not applicable")
    return [
        header,
        renderer.field_value_for(assessment_fqn, "rationale"),
        _measures_grid(renderer, obj_map, measures, versions=_pinned_versions(assessment)),
    ]


def _root_cause_card(
    renderer: TRLCRST, obj_map: dict, rc_fqn: str, diagrams: dict[str, str], assessments: dict[str, str]
) -> _Directive:
    """One root-cause card: title, source diagram/line, and its assessment."""
    fields = obj_map[rc_fqn].to_python_dict()
    diagram = fields.get("diagram", "")
    source = _ref_text(diagrams[diagram], diagram) if diagram in diagrams else _escape(diagram)
    body: list = [f"Source: {source}, line {fields.get('line')}"]
    body.extend(_assessment_blocks(renderer, obj_map, assessments.get(rc_fqn)))
    return _card(_escape(fields.get("title", "")), body)


def _root_causes_for_failure_mode(obj_map: dict, fm_fqn: str) -> list[str]:
    """``RootCause`` fqns whose ``failure_modes`` contain *fm_fqn*."""
    return [
        fqn
        for fqn, obj in obj_map.items()
        if obj.n_typ.name == "RootCause" and fm_fqn in (obj.to_python_dict().get("failure_modes") or [])
    ]


def _fm_dropdown(
    renderer: TRLCRST, obj_map: dict, fqn: str, diagrams: dict[str, str], assessments: dict[str, str]
) -> _Directive:
    """One collapsible failure-mode dropdown.

    The Root Causes part is omitted for a failure mode no fault tree covers.
    """
    body = [_attr_grid(obj_map[fqn]), _description_card(renderer, fqn)]
    cards = [
        _root_cause_card(renderer, obj_map, rc, diagrams, assessments)
        for rc in _root_causes_for_failure_mode(obj_map, fqn)
    ]
    if cards:
        body.append(_Directive("rubric", _ROOT_CAUSES_TITLE))
        body.append(_grid(cards, columns=1))
    return _Directive("dropdown", fqn, {"name": _anchor(fqn)}, body)


# ---------------------------------------------------------------------------
# Section renderers — top-level page sections (return RST strings)
# ---------------------------------------------------------------------------


def _render_overview(obj_map: dict, fm_fqns: list[str]) -> str:
    if not fm_fqns:
        return ""
    lines = [".. list-table::", "   :header-rows: 1", "", "   * - Failure Mode"]
    lines.extend(f"     - {header}" for header, _ in _OVERVIEW_COLUMNS)
    for fqn in fm_fqns:
        obj = obj_map[fqn]
        fields = obj.to_python_dict()
        lines.append(f"   * - {_ref(fqn, obj.name)}")
        for _, field_name in _OVERVIEW_COLUMNS:
            value = fields.get(field_name) or ""
            lines.append(f"     - {_code_list(value) if isinstance(value, list) else value}")
    lines.append("")
    return _heading(_OVERVIEW_TITLE, "-") + "\n" + "\n".join(lines) + "\n"


def _render_failure_modes(
    renderer: TRLCRST, obj_map: dict, fm_fqns: list[str], diagrams: dict[str, str], assessments: dict[str, str]
) -> str:
    dropdowns = [_fm_dropdown(renderer, obj_map, fqn, diagrams, assessments) for fqn in fm_fqns]
    body = "\n\n".join(d.render() for d in dropdowns)
    return _heading(_FAILURE_MODES_TITLE, "-") + "\n" + body + "\n"


def _render_fault_trees(diagrams: dict[str, str]) -> str:
    if not diagrams:
        return ""
    parts = [_heading(_FAULT_TREES_TITLE, "-")]
    for diagram, anchor in diagrams.items():
        label = f".. _{anchor}:"
        parts.append(f"{label}\n\n{_heading(diagram, '~')}\n{_Directive('uml', diagram).render()}\n")
    return "\n".join(parts)


def _render_measures(renderer: TRLCRST, obj_map: dict, measure_fqns: list[str], unused: set[str]) -> str:
    if not measure_fqns:
        return ""
    grid = _measures_grid(renderer, obj_map, measure_fqns, unused)
    body = grid.render() if grid is not None else ""
    return _heading(_SAFETY_MEASURES_TITLE, "-") + "\n" + body + "\n"


def _safety(obj: object) -> str:
    return obj.to_python_dict()["safety"]


def _asil_errors(fqn: str, assessment: object, obj_map: dict) -> list[str]:
    """ASIL of *assessment* below a failure mode of its root causes or above a measure.

    References missing from *obj_map* are reported by ``_check_inputs`` itself.
    """
    fields = assessment.to_python_dict()
    safety = fields["safety"]
    rank = _ASIL_ORDER.index(safety)
    errors = []
    for rc_fqn in fields.get("root_causes") or []:
        root_cause = obj_map.get(rc_fqn)
        for fm_fqn in (root_cause.to_python_dict().get("failure_modes") or []) if root_cause else []:
            fm = obj_map.get(fm_fqn)
            if fm is not None and _ASIL_ORDER.index(_safety(fm)) > rank:
                errors.append(
                    f"RootCauseAssessment {fqn} has ASIL {safety}, below ASIL {_safety(fm)} "
                    f"of failure mode {fm_fqn} of root cause {rc_fqn}"
                )
    for measure_fqn in _measures_of_assessment(assessment):
        measure = obj_map.get(measure_fqn)
        if measure is not None and _ASIL_ORDER.index(_safety(measure)) < rank:
            errors.append(
                f"RootCauseAssessment {fqn} has ASIL {safety}, above ASIL {_safety(measure)} of measure {measure_fqn}"
            )
    return errors


def _check_inputs(obj_map: dict, assessment_files: list[str]) -> list[str]:
    """Input errors.

    * a root cause whose failure mode is not rendered;
    * a record that is not a ``RootCauseAssessment`` in an ``--assessments`` file;
    * an assessment root cause that no fault tree generates;
    * an assessment measure that is not an own AoU / CompReq of ``safety_measures``;
    * an assessment ASIL below that of a failure mode of its root causes or above that of a measure;
    * a root cause listed by more than one assessment.

    Only records of the source files are in *obj_map*; a record parsed for
    reference resolution only (failure mode, measure of a dependency) is
    therefore reported.
    """
    errors = []
    fm_fqns = {fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "FailureMode"}
    rc_fqns = {fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "RootCause"}
    measure_fqns = {fqn for fqn, obj in obj_map.items() if obj.n_typ.name in _MEASURE_KINDS}
    assessment_paths = {os.path.abspath(f) for f in assessment_files}
    for fqn, obj in obj_map.items():
        kind = obj.n_typ.name
        if kind != "RootCauseAssessment" and os.path.abspath(obj.location.file_name) in assessment_paths:
            errors.append(f"{kind} {fqn} is in an assessments file; only RootCauseAssessment records belong there")
            continue
        if kind == "RootCause":
            for fm_fqn in obj.to_python_dict().get("failure_modes") or []:
                if fm_fqn not in fm_fqns:
                    errors.append(f"RootCause {fqn} references failure mode {fm_fqn}, which is not in failure_modes")
        elif kind == "RootCauseAssessment":
            for rc_fqn in obj.to_python_dict().get("root_causes") or []:
                if rc_fqn not in rc_fqns:
                    errors.append(
                        f"RootCauseAssessment {fqn} references root cause {rc_fqn}, "
                        "which is not generated by the fault_trees target"
                    )
            for measure_fqn in _measures_of_assessment(obj):
                if measure_fqn not in measure_fqns:
                    errors.append(
                        f"RootCauseAssessment {fqn} lists measure {measure_fqn}, "
                        "which is not an AoU or CompReq of the safety_measures targets"
                    )
            errors.extend(_asil_errors(fqn, obj, obj_map))
    for rc_fqn, assessment_fqns in _assessments_by_root_cause(obj_map).items():
        if len(assessment_fqns) > 1:
            errors.append(f"RootCause {rc_fqn} is listed by several assessments: {', '.join(sorted(assessment_fqns))}")
    return errors


def _build_body(renderer: TRLCRST, title: str, fta_package: str, diagram_names: list[str]) -> str:
    obj_map = renderer.objects_by_fqn()
    fm_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "FailureMode"]
    measure_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name in _MEASURE_KINDS]
    diagrams = {name: _diagram_anchor(fta_package, name) for name in sorted(set(diagram_names))}
    by_root_cause = _assessments_by_root_cause(obj_map)
    assessments = {rc_fqn: fqns[0] for rc_fqn, fqns in by_root_cause.items()}
    listed = {
        measure_fqn
        for fqn, obj in obj_map.items()
        if obj.n_typ.name == "RootCauseAssessment"
        for measure_fqn in _measures_of_assessment(obj)
    }

    sections = [
        _heading(title, "="),
        _render_overview(obj_map, fm_fqns),
        _render_failure_modes(renderer, obj_map, fm_fqns, diagrams, assessments),
        _render_fault_trees(diagrams),
        _render_measures(renderer, obj_map, measure_fqns, set(measure_fqns) - listed),
    ]
    return "\n".join(s for s in sections if s)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, help="Output safety_analysis.rst path.")
    parser.add_argument("--template", required=True, help="RST template path (must contain a '{body}' placeholder).")
    parser.add_argument("--title", required=True, help="Page title.")
    parser.add_argument(
        "--fta-package",
        required=True,
        dest="fta_package",
        help="TRLC package of the generated RootCause records; scopes the diagram labels.",
    )
    parser.add_argument(
        "--fta-events",
        required=True,
        dest="fta_events",
        help="fta_events.trlc produced by puml_cli FTA mode (the generated RootCause records).",
    )
    parser.add_argument(
        "--diagrams",
        nargs="*",
        default=[],
        help="Basenames of the fault-tree .puml diagrams staged next to the page; each is shown once.",
    )
    parser.add_argument("--failuremodes", nargs="*", default=[], help="FailureMode .trlc files.")
    parser.add_argument(
        "--assessments",
        nargs="*",
        default=[],
        help="RootCauseAssessment .trlc files; any other record type in them is an error.",
    )
    parser.add_argument(
        "--safetymeasures",
        nargs="*",
        default=[],
        help="AoU/CompReq .trlc files of the measure targets scoped to this safety_analysis target.",
    )
    parser.add_argument(
        "--dep-files",
        nargs="*",
        default=[],
        dest="dep_files",
        help="TRLC .rsl/.trlc files parsed for import resolution only; their records are not rendered.",
    )
    parser.add_argument(
        "--log-level",
        choices=["error", "warn", "info", "debug"],
        default="warn",
        dest="log_level",
        help="Log level for tool output (default: warn).",
    )
    args = parser.parse_args()

    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    source_files = list(args.failuremodes) + list(args.assessments) + list(args.safetymeasures) + [args.fta_events]
    renderer = TRLCRST(
        input_directory=None,
        source_files=source_files,
        dep_files=list(args.dep_files),
    )
    try:
        renderer.parse_trlc_files()
    except TRLCParseError as exc:
        logger.error("TRLC parse error: %s", exc)
        sys.exit(1)

    errors = _check_inputs(renderer.objects_by_fqn(), args.assessments)
    if errors:
        for error in errors:
            logger.error("%s", error)
        sys.exit(1)

    body = _build_body(renderer, args.title, args.fta_package, args.diagrams)

    with open(args.template, encoding="utf-8", newline="") as fh:
        template = fh.read()
    if "{body}" not in template:
        logger.error("Template %r does not contain a '{body}' placeholder", args.template)
        sys.exit(1)
    rendered = template.replace("{body}", body)

    with open(args.output, "w", newline="", encoding="utf-8") as fh:
        fh.write(rendered)


if __name__ == "__main__":
    main()
