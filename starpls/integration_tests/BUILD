load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@pip_score_starpls//:requirements.bzl", "all_requirements")




compile_pip_requirements(
    name = "requirements",
    src = "requirements.txt",
    requirements_txt = "requirements_lock.txt",
    tags = [
        "manual",
    ],
)


py_test(
    name = "score_strpls_tests",
    srcs = ["starpls_test.py"],
    main = "starpls_test.py",
    deps = all_requirements,
    data = ["@score_starpls//:starpls"],
    visibility = ["//visibility:public"],

)

