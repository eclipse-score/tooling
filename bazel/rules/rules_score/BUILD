load(
    "//bazel/rules/rules_score:rules_score.bzl",
    "sphinx_module",
)

exports_files([
    "templates/conf.template.py",
    "templates/seooc_index.template.rst",
    "templates/unit.template.rst",
    "templates/component.template.rst",
])

# HTML merge tool
py_binary(
    name = "sphinx_html_merge",
    srcs = ["src/sphinx_html_merge.py"],
    main = "src/sphinx_html_merge.py",
    visibility = ["//visibility:public"],
)

# Sphinx build binary with all required dependencies
py_binary(
    name = "score_build",
    srcs = ["src/sphinx_wrapper.py"],
    data = [],
    env = {
        "SOURCE_DIRECTORY": "",
        "DATA": "",
        "ACTION": "check",
    },
    main = "src/sphinx_wrapper.py",
    visibility = ["//visibility:public"],
    deps = [
        "@score_docs_as_code//src:plantuml_for_python",
        "@score_docs_as_code//src/extensions/score_sphinx_bundle",
    ],
)

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
        "@score_process//:score_process_module",
    ],
)
