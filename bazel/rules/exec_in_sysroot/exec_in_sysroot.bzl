# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************
_TAR_TOOLCHAIN_TYPE = "@tar.bzl//tar/toolchain:type"

def _merge_default_and_data_runfiles(target, runfiles):
    default_info = target[DefaultInfo]
    if default_info.default_runfiles:
        runfiles = runfiles.merge(default_info.default_runfiles)
    if default_info.data_runfiles:
        runfiles = runfiles.merge(default_info.data_runfiles)
    return runfiles

def _extract_and_clean(tar_bin, src, dest):
    """POSIX-sh snippet: extract `src` into `dest`, then drop the symlinks that
    break Bazel TreeArtifact validation:
      * self-referential links (e.g. Debian x11-common's `usr/bin/X11 -> .`),
        which make validation recurse infinitely; and
      * now-dangling links, which validation also rejects.
    """
    return (
        "mkdir -p \"" + dest + "\"\n" +
        tar_bin + " -xf " + src + " -C \"" + dest + "\"\n" +
        "find \"" + dest + "\" -type l -lname '.' -delete\n" +
        "find \"" + dest + "\" -xtype l -delete\n"
    )

def _setup_block(sysroot_dir, host_setup_commands, sysroot_setup_commands):
    """POSIX-sh snippet running the optional post-extract setup against
    `sysroot_dir` (an unquoted shell path expression) while it is still writable.

    host_setup_commands run in the outer shell with $SYSROOT set to sysroot_dir.

    sysroot_setup_commands run inside a temporary /bin/sh script with
    LD_PRELOAD=libfakechroot.so + FAKECHROOT_BASE=sysroot_dir, so absolute-path
    accesses are transparently redirected into the sysroot.
    """
    block = ""
    if host_setup_commands:
        block += "SYSROOT=\"" + sysroot_dir + "\"\n"
        block += "\n".join(host_setup_commands) + "\n"
    if sysroot_setup_commands:
        script_lines = ["#!/bin/sh", "set -eu"] + sysroot_setup_commands
        printf_calls = "\n".join([
            "printf '%s\\n' '" + line.replace("'", "'\\''") + "' >> \"$_FC_SCRIPT\""
            for line in script_lines
        ])
        block += (
            "_FC_LIB=\"$(find \"" + sysroot_dir + "/usr/lib\" -path '*/fakechroot/libfakechroot.so' -type f 2>/dev/null | head -1 || true)\"\n" +
            "if [ -z \"$_FC_LIB\" ]; then\n" +
            "  echo \"ERROR: sysroot_setup_commands require fakechroot, but libfakechroot.so was not found under " + sysroot_dir + "/usr/lib\" >&2\n" +
            "  exit 1\n" +
            "fi\n" +
            "_FC_SCRIPT=\"$(mktemp \"${TMPDIR:-/tmp}/_prepare_sysroot_setup_XXXXXX.sh\")\"\n" +
            printf_calls + "\n" +
            "chmod +x \"$_FC_SCRIPT\"\n" +
            "LD_PRELOAD=\"$_FC_LIB\" " +
            "LD_LIBRARY_PATH=\"$(dirname \"$_FC_LIB\")${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}\" " +
            "FAKECHROOT_BASE=\"" + sysroot_dir + "\" " +
            "FAKECHROOT_EXCLUDE_PATH=\"$_FC_SCRIPT\" \"$_FC_SCRIPT\" 2>&1\n" +
            "rm -f \"$_FC_SCRIPT\"\n"
        )
    return block

def _prepare_sysroot_impl(ctx):
    if len(ctx.files.sysroot) != 1:
        fail("sysroot '{}' must provide exactly one archive file".format(ctx.attr.sysroot.label))

    sysroot_archive = ctx.files.sysroot[0]
    bsdtar = ctx.toolchains[_TAR_TOOLCHAIN_TYPE]
    out_archive = ctx.actions.declare_file(ctx.label.name + ".tar")
    work = out_archive.path + ".work"
    tar_bin = bsdtar.tarinfo.binary.path

    command = (
        "set -eu\n" +
        "rm -rf \"" + work + "\"\n" +
        _extract_and_clean(tar_bin, sysroot_archive.path, work) +
        _setup_block(work, ctx.attr.host_setup_commands, ctx.attr.sysroot_setup_commands) +
        tar_bin + " -cf " + out_archive.path + " -C \"" + work + "\" .\n" +
        "rm -rf \"" + work + "\"\n"
    )

    ctx.actions.run_shell(
        inputs = [sysroot_archive],
        outputs = [out_archive],
        tools = [bsdtar.default.files],
        command = command,
        mnemonic = "PrepareSysroot",
        progress_message = "Preparing sysroot archive %s" % ctx.label.name,
    )
    return [DefaultInfo(files = depset([out_archive]))]

prepare_sysroot = rule(
    implementation = _prepare_sysroot_impl,
    attrs = {
        "sysroot": attr.label(
            mandatory = True,
            allow_single_file = True,
            doc = "Input sysroot archive (e.g. a rules_distroless `:flat` tar).",
        ),
        "host_setup_commands": attr.string_list(
            default = [],
            doc = "Shell lines run in the outer (host) shell after extraction while " +
                  "the sysroot is still writable.  $SYSROOT is set to the sysroot " +
                  "directory.  Use this for filesystem operations that only need the " +
                  "host shell (e.g. removing unwanted plugins with find/rm).",
        ),
        "sysroot_setup_commands": attr.string_list(
            default = [],
            doc = "Shell lines run inside the sysroot via LD_PRELOAD=libfakechroot.so " +
                  "+ FAKECHROOT_BASE after host_setup_commands complete.  All absolute " +
                  "path accesses are redirected into the sysroot.  Use this for post-" +
                  "install steps that need the sysroot's own binaries (e.g. " +
                  "'/usr/bin/dot -c' to regenerate the graphviz plugin manifest).  " +
                  "Requires fakechroot to be present in the sysroot.",
        ),
    },
    toolchains = [_TAR_TOOLCHAIN_TYPE],
    doc = """
    Unpacks a sysroot archive, removes symlinks that break Bazel TreeArtifact
    validation, runs optional host/sysroot setup commands while the tree is
    writable, and repackages the result into a single `<name>.tar` archive.

    """,
)

def _exec_in_sysroot_impl(ctx):
    if len(ctx.files.sysroot) != 1:
        fail("sysroot '{}' must provide exactly one archive file".format(ctx.attr.sysroot.label))

    sysroot_archive = ctx.files.sysroot[0]
    bsdtar = ctx.toolchains[_TAR_TOOLCHAIN_TYPE]
    sysroot = ctx.actions.declare_directory(ctx.label.name + "_sysroot")

    # Extract the sysroot archive into a TreeArtifact so the wrapped executable
    # can reference it at action time via fakechroot.  Any filesystem preparation
    # (plugin pruning, post-install commands, …) should be done upfront in a
    # prepare_sysroot rule; the symlink cleanup from _extract_and_clean still
    # runs here because Bazel rejects TreeArtifacts with broken symlinks.
    ctx.actions.run_shell(
        inputs = [sysroot_archive],
        outputs = [sysroot],
        tools = [bsdtar.default.files],
        command = "set -eu\n" + _extract_and_clean(
            bsdtar.tarinfo.binary.path,
            sysroot_archive.path,
            sysroot.path,
        ),
        mnemonic = "ExecInSysrootExtract",
        progress_message = "Extracting sysroot %s" % ctx.label.name,
    )

    sysroot_short_path = sysroot.short_path
    if sysroot_short_path.startswith("../"):
        sysroot_runfiles_path = sysroot_short_path[3:]
    else:
        sysroot_runfiles_path = ctx.workspace_name + "/" + sysroot_short_path

    executable_file = ctx.executable.executable
    if executable_file == None:
        fail("executable must provide a runnable target")
    executable_short_path = executable_file.short_path
    if executable_short_path.startswith("../"):
        executable_runfiles_path = executable_short_path[3:]
    else:
        executable_runfiles_path = ctx.workspace_name + "/" + executable_short_path

    out = ctx.actions.declare_file(ctx.label.name)

    # Build exclude paths string - colon-separated list
    exclude_paths = ":".join(ctx.attr.exclude_paths) if ctx.attr.exclude_paths else ""

    wrapper_script = """#!/usr/bin/env bash
set -euo pipefail

# --- begin runfiles.bash initialization ---
if [[ ! -d "${{RUNFILES_DIR:-/dev/null}}" && ! -f "${{RUNFILES_MANIFEST_FILE:-/dev/null}}" ]]; then
  if [[ -f "$0.runfiles_manifest" ]]; then
    export RUNFILES_MANIFEST_FILE="$0.runfiles_manifest"
  elif [[ -f "$0.runfiles/MANIFEST" ]]; then
    export RUNFILES_MANIFEST_FILE="$0.runfiles/MANIFEST"
  elif [[ -f "$0.runfiles/bazel_tools/tools/bash/runfiles/runfiles.bash" ]]; then
    export RUNFILES_DIR="$0.runfiles"
  fi
fi
if [[ -f "${{RUNFILES_DIR:-/dev/null}}/bazel_tools/tools/bash/runfiles/runfiles.bash" ]]; then
  source "${{RUNFILES_DIR}}/bazel_tools/tools/bash/runfiles/runfiles.bash"
elif [[ -f "${{RUNFILES_MANIFEST_FILE:-/dev/null}}" ]]; then
  source "$(grep -m1 '^bazel_tools/tools/bash/runfiles/runfiles.bash ' "$RUNFILES_MANIFEST_FILE" | cut -d ' ' -f 2-)"
else
  echo >&2 "ERROR: cannot find @bazel_tools//tools/bash/runfiles:runfiles.bash"
  exit 1
fi
# --- end runfiles.bash initialization ---

FAKECHROOT_WRAPPER="$(rlocation '{wrapper_short_path}')"
SYSROOT_DIR="$(rlocation '{sysroot_short_path}')"
EXECUTABLE_FILE="$(rlocation '{executable_runfiles_path}')"

if [[ -z "${{FAKECHROOT_WRAPPER}}" || ! -x "${{FAKECHROOT_WRAPPER}}" ]]; then
  echo "ERROR: could not resolve fakechroot wrapper: {wrapper_short_path}" >&2
  exit 1
fi

if [[ -z "${{SYSROOT_DIR}}" || ! -d "${{SYSROOT_DIR}}" ]]; then
  echo "ERROR: could not resolve sysroot directory: {sysroot_short_path}" >&2
  exit 1
fi

if [[ ! -x "${{SYSROOT_DIR}}/usr/bin/fakechroot" ]]; then
  echo "ERROR: sysroot does not provide /usr/bin/fakechroot: ${{SYSROOT_DIR}}" >&2
  exit 1
fi

if [[ -z "${{EXECUTABLE_FILE}}" || ! -f "${{EXECUTABLE_FILE}}" ]]; then
  echo "ERROR: could not resolve executable target: {executable_runfiles_path}" >&2
  exit 1
fi

export SYSROOT_DIR
if [[ -n "{exclude_paths}" ]]; then
  export FAKECHROOT_EXCLUDE_PATH="{exclude_paths}"
fi

# The executable lives in host runfiles, not in the sysroot. Exclude its path
# so fakechroot does not redirect accesses to it into the sysroot.
EXECUTABLE_DIR="$(dirname "${{EXECUTABLE_FILE}}")"
if [[ -n "${{FAKECHROOT_EXCLUDE_PATH:-}}" ]]; then
  export FAKECHROOT_EXCLUDE_PATH="${{EXECUTABLE_DIR}}:${{EXECUTABLE_FILE}}:${{FAKECHROOT_EXCLUDE_PATH}}"
else
  export FAKECHROOT_EXCLUDE_PATH="${{EXECUTABLE_DIR}}:${{EXECUTABLE_FILE}}"
fi

exec "${{FAKECHROOT_WRAPPER}}" "${{EXECUTABLE_FILE}}" "$@"
""".format(
        wrapper_short_path = ctx.workspace_name + "/" + ctx.executable._fakechroot_wrapper.short_path,
        sysroot_short_path = sysroot_runfiles_path,
        executable_runfiles_path = executable_runfiles_path,
        exclude_paths = exclude_paths,
    )
    ctx.actions.write(output = out, content = wrapper_script, is_executable = True)

    runfiles = ctx.runfiles(
        files = [out, ctx.executable._fakechroot_wrapper, sysroot, executable_file] + ctx.files._bash_runfiles,
    )
    runfiles = _merge_default_and_data_runfiles(ctx.attr.executable, runfiles)
    runfiles = _merge_default_and_data_runfiles(ctx.attr._fakechroot_wrapper, runfiles)
    runfiles = _merge_default_and_data_runfiles(ctx.attr._bash_runfiles, runfiles)
    runfiles = _merge_default_and_data_runfiles(ctx.attr.sysroot, runfiles)

    return [DefaultInfo(
        executable = out,
        files = depset([out]),
        runfiles = runfiles,
    )]

exec_in_sysroot = rule(
    implementation = _exec_in_sysroot_impl,
    executable = True,
    attrs = {
        "executable": attr.label(mandatory = True, executable = True, cfg = "exec"),
        "sysroot": attr.label(mandatory = True, allow_single_file = True),
        "exclude_paths": attr.string_list(
            default = [],
            doc = "Paths to exclude from fakechroot path-redirection (colon-separated).",
        ),
        "_bash_runfiles": attr.label(
            default = Label("@bazel_tools//tools/bash/runfiles"),
            allow_files = True,
        ),
        "_fakechroot_wrapper": attr.label(
            default = Label("//bazel/rules/exec_in_sysroot"),
            executable = True,
            cfg = "exec",
        ),
    },
    toolchains = [_TAR_TOOLCHAIN_TYPE],
    doc = """
    Produces an executable wrapper that runs a given executable target using the
    supplied sysroot archive.  The archive is unpacked in-rule and the wrapped
    executable runs within fakechroot via LD_PRELOAD, allowing access to sysroot
    tools and libraries hermetically.

    The archive is expected to be a reworked sysroot (see prepare_sysroot), which
    performs plugin pruning / post-install setup once and caches the result.
    """,
)
