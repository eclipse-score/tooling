..
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

Scope Check
===========

The scope check verifies that every Bazel target which ends up in a
``dependable_element``'s transitive implementation closure is covered by a
**certified scope** declared somewhere in that or another element. It catches the case
where a unit's implementation silently starts depending on code that nobody
certified.

Declaring scope
----------------

Each ``unit`` declares the scope it certifies via its ``scope`` attribute —
labels, packages (``//some/package:__pkg__``), or packages-and-subpackages
(``//some/package:__subpackages__``), following normal Bazel visibility
patterns:

.. code-block:: starlark

   unit(
       name           = "MyUnit",
       implementation = [":my_unit_lib"],
       scope          = ["//third_party/foo:__subpackages__"],
       unit_design    = [":MyUnit_design"],
       tests          = [],
   )

``scope`` is for dependencies that are *not* explicitly named as
``implementation`` targets themselves (e.g. third-party libraries pulled in
transitively) but are still known and accepted as part of the unit. Every
``unit`` also implicitly certifies its own ``implementation`` targets.
Components aggregate the scopes of their units and nested components.

Scopes are collected transitively: a ``component``'s certified scope is the
union of its own units' and nested components' scopes, and a
``dependable_element``'s certified scope is the union of all its components'
scopes plus any scopes brought in through ``deps`` on other dependable
elements.

What is checked
----------------

For each ``unit``, an aspect (``cc_dependencies_aspect``) walks the
``deps`` / ``implementation_deps`` / ``exported_deps`` attributes of its
``implementation`` targets and collects every transitively reached label.
``dependable_element`` checks each collected dependency label against the
combined certified scopes, including the units' own ``implementation``
targets. A dependency is covered if any of the following applies:

* Its exact target label is included.
* Its package is covered by a ``__pkg__`` declaration.
* Its package or an ancestor package is covered by a
   ``__subpackages__`` declaration.

All matches require the scope declaration and dependency to belong to
the same repository.

Uncovered dependencies are reported as warnings when the enclosing
``dependable_element`` has ``maturity = "development"``; otherwise they
fail the build. Each message identifies the originating unit and dependency:

.. code-block:: text

   Unit <unit label> depends on <dependency label>, but the dependency
   is not in any certified scope (<reason>).

A certified scope that is declared more than once (e.g. by two units) is
also rejected:

.. code-block:: text

   The same scope is covered twice: <label>

Implementation
---------------

.. code-block:: text

   bazel/rules/rules_score/private/
   ├── cc_dependency_aspect.bzl   # CcDependencyInfo: transitive labels reached from a cc target
   ├── unit.bzl                   # CertifiedScope: scope attr + implementation targets
   ├── component.bzl              # aggregates CertifiedScope + dependent_labels from children
   ├── certified_scope.bzl        # builds the repository/package index and collects scope violations
   └── dependable_element.bzl     # aggregates scopes and applies its maturity policy

See the ``CertifiedScope``, ``UnitInfo.dependent_labels`` and
``ComponentInfo.dependent_labels`` providers in
``//bazel/rules/rules_score:providers.bzl`` for the exact data flow.
