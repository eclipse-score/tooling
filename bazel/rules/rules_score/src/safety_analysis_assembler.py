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
"Root Cause Analysis" block per fault tree (``FtaFailureMode``) that covers it — the
diagram inline plus a "Safety Measures" subsection holding only the measures
(``Mitigation``, ``AoU``, ``CompReq``) that address that tree's root causes.
Failure modes not covered by any fault tree, and measures not referenced by
any generated stub, still render (with an empty root-cause / measures part)
so nothing is dropped.

A fault-tree root cause (``RootCause``) is addressed by any of three measure
kinds: a ``Mitigation`` or ``AoU`` referencing it directly via their
``root_causes`` field, or a ``CompReq`` referencing it as one item in its
``derived_from`` (a control measure).  ``--uncovered-root-causes-output``
emits the set of ``RootCause`` fqns addressed by none of the three, so
``dependability_analysis`` can fail (or warn) on incomplete coverage.

The fault-tree topology (``FtaFailureMode``/``RootCause`` -> ``FailureMode``/
measures) is carried entirely by regular, strongly-typed TRLC references (see
``puml_fta::render_trlc_stub``) rather than an external alias-matched
``fta_chains.json``, so a single in-process TRLC parse (via the extended
``TRLCRST`` library) backs the whole page — no per-record Bazel actions and
no ``.inc`` splitting.
"""

import argparse
import dataclasses
import json
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
_SAFETY_MEASURES_TITLE = "Safety Measures"
_ROOT_CAUSE_TITLE = "Root Cause Analysis"

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
    return f":ref:`{name} <{_anchor(fqn)}>`"


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


def _fm_dropdown(renderer: TRLCRST, fqn: str, obj: object, chains: list[dict]) -> _Directive:
    """One collapsible failure-mode dropdown.

    *chains* holds one entry per fault tree (``FtaFailureMode``) covering this
    failure mode; it is empty for an orphan failure mode (no fault tree), in
    which case the Root Cause Analysis / Safety Measures parts are omitted.
    """
    body = [_attr_grid(obj), _description_card(renderer, fqn)]
    for chain in chains:
        body.append(_Directive("rubric", _ROOT_CAUSE_TITLE))
        body.append(_Directive("uml", chain["puml"]))
        measures = _measures_grid(renderer, renderer.objects_by_fqn(), chain["measures"])
        if measures is not None:
            body.append(_Directive("rubric", _SAFETY_MEASURES_TITLE))
            body.append(measures)
    return _Directive("dropdown", fqn, {"name": _anchor(fqn)}, body)


# ---------------------------------------------------------------------------
# Section renderers — top-level page sections (return RST strings)
# ---------------------------------------------------------------------------


def _root_causes_for_fta_failure_mode(obj_map: dict, ffm_fqn: str) -> list[str]:
    """RootCause fqns whose ``failure_modes`` field references *ffm_fqn*."""
    return [
        fqn
        for fqn, obj in obj_map.items()
        if obj.n_typ.name == "RootCause" and ffm_fqn in (obj.to_python_dict().get("failure_modes") or [])
    ]


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


def _uncovered_root_causes(obj_map: dict) -> list[str]:
    """``RootCause`` fqns addressed by no Mitigation, AoU, or CompReq."""
    rc_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "RootCause"]
    return sorted(fqn for fqn in rc_fqns if not _measures_for_root_cause(obj_map, fqn))


def _chains_for_failure_mode(obj_map: dict, fm_fqn: str) -> list[dict]:
    """One chain entry per ``FtaFailureMode`` covering *fm_fqn*: its diagram plus the
    measures attached (via ``root_causes``/``derived_from``) to its root
    causes."""
    chains = []
    for ffm_fqn, ffm_obj in obj_map.items():
        if ffm_obj.n_typ.name != "FtaFailureMode":
            continue
        if fm_fqn not in (ffm_obj.to_python_dict().get("failure_modes") or []):
            continue
        measures = []
        for rc_fqn in _root_causes_for_fta_failure_mode(obj_map, ffm_fqn):
            for measure_fqn in _measures_for_root_cause(obj_map, rc_fqn):
                if measure_fqn not in measures:
                    measures.append(measure_fqn)
        chains.append(
            {
                "ffm_fqn": ffm_fqn,
                "puml": ffm_obj.to_python_dict()["diagram"],
                "measures": measures,
            }
        )
    return chains


def _render_overview(renderer: TRLCRST, fm_fqns: list[str]) -> str:
    if not fm_fqns:
        return ""
    table = renderer.render_table_to_string(_FM_TABLE_COLUMNS, fqns=fm_fqns, name_header="Failure Mode", link_fn=_ref)
    return _heading(_OVERVIEW_TITLE, "-") + "\n" + table


def _render_failure_modes(renderer: TRLCRST, obj_map: dict, fm_fqns: list[str]) -> str:
    dropdowns = [_fm_dropdown(renderer, fqn, obj_map[fqn], _chains_for_failure_mode(obj_map, fqn)) for fqn in fm_fqns]
    body = "\n\n".join(d.render() for d in dropdowns)
    return _heading(_FAILURE_MODES_TITLE, "-") + "\n" + body + "\n"


def _render_measures(renderer: TRLCRST, obj_map: dict, measure_fqns: list[str]) -> str:
    if not measure_fqns:
        return ""
    grid = _measures_grid(renderer, obj_map, measure_fqns)
    body = grid.render() if grid is not None else ""
    return _heading(_SAFETY_MEASURES_TITLE, "-") + "\n" + body + "\n"


def _build_body(renderer: TRLCRST, title: str) -> str:
    obj_map = renderer.objects_by_fqn()
    fm_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "FailureMode"]
    measure_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name in _MEASURE_KINDS]

    sections = [
        _heading(title, "="),
        _render_overview(renderer, fm_fqns),
        _render_failure_modes(renderer, obj_map, fm_fqns),
        _render_measures(renderer, obj_map, measure_fqns),
    ]
    return "\n".join(s for s in sections if s)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output", default=None, help="Output safety_analysis.rst path (omit for a coverage-check-only run)."
    )
    parser.add_argument("--template", default=None, help="RST template path (required together with --output).")
    parser.add_argument("--title", default="", help="Page title (required together with --output).")
    parser.add_argument(
        "--fta-events",
        nargs="+",
        required=True,
        dest="fta_events",
        help="One or more fta_events.trlc files produced by puml_cli FTA mode (generated "
        "FtaFailureMode/RootCause stubs) -- one per safety_analysis target when combining several for a "
        "dependability_analysis-wide coverage check.",
    )
    parser.add_argument("--failuremodes", nargs="*", default=[], help="FailureMode .trlc files.")
    parser.add_argument(
        "--safetymeasures",
        nargs="*",
        default=[],
        help="Mitigation/AoU/CompReq .trlc files scoped to this safety_analysis target.",
    )
    parser.add_argument(
        "--measures",
        nargs="*",
        default=[],
        help="Additional Mitigation/AoU/CompReq .trlc files not scoped to a single safety_analysis target "
        "(e.g. from a dependability_analysis's own 'measures' attribute).",
    )
    parser.add_argument(
        "--dep-files",
        nargs="*",
        default=[],
        dest="dep_files",
        help="Extra .trlc files needed only for reference resolution (e.g. FeatReq/AssumedSystemReq "
        "records referenced by a CompReq's derived_from) -- not rendered on the page themselves.",
    )
    parser.add_argument(
        "--spec",
        nargs="*",
        default=[],
        help="TRLC .rsl/.trlc spec files for import resolution.",
    )
    parser.add_argument(
        "--uncovered-root-causes-output",
        dest="uncovered_root_causes_output",
        default=None,
        help="Optional path to write a JSON array of RootCause fqns addressed by no "
        "Mitigation, AoU, or CompReq (root-cause coverage completeness check).",
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

    if bool(args.output) != bool(args.template):
        parser.error("--output and --template must be given together")

    source_files = list(args.failuremodes) + list(args.safetymeasures) + list(args.measures) + list(args.fta_events)
    renderer = TRLCRST(
        input_directory=None,
        source_files=source_files,
        dep_files=list(args.spec) + list(args.dep_files),
    )
    try:
        renderer.parse_trlc_files()
    except TRLCParseError as exc:
        logger.error("TRLC parse error: %s", exc)
        sys.exit(1)

    if args.output:
        body = _build_body(renderer, args.title)

        with open(args.template, encoding="utf-8", newline="") as fh:
            template = fh.read()
        if "{body}" not in template:
            logger.error("Template %r does not contain a '{body}' placeholder", args.template)
            sys.exit(1)
        rendered = template.replace("{body}", body)

        with open(args.output, "w", newline="", encoding="utf-8") as fh:
            fh.write(rendered)

    if args.uncovered_root_causes_output:
        uncovered = _uncovered_root_causes(renderer.objects_by_fqn())
        with open(args.uncovered_root_causes_output, "w", encoding="utf-8") as fh:
            json.dump(uncovered, fh, indent=2)
            fh.write("\n")


if __name__ == "__main__":
    main()
