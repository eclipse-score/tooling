load("@score_python_basics//:defs.bzl", "score_py_pytest")
load("@rules_python//python:defs.bzl", "py_binary")
load("@pip//:requirements.bzl", "requirement")

# CLI application using click
py_binary(
    name = "web_client",
    srcs = ["web_client.py"],
    deps = [
        requirement("requests"),
        requirement("click"),
    ],
)

# Test the application
score_py_pytest(
    name = "test_web_client",
    srcs = ["test_web_client.py"],
    deps = [
        requirement("requests"),
        requirement("click"),
    ],
)