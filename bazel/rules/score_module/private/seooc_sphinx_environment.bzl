load("@rules_python//sphinxdocs/private:sphinx_docs_library_info.bzl", "SphinxDocsLibraryInfo")
load("//bazel/rules/score_module/private:seooc.bzl", "seooc_artifacts")

index_content = """

.. toctree::
   :maxdepth: 2
   :caption: Contents:

"""

def _seooc_sphinx_environment_impl(ctx):
    """Generate the index.rst file for a seooc"""

    index_rst = ctx.actions.declare_file("docs/safety_elements/" + ctx.attr.module_name + "/index.rst")

    header = ctx.attr.module_name.upper()
    header += "\n" + "=" * len(header)

    file_content = header + index_content

    for artifact in seooc_artifacts:
        attr = getattr(ctx.attr, artifact)
        if attr:
            # Get all files from the SphinxDocsLibraryInfo
            src_files = list(attr[SphinxDocsLibraryInfo].files)
            if src_files:
                # Use the first file (typically the main documentation file)
                artifact_index_file = src_files[0]

                # Create link path from the file
                link = artifact_index_file.short_path.replace(".rst", "").replace(".md", "")
                if ctx.label.package:
                    print("replacing link: " + ctx.label.package + "/")
                    link = link.replace(ctx.label.package + "/", "")
                file_content += "   " + link + "\n"

    ctx.actions.write(
        output = index_rst,
        content = file_content,
    )

    return (
        DefaultInfo(
            files = depset([index_rst]),
        )
    )

seooc_sphinx_environment = rule(
    implementation = _seooc_sphinx_environment_impl,
    attrs = seooc_artifacts | {
        "module_name": attr.string(),
    },
)
