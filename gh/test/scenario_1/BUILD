load("//tools:gh/defs.bzl", "generate_github_library")

# Note: so far this is only an idea.
# Written from a user perspective, not a bazel/realistic perspective.

generate_github_library(
    name = "generated_github_client",
    queries = glob("*.graphql"),
)

py_binary(
    name = "main",
    srcs = ["main.py"],
    deps = [":generated_github_client"],
)