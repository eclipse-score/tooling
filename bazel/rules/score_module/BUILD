load(
    "//bazel/rules/score_module:score_module.bzl",
    "score_module",
)

exports_files([
    "templates/conf.template.py",
    "templates/seooc_index.template.rst",
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

score_module(
    name = "score_module",
    srcs = glob(
        [
            "docs/**/*.rst",
        ],
        allow_empty = True,
    ),
    index = "docs/index.rst",
    visibility = ["//visibility:public"],
    deps = [
        "@score_process//:score_process_module",
    ],
)
