# Run C++ parser targets

## Configure a parser target in `BUILD`

If you want to parse a specific Bazel target, use the `cpp_parser(...)` rule in the `BUILD` file like:

```
load("//cpp/libclang:cpp_parser.bzl", "cpp_parser")

cpp_parser(
  name = "cpp_parser_include_3rdparty",
  extra_args = [
  ],
  target = "//cpp/libclang/integration_test/cases/include_3rdparty",
  tool = ":clang_rs_parser",
)
```

Where:

- `target` is the Bazel target you want to parse.

Expected result:

- Bazel creates parser output artifact:
  - `bazel-bin/cpp/libclang/cpp_parser_include_3rdparty_result.json`

## Quick check (optional)

```bash
ls -l bazel-bin/cpp/libclang/cpp_parser_include_3rdparty_result.json
