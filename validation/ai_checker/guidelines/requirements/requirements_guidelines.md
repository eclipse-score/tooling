<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Requirements Writing Guidelines

General guidelines for creating and formulating requirements. Project-specific
details (e.g. the available requirement levels) are supplied separately by the
project guidelines.

## Sentence Template

Every requirement **shall** follow this structure:

> **\<Subject\>** shall **\<main verb\>** **\<object\>** **\<parameter\>** **\<temporal/logical conjunction\>**

Of the last three parts (object, parameter, conjunction), at least one is mandatory — the others are optional.

### Examples

| Subject | shall | Verb | Object | Parameter | Condition |
|---|---|---|---|---|---|
| The component | shall | detect | if a key-value pair got corrupted | and set its status to INVALID | during every restart of the SW platform. |
| The software platform | shall | enable | users | to ensure the compatibility of application software | across vehicle variants and releases. |
| The linter-tool | shall | check | correctness of .rst files format | | upon each commit. |

## Quality Criteria

A well-written requirement is:

- **Unambiguous** — only one possible interpretation
- **Verifiable** — can be tested or reviewed
- **Atomic** — expresses a single need (one "shall" per requirement)
- **Consistent** — no contradictions with other requirements
- **Complete** — contains subject, verb, and at least one of: object, parameter, or condition
- **Necessary** — traceable to a parent requirement or rationale

### Avoid

- Vague terms: *approximately*, *as appropriate*, *user-friendly*, *fast*, *efficient*
- Unbounded lists: *etc.*, *and so on*, *such as* (without closing the list)
- Compound requirements: multiple "shall" statements in one requirement
- Implementation details in stakeholder/feature requirements
- Missing conditions or parameters that leave behaviour undefined

## Requirement Types Explained

| Type | Meaning | Verification |
|---|---|---|
| **Functional** | Behaviour that can be observed | Unit/integration test |
| **Interface** | API or protocol specification | Test or inspection |
| **Non-Functional** | Quality attribute (performance, reliability) | Review/analysis |
| **Process** | Process-related constraint | Process review |

## Pre-flight Checks — Apply Before Raising Any Finding

1. **Domain terms** — Before flagging a term as *vague*, check whether it has a well-established formal or domain-specific definition (mathematical, safety-engineering, OS/systems, or software-engineering). Formally defined terms (e.g. *monotonic*, *bounded*, *idempotent*, *deterministic*, *ASIL*, …) are precise by definition and shall not be flagged.

2. **Clarification clauses** — Before flagging a second sentence or clause as a *compound requirement*, check whether it is a clarification, exclusion, negative-scope statement, or rationale that qualifies the main behaviour. Only independent normative *shall* statements and different scopes constitute separate requirements.

3. **Abstraction patterns** — Before flagging a phrase as a *contradiction*, check whether it describes an abstraction layer. A phrase such as "OS-independent API for OS-native" expresses an abstraction of the underlying mechanism — this is intentional, not contradictory.

4. **Architectural subjects** — Before flagging a named component in the subject as a *level mismatch*, check whether the requirement constrains an architectural design decision rather than prescribing a code-level implementation detail. Naming an architectural element as the subject does not automatically lower the requirement's level.

5. **Higher-level parameters** — Before flagging a missing parameter (e.g. list of supported OSes, applicable safety standard) as a *major* finding, consider whether that parameter is resolved at system or project level and intentionally omitted from individual requirements. Downgrade to *minor* unless there is clear evidence it is undefined everywhere.

6. **Design / implementation constraints** — Before flagging a requirement as *implementation-specific* or *below the expected abstraction level*, check whether it is an intentional design constraint that deliberately restricts the implementation (e.g. mandating a particular transport, mechanism, platform, or safety property, or scoping behaviour to a specific OS such as QNX). Such constraints are legitimate requirements: naming an implementation, technology, or OS context in their subject or object is the *purpose* of the requirement and shall not be flagged as a level mismatch.

7. **Feature self-reference** — Before flagging a subject as *underspecified* or *ambiguous*, check whether it names the requirement's own feature (e.g. "The communication" in a communication feature, "The logging" in a logging feature). Referring to the feature by name is an acceptable subject and unambiguous in context. Do not raise it as a defect; at most note it as a *minor* consistency suggestion if other requirements use a different wording.

8. **Detail that belongs to a lower level** — Before flagging a requirement as *underspecified*, *incomplete* or *missing a parameter*, check whether the missing detail belongs to a **lower** abstraction level than the one being reviewed (see the project requirement levels). A requirement is complete when it specifies the behaviour expected *at its own level*; it must **not** pre-empt decisions owned by the level(s) below it. For example, at feature or component level "shall report an error" is complete — the concrete error code, type, message, internal data structure or algorithm is settled in detailed design and shall not be demanded here. Only flag genuinely missing behaviour at the element's own level, not absent lower-level detail.
