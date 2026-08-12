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

Requirements
============

.. note::
   A complete working example covering all requirement rules is available in
   `bazel/rules/rules_score/examples/seooc/ <https://github.com/eclipse-score/tooling/tree/main/bazel/rules/rules_score/examples/seooc>`_ (standalone Bazel workspace).

``rules_score`` provides three rules for capturing different levels of requirements.

Requirement Hierarchy & Traceability
-------------------------------------

::

    AssumedSystemReq  →  FeatReq  →  CompReq
        (System)        (Feature)   (Component)
             \                          ↑
              \________________________/

.. list-table::
   :header-rows: 1
   :widths: 18 47 35

   * - Type
     - Description
     - Traceability
   * - **AssumedSystemReq**
     - Requirements from the user / assumed system towards the SEooC.

       Too high-level for a single component — can only be satisfied by
       multiple components working together.
     - Root — no parent
   * - **FeatReq**
     - Refined requirements derived from ``AssumedSystemReq``.

       Used when assumed system requirements are too high-level to be broken
       down directly to one component — still require multiple components.
     - **Must** reference ≥ 1 ``AssumedSystemReq`` via ``derived_from``
   * - **CompReq**
     - Requirements assigned to exactly one component.

       Can be directly implemented and tested within that component.
     - Optionally references ≥ 1 ``FeatReq`` via ``derived_from``
       using ``[Package.FeatReq@version]``

Traceability throughout the complete requirements traceability is performed via TRLC.
It includes also (manual) version pinning (e.g. ``@1``) of requirements, which ensures
that when a parent requirement changes its content (and thus version), all downstream
references must be explicitly updated.

Writing Good Requirements
-------------------------

The rules only check that requirements are *well-formed and traceable* — not that
they are *well-written*. The authoritative writing rules live in the
`Requirements Writing Guidelines <https://github.com/eclipse-score/tooling/blob/main/validation/ai_checker/guidelines/requirements/requirements_guidelines.md>`_
(the same guidelines the AI quality check applies) and the S-CORE
`Requirements Engineering process area <https://eclipse-score.github.io/process_description/main/process_areas/requirements_engineering/index.html>`_.

The guideline document defines: a mandatory **sentence template** for every requirement; a set
of **quality criteria** (unambiguous, verifiable, atomic, consistent, complete, necessary) with
terms and patterns to avoid; a short explanation of the **requirement types** (functional,
interface, non-functional, process) and how each is verified; and a list of **pre-flight checks**
that the AI quality check applies before raising a finding (e.g. not flagging formally defined
domain terms as vague, or intentional design constraints as level mismatches). The essentials are
distilled below.

Sentence template
~~~~~~~~~~~~~~~~~~

Every requirement follows one structure — **subject** *shall* **verb** [*object*]
[*parameter*] [*condition*] — with at least one of object / parameter / condition present:

    The component *shall* detect if a key-value pair got corrupted and set its status to
    ``INVALID`` during every restart of the SW platform.

Quality criteria
~~~~~~~~~~~~~~~~~~

- **Unambiguous** — one interpretation. Prefer ``:term:`` glossary nouns over pronouns and
  vague words (*fast*, *efficient*, *as appropriate*).
- **Verifiable** — a test or review can objectively pass or fail it.
- **Atomic** — one ``shall``; split "and"/"or" and unbounded lists (*etc.*, *and so on*).
- **Consistent** — no contradiction with other requirements.
- **Complete** — has subject + verb + at least one of object/parameter/condition, and fully
  specifies behaviour *at its own level* without pre-empting lower-level detail.
- **Necessary** — traces to a parent or a ``rationale``.

Use *shall* for obligations; put non-normative notes in the ``note`` field, not
``description``. Keep implementation detail out of system/feature requirements — but an
*intentional* design constraint (mandating a transport, mechanism, or safety property) is a
legitimate requirement, not a level violation.

Choosing the level
~~~~~~~~~~~~~~~~~~~~

Levels differ by **scope and observer**, not wording:

- **AssumedSystemReq** — the need comes from *outside* the SEooC; describes it as a black box.
- **FeatReq** — spans **several** components on one feature; phrased against the public
  interface, solution-neutral.
- **CompReq** — fully implementable and testable **inside one component**.

If the level is unclear, discuss the boundary with the requirements owner — it drives
allocation and traceability.

Refining a parent into children
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``derived_from`` is a refinement claim: each child is a narrower, consistent refinement; the
children together must be **sufficient** to satisfy the parent; a child inherits the parent's
``safety`` level unless a lower one is justified. On any content change, bump ``version`` and
re-pin every child (``@1`` → ``@2``).

.. code-block:: text

    # Weak — unverifiable, multi-concern, leaks implementation
    "The manager should quickly handle values and store them efficiently in a hash map."
    # Better — atomic, verifiable, follows the template
    "The numeric value manager shall return the most recently stored uint8_t value on every read access."

Modeling Requirements
---------------------

All requirements are written in `TRLC <https://github.com/bmw-software-engineering/trlc>`_
(Traceability Requirements Language Checker). Each record maps to a specific ``ScoreReq``
type defined in the
`S-CORE requirements model <https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/trlc/config/score_requirements_model.rsl>`_.

For ``TRLC`` both a VSCode Extension and a LSP Server (e.g. for Clion) are
`available <https://github.com/bmw-software-engineering/trlc-vscode-extension>`_

.. _Assumed System Requirements:

Assumed System Requirements
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

System-level requirements that your SEooC receives from the wider context — for example,
from a system specification. The excerpt below is taken from the reference example's
`assumed_system_requirements.trlc <https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/examples/seooc/docs/requirements/assumed_system_requirements.trlc>`_:

.. code-block:: text

    package SampleSEooC

    import ScoreReq

    ScoreReq.AssumedSystemReq ASR_SAMPLE_001 {
        description = "The system shall provide safe and reliable numeric value management through encapsulated classes, compliant with the selected :term:`integrity level`."
        safety = ScoreReq.Asil.B
        version = 1
        rationale = "System-level requirement for managing numeric values in a safety-critical context"
    }

.. code-block:: starlark
   :caption: docs/requirements/BUILD

   assumed_system_requirements(
       name = "assumed_system_requirements",
       srcs = ["assumed_system_requirements.trlc"],
       visibility = ["//visibility:public"],
   )

Feature Requirements
~~~~~~~~~~~~~~~~~~~~~

Taken from
`feature_requirements.trlc <https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/examples/seooc/docs/requirements/feature_requirements.trlc>`_,
derived from the ``ASR_SAMPLE_001`` requirement above:

.. code-block:: text

    package SampleSEooC

    import ScoreReq

    ScoreReq.FeatReq FEAT_001 {
        description = "The :term:`component` shall provide a numeric value management interface that returns a `uint8_t` value on every read access, aligned with the :term:`feature requirements`."
        safety = ScoreReq.Asil.B
        derived_from = [SampleSEooC.ASR_SAMPLE_001@1]
        version = 1
    }

.. code-block:: starlark
   :caption: docs/requirements/BUILD

   feature_requirements(
       name = "feature_requirements",
       srcs = ["feature_requirements.trlc"],
       visibility = ["//visibility:public"],
       deps = [":assumed_system_requirements"],
   )

Component Requirements
~~~~~~~~~~~~~~~~~~~~~~~

``derived_from`` uses the versioned tuple syntax ``[Package.RecordId@version]`` and may
reference more than one parent requirement, as ``REQ_COMP_004`` does below. Taken from
`component_requirements.trlc <https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/examples/seooc/docs/requirements/component_requirements.trlc>`_:

.. code-block:: text

    package SampleComponent

    import ScoreReq
    import SampleSEooC

    ScoreReq.CompReq REQ_COMP_001 {
        description = "The numeric value management interface shall provide a read operation that returns a uint8_t value"
        safety = ScoreReq.Asil.B
        derived_from = [SampleSEooC.FEAT_001@1]
        version = 1
    }

    ScoreReq.CompReq REQ_COMP_004 {
        description = "The numeric value validator shall accept a numeric value manager instance as its sole constructor argument"
        safety = ScoreReq.Asil.B
        derived_from = [SampleSEooC.FEAT_003@1, SampleSEooC.FEAT_004@1]
        version = 1
    }

.. code-block:: starlark
   :caption: docs/requirements/BUILD

   component_requirements(
       name = "component_requirements",
       srcs = ["component_requirements.trlc"],
       visibility = ["//visibility:public"],
       deps = [
           ":assumed_system_requirements",
           ":feature_requirements",
       ],
   )

Validation
----------

Every requirement target generates a ``<name>_test`` target that runs ``trlc --verify``
on your ``.trlc`` sources. This check runs automatically as part of ``bazel test ...``.

The validation catches:

- **Syntax errors** — malformed TRLC records
- **Type errors** — wrong value types for fields (e.g. a string where an enum is expected)
- **Mandatory field violations** — missing ``description``, ``safety``, or ``version``
- **Broken cross-references** — a ``derived_from`` or ``satisfies`` pointing to a non-existent record
- **Unknown fields** — fields not defined in the S-CORE requirements model

To run the validation for a single target:

.. code-block:: bash

    bazel test //my/package:my_feature_req_test

.. _requirements-images:

Adding Images and Diagrams to Requirement Descriptions
--------------------------------------------------------

A requirement's ``description`` field can embed images and PlantUML diagrams so
they are rendered directly in the generated Sphinx documentation, right next
to the requirement text.

**Markdown-style images** — use ``![alt](path)``; it is converted to an RST
``.. image::`` directive:

.. code-block:: text

    ScoreReq.CompReq COMP_002 {
        description = '''The system shall expose the following architecture.

        ![Architecture overview](diagrams/arch.svg)'''
        safety       = ScoreReq.Asil.B
        derived_from = [MySeooc.FEAT_001@1]
        version      = 1
    }

**PlantUML diagrams** — write a raw ``.. uml::`` RST directive; it is passed
through unmodified (any RST directive, e.g. ``.. image::``, ``.. figure::``, or
``.. uml::``, is preserved as-is):

.. code-block:: text

    ScoreReq.CompReq COMP_003 {
        description = '''The `ClientConnection` shall maintain a state machine.

        .. uml:: client_connection_activity_diagram.puml'''
        safety       = ScoreReq.Asil.B
        derived_from = [MySeooc.FEAT_001@1]
        version      = 1
    }

In both cases, the referenced file must also be declared via the ``image_srcs``
attribute (available on ``assumed_system_requirements``, ``feature_requirements``,
and ``component_requirements``) so it gets staged next to the rendered ``.rst``
file. The path written in the directive must match the file's package-relative
path:

.. code-block:: starlark

    component_requirements(
        name = "comp_req",
        srcs = ["docs/requirements.trlc"],
        image_srcs = [
            "diagrams/arch.svg",
            "//path/to:client_connection_activity_diagram.puml",
        ],
    )

.. note::
   Prefer ``.svg`` over ``.png`` for images checked into git. SVG is text-based and
   diffs/compresses cleanly, whereas ``.png`` is a binary blob — every change adds a
   full new copy to the git history and bloats the repository over time.

Allocation of Requirements to Architectural Elements
------------------------------------------------------

Requirements are allocated to architectural elements differently depending on their level:

**Component Requirements (``CompReq``)**
``CompReq`` records are associated with exactly one component. The allocation is
expressed implicitly through Bazel: The Bazel Component rule exposes an attribute
for requirements that accepts any component_requirements target. However, since
the entire file is assigned to a single component, requirements must be split into
separate files.

.. code-block:: starlark

    component(
        name = "MyComponent",
        components = [":MyUnit"],
        requirements = [":component_requirements"],
        tests = [],
    )

**Feature Requirements (``FeatReq``)**
``FeatReq`` records operate at the integration level — they are too broad for a
single component and can only be satisfied by multiple components working together.
They are therefore allocated to the ``dependable_element`` as a whole via the Bazel
``requirements`` attribute:

.. code-block:: starlark

    dependable_element(
        name = "my_element",
        requirements = [":feature_requirements"],   # FeatReq targets
        ...
    )

The traceability from ``FeatReq`` down to the components that implement it runs
through the ``component_requirements`` chain (``FeatReq → CompReq → component``).

AI-Powered Quality Check
------------------------

In addition to the structural TRLC validation described above, ``rules_score``
provides an AI-powered quality check for requirements via the
``trlc_requirements_ai_test`` rule. Unlike the structural check — which validates
syntax, types, and cross-references — the AI check evaluates the *quality* of each
requirement against requirements engineering guidelines (clarity, testability,
completeness, etc.). However since LLM are not deterministic, it is not recommended
to run it in the CI.

``trlc_requirements_ai_test``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: starlark

    load("@score_tooling//validation/ai_checker:ai_checker.bzl",
         "trlc_requirements_ai_test")

    trlc_requirements_ai_test(
        name = "feature_requirements_ai_check",
        reqs = [":feature_requirements"],
        score_threshold = "6.0",
        tags = ["manual"],
    )

Run the check explicitly with:

.. code-block:: bash

    bazel test //my/package:feature_requirements_ai_check

**Prerequisites:** a GitHub Copilot licence (default) or a custom AI model
configured via the ``_custom_ai_model`` attribute — see
``https://github.com/eclipse-score/tooling/blob/main/validation/ai_checker/README.md``
in the score-tooling repository for details.

Modeling Requirements in Bazel Rules
--------------------------------------

For the complete attribute reference for all requirements Bazel rules, see the
rule index:

- :ref:`assumed_system_requirements <rule-assumed-system-req>`
- :ref:`feature_requirements <rule-feature-requirements>`
- :ref:`component_requirements <rule-component-requirements>`
- :ref:`assumptions_of_use <rule-assumptions-of-use>`
