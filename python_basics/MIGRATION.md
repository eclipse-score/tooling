# Migration Guide: Simplified score_python_basics Setup

This guide helps you migrate from the old ~20-line boilerplate to the new simplified setup.

## Before (Old Pattern)

```starlark
# MODULE.bazel - OLD WAY (❌ No longer needed)

# Manual Python setup - ~20 lines of boilerplate
PYTHON_VERSION = "3.12"
bazel_dep(name = "rules_python", version = "1.4.1")

python = use_extension("@rules_python//python/extensions:python.bzl", "python")
python.toolchain(is_default = True, python_version = PYTHON_VERSION)
use_repo(python)

pip = use_extension("@rules_python//python/extensions:pip.bzl", "pip")
pip.parse(
    hub_name = "pip",
    python_version = PYTHON_VERSION,
    requirements_lock = "//tools:requirements.txt",
)
use_repo(pip, "pip")

# Finally, add score_python_basics
bazel_dep(name = "score_python_basics", version = "0.3.0")
```

## After (New Simplified Pattern)

### Option 1: No pip dependencies (1 line!)
```starlark
# MODULE.bazel - NEW WAY ✅
bazel_dep(name = "score_python_basics", version = "0.3.0")
```

### Option 2: With pip dependencies (~6 lines)
```starlark
# MODULE.bazel - NEW WAY ✅
bazel_dep(name = "score_python_basics", version = "0.3.0")

# Only configure your pip dependencies - Python toolchain already setup!
pip = use_extension("@rules_python//python/extensions:pip.bzl", "pip")
pip.parse(
    hub_name = "pip",
    python_version = "3.12",  # Must match score_python_basics version
    requirements_lock = "//tools:requirements.txt",
)
use_repo(pip, "pip")
```

## What Changed?

### ✅ Automatically Provided by score_python_basics:
- `rules_python` dependency (version 1.4.1)
- Python 3.12 toolchain configured as default
- Python development tools (pytest, linting, formatting)

### ✅ No Longer Required:
- Manual `bazel_dep(name = "rules_python")` 
- Manual `python.toolchain()` setup
- Manual `use_repo(python)` call

### ⚠️ Important Notes:
- **Python version must be 3.12** (to match score_python_basics)
- **pip hub name can be customized** (use any name you want)
- **Multiple pip configurations are supported**

## Migration Steps:

1. **Remove manual rules_python setup** from your MODULE.bazel
2. **Keep or add score_python_basics dependency**
3. **Keep only your pip.parse() configuration** (if you have pip dependencies)
4. **Ensure python_version = "3.12"** in pip.parse()
5. **Test your build**

## Common Migration Issues:

### Issue: Python version mismatch
```
ERROR: Python version mismatch: you specified 3.11 but score_python_basics uses 3.12
```
**Solution:** Change your pip.parse() to use `python_version = "3.12"`

### Issue: Duplicate rules_python dependency
```
ERROR: Module rules_python has multiple versions: 1.4.1, 1.3.0
```
**Solution:** Remove your manual `bazel_dep(name = "rules_python")` line

## Examples:

### Simple Python project (no external dependencies):
```starlark
bazel_dep(name = "score_python_basics", version = "0.3.0")
```

### Web application with requests:
```starlark
bazel_dep(name = "score_python_basics", version = "0.3.0")

pip = use_extension("@rules_python//python/extensions:pip.bzl", "pip")
pip.parse(
    hub_name = "pip",
    python_version = "3.12",
    requirements_lock = "//:requirements.txt",
)
use_repo(pip, "pip")
```

### Multi-environment setup (main + test dependencies):
```starlark
bazel_dep(name = "score_python_basics", version = "0.3.0")

pip = use_extension("@rules_python//python/extensions:pip.bzl", "pip")
pip.parse(
    hub_name = "pip_main",
    python_version = "3.12",
    requirements_lock = "//main:requirements.txt",
)
pip.parse(
    hub_name = "pip_test", 
    python_version = "3.12",
    requirements_lock = "//test:requirements.txt",
)
use_repo(pip, "pip_main", "pip_test")
```