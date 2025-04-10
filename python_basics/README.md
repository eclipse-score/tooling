# S-CORE Python Basics

* ✅ Makes development of Python code easier inside Bazel
* ✅ Provides a Python virtualenv target
* ✅ Provides S-CORE wide defaults for linting and formatting
* ✅ Provides pytest with S-CORE wide defaults for pytest

## How To: Integrate

In the consuming Bazel project, in your `MODULE.bazel` import the python basics

```python
bazel_dep(name = "score_python_basics", version = "0.1.0")
```

## Python Virtualenv
The `score_virtualenv` rule creates a virtualenv for your IDE (syntax highlighting, formatting, linting etc).

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

You can create the virtualenv via the name you have defined above, e.g. `bazel
run //:ide_support`.

## Pytest

The `score_py_pytest` rule creates a pytest target.

*Note: the pytest version is determined by the `score_pytest` module. It is intentionally not possible to overwrite it.*

```python
load("@score_python_basics//:defs.bzl", "score_py_pytest")

score_py_pytest(
    name = "test_my_first_check",
    srcs = [
        "test_my_first_check.py"
    ],
    plugins = [
        # Optionally specify pytest plugins, that will register their fixtures
    ],
    args = [
        # Specify optional arguments, ex:
        "--basetemp /tmp/pytest",
    ],
    # Optionally provide pytest.ini file, that will override the default one
    pytest_ini = "//my_pytest:my_pytest_ini",

    # Optionally provide tags the test should have, in order to allow for execution grouping
    tags = ["integration", #...]
)
```

## basedpyright

*Not ready to be used yet!*

## Development of score_python_basics

### Setting up the development environment
To set up the development environment, you need to create a python virtual
environment:
```bash
bazel run private:ide_support
```

*Note: for now the virtualenv is created in the `python_basics` directory. VS
Code will not detect it automatically. You need to set the interpreter
(`python_basics/.venv/bin/python`) manually via ctrl+shift+p and "Python: Select
Interpreter".*


### Updating pytest in score_pytest
It uses the dependencies from `requirements.txt`.  
If you have added new dependencies, make sure to update the *requirements.txt* file like so: 
```
# Add new dependencies:
bazel run //private:requirements.update

# Upgrade all dependencies:
bazel run //private:requirements.update -- --upgrade
```

### Running tests
To run the tests of the pytest module use:
```
$ bazel test //...
```
