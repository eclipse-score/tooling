load("@rules_python//sphinxdocs:sphinx.bzl", "sphinx_docs")
load("@rules_python//sphinxdocs:sphinx_docs_library.bzl", "sphinx_docs_library")
load(
    "//bazel/rules/score_module/private:score_component.bzl",
    _score_component = "score_component",
)
load(
    "//bazel/rules/score_module/private:sphinx_module.bzl",
    _sphinx_module = "sphinx_module",
)

sphinx_module = _sphinx_module
score_component = _score_component
