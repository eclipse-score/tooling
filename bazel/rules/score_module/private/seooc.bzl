load("@rules_python//sphinxdocs/private:sphinx_docs_library_info.bzl", "SphinxDocsLibraryInfo")

seooc_artifacts = {
    "assumptions_of_use": attr.label(
        providers = [SphinxDocsLibraryInfo],
        mandatory = True,
        doc = "Label to a sphinx_docs_library target containing the Assumptions of Use, which define the safety-relevant operating conditions and constraints for the SEooC as required by ISO 26262-10 clause 5.4.4.",
    ),
    "component_requirements": attr.label(
        providers = [SphinxDocsLibraryInfo],
        mandatory = True,
        doc = "Label to a sphinx_docs_library target containing the component requirements specification, defining functional and safety requirements as required by ISO 26262-3 clause 7.",
    ),
    "architectural_design": attr.label(
        providers = [SphinxDocsLibraryInfo],
        mandatory = True,
        doc = "Label to a sphinx_docs_library target containing the architectural design specification, describing the software architecture and design decisions as required by ISO 26262-6 clause 7.",
    ),
    "safety_analysis": attr.label(
        providers = [SphinxDocsLibraryInfo],
        mandatory = True,
        doc = "Label to a sphinx_docs_library target containing the safety analysis, including FMEA, FMEDA, FTA, or other safety analysis results as required by ISO 26262-9 clause 8. Documents hazard analysis and safety measures.",
    ),
}

seooc_targets = {
    "implementations": attr.label(
        mandatory = False,
        doc = "",
    ),
    "tests": attr.label(
        mandatory = False,
        doc = "",
    ),
}

def _seooc_build_impl(ctx):
    """Implementation of safety_element build rule for ISO 26262 SEooC."""

    all_files = []
    for artifact in seooc_artifacts:
        dep = getattr(ctx.attr, artifact)
        for t in dep[SphinxDocsLibraryInfo].transitive.to_list():
            entry = struct(
                strip_prefix = t.strip_prefix,
                prefix = "docs/safety_elements/" + ctx.attr.name + "/" + t.prefix,
                files = t.files,
            )
            all_files.append(entry)

    index = ctx.attr.index
    for t in index[SphinxDocsLibraryInfo].transitive.to_list():
        entry = struct(
            strip_prefix = t.strip_prefix,
            prefix = "",
            files = t.files,
        )
        all_files.append(entry)

    result = depset(all_files)
    return [
        DefaultInfo(
            files = depset([]),
        ),
        SphinxDocsLibraryInfo(
            strip_prefix = "",
            prefix = "",
            files = [],
            transitive = result,
        ),
    ]

seooc = rule(
    implementation = _seooc_build_impl,
    attrs = seooc_artifacts | {
        "index": attr.label(
            allow_files = [".rst", ".md", ".py"],
            mandatory = True,
            doc = "",
        ),
    },
)
