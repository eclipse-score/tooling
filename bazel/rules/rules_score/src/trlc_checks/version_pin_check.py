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

"""Check versioned ``derived_from`` references of requirements."""

from trlc import ast

from trlc_checks import model_type_names

_FIELDS = {
    "FeatReq": "derived_from",
    "CompReq": "derived_from",
}


def check(symbols, mh):
    """Report version pins that differ from the referenced record's version."""
    ok = True
    for package in symbols.values(ast.Package):
        for obj in package.symbols.values(ast.Record_Object):
            fields = {_FIELDS[name] for name in model_type_names(obj.n_typ) if name in _FIELDS}
            for field in sorted(fields):
                references = obj.field[field]
                if isinstance(references, ast.Implicit_Null):
                    continue
                for reference in references.value:
                    target = reference.value["item"].target
                    pinned = reference.value["version"].to_python_object()
                    current = target.field["version"].to_python_object()
                    if pinned != current:
                        mh.error(
                            reference.location,
                            "%s %s references %s@%s, but current version is %s"
                            % (obj.fully_qualified_name(), field, target.fully_qualified_name(), pinned, current),
                            fatal=False,
                        )
                        ok = False
    return ok
