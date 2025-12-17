# SEooC Rule Tests

This directory contains unit tests for the `seooc` rule, which is used to define Safety Elements out of Context (SEooC) following ISO 26262 standards.

## Test Suite Overview

The test suite (`seooc_test.bzl`) uses Bazel Skylib's `unittest` framework to verify the correctness of the `seooc` rule implementation. The tests are organized into several categories:

### Test Categories

1. **Provider Tests** (`seooc_providers_test`)
   - Verifies that the `seooc` rule provides the required providers
   - Checks for `DefaultInfo` provider
   - Checks for `SphinxDocsLibraryInfo` provider

2. **Transitive Documentation Tests** (`seooc_transitive_docs_test`)
   - Verifies that the rule correctly aggregates transitive documentation
   - Ensures that the transitive field is a depset
   - Validates the structure of transitive documentation entries
   - Confirms that documentation paths start with `docs/safety_elements/`

3. **Attribute Handling Tests** (`seooc_attributes_test`)
   - Verifies that mandatory attributes are correctly handled
   - Ensures that at least index documentation is present

4. **Path Prefixing Tests** (`seooc_path_prefixing_test`)
   - Verifies that documentation paths are correctly prefixed with the module name
   - Ensures proper path organization for Sphinx documentation

## Test Fixtures

The test suite uses the following fixture files located in `fixtures/`:

- **`assumptions_of_use.rst`**: Sample assumptions of use document
- **`component_requirements.rst`**: Sample component requirements document
- **`index.rst`**: Sample index file for documentation structure

These fixtures are wrapped as `sphinx_docs_library` targets in the `BUILD` file to create realistic test scenarios.

## Running the Tests

To run all seooc tests:

```bash
bazel test //bazel/rules/score_module/test:seooc_tests
```

To run with verbose output:

```bash
bazel test //bazel/rules/score_module/test:seooc_tests --test_output=all
```

To run individual tests:

```bash
bazel test //bazel/rules/score_module/test:seooc_providers_test
bazel test //bazel/rules/score_module/test:seooc_transitive_docs_test
bazel test //bazel/rules/score_module/test:seooc_attributes_test
bazel test //bazel/rules/score_module/test:seooc_path_prefixing_test
```

## Test Targets

The `BUILD` file defines two test targets:

1. **`test_seooc_minimal`**: Tests the seooc rule with minimal required attributes
   - Only includes `assumptions_of_use` and `index`
   - Verifies basic functionality

2. **`test_seooc_complete`**: Tests the seooc rule with all optional attributes
   - Includes all optional documentation attributes
   - Verifies handling of complete documentation sets

## Adding New Tests

To add a new test:

1. Define a test implementation function in `seooc_test.bzl`:

   ```python
   def _my_new_test_impl(ctx):
       env = analysistest.begin(ctx)
       target_under_test = analysistest.target_under_test(env)

       # Your test assertions here
       asserts.true(env, condition, "error message")

       return analysistest.end(env)

   my_new_test = analysistest.make(_my_new_test_impl)
   ```

2. Add the test to `_test_seooc()` function:

   ```python
   my_new_test(
       name = "my_new_test",
       target_under_test = ":test_seooc_minimal",
   )
   ```

3. Include it in the test suite:

   ```python
   native.test_suite(
       name = name,
       tests = [
           # ... existing tests ...
           ":my_new_test",
       ],
   )
   ```

## Test Coverage

The current test suite covers:

- ✅ Provider generation
- ✅ Transitive documentation aggregation
- ✅ Mandatory attribute handling
- ✅ Path prefixing with module names
- ✅ Optional attribute handling

## Dependencies

The tests depend on:

- `@bazel_skylib//lib:unittest` - Bazel Skylib testing framework
- `@rules_python//sphinxdocs` - Sphinx documentation rules
- `//bazel/rules/score_module/private:seooc.bzl` - The rule being tested

## Notes

- All test targets are tagged with `"manual"` to prevent them from being built during normal builds
- Tests use the `analysistest` framework, which performs analysis-phase validation
- The test suite is part of the continuous integration pipeline
