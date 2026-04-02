<!--
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
-->

# PlantUML Linker

Reads `.fbs.bin` files produced by the [PlantUML parser](../parser/README.md) and generates a
`plantuml_links.json` file consumed by the `clickable_plantuml` Sphinx extension.

## What it does

When an architecture is described across multiple PlantUML component diagrams, the linker
correlates components between them: if a component alias in diagram **A** matches a top-level
component alias in diagram **B**, the linker emits a link from A → B.  This lets the Sphinx
documentation render clickable diagrams where high-level overview components link through to
their detailed sub-diagrams.

## Usage

```
linker --fbs-files <file1.fbs.bin> [<file2.fbs.bin> ...] --output plantuml_links.json
```

The tool is invoked automatically by the `architectural_design()` Bazel rule — there is
normally no need to call it manually.

## Build

```bash
bazel build //plantuml/linker:linker
```
