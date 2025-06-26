# Eclipse SCORE Tooling

A unified Bazel module providing essential development tools for Eclipse SCORE projects.

## 📦 Included Tools

This repository consolidates multiple development tools into a single, cohesive Bazel module:

### 🔍 [Copyright Checker](./cr_checker/)
- **Package**: `//cr_checker`
- **Description**: Validates and fixes copyright headers in source files
- **Usage**: Check and automatically add copyright headers to ensure compliance

### 📋 [DASH License Checker](./dash/)
- **Package**: `//dash` 
- **Description**: Integration with Eclipse DASH license checker for dependency compliance
- **Usage**: Analyze project dependencies for license compatibility

### 🎨 [Format Checker](./format_checker/)
- **Package**: `//format_checker`
- **Description**: Consistent source code formatting for Python, YAML, and Starlark files
- **Usage**: Format source files and enforce consistent style across projects

### 🐍 [Python Basics](./python_basics/)
- **Package**: `//python_basics`
- **Description**: Essential Python tooling and utilities for Bazel-based Python projects
- **Usage**: Common Python development patterns and testing utilities

### 🌟 [Starlark Language Server](./starpls/)
- **Package**: `//starpls`
- **Description**: Language server protocol implementation for Starlark/Bazel files
- **Usage**: Enhanced IDE support for Bazel BUILD files and .bzl files

## 🚀 Usage

### Adding to Your Project

Add the following to your project's `MODULE.bazel`:

```starlark
bazel_dep(name = "score_tooling", version = "1.0.0")

# For local development:
local_path_override(
    module_name = "score_tooling",
    path = "../tooling",
)
```

### Using Individual Tools

Each tool can be used independently within the unified module:

```starlark
load("@score_tooling//cr_checker:cr_checker.bzl", "copyright_checker")
load("@score_tooling//dash:dash.bzl", "dash_license_checker")
load("@score_tooling//format_checker:macros.bzl", "use_format_targets")
load("@score_tooling//python_basics:defs.bzl", "py_venv_test")
```

## 🏗️ Development

### Prerequisites
- Bazel 7.0+ 
- Python 3.12
- Java 8+ (for DASH license checker)

### Building
```bash
bazel build ...
```

### Testing
```bash
bazel test ...
```

## 📖 Documentation

Each tool maintains its own comprehensive documentation:
- [Copyright Checker README](./cr_checker/README.md)
- [DASH License Checker README](./dash/README.md)
- [Format Checker README](./format_checker/README.md)
- [Python Basics README](./python_basics/README.md)
- [Starlark Language Server README](./starpls/README.md)

## 🔄 Migration from Individual Modules

If you were previously using individual tool modules:

**Before:**
```starlark
bazel_dep(name = "score_cr_checker", version = "0.3.0")
bazel_dep(name = "score_format_checker", version = "0.1.1")
bazel_dep(name = "score_python_basics", version = "0.3.2")
# ... etc
```

**After:**
```starlark
bazel_dep(name = "score_tooling", version = "1.0.0")
```

Update your load statements to use the unified module:
```starlark
# Before
load("@score_cr_checker//cr_checker:cr_checker.bzl", "copyright_checker")

# After  
load("@score_tooling//cr_checker:cr_checker.bzl", "copyright_checker")
```

## 🤝 Contributing

1. Each tool maintains its own package structure under the root
2. Common dependencies are consolidated in the root `MODULE.bazel`
3. Integration tests validate the unified module functionality
4. Follow existing patterns when adding new tools

## 📄 License

This project is licensed under the Apache License 2.0 - see individual tool directories for specific license information.
