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
**not** parse PlantUML itself — it only reads the small `.idmap.json` sidecar
files produced upstream and uses them to inject link directives before
`sphinxcontrib-plantuml` renders the image.

A PlantUML class diagram of the pipeline (steps plus their input/output
artifacts) shows the three stages summarized below. Its source is stored in
`docs/assets/clickable_plantuml_pipeline.puml`:

```{uml} docs/assets/clickable_plantuml_pipeline.puml
:align: center
:alt: clickable_plantuml pipeline overview
```

1. **Parse & resolve** — `puml_cli` (in `plantuml/parser/`) is invoked once per
   `.puml` file by the `architectural_design()` Bazel rule. It preprocesses
   `!include`s, parses the diagram with a `pest`-grammar parser
   (`puml_parser`), and resolves aliases/references into a fully-qualified
   model (`puml_resolver`). This is the only stage that actually understands
   PlantUML syntax.
2. **idmap generation** — `puml_idmap` walks the resolved model and classifies
   each element as either a *define* (something this diagram elaborates,
   e.g. a component with children, or a class with members) or a *reference*
   (a leaf mention or relation endpoint that should link elsewhere). The
   result is written as one `<file_stem>.idmap.json` sidecar per diagram — see
   the *Role detection algorithm* and *idmap Format* sections below for the
   exact rules and shape.
3. **Sphinx consumption** — `clickable_plantuml.py` (this directory) never
   touches `.puml` source or FlatBuffers output. At `builder-inited` it scans
   the source tree for every `*.idmap.json`, builds a global
   `{alias|id → [definer source paths]}` index, and at `doctree-resolved` uses
   that index to rewrite each diagram's `uml` text with
   `url of <alias> is [[<link>]]` directives *before* handing it off to
   `sphinxcontrib-plantuml`'s own renderer, which does the actual PlantUML →
   SVG/PNG rendering.

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
  │  (one-time setup)                     Load all *.idmap.json files from
  │                                       srcdir (recursive).
  │                                       Build definition index:
  │                                       {alias|id → [definer source paths]}.
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
  │       (per document)                 For each plantuml node, load its idmap.
  │                                      For each reference entry, look up the
  │                                      definition index (FQN first, then alias).
  │                                      Apply proximity tiebreak on ambiguity.
  │                                      Build the URL (relative to _images/ in
  │                                      svg_obj mode, else page-relative via
  │                                      get_relative_uri), then append
  │                                      url of <alias> is [[url]] directives to
  │                                      node['uml'] before rendering.
  │
  build-finished
```

## How It Works

1. **idmap discovery** (`builder-inited`) – Scans for `*.idmap.json` files in
   the Sphinx source directory.  Each sidecar records *defines* (elements
   elaborated in that diagram, i.e. with children/members) and *references*
   (leaf mentions and relation endpoints).  A global definition index maps
   each alias/FQN to the set of diagrams that elaborate it.

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

3. **URL resolution & link injection** (`doctree-resolved`) – For each
   reference in a diagram's idmap, resolves the unique definer via the index.
   When multiple diagrams define the same element, a *proximity tiebreak*
   selects the definer sharing the longest common path prefix with the source
   diagram.  On a genuine tie, no link is emitted (safe over wrong).  URLs are
   built relative to `_images/` in `svg_obj` mode (else page-relative via
   `app.builder.get_relative_uri()`) and percent-encoded before injection.

4. **Incremental / parallel support** – `env-purge-doc` removes stale entries
   when a document is re-read; `env-merge-info` merges state from parallel
   worker processes.

## Automatic idmap Generation (Bazel)

`.idmap.json` sidecars are produced by the `architectural_design()` rule.

The rule passes `--source-name <puml_file.short_path>` and
`--idmap-output-dir` to `puml_cli` for every `.puml` file.  The
`--source-name` argument **must be the srcdir-relative workspace path** to
satisfy the canonical-key invariant.  Passing `short_path` (the workspace
root-relative path for Bazel sources) ensures this requirement is met.  The
resulting idmap `source` field is a stable, unique, workspace-relative path
(e.g. `score/mw/com/proxy_detail.puml`), which becomes the diagram's identity
key throughout the extension.  Duplicate `source` values across idmaps raise
an error; duplicate basenames (file stems) within one target also raise an
error.

### Role detection algorithm

Given the resolved model of one `.puml` diagram:

1. **defines** – An element is a *define* when any of the following hold:
   - At least one other element lists it as its `parent_id` (component diagrams).
   - It has member variables or methods (class diagrams).
   - The diagram's `@startuml <name>` matches its alias or display name
     (component and class diagrams).
   - It is a `$TopEvent` node — the tree root whose `connection` is `None`,
     never used as a relation source (FTA diagrams).
2. **references** – Elements that link away to another diagram:
   - Top-level leaf boxes and relation endpoints (component diagrams).
   - All participants (sequence diagrams — no defines in sequence).
   - `$TransferInGate` nodes whose alias is a TRLC-style FQN
     (`Package.Record`) referencing another diagram's top event (FTA diagrams).
   - Internal FTA nodes (`$BasicEvent`, `$IntermediateEvent`, `$AndGate`,
     `$OrGate`) are omitted — they do not cross-link to other diagrams.

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

`proxy_detail.idmap.json`:
```json
{ "source": "score/mw/com/proxy_detail.puml",
  "defines":    [{ "alias": "Proxy",          "id": "Proxy" }],
  "references": [{ "alias": "RequestHandler", "id": "Proxy.RequestHandler" }] }
```

`overview.idmap.json`:
```json
{ "source": "score/overview.puml",
  "defines":    [],
  "references": [{ "alias": "Gateway", "id": "Gateway" },
                 { "alias": "Proxy",   "id": "Proxy"   }] }
```

Result: `Proxy` in `overview.puml` links to `proxy_detail.puml`.
`Gateway` has no definer → no link.

## idmap Format

`.idmap.json` files are written by the parser and read by this extension.
They are not intended to be authored manually.

```json
{
  "source": "path/to/diagram.puml",
  "defines": [
    { "alias": "ComponentName", "id": "fully.qualified.Name" }
  ],
  "references": [
    { "alias": "OtherComponent", "id": "OtherComponent" }
  ]
}
```

## End-to-End Clickable Diagram Example

Rather than duplicating a hand-written, untested example here, this exact
scenario is built and regression-tested as part of `rules_score`'s own test
suite:

- [`overview.puml`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/fixtures/clickable_example/overview.puml)
  references `Proxy` (a top-level leaf, per the *Role detection algorithm*
  above).
- [`proxy_detail.puml`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/fixtures/clickable_example/proxy_detail.puml)
  defines `Proxy` as a `package` with a nested child, making it a *define*.

Both files are wired into a real `architectural_design()` +
`dependable_element()` target
(`clickable_example_lib` in [`bazel/rules/rules_score/test/BUILD`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/BUILD)),
so every change to the parser, `puml_idmap`, or this extension is checked
against genuinely Bazel-built `.idmap.json` artifacts — not just prose. The
[`clickable_example_link_rendered_test`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/check_clickable_example_link.sh)
`sh_test` asserts that:

- `overview.idmap.json` references `Proxy`
- `proxy_detail.idmap.json` defines `Proxy`

i.e. that clickable_plantuml has exactly what it needs to make the rendered
`Proxy` element in `overview.puml` clickable, linking to the page containing
`proxy_detail.puml`. Run it with:

```shell
bazel test //bazel/rules/rules_score/test:clickable_example_link_rendered_test
```

The same pattern covers the other diagram-type scenarios from the *Role
detection algorithm* above:

- **Cross-diagram-type interface linking** — `interface_overview.puml` (a
  component diagram) references `InternalInterface` via a unit's `-(`
  binding, and `interface_detail.puml` (auto-detected as a class diagram
  because the interface has a method) defines it. Both share the FQN
  `package_a.InternalInterface`, since ids are rooted at the enclosing
  `package_a` package rather than the `@startuml` name. See
  `interface_example_lib` and `interface_example_link_rendered_test` in
  [`bazel/rules/rules_score/test/BUILD`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/BUILD).
- **Pure class-diagram linking** — `class_overview.puml` references
  `AuditTrail` as a bare, member-less class, and `class_detail.puml` defines
  it (it has a method, making it the elaboration site). See
  `class_example_lib` and `class_example_link_rendered_test` in the same
  `BUILD` file.

The following three scenarios each cover a full, real-world "chain" from the
*Role detection algorithm* — a reference in one architectural view and the
matching definition in another — and are wired up the same way:

- **Interface in the static architecture links to the public API** —
  `public_api_overview.puml` (a component diagram, `static` attribute) shows a
  unit bound to `PublicInterface` via a `-(` port, and `public_api_detail.puml`
  (passed via architectural_design's `public_api` attribute, which also feeds
  FMEA/safety-analysis traceability) defines it. Both share the FQN
  `package_pub.PublicInterface`. See `public_api_example_lib` and
  `public_api_example_link_rendered_test` in
  [`bazel/rules/rules_score/test/BUILD`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/BUILD).
- **Interface in a `static_view` diagram links to the public API** — a
  `static_view` diagram (a component diagram passed via
  architectural_design's `static_view` attribute — a partial/subset view of
  the static architecture, parsed identically to `static`) is scanned for
  `*.idmap.json` sidecars exactly like every other architectural view, so an
  interface bound in it is clickable the same way as one in `static`:
  `static_view_overview.puml` shows a unit bound to `SvInterface` via a `-(`
  port, and `static_view_detail.puml` (passed via `public_api`) defines it.
  Both share the FQN `package_sv.SvInterface`. See `static_view_example_lib`
  and `static_view_example_link_rendered_test` in
  [`bazel/rules/rules_score/test/BUILD`](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/test/BUILD).
- **Static architecture unit links to its class diagram** —
  `unit_overview.puml` (a component diagram) shows `unit_one` as a leaf unit
  (no children, so a reference), and `unit_class_detail.puml` (a class
  diagram naming a class after the same unit, nested in the matching
  namespace) defines it. Both share the FQN `package_u.component_u.unit_one`,
  since component-diagram ids and class-diagram namespace FQNs use the same
  dot-joined scheme. See `unit_example_lib` and
  `unit_example_link_rendered_test` in the same `BUILD` file.
- **Dynamic architecture participant links to its own class diagram, with a
  shared public API** — `dynamic_overview.puml` (a sequence diagram, `dynamic`
  attribute) shows participants `Dispatcher` and `Worker` — both references,
  since sequence diagrams have no defines. `dynamic_component_detail.puml` (a
  component diagram, `static` attribute) shows both as leaf units (also
  references, since neither has children) sharing a `TaskInterface` binding
  (so the diagrams also pass the architectural-design consistency validator).
  Each unit is elaborated by its own class diagram
  (`dynamic_dispatcher_class.puml`, `dynamic_worker_class.puml`, also in
  `static`), which is where the click ultimately lands. `TaskInterface` itself
  is only ever a reference in the component diagram; its definition lives in
  the public API (`dynamic_public_api_detail.puml`, via the `public_api`
  attribute), exactly like the public-API scenario above. See
  `dynamic_example_lib`, `dynamic_example_link_rendered_test`,
  `dynamic_dispatcher_class_link_rendered_test`,
  `dynamic_worker_class_link_rendered_test` and
  `dynamic_public_api_link_rendered_test` in the same `BUILD` file.

  **Limitation: clicking on individual *messages* in a sequence diagram (e.g.
  `Assign`/`Done`) is not currently supported**, for two independent reasons:
  1. `sequence_model_to_idmap()` in `puml_idmap/src/lib.rs` only extracts
     `caller`/`callee` (participant names) into `references`. The resolved
     `SequenceTree` (`tools/metamodel/sequence/sequence_logic.rs`) *does*
     carry the message text (`Interaction.method`, `Return.return_content`),
     but the idmap converter never reads those fields, so no reference is
     ever produced for a message label today.
  2. Even if (1) were fixed, this extension's injection mechanism
     (`_inject_links_into_uml`) only knows how to append `url of <alias> is
     [[url]]` directives before `@enduml` — a PlantUML feature that attaches
     a link to a *declared, aliased element* (participant/class/component),
     not to an arrow's message text. Making a message clickable requires
     PlantUML's other, unrelated hyperlink form — rewriting the message line
     itself, e.g. `Dispatcher -> Worker : [[url Assign]]` — which needs a
     different, source-location-aware rewriting path (to target the right
     occurrence when a label repeats) that doesn't exist yet.
