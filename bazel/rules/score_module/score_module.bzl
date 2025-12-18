load("@rules_python//sphinxdocs:sphinx.bzl", "sphinx_docs")
load("@rules_python//sphinxdocs:sphinx_docs_library.bzl", "sphinx_docs_library")
load(
    "//bazel/rules/score_module/private:score_module.bzl",
    _score_module = "score_module",
)
load(
    "//bazel/rules/score_module/private:seooc.bzl",
    _safety_element_out_of_context = "safety_element_out_of_context",
)

score_module = _score_module
safety_element_out_of_context = _safety_element_out_of_context
