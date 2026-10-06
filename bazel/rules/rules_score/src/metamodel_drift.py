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
"""Report drift between a TRLC model and a Sphinx-needs metamodel."""

from __future__ import annotations

import argparse
import dataclasses
import io
import json
import re
import sys
from pathlib import Path
from typing import Any

import yaml
from trlc import ast
from trlc.errors import Message_Handler, TRLC_Error
from trlc.trlc import Source_Manager

_SEVERITY = {"info": 1, "warning": 2, "error": 3}
_SEVERITIES = ("error", "warning", "info")
_BUILTIN_NEED_FIELDS = frozenset({"content"})


@dataclasses.dataclass(frozen=True)
class Finding:
    """A single report finding."""

    id: str
    severity: str
    trlc_type: str | None
    need_type: str | None
    field: str | None
    message: str
    section: str


@dataclasses.dataclass(frozen=True)
class FieldInfo:
    """Semantically relevant properties of a TRLC record field."""

    name: str
    optional: bool
    type_signature: tuple[Any, ...]
    type_display: str
    bounds: tuple[int, int | None] | None
    link_targets: tuple[str, ...]
    enum_name: str | None
    enum_literals: tuple[str, ...]
    is_boolean: bool

    @property
    def minimum_links(self) -> int:
        if self.optional:
            return 0
        if self.bounds is not None:
            return self.bounds[0]
        return 1


@dataclasses.dataclass(frozen=True)
class RecordInfo:
    """A TRLC record and its fields, including inherited fields."""

    name: str
    abstract: bool
    parent: str | None
    fields: dict[str, FieldInfo]
    declared_fields: dict[str, FieldInfo]


@dataclasses.dataclass(frozen=True)
class RslModel:
    """Relevant declarations from one parsed TRLC package."""

    records: dict[str, RecordInfo]
    enums: dict[str, tuple[str, ...]]


def _field_type_signature(typ: Any) -> tuple[Any, ...]:
    if isinstance(typ, ast.Array_Type):
        return ("array", _field_type_signature(typ.element_type))
    if isinstance(typ, ast.Record_Type):
        return ("record", typ.name)
    if isinstance(typ, ast.Union_Type):
        return ("union", tuple(sorted(member.name for member in typ.types)))
    if isinstance(typ, ast.Tuple_Type):
        return (
            "tuple",
            tuple(
                (
                    component.name,
                    component.optional,
                    _field_type_signature(component.n_typ),
                )
                for component in typ.all_components()
            ),
        )
    if isinstance(typ, ast.Enumeration_Type):
        return ("enum", typ.name)
    return ("scalar", typ.__class__.__name__)


def _type_display(typ: Any) -> str:
    if isinstance(typ, ast.Array_Type):
        upper = "*" if typ.upper_bound is None else str(typ.upper_bound)
        return f"{_type_display(typ.element_type)}[{typ.lower_bound}..{upper}]"
    if isinstance(typ, ast.Union_Type):
        return "[" + ", ".join(sorted(member.name for member in typ.types)) + "]"
    return typ.name


def _record_targets(typ: Any) -> tuple[str, ...]:
    if isinstance(typ, ast.Array_Type):
        return _record_targets(typ.element_type)
    if isinstance(typ, ast.Record_Type):
        return (typ.name,)
    if isinstance(typ, ast.Union_Type):
        return tuple(sorted(member.name for member in typ.types))
    if isinstance(typ, ast.Tuple_Type):
        item = typ.components.table.get("item")
        return _record_targets(item.n_typ) if item is not None else ()
    return ()


def _value_type(typ: Any) -> Any:
    while isinstance(typ, ast.Array_Type):
        typ = typ.element_type
    return typ


def _field_info(component: Any) -> FieldInfo:
    typ = component.n_typ
    bounds = None
    if isinstance(typ, ast.Array_Type):
        bounds = (typ.lower_bound, typ.upper_bound)
    value_type = _value_type(typ)
    enum_name = value_type.name if isinstance(value_type, ast.Enumeration_Type) else None
    enum_literals = (
        tuple(sorted(literal.name for literal in value_type.literals.values())) if enum_name is not None else ()
    )
    return FieldInfo(
        name=component.name,
        optional=component.optional,
        type_signature=_field_type_signature(typ),
        type_display=_type_display(typ),
        bounds=bounds,
        link_targets=_record_targets(typ),
        enum_name=enum_name,
        enum_literals=enum_literals,
        is_boolean=isinstance(value_type, ast.Builtin_Boolean),
    )


def _model_from_symbol_table(symbol_table: Any) -> RslModel | None:
    package = next(
        (item for item in symbol_table.values(ast.Package) if item.name == "ScoreReq"),
        None,
    )
    if package is None:
        return None
    records = {}
    for record in package.symbols.values(ast.Record_Type):
        fields = {component.name: _field_info(component) for component in record.all_components()}
        records[record.name] = RecordInfo(
            name=record.name,
            abstract=record.is_abstract,
            parent=record.parent.name if record.parent else None,
            fields=fields,
            declared_fields={component.name: _field_info(component) for component in record.components.table.values()},
        )
    enums = {
        enum.name: tuple(sorted(literal.name for literal in enum.literals.values()))
        for enum in package.symbols.values(ast.Enumeration_Type)
    }
    return RslModel(records=records, enums=enums)


def _parse_rsl(paths: list[str]) -> tuple[RslModel | None, str | None]:
    output = io.StringIO()
    handler = Message_Handler(brief=True, out=output)
    source_manager = Source_Manager(
        handler,
        lint_mode=False,
        parse_trlc=False,
        error_recovery=True,
    )
    try:
        for path in paths:
            source_manager.register_rsl_file(path)
        symbol_table = source_manager.process()
    except (AssertionError, OSError, TRLC_Error) as exc:
        detail = output.getvalue().strip()
        return None, detail or str(exc)
    if symbol_table is None:
        return None, output.getvalue().strip() or "TRLC could not parse the RSL input"
    model = _model_from_symbol_table(symbol_table)
    if model is None:
        return None, "RSL input does not declare package ScoreReq"
    return model, None


def _as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def _load_yaml(path: str) -> dict[str, Any]:
    with Path(path).open(encoding="utf-8") as stream:
        value = yaml.safe_load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a YAML mapping")
    return value


def _need_options(
    metamodel: dict[str, Any],
    need_type: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    base = _as_dict(metamodel.get("needs_types_base_options"))
    need = _as_dict(_as_dict(metamodel.get("needs_types")).get(need_type))
    mandatory = dict(_as_dict(base.get("mandatory_options")))
    mandatory.update(_as_dict(need.get("mandatory_options")))
    optional = dict(_as_dict(base.get("optional_options")))
    optional.update(_as_dict(need.get("optional_options")))
    return mandatory, optional


def _need_links(
    metamodel: dict[str, Any],
    need_type: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    need = _as_dict(_as_dict(metamodel.get("needs_types")).get(need_type))
    return (
        _as_dict(need.get("mandatory_links")),
        _as_dict(need.get("optional_links")),
    )


def _alternatives(pattern: Any) -> tuple[str, ...] | None:
    if not isinstance(pattern, str):
        return None
    match = re.fullmatch(r"\^\(([^()]*)\)\$", pattern)
    if match is None:
        return None
    return tuple(match.group(1).split("|"))


def _matches(pattern: Any, value: str) -> bool:
    if not isinstance(pattern, str):
        return False
    try:
        return re.fullmatch(pattern, value) is not None
    except re.error:
        return False


def _link_targets(value: Any) -> tuple[str, ...]:
    if isinstance(value, str):
        return tuple(item.strip() for item in value.split(",") if item.strip())
    if isinstance(value, list):
        return tuple(str(item).strip() for item in value if str(item).strip())
    return ()


def _new_finding(
    findings: list[Finding],
    identifier: str,
    severity: str,
    section: str,
    message: str,
    trlc_type: str | None = None,
    need_type: str | None = None,
    field: str | None = None,
) -> None:
    findings.append(
        Finding(
            id=identifier,
            severity=severity,
            trlc_type=trlc_type,
            need_type=need_type,
            field=field,
            message=message,
            section=section,
        )
    )


def _is_target_allowed(
    target_need_type: str,
    allowed_targets: tuple[str, ...],
    need_type_names: set[str],
) -> bool:
    for allowed in allowed_targets:
        if allowed == "ANY" or allowed == target_need_type:
            return True
        if allowed not in need_type_names and _matches(allowed, target_need_type):
            return True
    return False


def _check_mapping(
    model: RslModel,
    metamodel: dict[str, Any],
    mapping: dict[str, Any],
    findings: list[Finding],
) -> None:
    need_types = _as_dict(metamodel.get("needs_types"))
    need_type_names = set(need_types)
    mapped_types = _as_dict(mapping.get("types"))
    unmapped_types = _as_dict(mapping.get("unmapped_types"))
    enum_mapping = _as_dict(mapping.get("enums"))

    for trlc_type in sorted(mapped_types):
        entry = _as_dict(mapped_types[trlc_type])
        need_type = entry.get("need_type")
        need_type = need_type if isinstance(need_type, str) else ""
        section = f"type:{trlc_type}"
        record = model.records.get(trlc_type)
        if record is None:
            _new_finding(
                findings,
                "MAPPED_TRLC_TYPE_NOT_FOUND",
                "error",
                section,
                f"Mapped TRLC type {trlc_type!r} is not declared in the RSL input.",
                trlc_type=trlc_type,
                need_type=need_type or None,
            )
        if need_type not in need_type_names:
            _new_finding(
                findings,
                "MAPPED_NEED_TYPE_NOT_FOUND",
                "error",
                section,
                f"Mapped need type {need_type!r} is not declared in the metamodel.",
                trlc_type=trlc_type,
                need_type=need_type or None,
            )

        options_map = _as_dict(entry.get("options"))
        links_map = _as_dict(entry.get("links"))
        ignored_map = _as_dict(entry.get("ignored"))
        fields = record.fields if record else {}
        known_fields = set(fields)

        for source_field in sorted(set(options_map) | set(links_map) | set(ignored_map)):
            if source_field not in known_fields:
                _new_finding(
                    findings,
                    "MAPPED_FIELD_NOT_FOUND",
                    "error",
                    section,
                    f"Mapped TRLC field {source_field!r} does not exist on {trlc_type}.",
                    trlc_type=trlc_type,
                    need_type=need_type or None,
                    field=source_field,
                )

        if need_type not in need_type_names:
            continue
        mandatory_options, optional_options = _need_options(metamodel, need_type)
        mandatory_links, optional_links = _need_links(metamodel, need_type)
        need_options = mandatory_options | optional_options
        need_links = mandatory_links | optional_links

        valid_option_fields = {
            source: destination
            for source, destination in options_map.items()
            if source in known_fields and isinstance(destination, str)
        }
        valid_link_fields = {
            source: destination
            for source, destination in links_map.items()
            if source in known_fields and isinstance(destination, str)
        }
        for source_field, option_name in sorted(valid_option_fields.items()):
            if option_name not in need_options and option_name not in _BUILTIN_NEED_FIELDS:
                _new_finding(
                    findings,
                    "MAPPED_OPTION_NOT_IN_NEED",
                    "error",
                    section,
                    f"Need option {option_name!r} is not defined for {need_type}.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=option_name,
                )
            if source_field in fields and option_name in mandatory_options and fields[source_field].optional:
                _new_finding(
                    findings,
                    "OPTIONAL_FIELD_FOR_MANDATORY_OPTION",
                    "warning",
                    section,
                    f"Optional TRLC field {source_field!r} maps to mandatory need option {option_name!r}.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=source_field,
                )
        for source_field, link_name in sorted(valid_link_fields.items()):
            if link_name not in need_links:
                _new_finding(
                    findings,
                    "MAPPED_LINK_NOT_IN_NEED",
                    "error",
                    section,
                    f"Need link {link_name!r} is not defined for {need_type}.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=link_name,
                )
            if source_field in fields and link_name in mandatory_links and fields[source_field].optional:
                _new_finding(
                    findings,
                    "OPTIONAL_FIELD_FOR_MANDATORY_OPTION",
                    "warning",
                    section,
                    f"Optional TRLC field {source_field!r} maps to mandatory need link {link_name!r}.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=source_field,
                )
            if source_field in fields and not fields[source_field].link_targets:
                _new_finding(
                    findings,
                    "MAPPED_LINK_FIELD_NOT_LINK",
                    "error",
                    section,
                    f"TRLC field {source_field!r} does not resolve to a record link.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=source_field,
                )

        mapped_options = set(valid_option_fields.values())
        mapped_links = set(valid_link_fields.values())
        for option_name in sorted(set(mandatory_options) - mapped_options):
            _new_finding(
                findings,
                "MANDATORY_OPTION_MISSING",
                "error",
                section,
                f"Mandatory need option {option_name!r} has no mapped TRLC field.",
                trlc_type=trlc_type,
                need_type=need_type,
                field=option_name,
            )
        for link_name in sorted(set(mandatory_links) - mapped_links):
            _new_finding(
                findings,
                "MANDATORY_LINK_MISSING",
                "error",
                section,
                f"Mandatory need link {link_name!r} has no mapped TRLC field.",
                trlc_type=trlc_type,
                need_type=need_type,
                field=link_name,
            )

        mapped_source_fields = set(options_map) | set(links_map) | set(ignored_map)
        for field_name in sorted(known_fields - mapped_source_fields):
            _new_finding(
                findings,
                "TRLC_FIELD_UNMAPPED",
                "warning",
                section,
                f"TRLC field {field_name!r} is not exported or explicitly ignored.",
                trlc_type=trlc_type,
                need_type=need_type,
                field=field_name,
            )

        for source_field, option_name in sorted(valid_option_fields.items()):
            field_info = fields[source_field]
            if option_name not in need_options:
                continue
            pattern = need_options[option_name]
            values: tuple[str, ...] = ()
            if field_info.enum_name is not None:
                per_enum = _as_dict(enum_mapping.get(field_info.enum_name))
                values = tuple(str(per_enum.get(literal, literal)) for literal in field_info.enum_literals)
                for literal, mapped_value in zip(field_info.enum_literals, values):
                    if not _matches(pattern, mapped_value):
                        _new_finding(
                            findings,
                            "ENUM_VALUE_REJECTED",
                            "error",
                            section,
                            f"TRLC enum literal {literal!r} maps to {mapped_value!r}, which does not match need option {option_name!r} pattern {pattern!r}.",
                            trlc_type=trlc_type,
                            need_type=need_type,
                            field=source_field,
                        )
            elif field_info.is_boolean and _alternatives(pattern) is not None:
                values = ("true", "false")
                for boolean_value in values:
                    if not _matches(pattern, boolean_value):
                        _new_finding(
                            findings,
                            "ENUM_VALUE_REJECTED",
                            "error",
                            section,
                            f"TRLC Boolean value {boolean_value!r} does not match need option {option_name!r} pattern {pattern!r}.",
                            trlc_type=trlc_type,
                            need_type=need_type,
                            field=source_field,
                        )
            alternatives = _alternatives(pattern)
            if alternatives is not None and (field_info.enum_name is not None or field_info.is_boolean):
                represented = set(values)
                for alternative in alternatives:
                    if alternative not in represented:
                        _new_finding(
                            findings,
                            "NEED_VALUE_UNREPRESENTABLE",
                            "warning",
                            section,
                            f"Need option {option_name!r} permits {alternative!r}, but no TRLC value maps to it.",
                            trlc_type=trlc_type,
                            need_type=need_type,
                            field=source_field,
                        )

        for source_field, link_name in sorted(valid_link_fields.items()):
            field_info = fields[source_field]
            if link_name not in need_links:
                continue
            allowed_targets = _link_targets(need_links[link_name])
            produced_targets: set[str] = set()
            for target_record in field_info.link_targets:
                target_entry = _as_dict(mapped_types.get(target_record))
                target_need_type = target_entry.get("need_type")
                if (
                    target_record in unmapped_types
                    or not isinstance(target_need_type, str)
                    or target_need_type not in need_type_names
                ):
                    reason = unmapped_types.get(target_record)
                    suffix = f" Reason: {reason}" if reason else ""
                    _new_finding(
                        findings,
                        "LINK_TARGET_UNMAPPED",
                        "warning",
                        section,
                        f"TRLC link target {target_record!r} has no exported need type.{suffix}",
                        trlc_type=trlc_type,
                        need_type=need_type,
                        field=source_field,
                    )
                    continue
                produced_targets.add(target_need_type)
                if not _is_target_allowed(target_need_type, allowed_targets, need_type_names):
                    _new_finding(
                        findings,
                        "LINK_TARGET_REJECTED",
                        "error",
                        section,
                        f"TRLC target {target_record!r} maps to {target_need_type!r}, which is not allowed by need link {link_name!r} ({', '.join(allowed_targets)}).",
                        trlc_type=trlc_type,
                        need_type=need_type,
                        field=source_field,
                    )

            for pattern in sorted(
                target for target in allowed_targets if target != "ANY" and target not in need_type_names
            ):
                matching_types = sorted(
                    target_type for target_type in need_type_names if _matches(pattern, target_type)
                )
                _new_finding(
                    findings,
                    "LINK_TARGET_PATTERN",
                    "info",
                    section,
                    f"Need link {link_name!r} target expression {pattern!r} matches need types: {', '.join(matching_types) or 'none'}.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=source_field,
                )

            for unreachable in sorted(
                target for target in allowed_targets if target in need_type_names and target not in produced_targets
            ):
                _new_finding(
                    findings,
                    "LINK_TARGET_UNREACHABLE",
                    "info",
                    section,
                    f"Need link {link_name!r} allows {unreachable!r}, but no TRLC target maps to it.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=source_field,
                )

            minimum = field_info.minimum_links
            is_need_mandatory = link_name in mandatory_links
            is_need_optional = link_name in optional_links
            if (minimum == 0 and is_need_mandatory) or (minimum >= 1 and is_need_optional):
                _new_finding(
                    findings,
                    "LINK_MULTIPLICITY",
                    "info",
                    section,
                    f"TRLC field {source_field!r} has minimum multiplicity {minimum}; need link {link_name!r} is {'mandatory' if is_need_mandatory else 'optional'}.",
                    trlc_type=trlc_type,
                    need_type=need_type,
                    field=source_field,
                )

    for trlc_type, record in sorted(model.records.items()):
        if record.abstract:
            continue
        if trlc_type in unmapped_types:
            _new_finding(
                findings,
                "TRLC_TYPE_NOT_EXPORTED",
                "info",
                "global",
                f"Concrete TRLC type {trlc_type!r} is intentionally not exported: {unmapped_types[trlc_type]}",
                trlc_type=trlc_type,
            )
        elif trlc_type not in mapped_types:
            _new_finding(
                findings,
                "UNMAPPED_TRLC_TYPE",
                "warning",
                "global",
                f"Concrete TRLC type {trlc_type!r} is neither mapped nor listed in unmapped_types.",
                trlc_type=trlc_type,
            )


def _compare_models(
    primary: RslModel,
    copy: RslModel,
    copy_name: str,
    findings: list[Finding],
) -> None:
    section = f"copy:{copy_name}"
    primary_names = set(primary.records)
    copy_names = set(copy.records)
    for trlc_type in sorted(copy_names - primary_names):
        _new_finding(
            findings,
            "RSL_COPY_TYPE_ADDED",
            "warning",
            section,
            f"Type {trlc_type!r} exists in the copy but not in the primary RSL.",
            trlc_type=trlc_type,
        )
    for trlc_type in sorted(primary_names - copy_names):
        _new_finding(
            findings,
            "RSL_COPY_TYPE_REMOVED",
            "warning",
            section,
            f"Type {trlc_type!r} exists in the primary RSL but not in the copy.",
            trlc_type=trlc_type,
        )

    for trlc_type in sorted(primary_names & copy_names):
        original = primary.records[trlc_type]
        current = copy.records[trlc_type]
        if original.parent != current.parent:
            _new_finding(
                findings,
                "RSL_COPY_FIELD_CHANGED",
                "warning",
                section,
                f"Parent of {trlc_type!r} changed from {original.parent!r} to {current.parent!r}.",
                trlc_type=trlc_type,
                field="<parent>",
            )
        if original.abstract != current.abstract:
            _new_finding(
                findings,
                "RSL_COPY_FIELD_CHANGED",
                "warning",
                section,
                f"Abstract flag of {trlc_type!r} changed from {original.abstract} to {current.abstract}.",
                trlc_type=trlc_type,
                field="<abstract>",
            )
        original_fields = original.declared_fields
        current_fields = current.declared_fields
        for field_name in sorted(set(current_fields) - set(original_fields)):
            _new_finding(
                findings,
                "RSL_COPY_FIELD_ADDED",
                "warning",
                section,
                f"Field {field_name!r} exists on {trlc_type} in the copy but not in the primary RSL.",
                trlc_type=trlc_type,
                field=field_name,
            )
        for field_name in sorted(set(original_fields) - set(current_fields)):
            _new_finding(
                findings,
                "RSL_COPY_FIELD_REMOVED",
                "warning",
                section,
                f"Field {field_name!r} exists on {trlc_type} in the primary RSL but not in the copy.",
                trlc_type=trlc_type,
                field=field_name,
            )
        for field_name in sorted(set(original_fields) & set(current_fields)):
            original_field = original_fields[field_name]
            current_field = current_fields[field_name]
            if original_field.type_signature != current_field.type_signature:
                _new_finding(
                    findings,
                    "RSL_COPY_FIELD_CHANGED",
                    "warning",
                    section,
                    f"Type of {trlc_type}.{field_name} changed from {original_field.type_display!r} to {current_field.type_display!r}.",
                    trlc_type=trlc_type,
                    field=field_name,
                )
            if original_field.optional != current_field.optional:
                _new_finding(
                    findings,
                    "RSL_COPY_FIELD_CHANGED",
                    "warning",
                    section,
                    f"Optional flag of {trlc_type}.{field_name} changed from {original_field.optional} to {current_field.optional}.",
                    trlc_type=trlc_type,
                    field=field_name,
                )
            if original_field.bounds != current_field.bounds:
                _new_finding(
                    findings,
                    "RSL_COPY_FIELD_CHANGED",
                    "warning",
                    section,
                    f"Bounds of {trlc_type}.{field_name} changed from {original_field.bounds!r} to {current_field.bounds!r}.",
                    trlc_type=trlc_type,
                    field=field_name,
                )

    for enum_name in sorted(set(primary.enums) & set(copy.enums)):
        primary_literals = set(primary.enums[enum_name])
        copy_literals = set(copy.enums[enum_name])
        for literal in sorted(copy_literals - primary_literals):
            _new_finding(
                findings,
                "RSL_COPY_ENUM_LITERAL_ADDED",
                "warning",
                section,
                f"Enum literal {enum_name}.{literal} exists in the copy but not in the primary RSL.",
                trlc_type=enum_name,
                field=literal,
            )
        for literal in sorted(primary_literals - copy_literals):
            _new_finding(
                findings,
                "RSL_COPY_ENUM_LITERAL_REMOVED",
                "warning",
                section,
                f"Enum literal {enum_name}.{literal} exists in the primary RSL but not in the copy.",
                trlc_type=enum_name,
                field=literal,
            )


def _finding_sort_key(finding: Finding) -> tuple[str, str, str, str, str, str]:
    return (
        finding.section,
        finding.id,
        finding.trlc_type or "",
        finding.need_type or "",
        finding.field or "",
        finding.message,
    )


def _report_data(inputs: dict[str, Any], findings: list[Finding]) -> dict[str, Any]:
    sorted_findings = sorted(findings, key=_finding_sort_key)
    summary = {severity: sum(finding.severity == severity for finding in sorted_findings) for severity in _SEVERITIES}
    return {
        "inputs": inputs,
        "summary": summary,
        "findings": [dataclasses.asdict(finding) for finding in sorted_findings],
    }


def _markdown_report(
    report: dict[str, Any],
    mapped_type_sections: list[tuple[str, str]],
    copy_names: list[str],
) -> str:
    findings = report["findings"]
    summary = report["summary"]
    lines = [
        "# Metamodel Drift Report",
        "",
        "## Summary",
        "",
        "| Severity | Findings |",
        "| --- | ---: |",
    ]
    for severity in _SEVERITIES:
        lines.append(f"| {severity} | {summary[severity]} |")

    sections = [
        (f"type:{trlc_type}", f"Mapped type: `{trlc_type}` → `{need_type}`")
        for trlc_type, need_type in mapped_type_sections
    ]
    sections.append(("global", "Global findings"))
    sections.extend((f"copy:{name}", f"Compared RSL copy: `{name}`") for name in copy_names)
    for section_key, title in sections:
        section_findings = [finding for finding in findings if finding["section"] == section_key]
        lines.extend(["", f"## {title}", ""])
        if not section_findings:
            lines.append("No findings.")
            continue
        lines.extend(
            [
                "| ID | Severity | Field / option | Message |",
                "| --- | --- | --- | --- |",
            ]
        )
        for finding in section_findings:
            field = _markdown_cell(finding["field"] or "")
            message = _markdown_cell(finding["message"])
            lines.append(f"| {finding['id']} | {finding['severity']} | {field} | {message} |")
    return "\n".join(lines) + "\n"


def _markdown_cell(value: str) -> str:
    return value.replace("|", "\\|").replace("\n", " ")


def analyze(
    rsl_paths: list[str],
    metamodel_path: str,
    mapping_path: str,
    compare_rsl: list[tuple[str, str]] | None = None,
) -> tuple[dict[str, Any], str]:
    """Analyze inputs and return JSON-ready report data and Markdown."""
    compare_rsl = compare_rsl or []
    inputs = {
        "rsl": rsl_paths,
        "metamodel": metamodel_path,
        "mapping": mapping_path,
        "compare_rsl": [{"name": name, "path": path} for name, path in compare_rsl],
    }
    findings: list[Finding] = []
    try:
        metamodel = _load_yaml(metamodel_path)
        mapping = _load_yaml(mapping_path)
    except (OSError, ValueError, yaml.YAMLError) as exc:
        _new_finding(
            findings,
            "INPUT_PARSE_ERROR",
            "error",
            "global",
            str(exc),
        )
        metamodel = {}
        mapping = {}

    primary, parse_error = _parse_rsl(rsl_paths)
    if parse_error:
        _new_finding(
            findings,
            "PRIMARY_RSL_PARSE_ERROR",
            "error",
            "global",
            parse_error,
        )
    if primary is not None:
        if metamodel and mapping:
            _check_mapping(primary, metamodel, mapping, findings)
        else:
            for trlc_type, record in sorted(primary.records.items()):
                if not record.abstract and trlc_type not in _as_dict(mapping.get("types")):
                    _new_finding(
                        findings,
                        "UNMAPPED_TRLC_TYPE",
                        "warning",
                        "global",
                        f"Concrete TRLC type {trlc_type!r} is neither mapped nor listed in unmapped_types.",
                        trlc_type=trlc_type,
                    )

        for copy_name, copy_path in compare_rsl:
            copy, copy_error = _parse_rsl([copy_path])
            if copy_error:
                _new_finding(
                    findings,
                    "RSL_COPY_PARSE_ERROR",
                    "error",
                    f"copy:{copy_name}",
                    copy_error,
                    field=copy_path,
                )
            elif copy is not None:
                _compare_models(primary, copy, copy_name, findings)

    report = _report_data(inputs, findings)
    type_sections = sorted(
        (
            name,
            str(_as_dict(entry).get("need_type", "")),
        )
        for name, entry in _as_dict(mapping.get("types")).items()
    )
    return report, _markdown_report(report, type_sections, [name for name, _ in compare_rsl])


def _parse_compare_rsl(values: list[str], parser: argparse.ArgumentParser) -> list[tuple[str, str]]:
    parsed = []
    for value in values:
        name, separator, path = value.partition("=")
        if not separator or not name or not path:
            parser.error(f"--compare-rsl must be NAME=PATH, got {value!r}")
        parsed.append((name, path))
    return parsed


def main(argv: list[str] | None = None) -> int:
    """Run the command-line tool and return its exit status."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rsl", action="append", nargs="+", required=True, metavar="PATH")
    parser.add_argument("--metamodel", required=True)
    parser.add_argument("--mapping", required=True)
    parser.add_argument("--compare-rsl", action="append", default=[], metavar="NAME=PATH")
    parser.add_argument("--report-md")
    parser.add_argument("--report-json")
    parser.add_argument("--fail-on", choices=("never", "warning", "error"), default="never")
    args = parser.parse_args(argv)

    rsl_paths = [path for group in args.rsl for path in group]
    compare_rsl = _parse_compare_rsl(args.compare_rsl, parser)
    report, markdown = analyze(
        rsl_paths,
        args.metamodel,
        args.mapping,
        compare_rsl=compare_rsl,
    )
    print(markdown, end="")

    if args.report_md:
        report_path = Path(args.report_md)
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(markdown, encoding="utf-8")
    if args.report_json:
        report_path = Path(args.report_json)
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

    threshold = _SEVERITY.get(args.fail_on, 4)
    return int(any(_SEVERITY[finding["severity"]] >= threshold for finding in report["findings"]))


if __name__ == "__main__":
    sys.exit(main())
