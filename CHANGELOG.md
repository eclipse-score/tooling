<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Changelog

## Unreleased

### Added

- `api_surface` lists friend functions (including hidden friends) and the
  friends and non-member operators of aliased types as `friend` /
  `free_operator` symbols. Existing lock files need regeneration.
- `validation_cli --links-output` writes `diagram_links.json` (element links
  and call links) for the `dependable-element` profile. `--links-only` writes
  it without validation and without a report.
- Zoom validation of sequence calls at dependable element scope (see
  `validation/core/docs/specifications/sequence_refinement.md`). Call links
  lead only to refinements that pass this check.
- The FlatBuffers of component, class and sequence diagrams carry the diagram
  `source` and `name`.

### Changed

- `clickable_plantuml` reads `diagram_links.json` instead of `*.idmap.json`.
  A change of the links re-reads all documents on incremental builds.
- Links are computed by `validation_cli`, not by the parser or the Sphinx
  extension.

### Deprecated

- `puml_cli --idmap-output-dir` is accepted and ignored with a warning. It is
  removed in the next major release.

### Removed

- The `*.idmap.json` outputs of `puml_cli` and the `puml_idmap` crate.
