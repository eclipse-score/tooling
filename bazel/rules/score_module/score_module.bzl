load("@rules_python//sphinxdocs:sphinx.bzl", "sphinx_docs")
load("@rules_python//sphinxdocs:sphinx_docs_library.bzl", "sphinx_docs_library")
load(
    "//bazel/rules/score_module:providers.bzl",
    _ComponentInfo = "ComponentInfo",
    _DependableElementInfo = "DependableElementInfo",
    _SphinxSourcesInfo = "SphinxSourcesInfo",
    _UnitInfo = "UnitInfo",
)
load(
    "//bazel/rules/score_module/private:architectural_design.bzl",
    _architectural_design = "architectural_design",
)
load(
    "//bazel/rules/score_module/private:assumptions_of_use.bzl",
    _assumptions_of_use = "assumptions_of_use",
)
load(
    "//bazel/rules/score_module/private:component.bzl",
    _component = "component",
)
load(
    "//bazel/rules/score_module/private:component_requirements.bzl",
    _component_requirements = "component_requirements",
)
load(
    "//bazel/rules/score_module/private:dependability_analysis.bzl",
    _dependability_analysis = "dependability_analysis",
)
load(
    "//bazel/rules/score_module/private:dependable_element.bzl",
    _dependable_element = "dependable_element",
)
load(
    "//bazel/rules/score_module/private:feature_requirements.bzl",
    _feature_requirements = "feature_requirements",
)
load(
    "//bazel/rules/score_module/private:safety_analysis.bzl",
    _safety_analysis = "safety_analysis",
)
load(
    "//bazel/rules/score_module/private:sphinx_module.bzl",
    _sphinx_module = "sphinx_module",
)
load(
    "//bazel/rules/score_module/private:unit.bzl",
    _unit = "unit",
)

architectural_design = _architectural_design
assumptions_of_use = _assumptions_of_use
component_requirements = _component_requirements
dependability_analysis = _dependability_analysis
feature_requirements = _feature_requirements
safety_analysis = _safety_analysis
sphinx_module = _sphinx_module
unit = _unit
component = _component
dependable_element = _dependable_element
SphinxSourcesInfo = _SphinxSourcesInfo
UnitInfo = _UnitInfo
ComponentInfo = _ComponentInfo
DependableElementInfo = _DependableElementInfo
