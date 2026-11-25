<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# clickable_plantuml

Sphinx extension that makes PlantUML diagrams clickable by injecting hyperlinks into rendered SVG/PNG diagrams.

## Concept: From `.puml` Source to Clickable Diagram

This extension is only the last of three stages in a pipeline that turns plain
`.puml` text files into cross-linked, clickable architecture diagrams. It does
**not** parse PlantUML and does **not** decide which diagram links where: it
reads the `diagram_links.json` files computed upstream and injects their links
before `sphinxcontrib-plantuml` renders the image.

A PlantUML diagram of the pipeline (steps plus their input/output artifacts)
shows the three stages summarized below. Its source is stored in
`docs/assets/clickable_plantuml_pipeline.puml`:

```{uml} docs/assets/clickable_plantuml_pipeline.puml
:align: center
:alt: clickable_plantuml pipeline overview
```

1. **Parse & resolve** — `puml_cli` (in `plantuml/parser/`) parses the `.puml`
   files of an `architectural_design()` or `unit_design()` view and writes one
   `<stem>.fbs.bin` FlatBuffer per diagram. The FlatBuffer carries the
   resolved model, the workspace-relative `source` path and, for component
   diagrams, the `@startuml` name.
2. **Diagram links** — the `dependable_element()` rule runs
   `validation_cli --profile dependable-element --links-output --links-only`
   over the FlatBuffers of all its architectural and unit designs and stages
   the result as `<name>_index/diagram_links.json`. The link rules live in
   `validation/core/src/links/` (see *Link Rules* below).
3. **Sphinx consumption** — `clickable_plantuml.py` (this directory) loads
   every `diagram_links.json` of the source tree at `builder-inited` and, at
   `doctree-resolved`, rewrites each diagram's `uml` text with
   `url of <alias> is [[<link>]]` directives and linked call labels *before*
   handing it off to `sphinxcontrib-plantuml`'s own renderer.

## Sphinx Integration

The extension hooks into the native Sphinx build lifecycle.  URL computation
depends on the configured `plantuml_output_format`: in `svg_obj` mode the
rendered SVG lives in `_images/`, so links are made relative to that directory
(`os.path.relpath(target_uri, imagedir)`); for inline `svg`/`png` the link is
relative to the containing HTML page via
`app.builder.get_relative_uri(from_docname, to_docname)`.

```
Sphinx build lifecycle                   clickable_plantuml hooks
═══════════════════════════════════      ═══════════════════════════════════════

  builder-inited                   ───► on_builder_inited()
  │  (one-time setup)                     Load all diagram_links.json files
  │                                       from srcdir (recursive, sorted).
  │                                       Index element links by source and
  │                                       call links by (source, line).
  │
  ├─ READ PHASE ──────────────────────────────────────────────────────────────
  │  for each document:
  │    env-purge-doc               ───► on_env_purge_doc()
  │    │  (incremental rebuild)          Remove stale puml→docname entries
  │    │                                 for the document being re-read.
  │    │
  │    parse RST → doctree
  │    │
  │    doctree-read                ───► on_doctree_read()
  │       (per document)                 Traverse the parsed doctree.
  │                                      For every plantuml node, record
  │                                      {normalized_source_path → docname}
  │                                      in app.env (path identity, not basename).
  │
  │  env-merge-info                ───► on_env_merge_info()
  │  (parallel builds only)              Merge puml→docname maps gathered
  │                                      by worker sub-processes into the
  │                                      main environment.
  │
  ├─ WRITE PHASE ─────────────────────────────────────────────────────────────
  │  for each document:
  │    post-transform / resolve
  │    │
  │    doctree-resolved            ───► on_doctree_resolved()
  │       (per document)                 For each plantuml node, look up the
  │                                      links of its source. Build the URL of
  │                                      each target document (relative to
  │                                      _images/ in svg_obj mode, else
  │                                      page-relative), append
  │                                      url of <alias> is [[url]] directives
  │                                      and link the labels of refined calls
  │                                      in node['uml'] before rendering.
  │
  build-finished
```

## How It Works

1. **Link loading** (`builder-inited`) – Loads every `diagram_links.json` in
   the Sphinx source directory in sorted path order. Nested dependable
   elements stage their own file; when two files link the same alias or call,
   the first wins. A file with an unknown `version` is skipped with a
   warning.

2. **Diagram location mapping** (`doctree-read`) – Records which `docname`
  contains which `.puml` diagram, keyed by the canonical workspace-relative
  path.  A node's identity is recovered by normalising its absolute path
  (`srcdir` + the node's `incdir` + `filename`) and deriving exactly one
  canonical key using the precomputed workspace offset from `builder-inited`.
  Matching is exact canonical-key equality after an `os.path.realpath` prefix
  strip; a safe *unique*-suffix fallback additionally covers symlinked/staged
  layouts where the prefix strip doesn't line up (only used when exactly one
  known source key matches — never a best-effort guess among several
  same-basename candidates), so same-basename diagrams in different packages
  remain distinct.

3. **Element links** (`doctree-resolved`) – Each element link of the diagram
   whose target diagram is part of the docs becomes
   `url of <alias> is [[url]]`. URLs are percent-encoded before injection.
   An alias that is not a plain identifier is written quoted
   (`url of "a::b" is [[url]]`); one that contains a double quote or a line
   break cannot be written and is reported with a warning.

4. **Call links** (`doctree-resolved`) – The label of a refined call
   (`A -> B : Get()`) on the linked line becomes `[[url Get()]]`; the arrow
   and the lifeline stay unlinked. The link goes to the first target that is
   part of the docs; with several targets the tooltip names all. No link is
   written when the label already holds a link or when the label on that line
   does not name the call's method (a stale `diagram_links.json`).

5. **Incremental / parallel support** – `env-purge-doc` removes stale entries
   when a document is re-read; `env-merge-info` merges state from parallel
   worker processes. The `clickable_plantuml_links_digest` config value
   (`config-inited`) holds a digest of all `diagram_links.json` files; Sphinx
   re-reads all documents when it changes, so changed links never stay stale
   in unchanged pages.

## Link Rules

The rules are implemented and unit-tested in `validation/core/src/links/`;
the call links follow the sequence refinement rule of
`docs/specifications/sequence_refinement.md`.

- **Defines and references** – A component diagram element is a *define* when
  it has children or the `@startuml` name is its alias or name. A class
  diagram entity is a define when it has members or the `@startuml` name is
  its name or last id segment; a namespace holding a define is a define too
  (a *namespace define*). All other elements, all relation endpoints and all
  sequence participants are *references*.
- **Element links** – A reference links to the one other diagram that
  defines its id. Without one, the last id segment is matched (a bare
  `unit_1` finds the diagram that defines `pkg.comp.unit_1`). Namespace
  defines lose against diagrams that draw the element; remaining ties go to
  the closest path, then to the diagram with most elements below the id. A
  tie after that gives no link and a warning (safe over wrong). The first
  reference per alias that has a definer is linked.
- **Call links** – A call links to the sequence diagrams that refine it, in
  any architectural design of the dependable element. Only refinements that
  pass the zoom check (closed windows, boundary kept, no exit missing) are
  linked; a call without such a refinement gets no link. Calls written in an
  `!include` fragment get no link.

### Concrete example

```text
' overview.puml — top-level leaves are REFERENCES
@startuml
[Gateway] --> [Proxy]
@enduml
```

```text
' proxy_detail.puml — Proxy has a child → DEFINE
@startuml
package Proxy { [RequestHandler] }
@enduml
```

Result: `Proxy` in `overview.puml` links to `proxy_detail.puml`.
`Gateway` has no definer → no link.

## `diagram_links.json` Format

Written by `validation_cli --links-output`, read by this extension. Paths are
workspace-relative diagram sources; links are sorted.

```json
{
  "version": 1,
  "element_links": [
    { "source": "pkg/overview.puml", "alias": "Proxy", "target": "pkg/proxy_detail.puml" }
  ],
  "call_links": [
    { "source": "pkg/overview.puml", "line": 19, "method": "Get",
      "targets": ["pkg/get_detail.puml"] }
  ]
}
```

`line` is the 1-based line of the call in `source`; `method` is the call label
up to the first `(`.

## End-to-End Examples

The scenarios are built and regression-tested by `rules_score`'s own test
suite in
[`bazel/rules/rules_score/test/BUILD`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/BUILD).
Each `*_link_json_test` runs
[`check_diagram_link.sh`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/check_diagram_link.sh)
to assert one element link in the Bazel-built `diagram_links.json`:

- **Component diagrams** — `overview.puml` references `Proxy`,
  `proxy_detail.puml` defines it (`clickable_example_link_json_test`).
- **Cross-diagram-type interface** — the component diagram
  `interface_overview.puml` references `InternalInterface`, the class diagram
  `interface_detail.puml` defines it
  (`interface_example_link_json_test`).
- **Class diagrams** — `class_overview.puml` references `AuditTrail`,
  `class_detail.puml` defines it (`class_example_link_json_test`).
- **Static architecture to public API** — `public_api_overview.puml`
  references `PublicInterface`, defined in `public_api_detail.puml`
  (`public_api_example_link_json_test`).
- **Unit to its class diagram** — `unit_overview.puml` references
  `unit_one`, `unit_class_detail.puml` defines it
  (`unit_example_link_json_test`); variants match by namespace
  (`namespace_unit_example_link_json_test`) and by last id segment
  (`leaf_unit_example_link_json_test`).
- **Dynamic architecture** — participants of `dynamic_overview.puml` and units
  of `dynamic_component_detail.puml` link to their class diagrams, and
  `TaskInterface` to the public API (`dynamic_*_link_json_test`).
- **Call links** — `diagram_links_golden_test` pins the `diagram_links.json`
  of a dependable element whose zoom levels span two architectural designs:
  an element link, a call link, a call with two details and a call in an
  `!include` fragment (no link). `diagram_links_validation_test` asserts that
  its diagrams have no zoom findings, so every call link is validated.

```shell
bazel test //bazel/rules/rules_score/test:clickable_example_link_json_test
```
