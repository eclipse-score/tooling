# Score pytest wrapper & plugin

This module provides support for running [pytest](https://docs.pytest.org/en/latest/contents.html)-based tests with Bazel, and includes a **pytest plugin** that adds structured metadata to JUnit XML reports, improving traceability, test classification, and documentation.

---

## Features

- **Test Classification**: Categorize tests by type and derivation technique  
- **Requirements Traceability**: Link tests to requirement IDs  
- **Automatic File/Line Attribution**: Annotates tests with file path and line number  
- **JUnit XML Integration**: Exports metadata as `<properties>` in test reports  
- **Bazel Integration**: Run tests with `score_py_pytest` Bazel rule  

---

## Usage

### In `MODULE.bazel`

```starlark
bazel_dep(name = "score_python_basics", version = "0.1.0")
```

> The `score_pytest` module will determine the appropriate `pytest` version. It is not possible to override this.

---

### In `BUILD`

```starlark
load("@score_python_basics//score_pytest:py_pytest.bzl", "score_py_pytest")

score_py_pytest(
    name = "test_my_first_check",
    srcs = ["test_my_first_check.py"],
    plugins = [
        # Optionally specify additional pytest plugins
    ],
    args = [
        "--basetemp=/tmp/pytest",  # Optional args
    ],
    env = {
        "LD_LIBRARY_PATH": "/path/to/dynamic/lib",  # Optional environment
    },
    pytest_ini = "//my_pytest:my_pytest_ini",  # Optional custom pytest.ini
    tags = ["integration"]  # Optional tags for test grouping
)
```

---

## Using the Test Properties Plugin

You can use the provided `add_test_properties` decorator to enhance your tests with metadata:

### Example

```python
from your_module import add_test_properties

@add_test_properties(
    partially_verifies=["REQ-001", "REQ-002"],
    test_type="interface-test",
    derivation_technique="boundary-values"
)
def test_user_login():
    """Test user login with valid credentials."""
    ...
```

### Required Parameters

- `test_type`: Type of test being executed  
- `derivation_technique`: Method used to derive the test  
- **Either** `partially_verifies` or `fully_verifies`: List of requirement IDs  

> ?? All decorated tests **must include a docstring**.

---

### Accepted Values

#### Test Types

- `fault-injection`  
- `interface-test`  
- `requirements-based`  
- `resource-usage`  

#### Derivation Techniques

- `requirements-analysis`  
- `design-analysis`  
- `boundary-values`  
- `equivalence-classes`  
- `fuzz-testing`  
- `error-guessing`  
- `explorative-testing`  

---

### More Examples

```python
@add_test_properties(
    fully_verifies=["REQ-AUTH-001"],
    test_type="requirements-based",
    derivation_technique="requirements-analysis"
)
def test_authentication_flow():
    """Complete authentication flow test."""
    ...

@add_test_properties(
    partially_verifies=["REQ-UI-001", "REQ-UI-002"],
    test_type="interface-test",
    derivation_technique="boundary-values"
)
def test_form_validation():
    """Test form input validation boundaries."""
    ...
```

---

## Output in JUnit XML

When using `--junit-xml=report.xml`, the plugin augments each test case with:

- **Properties**:
  - `PartiallyVerifies` or `FullyVerifies`
  - `TestType`
  - `DerivationTechnique`
- **File** and **Line**: Relative source path and line number of the test function

### Example Output

```xml
<testsuites>
  <testsuite name="TestInterfaceValidation" tests="2" failures="0" errors="0" time="0.123">
    <testcase name="test_api_response_format" file="src/testfile_1.py" line="10" time="0.056">
      <properties>
        <property name="PartiallyVerifies" value="TREQ_ID_2, TREQ_ID_3"/>
        <property name="TestType" value="interface-test"/>
        <property name="DerivationTechnique" value="design-analysis"/>
      </properties>
    </testcase>
    <testcase name="test_error_handling" file="src/testfile_1.py" line="38" time="0.067">
      <properties>
        <property name="PartiallyVerifies" value="TREQ_ID_2, TREQ_ID_3"/>
        <property name="TestType" value="interface-test"/>
        <property name="DerivationTechnique" value="design-analysis"/>
      </properties>
    </testcase>
  </testsuite>
</testsuites>
```

---

## Development of `score_pytest`

### Updating `pytest` and dependencies

Update the dependencies listed in `requirements.txt` and lock them with:

```bash
bazel run //:requirements.update -- --upgrade
```

---

### Running Tests

To run the internal tests for this module:

```bash
bazel test //...
```

---
