def my_rule():
  load(":my_rule.bzl", "my_rule")

  my_rule(
        name="my_target",
            srcs = ["some_file.txt"],
    visibility = ["//visibility:public"],
)
