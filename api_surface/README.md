<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# API Surface Stability Check

Ensures the public C++ API of a library doesn't change accidentally.
Uses **clang's AST dump** (`-Xclang -ast-dump=json`) for robust C++ parsing that
correctly handles preprocessor directives, macros, templates, ifdefs, and
conditional compilation.

## How it works

1. A Bazel build action runs `clang -ast-dump=json` on the target headers (in the
   proper sandbox with all include paths from CcInfo)
2. A Python tool (`extract_api.py`) parses the JSON AST and extracts public symbols
3. The extracted API surface is compared against a committed lock file (JSON)
4. If the API has changed, the test fails with a clear diff
5. An implicit `<name>.update` target allows intentional API changes to be committed

Both documented and undocumented public symbols are compared for API changes.
Documentation enforcement is a separate, optional check enabled with `check_docs = True`.

## Adopting it in another repository

1. Depend on `score_tooling` in your `MODULE.bazel`:

```starlark
bazel_dep(name = "score_tooling", version = "<version>")
```

2. Register a hermetic LLVM toolchain (e.g. with `toolchains_llvm`) and make its
   repository visible to your root module. By default the rule uses the
   `cc_toolchain` target `@llvm_toolchain//:cc-clang-x86_64-linux`, which is what
   `toolchains_llvm` generates for `llvm.toolchain(...)` + `use_repo(llvm, "llvm_toolchain")`.
   `score_tooling` does not provide this repository to consumers (its own LLVM
   toolchain is a dev dependency), so the `cc_toolchain` attribute below can be used
   to point at any other repository/target providing a clang `cc_toolchain`.
   The checks are Linux-only (`target_compatible_with = ["@platforms//os:linux"]`).

3. Load the macro in your `BUILD` file (also re-exported from `@score_tooling//:defs.bzl`):

```python
load("@score_tooling//api_surface:api_surface.bzl", "api_surface_test")
```

4. Add the test target (an implicit `<name>.update` target is created automatically):

```python
api_surface_test(
    name = "api_surface_test",
    target = ":your_cc_library",
    check_docs = False,  # Set True to enforce \api documentation
    lock_file = "api_surface.lock.json",
    # cc_toolchain = "@my_llvm//:cc-clang-x86_64-linux",  # optional override
)
```

5. Generate the initial lock file, then commit it:

```bash
bazel run //your/package:api_surface_test.update
```

6. Verify the API surface:

```bash
bazel test //your/package:api_surface_test
```

After an intentional API change, rerun the `.update` target, review the lock file diff
and commit it.

## Rule Attributes

### `api_surface_test`

| Attribute       | Type     | Description                                                  |
|-----------------|----------|--------------------------------------------------------------|
| `target`        | label    | cc_library target whose transitive direct public headers form the API |
| `lock_file`     | label    | The committed JSON lock file to compare against (in the current package) |
| `check_docs`    | bool     | If true (default), fail when public symbols lack `\api` documentation  |
| `cc_toolchain`  | label    | Hermetic LLVM `cc_toolchain` used to run clang. Default: `@llvm_toolchain//:cc-clang-x86_64-linux` (resolved in the calling repository) |

The C++ standard and all built-in compiler flags (target, sysroot, standard library)
come from this toolchain's `c++-header-parsing` action, so no system clang is needed.

## Lock file format

The lock file is a JSON document produced by `extract_api.py` with the target label
(`target`), the documented public symbols (`symbols`) and the public symbols lacking
`\api` documentation (`undocumented_symbols`). Each entry holds `name`,
`qualified_name`, `kind` and `signature`. The comparison uses the union of both lists
(sorted), so a symbol moving between them does not count as an API change; adding
documentation is therefore possible without touching the lock file.

An implicit `<name>.update` runnable target is automatically created alongside every
`api_surface_test`. Run it with `bazel run` to regenerate the lock file after an
intentional API change.

## How Public Symbols Are Identified

The tool tracks:
- `using` / `typedef` declarations
- `class` / `struct` declarations (non-forward)
- Forward-declared / incomplete types (pimpl), as `forward_class`
- `enum` / `enum class` declarations
- Free functions and global variables at namespace scope
- Public **and protected** methods in class bodies (protected members use a
  `protected_` kind prefix so public vs protected is distinguishable)
- Member function templates
- Ref-qualified overloads (`void f() &;` vs `void f() &&;`)
- Friend functions, including hidden friends defined in-class and friends declared
  in a private section, as `friend` / `template_friend`. They are qualified with
  the enclosing namespace of the class (e.g. `ns::operator<`), not the class
- Friends and non-member operators of a class re-exported through a `using`
  alias are followed like its members and listed in the alias's namespace
  (`friend`, or `free_operator` for non-member operators declared in an
  internal namespace such as `impl`). Non-member operators in public namespaces
  stay regular `function` entries
- Explicit `extern template` instantiations, as `extern_template`
- Template variants of all the above

The extracted signature also captures API-relevant qualifiers so that changing
them is detected: `constexpr`, `extern` / `extern "C"` linkage, and C++ standard
attributes (`[[nodiscard]]`, `[[deprecated]]`, ...). SFINAE constraints appearing
in a function's return type (e.g. `std::enable_if_t<...>`) are part of the
signature as well.

Symbols in `detail`, `internal`, or `impl` namespaces are automatically excluded.

## Documentation Enforcement

When `check_docs = True`, the tool verifies that every public symbol, friend functions and
operators included, has:
- A `\api` (or `@api`) doxygen marker
- A `\brief` (or `@brief`) description

This ensures the public API is fully documented for end-users.
