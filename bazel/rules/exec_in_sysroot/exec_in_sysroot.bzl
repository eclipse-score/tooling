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
        "\"" + tar_bin + "\" -xf \"" + src + "\" -C \"" + dest + "\"\n" +
        "find \"" + dest + "\" -type l -lname '.' -delete\n" +
        "find \"" + dest + "\" -xtype l -delete\n"
    )

def _setup_block(sysroot_dir, exec_wrapper, host_setup_commands, sysroot_setup_commands):
    """POSIX-sh snippet running the optional post-extract setup against
    `sysroot_dir` (an unquoted shell path expression) while it is still writable.

    host_setup_commands run in the outer shell with $SYSROOT set to sysroot_dir.
    Use them for pure filesystem edits that only need the host shell.

    sysroot_setup_commands are each executed inside the sysroot by delegating to
    the shared `exec_in_sysroot.sh` launcher (`exec_wrapper`) with SYSROOT_DIR
    set to sysroot_dir.  This is the *same* launcher exec_in_sysroot uses at
    runtime, so build-time setup and runtime execution share a single
    implementation of "run a binary inside the sysroot" (the sysroot's own
    ld-linux.so + --library-path + LD_PRELOAD=libfakechroot.so + FAKECHROOT_BASE,
    giving a fully consistent single-libc environment).

    Each entry in sysroot_setup_commands must be a space-separated ELF binary
    invocation starting with an absolute sysroot path, e.g. "/usr/bin/tool --flag".
    Shell metacharacters (pipes, redirects, etc.) are not supported.
    """
    block = ""
    if host_setup_commands:
        block += "SYSROOT=\"" + sysroot_dir + "\"\n"
        block += "\n".join(host_setup_commands) + "\n"
    for cmd in sysroot_setup_commands:
        # Delegate to the shared launcher.  `cmd` is intentionally left unquoted
        # so the shell word-splits "<binary> <args...>" into the launcher's
        # positional parameters (the space-separated contract documented above).
        block += (
            "SYSROOT_DIR=\"" + sysroot_dir + "\" " +
            "sh \"" + exec_wrapper + "\" " + cmd + "\n"
        )
    return block

def _prepare_sysroot_impl(ctx):
    if len(ctx.files.sysroot) != 1:
        fail("sysroot '{}' must provide exactly one archive file".format(ctx.attr.sysroot.label))

    sysroot_archive = ctx.files.sysroot[0]
    bsdtar = ctx.toolchains[_TAR_TOOLCHAIN_TYPE]
    out_archive = ctx.actions.declare_file(ctx.label.name + ".tar")
    tar_bin = bsdtar.tarinfo.binary.path

    command = (
        "set -eu\n" +
        "work=\"$(mktemp -d)\"\n" +
        "trap 'rm -rf \"$work\"' EXIT\n" +
        _extract_and_clean(tar_bin, sysroot_archive.path, "$work") +
        _setup_block("$work", ctx.file._exec_wrapper.path, ctx.attr.host_setup_commands, ctx.attr.sysroot_setup_commands) +
        "\"" + tar_bin + "\" -cf \"" + out_archive.path + "\" -C \"$work\" .\n"
    )

    ctx.actions.run_shell(
        inputs = [sysroot_archive, ctx.file._exec_wrapper],
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
                  "host shell (e.g. removing files or creating symlinks with standard " +
                  "shell tools such as find/rm/ln).",
        ),
        "sysroot_setup_commands": attr.string_list(
            default = [],
            doc = "ELF binary invocations run inside the sysroot after " +
                  "host_setup_commands complete.  Each entry is word-split by the " +
                  "shell to separate the binary path from its arguments; entries must " +
                  "not contain arguments with embedded spaces, and shell metacharacters " +
                  "(pipes, redirects) are not supported.  The binary path must be " +
                  "absolute within the sysroot (e.g. '/usr/bin/tool --flag').  The binary " +
                  "is executed via the sysroot's own ld-linux.so with --library-path " +
                  "pointing at the sysroot's /usr/lib tree, giving a fully consistent " +
                  "single-libc environment.  LD_PRELOAD=libfakechroot + FAKECHROOT_BASE " +
                  "are still active so absolute filesystem calls are transparently " +
                  "redirected into the sysroot.  Requires fakechroot and " +
                  "ld-linux.so to be present in the sysroot.  NOTE: sysroot binaries are " +
                  "executed via the sysroot's ELF interpreter on the host kernel, so the " +
                  "sysroot architecture must match the host architecture.",
        ),
        "_exec_wrapper": attr.label(
            default = Label("//bazel/rules/exec_in_sysroot:exec_in_sysroot.sh"),
            allow_single_file = True,
            doc = "Shared sysroot launcher reused to run sysroot_setup_commands, " +
                  "so build-time setup and runtime execution use one implementation.",
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

    out = ctx.actions.declare_file(ctx.label.name)

    # Build exclude paths string - colon-separated list
    exclude_paths = ":".join(ctx.attr.exclude_paths) if ctx.attr.exclude_paths else ""

    ctx.actions.expand_template(
        template = ctx.file._launcher_template,
        output = out,
        is_executable = True,
        substitutions = {
            "{{WRAPPER_SHORT_PATH}}": ctx.workspace_name + "/" + ctx.executable._fakechroot_wrapper.short_path,
            "{{SYSROOT_SHORT_PATH}}": sysroot_runfiles_path,
            "{{SYSROOT_BINARY}}": ctx.attr.sysroot_binary,
            "{{EXCLUDE_PATHS}}": exclude_paths,
        },
    )

    runfiles = ctx.runfiles(
        files = [ctx.executable._fakechroot_wrapper, sysroot],
    )
    runfiles = _merge_default_and_data_runfiles(ctx.attr._fakechroot_wrapper, runfiles)
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
        "sysroot_binary": attr.string(
            mandatory = True,
            doc = "Sysroot-relative path of the ELF binary to execute, e.g. '/usr/bin/tool'.",
        ),
        "sysroot": attr.label(
            mandatory = True,
            allow_single_file = True,
            doc = "Prepared sysroot archive, typically the output of a prepare_sysroot rule.",
        ),
        "exclude_paths": attr.string_list(
            default = [],
            doc = "List of paths to exclude from fakechroot path-redirection.",
        ),
        "_fakechroot_wrapper": attr.label(
            default = Label("//bazel/rules/exec_in_sysroot"),
            executable = True,
            cfg = "exec",
            doc = "Shared sysroot launcher (exec_in_sysroot.sh) invoked at runtime.",
        ),
        "_launcher_template": attr.label(
            default = Label("//bazel/rules/exec_in_sysroot:exec_in_sysroot_launcher.sh.tpl"),
            allow_single_file = True,
        ),
    },
    toolchains = [_TAR_TOOLCHAIN_TYPE],
    doc = """
    Produces an executable wrapper that runs a sysroot ELF binary via the
    sysroot's own ld-linux.so (SYSROOT_INTERP) and fakechroot (LD_PRELOAD),
    giving the binary a hermetic single-libc environment backed by the sysroot.

    The sysroot archive is expected to be prepared by a prepare_sysroot rule,
    which performs filesystem preparation / post-install setup once and caches the result.
    """,
)
