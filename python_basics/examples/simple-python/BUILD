load("@score_python_basics//:defs.bzl", "score_py_pytest")
load("@rules_python//python:defs.bzl", "py_binary")

# Simple Python binary - uses the Python 3.12 toolchain from score_python_basics
py_binary(
    name = "hello",
    srcs = ["hello.py"],
)

# Test using score_py_pytest rule
score_py_pytest(
    name = "test_hello",
    srcs = ["test_hello.py"],
)