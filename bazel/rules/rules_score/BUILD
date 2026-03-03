load("@aspect_rules_py//py:defs.bzl", "py_binary", "py_library")
load(
    "@score_tooling//bazel/rules/rules_score:sphinx_toolchain.bzl",
    "sphinx_toolchain",
)
load(
    "//bazel/rules/rules_score:rules_score.bzl",
    "sphinx_module",
)

exports_files([
    "templates/conf.template.py",
    "templates/seooc_index.template.rst",
    "templates/unit.template.rst",
    "templates/component.template.rst",
    "src/sphinx_wrapper.py",
])

sphinx_module(
    name = "rules_score_doc",
    srcs = glob(
        [
            "docs/**/*.rst",
            "docs/**/*.puml",
        ],
        allow_empty = True,
    ),
    index = "docs/index.rst",
    visibility = ["//visibility:public"],
    deps = [
        # "@score_process//:score_process_module",
    ],
)

# HTML merge tool
py_binary(
    name = "sphinx_html_merge",
    srcs = ["src/sphinx_html_merge.py"],
    main = "src/sphinx_html_merge.py",
    visibility = ["//visibility:public"],
)

py_library(
    name = "sphinx_module_ext",
    srcs = ["src/sphinx_module_ext.py"],
    imports = ["src"],
    visibility = ["//visibility:public"],
)

# Sphinx build binary with all required dependencies
py_binary(
    name = "sphinx_build",
    srcs = ["src/sphinx_wrapper.py"],
    main = "src/sphinx_wrapper.py",
    visibility = ["//visibility:public"],
    deps = [
        ":sphinx_module_ext",
        "@pip_tooling//myst_parser",
        "@pip_tooling//sphinx",
        "@pip_tooling//sphinx_design",
        "@pip_tooling//sphinx_needs",
        "@pip_tooling//sphinx_rtd_theme",
        "@pip_tooling//sphinxcontrib_plantuml",
    ],
)

sphinx_toolchain(
    name = "default_toolchain",
    conf_template = "@score_tooling//bazel/rules/rules_score:templates/conf.template.py",
    html_merge_tool = ":sphinx_html_merge",
    sphinx = ":sphinx_build",
)

toolchain_type(
    name = "toolchain_type",
    visibility = ["//visibility:public"],
)

toolchain(
    name = "sphinx_default_toolchain",
    exec_compatible_with = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
    target_compatible_with = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
    toolchain = ":default_toolchain",
    toolchain_type = ":toolchain_type",
)
