SphinxInfo = provider(
    doc = "Provider for Sphinx Toolchain",
    fields = {
        "sphinx": "sphinx executable",
        "conf_template": "template for conf.py",
        "html_merge_tool": "tool to merge html files",
    },
)

def _sphinx_toolchain_impl(ctx):
    toolchain_info = platform_common.ToolchainInfo(
        sphinxinfo = SphinxInfo(
            sphinx = ctx.attr.sphinx,
            conf_template = ctx.attr.conf_template,
            html_merge_tool = ctx.attr.html_merge_tool,
        ),
    )
    return [toolchain_info]

sphinx_toolchain = rule(
    implementation = _sphinx_toolchain_impl,
    attrs = {
        "sphinx": attr.label(
            default = Label("//bazel/rules/rules_score:raw_build"),
        ),
        "conf_template": attr.label(
            allow_single_file = True,
            default = Label("//bazel/rules/rules_score:templates/conf.template.py"),
        ),
        "html_merge_tool": attr.label(
            default = Label("//bazel/rules/rules_score:sphinx_html_merge"),
        ),
    },
)
