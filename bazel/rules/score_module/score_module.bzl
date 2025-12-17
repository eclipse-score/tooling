load("@rules_python//sphinxdocs:sphinx.bzl", "sphinx_docs")
load("@rules_python//sphinxdocs:sphinx_docs_library.bzl", "sphinx_docs_library")
load("//bazel/rules/score_module/private:seooc.bzl", "seooc")
load("//bazel/rules/score_module/private:seooc_sphinx_environment.bzl", "seooc_sphinx_environment")

def safety_element_out_of_context(
        name,
        assumptions_of_use,
        component_requirements,
        architectural_design,
        safety_analysis,
        implementations,
        tests,
        visibility):
    """Defines a Safety Element out of Context (SEooC) following ISO 26262 standards.

    This macro creates a complete SEooC module with integrated documentation generation
    using Sphinx. It packages all required ISO 26262 artifacts and generates HTML
    documentation for safety certification.

    Args:
        name: The name of the safety element module. Used as the base name for all
            generated targets.
        assumptions_of_use: Label to a .rst or .md file containing the Assumptions of Use,
            which define the safety-relevant operating conditions and constraints for the
            SEooC as required by ISO 26262-10 clause 5.4.4.
        component_requirements: Label to a .rst or .md file containing the component
            requirements specification, defining functional and safety requirements as
            required by ISO 26262-3 clause 7.
        architectural_design: Label to a .rst or .md file containing the architectural
            design specification, describing the software architecture and design decisions
            as required by ISO 26262-6 clause 7.
        safety_analysis: Label to a .rst or .md file containing the safety analysis,
            including FMEA, FMEDA, FTA, or other safety analysis results as required by
            ISO 26262-9 clause 8. Documents hazard analysis and safety measures.
        implementations: List of labels to Bazel targets representing the actual software
            implementation (cc_library, cc_binary, etc.) that realizes the component
            requirements. This is the source code that implements the safety functions
            as required by ISO 26262-6 clause 8.
        tests: List of labels to Bazel test targets (cc_test, py_test, etc.) that verify
            the implementation against requirements. Includes unit tests and integration
            tests as required by ISO 26262-6 clause 9 for software unit verification.
        visibility: Bazel visibility specification for the generated SEooC target. Controls
            which other packages can depend on this safety element.

    Generated Targets:
        <name>_index: Sphinx environment with generated index.rst and conf.py files
        <name>_seooc_index_lib: Sphinx documentation library for the module
        <name>: Main SEooC target aggregating all documentation
        <name>.html: HTML documentation output
    """

    # Generate index file for the seooc documentation
    seooc_sphinx_environment(
        name = name + "_index",
        module_name = name,
        assumptions_of_use = assumptions_of_use,
        component_requirements = component_requirements,
        architectural_design = architectural_design,
        safety_analysis = safety_analysis,
    )

    sphinx_docs_library(
        name = name + "_seooc_index_lib",
        srcs = [name + "_index"],
        prefix = "",
        visibility = ["//visibility:public"],
        deps = [],
    )

    # Create the main SEooC target
    seooc(
        name = name,
        index = name + "_seooc_index_lib",
        assumptions_of_use = assumptions_of_use,
        component_requirements = component_requirements,
        architectural_design = architectural_design,
        safety_analysis = safety_analysis,
        visibility = visibility,
    )
