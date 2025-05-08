load("@rules_python//python:defs.bzl", "py_library")

def _generate_github_library_impl(ctx):
    output_dir = ctx.actions.declare_directory("generated")

    generated_client = ctx.actions.declare_file("generated/client.py")
    generated_models = ctx.actions.declare_file("generated/models.py")
    generated_init = ctx.actions.declare_file("generated/__init__.py")

    ctx.actions.run(
        outputs = [generated_client, generated_models, generated_init],
        inputs = ctx.files.queries,
        arguments = [
            "--schema-path", ctx.executable._schema.path,
            "--queries-path", ctx.attr.queries_dir,
            "--output-path", output_dir.path,
            "--package-name", ctx.attr.package,
            "--client-name", ctx.attr.client_name,
            "--target-python-version", "3.12",
            "--async-client",
            "--plugins", "ariadne_codegen.plugins.pydantic",
        ],
        executable = ctx.executable._codegen,
        tools = [ctx.executable._schema],
        mnemonic = "AriadneCodegen",
        progress_message = "Generating GraphQL client for GitHub API",
    )

    pyinfo = ctx.attr._py_runtime[PyInfo]

    return [
        DefaultInfo(files = depset([generated_client, generated_models, generated_init])),
        PyInfo(
            transitive_sources = depset([generated_client, generated_models, generated_init]),
            uses_shared_libraries = False,
        ),
    ]

generate_github_library = rule(
    implementation = _generate_github_library_impl,
    attrs = {
        "queries": attr.label_list(allow_files = [".graphql"]),
        "queries_dir": attr.string(),
        "package": attr.string(default = "generated_github_client"),
        "client_name": attr.string(default = "Client"),
        "_codegen": attr.label(
            allow_single_file = True,
            executable = True,
            cfg = "exec",
            default = Label("@pip_deps//ariadne_codegen:ariadne-codegen"),
        ),
        "_schema": attr.label(
            allow_single_file = True,
            executable = True,
            cfg = "exec",
            default = Label("@github_schema//file:schema.graphql"),
        ),
        "_py_runtime": attr.label(default = Label("@rules_python//python:current_py_runtime")),
    },
    provides = [PyInfo],
    doc = "Generates a typed async GraphQL client for GitHub API using ariadne-codegen and exposes it as a py_library-compatible rule.",
)
