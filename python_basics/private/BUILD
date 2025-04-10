load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("//:defs.bzl", "score_virtualenv")
load("@aspect_rules_lint//format:defs.bzl", "format_multirun", "format_test")
#package(default_visibility = ["//visibility:public"])

# define ":ruff" target
#score_python()

# Generate the virtualenv
score_virtualenv()

format_multirun(
    name = "format.fix",
    python = "@aspect_rules_lint//format:ruff",
    visibility = [
        "//visibility:private",
    ],
)

format_test(
    name = "format.check",
    no_sandbox= True,
    python = "@aspect_rules_lint//format:ruff",
    workspace = "@score_python_basics//:MODULE.bazel",
    visibility = [
        "//visibility:private",
    ],
)


# In order to update the requirements, change the `requirements.txt` file and run:
# `bazel run //:requirements.update`.
# This will update the `requirements_lock.txt` file.
# To upgrade all dependencies to their latest versions, run:
# `bazel run //:requirements.update -- --upgrade`.
compile_pip_requirements(
    name = "requirements",
    src = "requirements.in",
    requirements_txt = "requirements.txt",
    tags = [
        "manual",
    ],
)