# Score pytest wrapper

This module implements support for running [pytest](https://docs.pytest.org/en/latest/contents.html) based tests.

## Usage
MODULE.bazel
```
bazel_dep(name = "score_pytest", version = "0.1.0")
```

> The 'score_pytest' module will determin the `pytest` version. 


BUILD
```
load("@score_pytest//:pytest.bzl", "score_py_pytest")

score_py_pytest(
    name = "test_my_first_check",
    srcs = [
        "test_my_first_check.py"
    ],
    plugins = [
        # Optionally specify pytest, that will register their fixtures
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

## Development

### Updating pytest in score_pytest
It uses the dependencies from `requirements.txt`.  
If you have added new dependencies, make sure to update the *requirements_lock* file like so: 
```
bazel run //:requirements.update -- --upgrade
```

### Running test
To run the tests of the pytest module use:
```
$ bazel test //...
```
