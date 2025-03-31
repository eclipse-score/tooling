load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@score_pytest//:pytest.bzl", "score_py_pytest")

score_py_pytest(
    name = "pytest_version_correct",
    srcs = [
        "int_test.py",
    ],
)   

compile_pip_requirements(
    name = "requirements",
    srcs = ["requirements.txt","@score_pytest//:requirements_lock.txt"],
    requirements_txt = "requirements_lock.txt",
    tags = [
        "manual",
    ],
)
