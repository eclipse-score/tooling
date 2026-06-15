<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# General Requirements Inspection Guidelines

## Markup and Notation Conventions

The following expressions are **fully defined** and shall not be flagged as vague or incomplete:

- Markdown links: `[label](http://example.com)`
- RST links: `` `a link`_ `` or `.. _a link: https://domain.invalid/`
- Backtick-quoted terms: `` `term` `` — treated as a defined reference to a known concept
- Parenthetical anchors: `(Term)` — treated as a lower-level context link, equivalent to a backtick reference
- Expressions provided as comments

## How to Inspect Each Requirement

Review every requirement individually against the project-specific guidelines. Apply all pre-flight checks below before raising a finding.

### Pre-flight Checks — Apply Before Raising Any Finding

1. **Domain terms** — Before flagging a term as *vague*, check whether it has a well-established formal or domain-specific definition (mathematical, safety-engineering, OS/systems, or software-engineering). Formally defined terms (e.g. *monotonic*, *bounded*, *idempotent*, *deterministic*, *ASIL*, …) are precise by definition and shall not be flagged.

2. **Clarification clauses** — Before flagging a second sentence or clause as a *compound requirement*, check whether it is a clarification, exclusion, negative-scope statement, or rationale that qualifies the main behaviour. Only independent normative *shall* statements and different scopes constitute separate requirements.

3. **Abstraction patterns** — Before flagging a phrase as a *contradiction*, check whether it describes an abstraction layer. A phrase such as "OS-independent API for OS-native" expresses an abstraction of the underlying mechanism — this is intentional, not contradictory.

4. **Architectural subjects** — Before flagging a named component in the subject as a *level mismatch*, check whether the requirement constrains an architectural design decision rather than prescribing a code-level implementation detail. Naming an architectural element as the subject does not automatically lower the requirement's level.

5. **Higher-level parameters** — Before flagging a missing parameter (e.g. list of supported OSes, applicable safety standard) as a *major* finding, consider whether that parameter is resolved at system or project level and intentionally omitted from individual requirements. Downgrade to *minor* unless there is clear evidence it is undefined everywhere.

### What to Accept Without Findings

- Expressions marked as fully defined (see Markup and Notation Conventions above)
- Acceptable clarification patterns:
  - **Exclusion / negative-scope** — declarative sentences that narrow the scope (e.g. "N:M connections are explicitly excluded.", "No guarantee is provided that the message is delivered."). These are qualifications of the main behaviour, not additional requirements.
  - **Parenthetical lower-level anchors** — e.g. "(SendWaitReply)" linking to a known API concept.
  - **Rationale fragments** — explanatory text following the normative sentence, provided they do not contradict it.

### What Not to Do

- Do not analyse relationships or hierarchy between requirements (e.g. `parent` attributes, traceability links)
- Do not flag a finding for something that is fully defined
- Do not apply the sentence template too rigidly — optional parts of the template are optional
- Do not invent context that is not present in the requirement text

## Quality Criteria

A well-written requirement is:

| Criterion | Meaning |
|---|---|
| **Unambiguous** | Only one valid interpretation |
| **Verifiable** | Can be tested, measured, or reviewed |
| **Atomic** | Expresses a single independent need |
| **Consistent** | No internal contradictions |
| **Complete** | Subject + verb + at least one of: object, parameter, or condition |
| **Necessary** | Traceable to a parent need or rationale |

### Terms That Indicate Vagueness (when not formally defined)

*approximately*, *as appropriate*, *user-friendly*, *fast*, *efficient*, *easy*, *sufficient*, *adequate*, *reasonable*, *and so on*, *etc.*, *such as* (when the list is not closed)

### Compound Requirement Indicators

Flag as compound only when a requirement contains **multiple independent normative *shall* statements**. A single *shall* followed by clarifying clauses is not compound.

## Sentence Template

Every requirement shall follow this structure:

> **\<Subject\>** shall **\<verb\>** **\<object\>** **\<parameter\>** **\<condition\>**

Of the last three parts (object, parameter, condition), at least one is mandatory — the others are optional. Apply the template with judgment; do not flag a requirement as incomplete solely because one optional field is absent.

### Examples

| Subject | shall | Verb | Object | Parameter | Condition |
|---|---|---|---|---|---|
| The component | shall | detect | if a key-value pair got corrupted | and set its status to INVALID | during every restart of the SW platform. |
| The software platform | shall | enable | users | to ensure the compatibility of application software | across vehicle variants and releases. |
| The linter-tool | shall | check | correctness of .rst files format | | upon each commit. |

## Writing Conventions

### Context and Notation

| Notation | Meaning |
|---|---|
| `` `term` `` (backtick) | The term is defined elsewhere; treat as fully specified |
| `(Term)` (parentheses) | Lower-level anchor or well-known API/OS term; treat as context, not implementation detail |
| Declarative exclusion sentence | Narrows scope of the main *shall*; not a separate requirement |
| Negative-scope clause | Describes what the requirement does *not* guarantee; not an additional requirement |

### Avoid

- Undefined vague qualifiers: *approximately*, *as appropriate*, *user-friendly*, *optimized* (without a measurable target), *fast*, *efficient*
- Unbounded lists: *etc.*, *and so on*, *such as* (without closing the enumeration)
- Multiple independent *shall* statements in one requirement (compound requirements)
- Code-level implementation details in FeatReq or Stakeholder Requirements
- Typos and grammatical fragments that change meaning (e.g. sentence-starting "Where" as a fragment)


## Output Format

- Include the requirement type in the analysis
- Report only findings — do not describe what is correct
- Categorise each finding as **major** or **minor**
- Make every finding specific and actionable; reference the relevant guideline keyword (e.g. *Vague*, *Compound*, *Unverifiable*)
- Provide one overall score per requirement
- Use HTML formatting where required by the output template

## Scoring

Score each requirement from 0 to 10:

| Score | Meaning |
|---|---|
| 0–3 | Critical issues — major rework required |
| 4–6 | Moderate issues — improvement needed |
| 7–8 | Good quality — minor improvements possible |
| 9–10 | Excellent quality — meets professional standards |
