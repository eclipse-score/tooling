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

"""Pure helpers for certified-scope validation."""

_PACKAGE_SCOPE = "__pkg__"
_SUBPACKAGES_SCOPE = "__subpackages__"

def build_scope_index(certified_scopes):
    """Index certified scopes by repository and full package path.

    Args:
        certified_scopes: Iterable of label strings or Label objects.

    Returns:
        Scope index keyed by repository and full package path.
    """
    scope_index = {}
    for certified_scope in certified_scopes:
        certified_scope = Label(certified_scope)
        repository = scope_index.setdefault(certified_scope.workspace_name, default = {})
        package_scope = repository.setdefault(certified_scope.package, default = {
            "targets": {},
            "covers_package": False,
            "covers_subpackages": False,
        })
        if certified_scope.name == _PACKAGE_SCOPE:
            package_scope["covers_package"] = True
        elif certified_scope.name == _SUBPACKAGES_SCOPE:
            package_scope["covers_subpackages"] = True
        else:
            package_scope["targets"][certified_scope.name] = True

    return scope_index

def find_scope_violation(scope_index, dependency):
    """Return the reason a dependency is outside ``scope_index``, if any.

    Args:
        scope_index: An index returned by ``build_scope_index``.
        dependency: Dependency Label to match against the certified scope.

    Returns:
        A violation reason string, or ``None`` when the dependency is covered.
    """
    repository = scope_index.get(dependency.workspace_name)
    if repository == None:
        return "repository {} is not covered".format(dependency.workspace_name or "<main>")

    package_scope = repository.get(dependency.package)
    if package_scope != None and (
        package_scope["covers_package"] or dependency.name in package_scope["targets"]
    ):
        return None

    package_segments = dependency.package.split("/") if dependency.package else []
    for end_index in range(len(package_segments) + 1):
        ancestor_scope = repository.get("/".join(package_segments[:end_index]))
        if ancestor_scope != None and ancestor_scope["covers_subpackages"]:
            return None

    if package_scope == None:
        return "package '//{}' is not covered".format(dependency.package)

    declared_targets = package_scope["targets"]
    return "dependency '{}' is not certified in package '{}'; declared certified targets: {}".format(
        dependency.name,
        dependency.package,
        ", ".join(declared_targets) if declared_targets else "none",
    )

def collect_scope_violations(scope_index, unit_dependency_closures):
    """Return unit-attributed messages for dependencies outside ``scope_index``.

    Args:
        scope_index: An index returned by ``build_scope_index``.
        unit_dependency_closures: Iterable of structs with ``unit_label`` and
            ``dependencies`` (a depset of dependency Labels) fields.

    Returns:
        List of violation messages.
    """
    violation_cache = {}
    violations = []
    for unit_dependency_closure in unit_dependency_closures:
        for dependency in unit_dependency_closure.dependencies.to_list():
            if dependency not in violation_cache:
                violation_cache[dependency] = find_scope_violation(scope_index, dependency)

            violation = violation_cache[dependency]
            if violation:
                violations.append(
                    "Unit {} depends on {}, but the dependency is not in any certified scope ({}).".format(
                        unit_dependency_closure.unit_label,
                        dependency,
                        violation,
                    ),
                )

    return violations
