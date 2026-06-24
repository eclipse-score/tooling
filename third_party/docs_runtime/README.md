<!--
*******************************************************************************
Copyright (c) 2026 Contributors to the Eclipse Foundation

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0

SPDX-License-Identifier: Apache-2.0
*******************************************************************************
-->

# Hermetic graphviz for docs (`docs_runtime` + `exec_in_sysroot`)

The docs build needs Graphviz `dot` at action runtime (Sphinx graphviz extension
and PlantUML `-graphvizdot`). This package makes that use hermetic:

1. `@docs_runtime//:flat` provides a distroless rootfs tar (from
   `docs_runtime.yaml` + `docs_runtime.lock.json`) containing `graphviz` and
   `fakechroot`.
2. `//third_party/docs_runtime:dot_sysroot` (a `prepare_sysroot` rule) unpacks
   that tar, prunes plugins with missing host dependencies, runs `dot -c` to
   generate the plugin manifest, and repackages the result as a single cached
   archive.
3. `//third_party/docs_runtime:dot` (an `exec_in_sysroot` rule) extracts the
   prepared archive at build time and wraps `dot.sh` so that `/usr/bin/dot`
   inside the sysroot is invoked via `LD_PRELOAD=libfakechroot.so`.

Rules that need graphviz use `//third_party/docs_runtime:dot` as the executable
and receive its path via the `GRAPHVIZ_DOT` environment variable.

## Caveats

This setup is reproducible but not *fully* hermetic — two host dependencies
remain:

1. **Host glibc / ld.so ABI compatibility.** `exec_in_sysroot` runs `dot` via
   `LD_PRELOAD=libfakechroot.so` rather than a real chroot, so the kernel still
   launches the sysroot's `dot` with the *host* ELF interpreter (`ld.so`), which
   then loads the sysroot's glibc. The sysroot is pinned to Ubuntu 24.04 (glibc
   2.39); on a build host whose glibc is older than the sysroot's, `dot` can
   fail to start with `GLIBC_2.xx not found`. Build hosts therefore need a glibc
   at least as new as the pinned sysroot.
2. **Host shell tools.** The sysroot-rework and extraction actions run under a
   POSIX `sh` and use standard coreutils (`find`, `mktemp`, `chmod`, `rm`),
   assumed present in the build environment. The generated `dot` launcher itself
   requires `bash`, because it sources Bazel's `runfiles.bash` library (there is
   no POSIX-`sh` runfiles equivalent in `@bazel_tools`).

## Updating packages

Edit `docs_runtime.yaml`, then regenerate and commit the lock:

```bash
bazel run @docs_runtime//:lock
```

## Targets in this package

- `//third_party/docs_runtime:dot` - hermetic executable used by doc actions.
- `//third_party/docs_runtime/tests:dot_smoke_test` - renders a tiny SVG through
  the wrapper to verify runtime wiring.
