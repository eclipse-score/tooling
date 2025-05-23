load("@rules_python//python:defs.bzl", "py_test")

def starpls_py_integration_test(name, srcs, data, **kwargs):
    """Creates a py_test target configured for starpls integration."""
    py_test(
        name = name,
        srcs = srcs,

        deps = ["@pip_deps_test//bazel_runfiles:bazel_runfiles"],
        data = data,
        python_version = "PY3",
        size = kwargs.pop("size", "small"),
        **kwargs
    )