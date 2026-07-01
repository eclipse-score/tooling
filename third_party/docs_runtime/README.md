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
   prepared archive at build time and wraps `exec_in_sysroot.sh` so that
   `/usr/bin/dot` inside the sysroot is invoked via `LD_PRELOAD=libfakechroot.so`.

Rules that need graphviz use `//third_party/docs_runtime:dot` as the executable
and receive its path via the `GRAPHVIZ_DOT` environment variable.

## Caveats

This setup is reproducible but not *fully* hermetic — two host dependencies
remain:

1. **Host kernel ABI.** `exec_in_sysroot` runs `dot` through the sysroot's own
   ELF interpreter (`ld-linux.so`) with `LD_PRELOAD=libfakechroot.so` rather
   than a real chroot. All shared libraries — including `libc.so.6` — are loaded
   from the sysroot, so the *host* glibc version does not matter. The one host
   dependency that remains is the Linux kernel: the sysroot's glibc (Ubuntu
   24.04, glibc 2.39) requires a kernel at least as new as its minimum supported
   version, so build hosts need a sufficiently recent kernel.
2. **Host shell tools.** The sysroot-rework and extraction actions run under a
   POSIX `sh` and use standard coreutils (`mkdir`, `find`, `rm`) plus `tar`,
   assumed present in the build environment. The generated `dot` launcher and
   the smoke test are plain POSIX `sh` scripts with an inline runfiles lookup

## Updating packages

Edit `docs_runtime.yaml`, then regenerate and commit the lock:

```bash
bazel run @docs_runtime//:lock
```

## Targets in this package

- `//third_party/docs_runtime:dot` - hermetic executable used by doc actions.
- `//third_party/docs_runtime/tests:dot_smoke_test` - renders a tiny SVG through
  the wrapper to verify runtime wiring.
