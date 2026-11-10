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
"""Convert Sphinx-Needs ``needs.json`` exports to LOBSTER requirement items.

Counterpart of ``lobster-trlc`` for requirements written as Sphinx-Needs
directives: every need of the selected type (``--need-type``) becomes one
``lobster-req-trace`` item, so the rest of the LOBSTER pipeline cannot tell it
apart from a TRLC requirement. ``--satisfied-by`` keeps only the needs whose
``satisfied_by`` link names the given feature or component. The central
configuration mirrors the ``lobster-trlc`` YAML::

    conversion-rules:
      - need-type: comp_req          # Sphinx-Needs type to convert
        kind: CompReq                # LOBSTER kind (the TRLC record type)
        namespace: req               # optional, default "req"
        version-field: version       # optional, appended as "@<version>"
        description-fields: content  # optional, default "content", falls back to the title
        status-field: status         # optional, copied to the LOBSTER status
        exclude-status: [invalid]    # optional, needs with these statuses are skipped
        tags:                        # link fields that become tracing targets
          - derived_from
          - field: covers
            namespace: req
        justification-up-fields: []  # optional, also -down- and -global-

Link values may carry a version filter (``feat_req__x[version==2]``); it is
translated to ``req feat_req__x@2`` like the ``$(item)@$(version)`` to-string
rules of ``lobster-trlc``. Optionally an RST page listing the converted needs is
written for the dependable element documentation.
"""

from __future__ import annotations

import argparse
import json
import logging
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml
from lobster.common.io import lobster_write
from lobster.common.items import Requirement, Tracing_Tag
from lobster.common.location import File_Reference

GENERATOR = "lobster_sphinx_needs"
FRAMEWORK = "Sphinx-Needs"

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

_RULE_KEYS = {
    "need-type",
    "kind",
    "namespace",
    "version-field",
    "description-fields",
    "status-field",
    "exclude-status",
    "tags",
    "justification-up-fields",
    "justification-down-fields",
    "justification-global-fields",
}

_RE_LINK = re.compile(r"^(?P<id>[^\[\]\s]+)\s*(?:\[(?P<filter>.*)\])?$")
_RE_FILTER_VERSION = re.compile(r"\bversion\s*==\s*(\d+)")
_RE_RST_SPECIAL = re.compile(r"([\\*`|_])")


class ConfigError(Exception):
    """Invalid conversion configuration or needs.json input."""


@dataclass(frozen=True)
class TagEntry:
    field: str
    namespace: str = "req"


@dataclass
class ConversionRule:
    need_type: str
    kind: str
    namespace: str = "req"
    version_field: str | None = None
    description_fields: list[str] = field(default_factory=lambda: ["content"])
    status_field: str | None = None
    exclude_status: list[str] = field(default_factory=list)
    tags: list[TagEntry] = field(default_factory=list)
    justification_up_fields: list[str] = field(default_factory=list)
    justification_down_fields: list[str] = field(default_factory=list)
    justification_global_fields: list[str] = field(default_factory=list)


def _as_list(value: Any, key: str) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [value]
    if isinstance(value, list) and all(isinstance(v, str) for v in value):
        return value
    raise ConfigError(f"'{key}' must be a string or a list of strings")


def _parse_tag(entry: Any) -> TagEntry:
    if isinstance(entry, str):
        return TagEntry(entry)
    if isinstance(entry, dict) and isinstance(entry.get("field"), str) and set(entry) <= {"field", "namespace"}:
        return TagEntry(entry["field"], entry.get("namespace", "req"))
    raise ConfigError(f"invalid tags entry {entry!r}")


def _parse_rule(raw: Any) -> ConversionRule:
    if not isinstance(raw, dict):
        raise ConfigError(f"conversion rule must be a mapping, got {raw!r}")
    unknown = set(raw) - _RULE_KEYS
    if unknown:
        raise ConfigError(f"unknown conversion rule keys: {sorted(unknown)}")
    for key in ("need-type", "kind"):
        if not isinstance(raw.get(key), str):
            raise ConfigError(f"conversion rule requires '{key}'")
    namespace = raw.get("namespace", "req")
    if not isinstance(namespace, str) or " " in namespace:
        raise ConfigError(f"invalid namespace {namespace!r}")
    rule = ConversionRule(
        need_type=raw["need-type"],
        kind=raw["kind"],
        namespace=namespace,
        version_field=raw.get("version-field"),
        status_field=raw.get("status-field"),
        exclude_status=_as_list(raw.get("exclude-status"), "exclude-status"),
        tags=[_parse_tag(t) for t in raw.get("tags") or []],
        justification_up_fields=_as_list(raw.get("justification-up-fields"), "justification-up-fields"),
        justification_down_fields=_as_list(raw.get("justification-down-fields"), "justification-down-fields"),
        justification_global_fields=_as_list(raw.get("justification-global-fields"), "justification-global-fields"),
    )
    if "description-fields" in raw:
        rule.description_fields = _as_list(raw["description-fields"], "description-fields")
    return rule


def load_config(path: str | Path) -> list[ConversionRule]:
    """Load and validate the conversion rules of a configuration file."""
    with open(path, encoding="utf-8") as f:
        data = yaml.safe_load(f) or {}
    if not isinstance(data, dict) or set(data) - {"conversion-rules"}:
        raise ConfigError(f"{path}: only 'conversion-rules' is supported at the top level")
    rules = [_parse_rule(r) for r in data.get("conversion-rules") or []]
    types = [r.need_type for r in rules]
    duplicates = sorted({t for t in types if types.count(t) > 1})
    if duplicates:
        raise ConfigError(f"{path}: more than one conversion rule for need types {duplicates}")
    return rules


def select_rule(rules: list[ConversionRule], need_type: str) -> ConversionRule:
    """Return the conversion rule of ``need_type``."""
    for rule in rules:
        if rule.need_type == need_type:
            return rule
    raise ConfigError(f"no conversion rule for need type {need_type!r}")


def load_needs(path: str | Path) -> dict[str, dict[str, Any]]:
    """Return the needs of the current version of a Sphinx-Needs export."""
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    versions = data.get("versions") or {}
    current = data.get("current_version", "")
    if current in versions:
        return versions[current].get("needs", {})
    if len(versions) == 1:
        return next(iter(versions.values())).get("needs", {})
    raise ConfigError(f"{path}: cannot select a version, current_version {current!r} not in {sorted(versions)}")


def parse_link(value: str, namespace: str) -> Tracing_Tag:
    """Translate a Sphinx-Needs link value into a LOBSTER tracing tag."""
    match = _RE_LINK.match(value.strip())
    if not match:
        raise ConfigError(f"cannot parse link value {value!r}")
    version = None
    if match.group("filter"):
        version_match = _RE_FILTER_VERSION.search(match.group("filter"))
        version = version_match.group(1) if version_match else None
    return Tracing_Tag(namespace, match.group("id"), version)


def _text_values(need: dict[str, Any], field_name: str) -> list[str]:
    value = need.get(field_name)
    if value is None or value == "":
        return []
    if isinstance(value, list):
        return [str(v) for v in value if str(v)]
    return [str(value)]


def _description(need: dict[str, Any], rule: ConversionRule) -> str:
    parts = [text for f in rule.description_fields for text in _text_values(need, f)]
    return "\n\n".join(parts) if parts else str(need.get("title", ""))


def _version(need: dict[str, Any], rule: ConversionRule) -> str | int | None:
    if not rule.version_field:
        return None
    value = need.get(rule.version_field)
    return None if value in (None, "") else value


def _location(need: dict[str, Any], source: str) -> File_Reference:
    docname = need.get("docname")
    lineno = need.get("lineno")
    filename = f"{docname}{need.get('doctype') or ''}" if docname else source
    return File_Reference(filename, line=lineno if isinstance(lineno, int) and lineno >= 1 else None)


def convert_need(need: dict[str, Any], rule: ConversionRule, source: str) -> Requirement:
    """Convert one need into a LOBSTER requirement item."""
    status = None
    if rule.status_field:
        values = _text_values(need, rule.status_field)
        status = values[0] if values else None
    item = Requirement(
        tag=Tracing_Tag(rule.namespace, need["id"], _version(need, rule)),
        location=_location(need, source),
        framework=FRAMEWORK,
        kind=rule.kind,
        name=need["id"],
        text=_description(need, rule),
        status=status,
    )
    for entry in rule.tags:
        for value in _text_values(need, entry.field):
            item.add_tracing_target(parse_link(value, entry.namespace))
    for target, fields in (
        (item.just_up, rule.justification_up_fields),
        (item.just_down, rule.justification_down_fields),
        (item.just_global, rule.justification_global_fields),
    ):
        for just_field in fields:
            target.extend(_text_values(need, just_field))
    return item


def _satisfies(need: dict[str, Any], satisfied_by: str) -> bool:
    return any(parse_link(v, "req").tag == satisfied_by for v in _text_values(need, "satisfied_by"))


def convert(
    needs_files: list[str], rule: ConversionRule, satisfied_by: str | None = None
) -> list[tuple[dict[str, Any], Requirement]]:
    """Convert all needs of the rule's type, sorted by need ID."""
    converted: dict[str, tuple[dict[str, Any], Requirement]] = {}
    for path in needs_files:
        for need_id, need in load_needs(path).items():
            if need.get("type") != rule.need_type:
                continue
            if satisfied_by and not _satisfies(need, satisfied_by):
                continue
            if rule.status_field and need.get(rule.status_field) in rule.exclude_status:
                logger.info("Skipping %s with status %s", need_id, need.get(rule.status_field))
                continue
            if need_id in converted:
                raise ConfigError(f"need {need_id} is defined in more than one needs.json")
            converted[need_id] = (need, convert_need({"id": need_id, **need}, rule, path))
    return [converted[k] for k in sorted(converted)]


def _escape(text: str) -> str:
    return _RE_RST_SPECIAL.sub(r"\\\1", " ".join(text.split()))


def render_rst(title: str, converted: list[tuple[dict[str, Any], Requirement]]) -> str:
    """Render the converted needs as a plain RST table (no Sphinx-Needs directives)."""
    lines = [title, "=" * len(title), ""]
    if not converted:
        return "\n".join(lines + ["No requirements.", ""])
    lines += [
        ".. list-table::",
        "   :header-rows: 1",
        "   :widths: 30 50 10 10",
        "",
        "   * - ID",
        "     - Requirement",
        "     - Version",
        "     - Safety",
    ]
    for need, item in converted:
        lines += [
            f"   * - ``{item.name}``",
            f"     - **{_escape(str(need.get('title', '')))}**",
            "",
            f"       {_escape(item.text or '')}",
            f"     - {_escape(str(item.tag.version or ''))}",
            f"     - {_escape(str(need.get('safety', '')))}",
        ]
    return "\n".join(lines + [""])


def main(argv: list[str] | None = None) -> int:
    """Entry point of the converter."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("needs", nargs="+", help="needs.json files to convert")
    parser.add_argument("--config", required=True, help="YAML file with the conversion rules")
    parser.add_argument("--need-type", required=True, help="need type to convert, e.g. comp_req")
    parser.add_argument("--satisfied-by", help="only convert needs whose satisfied_by link names this need ID")
    parser.add_argument("--out", required=True, help="output .lobster file")
    parser.add_argument("--rst-out", help="optional RST page listing the converted needs")
    parser.add_argument("--title", default="Requirements", help="heading of the RST page")
    parser.add_argument("--log-level", choices=sorted(_LEVEL_MAP), default="warn", dest="log_level")
    args = parser.parse_args(argv)
    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    try:
        rule = select_rule(load_config(args.config), args.need_type)
        converted = convert(args.needs, rule, args.satisfied_by)
    except ConfigError as e:
        logger.error("%s", e)
        return 1
    if args.satisfied_by and not converted:
        logger.warning("no %s need is satisfied by %s", args.need_type, args.satisfied_by)

    with open(args.out, "w", encoding="utf-8") as f:
        lobster_write(f, Requirement, GENERATOR, [item for _, item in converted])
    logger.info("Wrote %d item(s) to %s", len(converted), args.out)
    if args.rst_out:
        Path(args.rst_out).write_text(render_rst(args.title, converted), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
