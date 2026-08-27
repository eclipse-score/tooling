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
"""Assemble a failure-mode-centric ``fmea.rst`` page.

The page is pivoted around the safety chain: an overview summary table followed
by one section per failure mode, each containing the failure-mode detail, the
fault-tree diagram inline, and a "Control Measures" subsection holding only that
chain's basic events.  Failure modes and control measures not referenced by any
fault tree are appended under trailing "Unlinked …" sections so nothing is
dropped.

A single in-process TRLC parse (via the extended ``TRLCRST`` library) backs the
whole page — no per-record Bazel actions and no ``.inc`` splitting.
"""

import argparse
import dataclasses
import json
import logging
import re
import sys

from trlc_rst import TRLCRST, TRLCParseError

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# LOCAL, UNCOMMITTED WORKAROUND -- DO NOT SUBMIT
#
# trlc (Bazel module, pinned at v3.0.0 in MODULE.bazel) ships a TRLCRST class
# that only implements parse_trlc_files()/convert_symbols_to_tree()/
# render_to_file(). The functions below reimplement the
# objects_by_fqn()/field_value_for()/render_table_to_string() API that the
# rest of this file expects, driven directly by the already-parsed TRLC
# Symbol_Table (renderer._symbols), so the fmea() pipeline can be exercised
# end-to-end locally. Remove once the real trlc_rst gains this API (or once
# this module is rewritten against the shipped one).
# ---------------------------------------------------------------------------


def _objects_by_fqn(self):
    return {
        obj.fully_qualified_name(): obj for obj in self._symbols.iter_record_objects()
    }


def _field_value_for(self, fqn, field_name):
    obj = self.objects_by_fqn().get(fqn)
    if obj is None:
        return None
    field = obj.field.get(field_name)
    value = field.to_python_object() if field is not None else None
    if isinstance(value, str) and hasattr(field, "has_references") and field.has_references:
        return self._resolve_markup_references(value, field.references)
    if isinstance(value, list):
        return ", ".join(str(v) for v in value)
    return value


def _render_table_to_string(self, columns, fqns, name_header="Name", link_fn=None):
    obj_map = self.objects_by_fqn()
    headers = [name_header] + list(columns.values())

    def _cell(value):
        if isinstance(value, list):
            return ", ".join(str(v) for v in value)
        return "" if value is None else str(value)

    def _row_lines(cells):
        lines = [f"   * - {cells[0]}"]
        lines += [f"     - {cell}" for cell in cells[1:]]
        return lines

    lines = [".. list-table::", "   :header-rows: 1", ""]
    lines += _row_lines(headers)
    for fqn in fqns:
        obj = obj_map.get(fqn)
        if obj is None:
            continue
        fields = obj.to_python_dict()
        name_cell = link_fn(fqn, obj.name) if link_fn else obj.name
        row = [name_cell] + [_cell(fields.get(name)) for name in columns]
        lines += _row_lines(row)
    return "\n".join(lines) + "\n"


TRLCRST.objects_by_fqn = _objects_by_fqn
TRLCRST.field_value_for = _field_value_for
TRLCRST.render_table_to_string = _render_table_to_string

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

# Global Control Measures summary table columns.
_CM_TABLE_COLUMNS = {"safety": "ASIL", "description": "Description"}
# Overview summary table columns (one row per failure mode).
_FM_TABLE_COLUMNS = {
    "guidewords": "Guideword",
    "safety": "ASIL",
    "interface": "Interface",
}

_OVERVIEW_TITLE = "Overview"
_FAILURE_MODES_TITLE = "Failure Modes"
_CONTROL_MEASURES_TITLE = "Control Measures"
_ROOT_CAUSE_TITLE = "Root Cause Analysis"

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
    return "fmea-" + re.sub(r"[^0-9a-zA-Z]+", "-", fqn).strip("-").lower()


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
    guidewords = fields.get("guidewords")
    if guidewords:
        guideword_text = (
            ", ".join(str(g) for g in guidewords) if isinstance(guidewords, list) else str(guidewords)
        )
        items.append(_grid_item(_badge(_GUIDEWORD_BADGE, guideword_text), {"class": "sd-text-center"}))
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


def _cm_card(renderer: TRLCRST, fqn: str, obj: object) -> _Directive:
    """One control-measure card: bold ID with inline ASIL badge, then description.

    Both the ID line and the description are direct text content of the card
    (not nested grids), so they align at the same indentation as the Description
    card body above.
    """
    safety = obj.to_python_dict().get("safety", "")
    badge_str = " " + _badge(_ASIL_BADGE.get(safety, _DEFAULT_BADGE), f"ASIL {safety}") if safety else ""
    header_text = f"**{obj.name}**{badge_str}"
    description = renderer.field_value_for(fqn, "description")
    body: list = [header_text]
    if description:
        body.append(description)
    return _card("", body)


def _cm_grid(renderer: TRLCRST, obj_map: dict, cms: list[str]) -> _Directive | None:
    cards = [_cm_card(renderer, fqn, obj_map[fqn]) for fqn in cms if fqn in obj_map]
    return _grid(cards, columns=1) if cards else None


def _fm_dropdown(renderer: TRLCRST, fqn: str, obj: object, chain: dict | None) -> _Directive:
    """One collapsible failure-mode dropdown.

    *chain* is ``None`` for an orphan failure mode (no fault tree); the Root
    Cause Analysis and Control Measures parts are then omitted.
    """
    body = [_attr_grid(obj), _description_card(renderer, fqn)]
    if chain is not None:
        body.append(_Directive("rubric", _ROOT_CAUSE_TITLE))
        body.append(_Directive("uml", chain["puml"]))
        cms = _cm_grid(renderer, renderer.objects_by_fqn(), chain.get("control_measures", []))
        if cms is not None:
            body.append(_Directive("rubric", _CONTROL_MEASURES_TITLE))
            body.append(cms)
    return _Directive("dropdown", fqn, {"name": _anchor(fqn)}, body)


# ---------------------------------------------------------------------------
# Section renderers — top-level page sections (return RST strings)
# ---------------------------------------------------------------------------


def _validate_chain(chain) -> str:
    """Return a chain's ``fm_fqn`` or raise ``ValueError`` if it is malformed."""
    try:
        fqn = chain["fm_fqn"]
        chain["puml"]
    except (KeyError, TypeError) as exc:
        raise ValueError(f"Malformed chain entry {chain!r} in fta_chains.json: missing required key {exc}") from exc
    return fqn


def _render_overview(renderer: TRLCRST, fm_fqns: list[str]) -> str:
    if not fm_fqns:
        return ""
    table = renderer.render_table_to_string(_FM_TABLE_COLUMNS, fqns=fm_fqns, name_header="Failure Mode", link_fn=_ref)
    return _heading(_OVERVIEW_TITLE, "-") + "\n" + table


def _render_failure_modes(renderer: TRLCRST, chains: list, obj_map: dict, fm_fqns: list[str]) -> str:
    dropdowns = []
    linked = set()
    for chain in chains:
        fqn = _validate_chain(chain)
        if fqn in obj_map:
            linked.add(fqn)
            dropdowns.append(_fm_dropdown(renderer, fqn, obj_map[fqn], chain))
        else:
            logger.warning(
                "fta_chains.json references unknown FailureMode %r; no matching TRLC record — chain skipped",
                fqn,
            )
    # Orphan failure modes (no fault tree) still render, without FTA / CMs.
    for fqn in fm_fqns:
        if fqn not in linked:
            dropdowns.append(_fm_dropdown(renderer, fqn, obj_map[fqn], None))
    body = "\n\n".join(d.render() for d in dropdowns)
    return _heading(_FAILURE_MODES_TITLE, "-") + "\n" + body + "\n"


def _render_control_measures(renderer: TRLCRST, cm_fqns: list[str]) -> str:
    if not cm_fqns:
        return ""
    table = renderer.render_table_to_string(_CM_TABLE_COLUMNS, fqns=cm_fqns, name_header="Control Measure")
    return _heading(_CONTROL_MEASURES_TITLE, "-") + "\n" + table


def _build_body(renderer: TRLCRST, chains: list, title: str) -> str:
    obj_map = renderer.objects_by_fqn()
    fm_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "FailureMode"]
    cm_fqns = [fqn for fqn, obj in obj_map.items() if obj.n_typ.name == "ControlMeasure"]

    sections = [
        _heading(title, "="),
        _render_overview(renderer, fm_fqns),
        _render_failure_modes(renderer, chains, obj_map, fm_fqns),
        _render_control_measures(renderer, cm_fqns),
    ]
    return "\n".join(s for s in sections if s)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, help="Output fmea.rst path.")
    parser.add_argument("--template", required=True, help="RST template path.")
    parser.add_argument("--title", required=True, help="Page title.")
    parser.add_argument(
        "--chains",
        required=True,
        help="fta_chains.json produced by puml_cli FTA mode.",
    )
    parser.add_argument("--failuremodes", nargs="*", default=[], help="FailureMode .trlc files.")
    parser.add_argument(
        "--controlmeasures",
        nargs="*",
        default=[],
        help="ControlMeasure .trlc files.",
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

    try:
        with open(args.chains, encoding="utf-8") as fh:
            chains = json.load(fh)
    except (OSError, json.JSONDecodeError) as exc:
        logger.error("Error reading chains file %s: %s", args.chains, exc)
        sys.exit(1)

    source_files = list(args.failuremodes) + list(args.controlmeasures)
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

    body = _build_body(renderer, chains, args.title)

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
