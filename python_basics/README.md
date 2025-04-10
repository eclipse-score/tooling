# S-CORE Python Basics

✅ Makes development of Python code easier inside Bazel
✅ Provides a Python virtualenv target
✅ Provides S-CORE wide defaults for linting and formatting
✅ Provides pytest with S-CORE wide defaults for pytest

## How To: Integrate

In the consuming Bazel project:

### 1. In your `MODULE.bazel` import the python basics

```python
bazel_dep(name = "score_python_basics", version = "0.1.0")
```

### 2. In your `BUILD` file

```python
load("@score_python_basics//:defs.bzl", "score_virtualenv")

score_virtualenv(
    # optional: change target name
    name = "ide_support",

    # optional: change generated venv name
    venv_name = ".venv",

    # optional: add your own requirements
    # e.g. all_requirements comming from your pip installation via '@pip...
    reqs = []
)
```

---

## How To: Use

You can create the virtualenv via the name you have defined above: `bazel run //:ide_support`.

