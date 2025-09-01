# DASH License Checker Bazel Integration

This module provides Bazel build configurations and a macro to integrate the [Eclipse DASH license checker](https://github.com/eclipse/dash-licenses) into S-CORE Bazel-based project. It includes:

- A `dash_license_checker` macro to create a `java_binary` for license checking.
- A formatter rule to convert `requirements_lock.txt` or `Cargo.lock` into the DASH-compatible JSON format.
- The DASH tool JAR embedded directly in the module, so no additional setup is required, downloaded from [here]("https://repo.eclipse.org/content/repositories/dash-licenses/org/eclipse/dash/org.eclipse.dash.licenses/1.1.0/org.eclipse.dash.licenses-1.1.0.jar")

---

## Directory Structure

```bash
├── BUILD                       # Defines the embedded dash JAR
├── MODULE.bazel               # Bazel module definition
├── dash.bzl                   # Contains the dash_license_checker macro
├── org.eclipse.dash.licenses-1.1.0.jar  # Embedded DASH license checker
├── tool/
│   └── formatters/
│       ├── BUILD
│       ├── dash_format_converter.bzl
│       └── dash_format_converter.py
└── README.md
```

---

## Key Files

### `BUILD`

- Uses `java_import` to expose the embedded `org.eclipse.dash.licenses-1.1.0.jar` as a Bazel target `//:jar`.

### `MODULE.bazel`

- Declares this directory as a Bazel module: `dash_license_checker`.

### `dash.bzl`

- Provides the macro `dash_license_checker` to:
  - Convert a lockfile into the DASH-compatible format.
  - Run the DASH license checker using the embedded JAR.
  - Auto-detect `file_type` from a project config (e.g. Rust or Python).

### `tool/formatters/dash_format_converter.bzl`

- Custom Bazel rule `dash_format_converter` that wraps the Python script for converting inputs.

### `tool/formatters/dash_format_converter.py`

- Transforms `requirements_lock.txt` or `Cargo.lock` into the required JSON format.

---

## Usage

In the consuming Bazel project:

### 1. In `MODULE.bazel`

```python
bazel_dep(name = "dash_license_checker", version = "0.1.0")
```

### 2. In the `BUILD` file

#### For Python dependencies:

```python
load("@dash_license_checker//:dash.bzl", "dash_license_checker")

filegroup(
    name = "requirements_lock",
    srcs = ["requirements_lock.txt"],
)

dash_license_checker(
    name = "python_license_check",
    src = "//:requirements_lock",
    visibility = ["//visibility:public"],
)
```

#### For Rust dependencies:

```python
filegroup(
    name = "cargo_lock",
    srcs = ["Cargo.lock"],
)

dash_license_checker(
    name = "rust_license_check",
    src = "//:cargo_lock",
    visibility = ["//visibility:public"],
    file_type = "cargo",
    #optional
    skip_source_filter = False, # internal packages(that dont contain source) are skipped, only 3rd party verified
    filter_keywords = ["qorix-group", "eclipse-score"] # keywords to filter packages containing this words
)
```

---

## Output

Running the generated target will:

1. Convert the input lockfile to DASH-compatible JSON.
2. Run the embedded DASH license checker JAR.
3. Output licensing report or violations to standard output.

---

## Benefits

✅ No need to fetch the JAR separately — it's embedded.
✅ Language-agnostic input support (Python, Rust).
✅ Fully reproducible and self-contained via Bazel module.

---
