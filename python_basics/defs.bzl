# load("//tools:ruff.bzl", "ruff_binary")
# load("//score_venv/py_pyvenv.bzl", "score_virtualenv")

load("@aspect_rules_py//py:defs.bzl", "py_binary", "py_library")
load("@aspect_rules_py//py:defs.bzl", "py_venv")
load("//score_pytest:py_pytest.bzl", _score_py_pytest = "score_py_pytest")
load("@pip_score_python_basics//:requirements.bzl", "all_requirements")

# Export score_py_pytest
score_py_pytest = _score_py_pytest


def score_virtualenv(name = "ide_support", venv_name =".venv",  reqs = []):
    py_venv(
        name = name,
        venv_name = venv_name,
        deps = all_requirements + reqs + [":config", "@rules_python//python/runfiles"] 
    )

    py_library(
        # Provides pyproject.toml as bazel-bin/{name}/_main/runfiles/pyproject.toml
        name = "config",
        srcs = ["@score_python_basics//:dummy_venv.py"],
        data = ["@score_python_basics//:pyproject.toml"],
    )
