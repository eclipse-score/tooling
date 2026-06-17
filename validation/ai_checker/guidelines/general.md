<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# General Review Guidelines

Common review methodology that applies to every artefact type (requirements,
architecture, …). Element-type-specific criteria are supplied by the matching
type guideline, and project-specific rules are supplied by the project
guidelines; this document only defines *how* to review and *how* to report.

## Expression Syntax

Treat the following as already defined; do not flag them as undefined:

- Markdown expressions (link: `[label](http://example.com)`)
- RST expressions (link: `.. _a link: https://domain.invalid/` or `` `a link`_ ``)
- Expressions provided as a comment or inside `` `` `` as fully defined.

## Applicable Standards

The project develops safety-related software under **ISO 26262** (road-vehicle
functional safety). Treat ISO 26262 as the governing functional-safety standard:

- Terms such as *safety-certified*, *safety-related*, *freedom from
  interference*, and *ASIL* (incl. ASIL A–D / QM) are defined by ISO 26262 and
  shall **not** be flagged as vague or undefined.
- Do not flag a safety requirement merely because it does not restate the
  standard or an ASIL level; the applicable standard and integrity level are
  established here at project level.

## Review Methodology

Review each element on its own:

- Evaluate the element against the applicable type and project guidelines.
- Accept expressions as defined if in doubt.

Do not:

- analyze the hierarchy of elements or relations between them (e.g. a parent
  attribute);
- mention any findings for fully defined items;
- apply any template too strictly.

## Scoring

Score each element from 0-10 where:

- 0-3: Critical issues, the element needs major rework
- 4-6: Moderate issues, improvement needed
- 7-8: Good quality with minor improvements possible
- 9-10: Excellent quality, meets professional standards

The score **must be consistent with the findings**:

- If an element has **no findings and no suggestions**, it has no identified
  weakness and **shall score 10**. Do not award 8 or 9 "just in case" — a score
  below 10 must be justified by at least one finding or suggestion.

## Analysis Result

- Provide specific, actionable findings and refer each to a concrete keyword
  from the element (e.g. *Vague* — element contains …).
- Only point out findings; do not mention if something is particularly well
  defined.
- Categorize findings as major and minor.
- **Suggestions are held to the same bar as findings.** A suggestion is only
  warranted when it points at a genuine, actionable weakness in the element not for
  stylistic "nice to have" rewordings of an already-clear element.
- Provide an overall score for the element.
- If required, use HTML formatting.
