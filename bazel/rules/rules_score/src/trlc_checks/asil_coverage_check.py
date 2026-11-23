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

"""ASIL coverage check for the requirement chain.

A parent requirement (AssumedSystemReq or FeatReq) that has downstream
requirements must have at least one direct child (FeatReq or CompReq whose
``derived_from`` names the parent) with the same ASIL. Parents without
children are not checked.

TRLC ``checks`` blocks only see forward references, so this reverse
relation is evaluated here on the parsed TRLC symbol table. All requirement
levels must be passed in one run. Run it with
``trlc_check_runner --check trlc_checks.asil_coverage_check``.
"""

from trlc import ast

from trlc_checks import model_type_names

_PARENT_TYPES = frozenset({"AssumedSystemReq", "FeatReq"})
_CHILD_TYPES = frozenset({"FeatReq", "CompReq"})


def _fqn(obj):
    return obj.n_package.name + "." + obj.name


def check_asil_coverage(symbols, mh):
    """Report every parent whose children all differ in ASIL; return the count."""
    parents = {}
    children = {}
    for package in symbols.values(ast.Package):
        for obj in package.symbols.values(ast.Record_Object):
            type_names = model_type_names(obj.n_typ)
            if type_names & _PARENT_TYPES:
                parents[_fqn(obj)] = obj
            if type_names & _CHILD_TYPES:
                children[_fqn(obj)] = obj

    children_of = {}
    for child_fqn, child in children.items():
        derived_from = child.to_python_dict().get("derived_from") or []
        for source in derived_from:
            if source["item"] in parents:
                children_of.setdefault(source["item"], set()).add(child_fqn)

    violations = 0
    for parent_fqn in sorted(children_of):
        parent_safety = parents[parent_fqn].to_python_dict()["safety"]
        child_safety = {
            child_fqn: children[child_fqn].to_python_dict()["safety"] for child_fqn in children_of[parent_fqn]
        }
        if parent_safety in child_safety.values():
            continue
        listing = ", ".join("%s (%s)" % (child_fqn, child_safety[child_fqn]) for child_fqn in sorted(child_safety))
        mh.error(
            parents[parent_fqn].location,
            "%s has ASIL %s but no downstream requirement with the same ASIL: %s"
            % (parent_fqn, parent_safety, listing),
            fatal=False,
        )
        violations += 1
    return violations


def check(symbols, mh):
    """TRLC check plugin: True if no parent lacks a child of the same ASIL."""
    return check_asil_coverage(symbols, mh) == 0
