# Requirements Writing Guidelines

Guidelines for creating and formulating requirements, derived from the SCORE Requirements Engineering Process.

## Requirement Levels

| Level | Scope | Derived From |
|---|---|---|
| **Stakeholder Requirement** | Platform-level functionality and safety mechanisms | Standards, customer needs |
| **Feature Requirement** | Integration-level behaviour, independent of component decomposition | Stakeholder Requirements |
| **Component Requirement** | Component-specific implementation details | Feature Requirements |
| **Assumption of Use (AoU)** | Boundary conditions for using a software element (any level) | Safety analyses, architecture |

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
