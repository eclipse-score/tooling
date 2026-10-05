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
card per root cause (``RootCause``) of that failure mode, holding the measures
(``Mitigation``, ``AoU``, ``CompReq``) that address it.  A "Fault Trees" section
shows every fault-tree diagram once.  Failure modes without a root cause and
measures not referencing any generated root cause still render, so nothing is
dropped.

A fault-tree root cause is addressed by any of three measure kinds: a
``Mitigation`` or ``AoU`` referencing it directly via their ``root_causes``
field, or a ``CompReq`` referencing it as one item in its ``derived_from`` (a
control measure).

All links (``RootCause`` -> ``FailureMode``, measure -> ``RootCause``) are
regular, strongly-typed TRLC references (see ``puml_fta::render_trlc_stub``), so
a single in-process TRLC parse (via the extended ``TRLCRST`` library) backs the
whole page.  Coverage of the links is checked by lobster, not here.
"""

import argparse
import dataclasses
import logging
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

# Overview summary table columns (one row per failure mode).
_FM_TABLE_COLUMNS = {
    "guideword": "Guideword",
    "safety": "ASIL",
    "interface": "Interface",
}

_OVERVIEW_TITLE = "Overview"
_FAILURE_MODES_TITLE = "Failure Modes"
_FAULT_TREES_TITLE = "Fault Trees"
_SAFETY_MEASURES_TITLE = "Safety Measures"
_ROOT_CAUSES_TITLE = "Root Causes"

# TRLC record types that count as a measure addressing a fault-tree root
# cause (RootCause): Mitigation/AoU via their own root_causes field, CompReq
# indirectly via a RootCause item inside its derived_from.
_MEASURE_KINDS = ("Mitigation", "AoU", "CompReq")

# Measure kind -> sphinx-design badge role and human-readable label.
_MEASURE_KIND_BADGE = {
    "Mitigation": "bdg-warning",
    "AoU": "bdg-info",
    "CompReq": "bdg-success",
}
_MEASURE_KIND_LABEL = {
    "Mitigation": "Mitigation",
    "AoU": "Assumption of Use",
    "CompReq": "Control Measure",
}

# ASIL value -> sphinx-design badge role (severity-coloured).
_ASIL_BADGE = {
    "QM": "bdg-secondary",
    "B": "bdg-warning",
    "D": "bdg-danger",
}
_DEFAULT_BADGE = "bdg-secondary"
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


# ---------------------------------------------------------------------------
# Element renderers — each returns a directive node (or None)
# ---------------------------------------------------------------------------


def _attr_grid(obj: object) -> _Directive | None:
    """FM attributes: guideword/ASIL as centred chips (no card chrome),
    interface/failure-effect as titled cards; a gutter separates the rows."""
    fields = obj.to_python_dict()
    items = []
    guideword = fields.get("guideword")
    if guideword:
        items.append(_grid_item(_badge(_GUIDEWORD_BADGE, guideword), {"class": "sd-text-center"}))
    safety = fields.get("safety")
    if safety:
        role = _ASIL_BADGE.get(safety, _DEFAULT_BADGE)
        items.append(_grid_item(_badge(role, f"ASIL {safety}"), {"class": "sd-text-center"}))
    for field_name, label in (
        ("interface", "Interface"),
        ("failureeffect", "Failure Effect"),
    ):
        value = fields.get(field_name)
        if value:
            items.append(_card(label, value))
    return _grid(items, columns=2, gutter=3) if items else None


def _description_card(renderer: TRLCRST, fqn: str) -> _Directive | None:
    """Description as a prominent card in a ``grid:: 1`` so its borders align
    with the attribute grid above."""
    description = renderer.field_value_for(fqn, "description")
    if not description:
        return None
    return _grid([_card("Description", description)], columns=1)


def _measure_card(renderer: TRLCRST, fqn: str, obj: object) -> _Directive:
    """One measure card: bold ID with an inline kind badge and (if present) an
    ASIL badge, then description.

    Both the ID line and the description are direct text content of the card
    (not nested grids), so they align at the same indentation as the Description
    card body above.
    """
    kind = obj.n_typ.name
    fields = obj.to_python_dict()
    kind_badge = _badge(_MEASURE_KIND_BADGE.get(kind, _DEFAULT_BADGE), _MEASURE_KIND_LABEL.get(kind, kind))
    safety = fields.get("safety", "")
    safety_badge = " " + _badge(_ASIL_BADGE.get(safety, _DEFAULT_BADGE), f"ASIL {safety}") if safety else ""
    header_text = f"**{obj.name}** {kind_badge}{safety_badge}"
    description = renderer.field_value_for(fqn, "description")
    body: list = [header_text]
    if description:
        body.append(description)
    return _card("", body)


def _measures_grid(renderer: TRLCRST, obj_map: dict, measures: list[str]) -> _Directive | None:
    cards = [_measure_card(renderer, fqn, obj_map[fqn]) for fqn in measures if fqn in obj_map]
    return _grid(cards, columns=1) if cards else None


def _root_cause_card(renderer: TRLCRST, obj_map: dict, rc_fqn: str, diagrams: dict[str, str]) -> _Directive:
    """One root-cause card: title, source diagram/line, and the measures addressing it."""
    fields = obj_map[rc_fqn].to_python_dict()
    diagram = fields.get("diagram", "")
    source = _ref_text(diagrams[diagram], diagram) if diagram in diagrams else _escape(diagram)
    body: list = [f"Source: {source}, line {fields.get('line')}"]
    measures = _measures_grid(renderer, obj_map, _measures_for_root_cause(obj_map, rc_fqn))
    body.append(measures if measures is not None else _badge("bdg-danger", "No safety measure"))
    return _card(_escape(fields.get("title", "")), body)


def _root_causes_for_failure_mode(obj_map: dict, fm_fqn: str) -> list[str]:
    """``RootCause`` fqns whose ``failure_modes`` contain *fm_fqn*."""
    return [
        fqn
        for fqn, obj in obj_map.items()
        if obj.n_typ.name == "RootCause" and fm_fqn in (obj.to_python_dict().get("failure_modes") or [])
    ]


def _fm_dropdown(renderer: TRLCRST, obj_map: dict, fqn: str, diagrams: dict[str, str]) -> _Directive:
    """One collapsible failure-mode dropdown.

    The Root Causes part is omitted for a failure mode no fault tree covers.
    """
    body = [_attr_grid(obj_map[fqn]), _description_card(renderer, fqn)]
    cards = [_root_cause_card(renderer, obj_map, rc, diagrams) for rc in _root_causes_for_failure_mode(obj_map, fqn)]
    if cards:
        body.append(_Directive("rubric", _ROOT_CAUSES_TITLE))
        body.append(_grid(cards, columns=1))
    return _Directive("dropdown", fqn, {"name": _anchor(fqn)}, body)


# ---------------------------------------------------------------------------
# Section renderers — top-level page sections (return RST strings)
# ---------------------------------------------------------------------------


def _measures_for_root_cause(obj_map: dict, rc_fqn: str) -> list[str]:
    """Mitigation/AoU/CompReq fqns that address *rc_fqn* (a RootCause root cause).

    ``Mitigation`` and ``AoU`` reference root causes directly via their
    ``root_causes`` field.  ``CompReq`` references them indirectly, as one
    item among the FeatReq/AssumedSystemReq/AoU/RootCause union inside its
    ``derived_from`` ``CompReqSourceId`` tuples -- only tuple items that
    resolve to *rc_fqn* count as a control measure for this root cause.
    """
    result = []
    for fqn, obj in obj_map.items():
        kind = obj.n_typ.name
        fields = obj.to_python_dict()
        if kind in ("Mitigation", "AoU"):
            if rc_fqn in (fields.get("root_causes") or []):
                result.append(fqn)
        elif kind == "CompReq":
            sources = fields.get("derived_from") or []
            if any(src.get("item") == rc_fqn for src in sources):
                result.append(fqn)
    return result


def _render_overview(renderer: TRLCRST, fm_fqns: list[str]) -> str:
    if not fm_fqns:
        return ""
    table = renderer.render_table_to_string(_FM_TABLE_COLUMNS, fqns=fm_fqns, name_header="Failure Mode", link_fn=_ref)
    return _heading(_OVERVIEW_TITLE, "-") + "\n" + table


def _render_failure_modes(renderer: TRLCRST, obj_map: dict, fm_fqns: list[str], diagrams: dict[str, str]) -> str:
    dropdowns = [_fm_dropdown(renderer, obj_map, fqn, diagrams) for fqn in fm_fqns]
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


def _render_measures(renderer: TRLCRST, obj_map: dict, measure_fqns: list[str]) -> str:
    if not measure_fqns:
        return ""
    grid = _measures_grid(renderer, obj_map, measure_fqns)
    body = grid.render() if grid is not None else ""
    return _heading(_SAFETY_MEASURES_TITLE, "-") + "\n" + body + "\n"


def _build_body(renderer: TRLCRST, title: str, fta_package: str, diagram_names: list[str]) -> str:
    obj_map = renderer.objects_by_fqn()
    fm_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "FailureMode"]
    measure_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name in _MEASURE_KINDS]
    diagrams = {name: _diagram_anchor(fta_package, name) for name in sorted(set(diagram_names))}

    sections = [
        _heading(title, "="),
        _render_overview(renderer, fm_fqns),
        _render_failure_modes(renderer, obj_map, fm_fqns, diagrams),
        _render_fault_trees(diagrams),
        _render_measures(renderer, obj_map, measure_fqns),
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
        "--safetymeasures",
        nargs="*",
        default=[],
        help="Mitigation/AoU/CompReq .trlc files scoped to this safety_analysis target.",
    )
    parser.add_argument(
        "--spec",
        nargs="*",
        default=[],
        help="TRLC .rsl/.trlc spec files for import resolution.",
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

    source_files = list(args.failuremodes) + list(args.safetymeasures) + [args.fta_events]
    renderer = TRLCRST(
        input_directory=None,
        source_files=source_files,
        dep_files=list(args.spec),
    )
    try:
        renderer.parse_trlc_files()
    except TRLCParseError as exc:
        logger.error("TRLC parse error: %s", exc)
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
