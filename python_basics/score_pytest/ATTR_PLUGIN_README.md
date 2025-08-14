# 🧪 `@add_test_properties` — Make Your Tests Tell a Story

Writing tests is great. Writing **traceable, well-documented tests** is even better.

This plugin lets you attach rich metadata to your test cases — like what requirements they verify, how they were derived, and what kind of test they are. That metadata ends up in your `junitxml` reports, making your testing output more meaningful, especially in traceability-driven projects.

---

## 🚀 How to Use

Import the decorator:

```python
from your_plugin_module import add_test_properties
```

### ✅ Minimal Example

```python
@add_test_properties(
    partially_verifies=["REQ-123"],
    test_type="resource-usage",
    derivation_technique="explorative-testing"
)
def test_my_feature():
    """Tests feature performance under expected load."""
    ...
```

### ✅ Full Example

```python
@add_test_properties(
    fully_verifies=[
        "REQ-456",
        "REQ-789",
        "REQ-999",
    ],
    test_type="requirements-based",
    derivation_technique="equivalence-classes",
)
def test_invalid_ids_handling():
    """Ensures system rejects invalid IDs cleanly."""
    ...
```

---

## 📦 Parameters

| Name                   | Type         | Required | Description |
|------------------------|--------------|----------|-------------|
| `partially_verifies`   | `list[str]`  | No*      | Requirement IDs partially verified by the test |
| `fully_verifies`       | `list[str]`  | No*      | Requirement IDs fully verified by the test |
| `test_type`            | Literal      | ✅ Yes   | What kind of test is this? (see below) |
| `derivation_technique` | Literal      | ✅ Yes   | How was this test derived? (see below) |

> \* At least one of `partially_verifies` or `fully_verifies` is required.

---

## 🧰 Allowed Values

### `test_type`

- `"fault-injection"`
- `"interface-test"`
- `"requirements-based"`
- `"resource-usage"`

### `derivation_technique`

- `"requirements-analysis"`
- `"design-analysis"`
- `"boundary-values"`
- `"equivalence-classes"`
- `"fuzz-testing"`
- `"error-guessing"`
- `"explorative-testing"`

---

## 🧪 What Goes Into the Report?

When running with `--junitxml=report.xml`, the following is included per test:

- All test properties you defined (`<property>` tags)
- The file path and line number of the test (`file` and `line` attributes)

This metadata shows up in CI dashboards, static reports, or traceability tools.

---

## ⚠️ Notes

- Each test must have a **docstring** — it's required.
- To silence marker warnings, register the marker in `pytest.ini`:

```ini
[pytest]
markers =
    test_properties(dict): Adds metadata to tests for enhanced reporting
```

---

## 💡 Why Use It?

This plugin helps you:

- Link tests to requirements (partial or full verification)
- Clarify the purpose and origin of tests
- Enhance automated reports with useful context

Less guessing, more clarity. Just decorate and go.

