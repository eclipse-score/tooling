# load("//tools:ruff.bzl", "ruff_binary")
# load("//score_venv/py_pyvenv.bzl", "score_virtualenv")

load("@aspect_rules_py//py:defs.bzl", "py_binary")
load("@aspect_rules_lint//lint:lint_test.bzl", "lint_test")
load("@aspect_rules_lint//lint:ruff.bzl", "lint_ruff_aspect")



def score_python(ruff_name="ruff", ruff_format=True, ruff_args=[]):

    _ruff_exec(name=ruff_name, format=ruff_format, ruff_args=ruff_args)


def _ruff_exec(
    name="ruff",
    format=True, 
    ruff_args=[],
):
    args = [str(format)] 
    args += ruff_args
    args.append("--config=$(location %s)" % "//:conf")
    py_binary(
        name=name,
        srcs=["//tools:ruff_script.py"],
        args=args,
        data=["@multitool//tools/ruff", "//:conf"]
    )
# def _ruff_binary(
#     name = "ruff",
#     config = ":pyproject.toml",
#     format = True,
# ):  
#     # Create the bash script instead of having it extra
#     native.genrule(
#         name = name + "_wrapper_script",
#         outs = [name + "_wrapper.sh"],
#         cmd = """cat > $@ << 'EOF'
# #!/bin/bash
# RUFF_BIN="$$1"
# shift 1
#
# # Run ruff with any additional arguments
# exec "$$RUFF_BIN" "$$@"
# EOF
# chmod +x $@
# """,
#     )
#
#     data_deps = ["@ruff//:ruff"]
#     args = ["$(location @ruff//:ruff)"]
#
#     data_deps.append(config)
#     if format:
#         args.extend(["format"])
#     else:
#         args.extend(["check"])
#     args.extend(["--config", "$(location {})".format(config)])
#
#     native.sh_binary(
#         name = name,
#         srcs = [":{}_wrapper_script".format(name)],
#         data = data_deps,
#         args = args,
#         visibility = ["//visibility:public"]
#     )
#
