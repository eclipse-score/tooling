load("@aspect_rules_py//py:defs.bzl", "py_binary")

def cli_helper(name, visibility):
    py_binary(
        name = name,
        srcs = ["@score_cli_helper//tool:cli_help_lib"],
        visibility = visibility,
    )

    native.alias(
        name = "help",
        actual = ":" + name,
        visibility = visibility,
        tags = [
            "cli_help=Output all bazel targets with cli_help tag:\n" + \
            "bazel run //:help"
        ],
    )
