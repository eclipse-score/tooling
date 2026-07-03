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
  Matching is exact canonical-key equality (no basename fallback, no suffix
  fallback), so same-basename diagrams in different packages remain distinct.

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

## Invariants

The extension relies on two invariants held by the idmap producer
(`architectural_design()` / `puml_cli`):

1. **The `--source-name` must be the srcdir-relative workspace path.**
   `--source-name` is passed to `puml_cli` and becomes the `source` field in
   the idmap.  This value is the canonical key used for all matching: a
  plantuml node resolves only when its derived canonical key equals the idmap
  `source` key exactly (normalized to POSIX form).  The value must be stable
  across builds and unique — non-unique `source` values raise an
  `ExtensionError` rather than silently mislinking.  In Bazel: pass
   `puml_file.short_path` to satisfy this invariant regardless of how Sphinx
   roots `srcdir` or how Bazel symlinks the staged sources.

2. **PlantUML basenames (file stems) must be unique within a single
   `architectural_design` target.**  Each `.idmap.json` is written as
   `<file_stem>.idmap.json` under the target's output directory, so two
   diagrams sharing a stem in one target would collide on output (build
   error).  Same basenames across *different* targets/packages are fine —
   exact canonical-key matching via `--source-name` keeps them independent.

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

This minimal example shows what users should create in docs to get a clickable
diagram:

`docs/arch/overview.puml`
```text
@startuml
[Gateway] --> [Proxy]
@enduml
```

`docs/arch/proxy_detail.puml`
```text
@startuml
package Proxy {
   [RequestHandler]
}
@enduml
```

`docs/arch/overview.rst`
```rst
Overview
========

.. uml:: overview.puml
```

`docs/arch/proxy_detail.rst`
```rst
Proxy Detail
============

.. uml:: proxy_detail.puml
```

When the idmaps contain:

- `overview.puml` references `Proxy`
- `proxy_detail.puml` defines `Proxy`

the rendered `Proxy` element in `overview.puml` becomes clickable and opens
the page containing `proxy_detail.puml`.
